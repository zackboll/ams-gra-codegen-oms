with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with GNAT.Directory_Operations;
with Programs.Oam;
with Service_API;
with Service_Codec;
with UCI_Client.Build_Config;
with UCI_Client.Policy;
with OMS_LACAL; use OMS_LACAL;
with OMS_LACAL.Connections;
with OMS_LACAL.Dispatchers;
with OMS_LACAL.Transports.WebSocket;
with OMS_LACAL.Typed;

procedure Ada_UCI_Client is
   use type Ada.Real_Time.Time;
   use type UCI_Client.Policy.Outcome;
   package P renames UCI_Client.Policy;
   package C renames OMS_LACAL.Connections;
   package D renames OMS_LACAL.Dispatchers;
   package WS renames OMS_LACAL.Transports.WebSocket;
   package M renames Programs.Oam;
   package LI renames Service_API.Function_Log.Exchange_Receive;
   package RI renames Service_API.Function_Rdma.Exchange_Receive;
   package SI renames Service_API.Function_Survivability.Exchange_Receive;
   Schema : constant String := UCI_Client.Build_Config.Schema;
   Config : P.Options;
   CA_PEM : Unbounded_String;
   Deadline : Ada.Real_Time.Time;
   Stop_Requested : exception;
   Session_Lost : exception;
   Application_Error : exception;

   function Image (N : Natural) return String is
     (Ada.Strings.Fixed.Trim (N'Image, Ada.Strings.Both));
   procedure Say (Text : String) is
   begin
      Ada.Text_IO.Put_Line (Text);
      Ada.Text_IO.Flush;
   end Say;
   function Entries (Path : String) return Natural is
      use GNAT.Directory_Operations;
      Dir : Dir_Type;
      Name : String (1 .. 1024);
      Last, Count : Natural := 0;
   begin
      Open (Dir, Path);
      loop
         Read (Dir, Name, Last);
         exit when Last = 0;
         if Name (1 .. Last) /= "." and then Name (1 .. Last) /= ".." then
            Count := Count + 1;
         end if;
      end loop;
      Close (Dir);
      return Count;
   end Entries;
   procedure Require (OK : Boolean; Detail : String) is
   begin
      if not OK then raise Application_Error with Detail; end if;
   end Require;
   procedure Check_Time is
   begin
      if Ada.Real_Time.Clock >= Deadline then raise Stop_Requested; end if;
   end Check_Time;

   --  The entire borrowed object graph is local to this master. No SID,
   --  receiver access, binding, Delivery or transport survives its return.
   procedure Epoch (Number : Positive; Result : out P.Outcome;
                    Established : out Boolean) is
      Wire : aliased WS.Client;
      Core : aliased C.Connection (Wire'Access);
      function Is_Codec (E : Ada.Exceptions.Exception_Occurrence) return Boolean is
         use type Ada.Exceptions.Exception_Id;
      begin
         return Ada.Exceptions.Exception_Identity (E) = Service_Codec.Codec_Error'Identity;
      end Is_Codec;
      package L is new OMS_LACAL.Typed
        (M.LogMT, LI.Handler, LI.Handle,
         Service_Codec.Encode_LogMT, Service_Codec.Decode_LogMT, Is_Codec);
      package R is new OMS_LACAL.Typed
        (M.RDMA_InitializeSetupMT, RI.Handler, RI.Handle,
         Service_Codec.Encode_RDMA_InitializeSetupMT,
         Service_Codec.Decode_RDMA_InitializeSetupMT, Is_Codec);
      package S is new OMS_LACAL.Typed
        (M.SurvivabilityRiskLevelMT, SI.Handler, SI.Handle,
         Service_Codec.Encode_SurvivabilityRiskLevelMT,
         Service_Codec.Decode_SurvivabilityRiskLevelMT, Is_Codec);
      Retired, Fatal : Boolean := False;
      Init_Count, Ack_Count, Close_Count : Natural := 0;
      SIDs : array (1 .. 3) of Subscription_ID := (others => No_Subscription);
      type Log_Receiver is new LI.Handler with null record;
      type Rdma_Receiver is new RI.Handler with null record;
      type Risk_Receiver is new SI.Handler with null record;
      overriding procedure Handle (Self : in out Log_Receiver; Message : LI.Payload);
      overriding procedure Handle (Self : in out Rdma_Receiver; Message : RI.Payload);
      overriding procedure Handle (Self : in out Risk_Receiver; Message : SI.Payload);

      procedure Header (Value : M.HeaderType) is
      begin
         Require (not Retired and then C.State (Core) = Active, "retired callback");
         Require (M.Value (Value.SchemaVersion) = Schema, "message schema mismatch");
         --  These accessors expose checked generated representations, not JSON.
         Require (M.Value (Value.SystemID.UUID)'Length = 36 and then
                  M.Value (Value.Timestamp)'Length > 0, "invalid checked header");
      end Header;
      overriding procedure Handle (Self : in out Log_Receiver; Message : LI.Payload) is
         pragma Unreferenced (Self);
      begin
         Header (Message.MessageHeader);
         Say ("RECEIVED epoch=" & Image (Number) & " Log timestamp=" &
              M.Value (Message.MessageHeader.Timestamp) & " severity=" &
              Message.MessageData.Severity'Image & " data_present=" &
              Message.MessageData.Data.Is_Present'Image);
      end Handle;
      overriding procedure Handle (Self : in out Rdma_Receiver; Message : RI.Payload) is
         pragma Unreferenced (Self);
      begin
         Header (Message.MessageHeader);
         Say ("RECEIVED epoch=" & Image (Number) & " RDMA_InitializeSetup timestamp=" &
              M.Value (Message.MessageHeader.Timestamp) & " setup=" &
              M.Value (Message.MessageData.RDMA_InitializeSetupID.UUID) & " provider=" &
              Message.MessageData.RDMA_Provider'Image & " endpoints=" &
              Image (M.Length (Message.MessageData.Endpoint)));
      end Handle;
      overriding procedure Handle (Self : in out Risk_Receiver; Message : SI.Payload) is
         pragma Unreferenced (Self);
      begin
         Header (Message.MessageHeader);
         Say ("RECEIVED epoch=" & Image (Number) & " SurvivabilityRiskLevel timestamp=" &
              M.Value (Message.MessageHeader.Timestamp) & " risk=" &
              M.Value (Message.MessageData.SurvivabilityRiskLevelID.UUID) & " settings=" &
              Image (M.Length (Message.MessageData.RiskSetting)));
      end Handle;
      LH : aliased Log_Receiver;
      RH : aliased Rdma_Receiver;
      SH : aliased Risk_Receiver;
      LB : aliased L.Binding (Core'Access);
      RB : aliased R.Binding (Core'Access);
      SB : aliased S.Binding (Core'Access);
      Bindings : D.Binding_List := (1 => (Value => LB'Access),
                                    2 => (Value => RB'Access),
                                    3 => (Value => SB'Access));
      function Sub_Log (Namespace, Name, Topic : String; Has_Group : Boolean; Group_Name : String;
                        Receiver : not null access LI.Handler'Class) return Subscription_ID is
        (L.Subscribe (LB, Namespace, Name, Topic, Has_Group, Group_Name, Receiver));
      function Sub_Rdma (Namespace, Name, Topic : String; Has_Group : Boolean; Group_Name : String;
                         Receiver : not null access RI.Handler'Class) return Subscription_ID is
        (R.Subscribe (RB, Namespace, Name, Topic, Has_Group, Group_Name, Receiver));
      function Sub_Risk (Namespace, Name, Topic : String; Has_Group : Boolean; Group_Name : String;
                         Receiver : not null access SI.Handler'Class) return Subscription_ID is
        (S.Subscribe (SB, Namespace, Name, Topic, Has_Group, Group_Name, Receiver));
      package LS is new LI.Subscriber (Subscription_ID, Sub_Log);
      package RS is new RI.Subscriber (Subscription_ID, Sub_Rdma);
      package SS is new SI.Subscriber (Subscription_ID, Sub_Risk);

      procedure Drain is
         Available : Boolean;
         E : Event;
      begin
         loop
            C.Next_Event (Core, Available, E);
            exit when not Available;
            case E.Kind is
               when Initialized => Init_Count := Init_Count + 1;
               when Information => null; -- runtime has validated INFO
               when Command_Accepted => Ack_Count := Ack_Count + 1;
               when Connection_Closed => Close_Count := Close_Count + 1;
               when others =>
                  Fatal := True;
                  --  Do not expose arbitrary broker/decoder/handler payload text.
                  Say ("ERROR epoch=" & Image (Number) & " event=" & E.Kind'Image);
            end case;
         end loop;
         if C.Lost_Events (Core) /= 0 then
            Fatal := True;
            Say ("ERROR epoch=" & Image (Number) & " lost_events");
         end if;
      end Drain;
      procedure Step is
      begin
         Check_Time;
         D.Poll_And_Dispatch (Core'Access, Bindings);
         Drain;
         Require (not Fatal, "terminal runtime event");
         if C.State (Core) = Closed then raise Session_Lost; end if;
      end Step;
      procedure Await_Command is
         Limit : constant Ada.Real_Time.Time :=
           P.Wake_Time (Ada.Real_Time.Clock, Deadline, 5000);
      begin
         while C.Busy (Core) loop
            Step;
            Check_Time;
            Require (Ada.Real_Time.Clock < Limit, "OWP acknowledgment timeout");
            delay 0.001;
         end loop;
      end Await_Command;
      procedure Retire is
      begin
         Retired := True;
         L.Close (LB); R.Close (RB); S.Close (SB);
         C.Close (Core);
         Drain;
         Require (not Fatal and then Close_Count = 1, "terminal cleanup events");
      end Retire;
   begin
      Result := P.Terminal_Failure;
      Established := False;
      begin
         Check_Time;
         Say ("CONNECTING epoch=" & Image (Number) & " schema=" & Schema);
          WS.Set_Absolute_Deadline (Wire, Deadline);
         WS.Open (Wire, To_String (Config.URL), CA_Path => To_String (Config.CA_Path),
                  CA_PEM => To_String (CA_PEM));
         Check_Time;
         Require (WS.Protocol (Wire) = "owp", "missing owp negotiation");
         C.Initialize (Core, Schema, Service_API.Service_Name, True);
         Await_Command;
         declare Limit : constant Ada.Real_Time.Time :=
           P.Wake_Time (Ada.Real_Time.Clock, Deadline, 5000);
         begin
            while C.State (Core) /= Active loop
               Step;
               Check_Time;
               Require (Ada.Real_Time.Clock < Limit, "OWP INFO timeout");
               delay 0.001;
            end loop;
         end;
         Require (Init_Count = 1 and then Ack_Count = 1, "INIT acknowledgment and INFO");
         Say ("CONNECTED epoch=" & Image (Number) & " schema=" & Schema);
         Check_Time;
         SIDs (1) := LS.Subscribe (LH'Access); Await_Command;
         Say ("SUBSCRIBED epoch=" & Image (Number) & " Log sid=1");
         Check_Time;
         SIDs (2) := RS.Subscribe (RH'Access); Await_Command;
         Say ("SUBSCRIBED epoch=" & Image (Number) & " RDMA_InitializeSetup sid=2");
         Check_Time;
         SIDs (3) := SS.Subscribe (SH'Access); Await_Command;
         Require (SIDs (1) = 1 and then SIDs (2) = 2 and then SIDs (3) = 3 and then
                  Ack_Count = 4 and then C.Is_Active (Core, SIDs (3)), "fresh subscription state");
         Say ("SUBSCRIBED epoch=" & Image (Number) & " SurvivabilityRiskLevel sid=3");
         Established := True;
         loop Step; delay 0.001; end loop;
      exception
         when Stop_Requested =>
            --  Expiry is successful only while this epoch is fully usable.
            --  Neither initial nor replacement setup may masquerade as success.
            Result := P.Duration_Result (Established);
         when Session_Lost =>
            Result := P.Peer_Loss;
            Say ("LOST epoch=" & Image (Number));
         when E : others =>
            --  Transport_Error is deliberately NOT classified by backend text:
            --  it also represents invalid WS frames and unexpected exceptions.
            Say ("ERROR epoch=" & Image (Number) & " exception=" &
                 Ada.Exceptions.Exception_Name (E));
            Result := P.Terminal_Failure;
      end;
      Retire;
      Say ("RETIRED epoch=" & Image (Number) & " lost=0");
      --  Reverse declaration order finalizes bindings before receivers, Core,
      --  and Wire. The supervisor runs only after this master is finalized.
   exception
      when E : others =>
         Result := P.Terminal_Failure;
         Say ("ERROR epoch=" & Image (Number) & " cleanup=" &
              Ada.Exceptions.Exception_Name (E));
   end Epoch;
begin
   declare
      Args : P.Arguments (1 .. Ada.Command_Line.Argument_Count);
   begin
      for I in Args'Range loop Args (I) := To_Unbounded_String (Ada.Command_Line.Argument (I)); end loop;
      Config := P.Parse (Args);
   end;
   if Config.Help then
      Say ("ada-uci-client (compiled schema " & Schema & ")");
      Say ("  --url wss://HOST:PORT/PATH (required; explicit port)");
      Say ("  --ca-path PATH | --ca-pem-file FILE (exactly one required)");
      Say ("  --duration-seconds 1..3600 (default 30)");
      Say ("  --reconnect-attempts 0..8 (default 0)");
      Say ("  --reconnect-delay-ms 50..2000 (default 250)");
      Say ("  --help");
      return;
   end if;
   if Length (Config.CA_PEM_File) /= 0 then
      declare
         File : Ada.Text_IO.File_Type;
         Ch : Character;
      begin
         Ada.Text_IO.Open (File, Ada.Text_IO.In_File, To_String (Config.CA_PEM_File));
         while not Ada.Text_IO.End_Of_File (File) loop
            if Ada.Text_IO.End_Of_Line (File) then
               Ada.Text_IO.Skip_Line (File);
               Append (CA_PEM, ASCII.LF);
            else
               Ada.Text_IO.Get (File, Ch);
               Append (CA_PEM, Ch);
            end if;
            Require (Length (CA_PEM) <= 1_048_576, "CA PEM file too large");
         end loop;
         Ada.Text_IO.Close (File);
         Require (Length (CA_PEM) /= 0, "empty CA PEM file");
      end;
   end if;
   Deadline := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (Config.Duration_Seconds);
   declare
      Remaining : Natural := Config.Reconnect_Attempts;
      Number : Positive := 1;
      Result : P.Outcome;
      Established : Boolean;
   begin
      loop
         Epoch (Number, Result, Established);
         Say ("FINALIZED epoch=" & Image (Number));
         Say ("RESOURCES epoch=" & Image (Number) & " fd=" &
              Image (Entries ("/proc/self/fd")) & " tasks=" &
              Image (Entries ("/proc/self/task")));
         if Result = P.Completed then
            Say ("COMPLETE epochs=" & Image (Number));
            exit;
         end if;
         if not P.Take_Attempt (Result, Established, Remaining, Ada.Real_Time.Clock, Deadline) then
            Say ("FAILED remaining=" & Image (Remaining));
            Ada.Command_Line.Set_Exit_Status (1);
            exit;
         end if;
         Say ("RECONNECT remaining=" & Image (Remaining));
         delay until P.Wake_Time (Ada.Real_Time.Clock, Deadline, Config.Reconnect_Delay_MS);
         if Ada.Real_Time.Clock >= Deadline then
             Say ("FAILED deadline_during_delay");
             Ada.Command_Line.Set_Exit_Status (1);
            exit;
         end if;
         Number := Number + 1;
      end loop;
   end;
exception
   when E : P.Invalid_Arguments =>
      Say ("ARGUMENT_ERROR " & Ada.Exceptions.Exception_Message (E));
      Ada.Command_Line.Set_Exit_Status (2);
   when E : others =>
      Say ("FAILED exception=" & Ada.Exceptions.Exception_Name (E));
      Ada.Command_Line.Set_Exit_Status (1);
end Ada_UCI_Client;