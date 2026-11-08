with Ada.Command_Line;
with Ada.Environment_Variables;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with GNAT.Directory_Operations;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with AHT.Bytes;
with AHT.Random;
with Programs.Oam;
with Service_API;
with Service_Codec;
with OMS_LACAL; use OMS_LACAL;
with OMS_LACAL.Connections;
with OMS_LACAL.Transports.WebSocket;
with OMS_LACAL.Typed;

procedure Recovery_Consumer is
   use type Ada.Real_Time.Time;
   package C renames OMS_LACAL.Connections;
   package WS renames OMS_LACAL.Transports.WebSocket;
   package Model renames Programs.Oam;
   package Guard renames AMS_GRA_OMS_JSON_Guard;
   package LI renames Service_API.Function_Log.Exchange_Receive;
   package RI renames Service_API.Function_Rdma.Exchange_Receive;
   package SI renames Service_API.Function_Survivability.Exchange_Receive;
   type Counts is array (1 .. 3) of Natural;
   type Documents is array (1 .. 3) of JSON_Value;
   Plan : Ada.Text_IO.File_Type;
   Baseline_FD, Baseline_Tasks : Natural;

   procedure Check (OK : Boolean; Detail : String) is
   begin
      if not OK then raise Program_Error with Detail; end if;
   end Check;

   procedure Say (Text : String) is
   begin
      Ada.Text_IO.Put_Line (Text); Ada.Text_IO.Flush;
   end Say;

   function Entries (Path : String) return Natural is
      use GNAT.Directory_Operations;
      Dir : Dir_Type;
      Name : String (1 .. 1024);
      Last, Count : Natural := 0;
   begin
      Open (Dir, Path);
      loop Read (Dir, Name, Last); exit when Last = 0; Count := Count + 1; end loop;
      Close (Dir); return Count;
   end Entries;

   procedure Epoch (Number : Positive; Item : JSON_Value) is
      Mode : constant String := Get (Item.Get ("mode"));
      Wire : aliased WS.Client;
      Core : aliased C.Connection (Wire'Access);
      --  Everything that can borrow Core is inside its master. No heap owners,
      --  unchecked access, or retained Delivery crosses this procedure boundary.
      function Is_Codec (E : Ada.Exceptions.Exception_Occurrence) return Boolean is
         use type Ada.Exceptions.Exception_Id;
      begin
         return Ada.Exceptions.Exception_Identity (E) = Service_Codec.Codec_Error'Identity;
      end Is_Codec;
      package L is new OMS_LACAL.Typed
        (Model.LogMT, LI.Handler, LI.Handle,
         Service_Codec.Encode_LogMT, Service_Codec.Decode_LogMT, Is_Codec);
      package R is new OMS_LACAL.Typed
        (Model.RDMA_InitializeSetupMT, RI.Handler, RI.Handle,
         Service_Codec.Encode_RDMA_InitializeSetupMT, Service_Codec.Decode_RDMA_InitializeSetupMT, Is_Codec);
      package S is new OMS_LACAL.Typed
        (Model.SurvivabilityRiskLevelMT, SI.Handler, SI.Handle,
         Service_Codec.Encode_SurvivabilityRiskLevelMT, Service_Codec.Decode_SurvivabilityRiskLevelMT, Is_Codec);
      Calls : Counts := (others => 0);
      SIDs : array (1 .. 3) of Subscription_ID := (others => No_Subscription);
      Expected : Documents;
      Init_Count, Ack_Count, Close_Count : Natural := 0;
      Last_SID : Subscription_ID := No_Subscription;
      Last_Name : Text;
      Retired, Failing : Boolean := False;
      type Log_Receiver is new LI.Handler with null record;
      type Rdma_Receiver is new RI.Handler with null record;
      type Risk_Receiver is new SI.Handler with null record;
      overriding procedure Handle (Self : in out Log_Receiver; Message : LI.Payload);
      overriding procedure Handle (Self : in out Rdma_Receiver; Message : RI.Payload);
      overriding procedure Handle (Self : in out Risk_Receiver; Message : SI.Payload);

      procedure Validate (Family : Positive; Name, Document : String) is
      begin
         Check (not Retired and then C.State (Core) = Active, "retired callback");
         Check (Last_SID = SIDs (Family) and then Last_Name = Name, "SID/family isolation");
         --  Compare every decoded value against independently authored fixtures,
         --  including the epoch-specific standard UCI header Timestamp.
         Check (Guard.Read (Document) = Expected (Family), "independent epoch value oracle");
         Calls (Family) := Calls (Family) + 1;
         Say ("CALL" & Number'Image & Family'Image & Calls (Family)'Image);
      end Validate;
      overriding procedure Handle (Self : in out Log_Receiver; Message : LI.Payload) is
         pragma Unreferenced (Self);
      begin Validate (1, "Log", Service_Codec.Encode_LogMT (Message)); end Handle;
      overriding procedure Handle (Self : in out Rdma_Receiver; Message : RI.Payload) is
         pragma Unreferenced (Self);
      begin Validate (2, "RDMA_InitializeSetup", Service_Codec.Encode_RDMA_InitializeSetupMT (Message)); end Handle;
      overriding procedure Handle (Self : in out Risk_Receiver; Message : SI.Payload) is
         pragma Unreferenced (Self);
      begin Validate (3, "SurvivabilityRiskLevel", Service_Codec.Encode_SurvivabilityRiskLevelMT (Message)); end Handle;
      LH : aliased Log_Receiver;
      RH : aliased Rdma_Receiver;
      SH : aliased Risk_Receiver;
      LB : L.Binding (Core'Access);
      RB : R.Binding (Core'Access);
      SB : S.Binding (Core'Access);
      function Sub_Log (Namespace, Name, Topic : String; Has_Group : Boolean; Group_Name : String;
                        Receiver : not null access LI.Handler'Class) return Subscription_ID is
        (L.Subscribe (LB, Namespace, Name, Topic, Has_Group, Group_Name, Receiver));
      function Sub_Rdma (Namespace, Name, Topic : String; Has_Group : Boolean; Group_Name : String;
                         Receiver : not null access RI.Handler'Class) return Subscription_ID is
        (R.Subscribe (RB, Namespace, Name, Topic, Has_Group, Group_Name, Receiver));
      function Sub_Risk (Namespace, Name, Topic : String; Has_Group : Boolean; Group_Name : String;
                         Receiver : not null access SI.Handler'Class) return Subscription_ID is
        (S.Subscribe (SB, Namespace, Name, Topic, Has_Group, Group_Name, Receiver));
      function Pub_Log (Namespace, Name, Topic : String; Value : Model.LogMT) return Boolean is
        (L.Publish (Core, Namespace, Name, Topic, Value));
      function Pub_Rdma (Namespace, Name, Topic : String; Value : Model.RDMA_InitializeSetupMT) return Boolean is
        (R.Publish (Core, Namespace, Name, Topic, Value));
      function Pub_Risk (Namespace, Name, Topic : String; Value : Model.SurvivabilityRiskLevelMT) return Boolean is
        (S.Publish (Core, Namespace, Name, Topic, Value));
      package LS is new LI.Subscriber (Subscription_ID, Sub_Log);
      package RS is new RI.Subscriber (Subscription_ID, Sub_Rdma);
      package SS is new SI.Subscriber (Subscription_ID, Sub_Risk);
      package LP is new Service_API.Function_Log.Exchange_Publish.Publisher (Boolean, Pub_Log);
      package RP is new Service_API.Function_Rdma.Exchange_Publish.Publisher (Boolean, Pub_Rdma);
      package SP is new Service_API.Function_Survivability.Exchange_Publish.Publisher (Boolean, Pub_Risk);

      procedure Step is
         Delivery : C.Delivery;
         Available : Boolean;
         E : Event;
      begin
         begin
            C.Poll (Core, Delivery);
         exception
            when Error : WS.Transport_Error =>
               Delivery := (others => <>);
               Check (Failing and then (Mode = "reset" or Mode = "pending"),
                      "unexpected transport exception");
               Say ("TRANSPORT_EXCEPTION" & Number'Image & " " & Ada.Exceptions.Exception_Information (Error));
         end;
         if Delivery.Available then Last_SID := Delivery.SID; Last_Name := Delivery.Name; end if;
         L.Dispatch (LB, Delivery); R.Dispatch (RB, Delivery); S.Dispatch (SB, Delivery);
         loop
            C.Next_Event (Core, Available, E); exit when not Available;
            Say ("EVENT" & Number'Image & " " & E.Kind'Image & E.SID'Image & " " & To_String (E.Detail));
            case E.Kind is
               when Initialized =>
                  Init_Count := Init_Count + 1;
                  declare Info : constant JSON_Value := Guard.Read (To_String (E.Detail));
                  begin
                     Check (String'(Get (Info.Get ("version"))) = "1.0" and then
                       String'(Get (Info.Get ("server_id"))) = "task112" and then
                       String'(Get (Info.Get ("uuids").Get ("service"))) =
                         "550e8400-e29b-41d4-a716-446655440112", "correct broker INFO");
                  end;
               when Command_Accepted => Ack_Count := Ack_Count + 1;
               when Connection_Closed => Close_Count := Close_Count + 1;
               when others => raise Program_Error with "unexpected event " & E.Kind'Image;
            end case;
         end loop;
         Check (C.Lost_Events (Core) = 0, "lost events");
      end Step;

      procedure Await_Command is
         Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
      begin
         while C.Busy (Core) loop
            Step; Check (C.State (Core) /= Closed, "closed during command");
            Check (Ada.Real_Time.Clock < Deadline, "application command deadline"); delay 0.001;
         end loop;
      end Await_Command;

      procedure Publish (Family : Positive; Document : JSON_Value) is
         Text : constant String := Guard.Write (Document);
         Sent : Boolean;
      begin
         case Family is
            when 1 => Sent := LP.Publish (Service_Codec.Decode_LogMT (Text));
            when 2 => Sent := RP.Publish (Service_Codec.Decode_RDMA_InitializeSetupMT (Text));
            when 3 => Sent := SP.Publish (Service_Codec.Decode_SurvivabilityRiskLevelMT (Text));
            when others => raise Program_Error;
         end case;
         Check (Sent, "generated publish not sent");
      end Publish;
   begin
      WS.Open (Wire, Ada.Command_Line.Argument (1),
               CA_Path => Ada.Environment_Variables.Value ("OMS_WSS_TEST_CA"));
      Check (WS.Protocol (Wire) = "owp", "verified upgrade");
      C.Initialize (Core, Ada.Command_Line.Argument (3), Service_API.Service_Name, True);
      Await_Command;
      declare Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
      begin
         while C.State (Core) /= Active loop
            Step; Check (Ada.Real_Time.Clock < Deadline, "application INFO deadline"); delay 0.001;
         end loop;
      end;
      Check (Init_Count = 1, "one fresh INFO");
      SIDs (1) := LS.Subscribe (LH'Access); Await_Command;
      SIDs (2) := RS.Subscribe (RH'Access); Await_Command;
      SIDs (3) := SS.Subscribe (SH'Access); Await_Command;
      Check (SIDs (1) = 1 and then SIDs (2) = 2 and then SIDs (3) = 3, "epoch-local SIDs");
      Check (Ack_Count = 4, "fresh INIT and three SUB acknowledgments");
      for SID of SIDs loop Check (C.Is_Active (Core, SID), "accepted subscription"); end loop;
      Say ("SUBSCRIBED" & Number'Image & " 1 2 3");
      for Phase in 1 .. 2 loop
         for Family in 1 .. 3 loop
            Expected (Family) := Get (Get (Item.Get (if Phase = 1 then "publish" else "external")), Family);
         end loop;
         Say ("READY" & Number'Image & Phase'Image);
         if Phase = 1 then
            for Family in 1 .. 3 loop Publish (Family, Expected (Family)); Await_Command; end loop;
         end if;
         declare Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
         begin
            while Calls /= Counts'(others => Phase) loop
               Step; Check (C.State (Core) = Active, "closed during delivery");
               Check (Ada.Real_Time.Clock < Deadline, "application delivery deadline"); delay 0.001;
            end loop;
         end;
      end loop;
      Check (Ack_Count = 7, "no phantom acknowledgments");
      Retired := True;
      if Mode = "normal" or Mode = "explicit" then
         if Mode = "explicit" then L.Close (LB); R.Close (RB); S.Close (SB); end if;
         C.Close (Core); Step;
      else
         Failing := True;
         if Mode = "pending" then
            Publish (1, Item.Get ("ambiguous"));
            Check (C.Busy (Core), "publication must be outstanding");
         end if;
         Say ("ARMED" & Number'Image & " " & Mode);
         declare Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
         begin
            while C.State (Core) /= Closed loop
               Step; Check (Ada.Real_Time.Clock < Deadline, "application failure deadline"); delay 0.001;
            end loop;
         end;
      end if;
      Check (C.State (Core) = Closed and then not C.Busy (Core), "terminal clears command");
      Check (Close_Count = 1 and then Ack_Count = 7, "one close, no stale ACK");
      for SID of SIDs loop Check (not C.Is_Active (Core, SID), "terminal subscription"); end loop;
      for Attempt in 1 .. 10 loop Step; end loop;
      Check (Calls = Counts'(others => 2) and then Close_Count = 1, "post-retirement isolation");
      Say ("RETIRED" & Number'Image & " callbacks=2,2,2 lost=0 busy=false");
      --  Automatic finalization: SB/RB/LB, receivers, Core, then Wire.
   end Epoch;
begin
   declare Nonce : AHT.Bytes.Byte_Array (1 .. 16);
   begin AHT.Random.Fill (Nonce); end;
   Ada.Text_IO.Open (Plan, Ada.Text_IO.In_File, Ada.Command_Line.Argument (2));
   Baseline_Tasks := Entries ("/proc/self/task");
   for Number in 1 .. 8 loop
      declare Item : constant JSON_Value := Guard.Read (Ada.Text_IO.Get_Line (Plan));
      begin Epoch (Number, Item); end;
      --  The first complete TLS epoch warms process-owned provider resources.
      --  DNS worker quiescence is a condition, bounded by a monotonic deadline.
      if Number = 1 then
         declare Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (2);
         begin
            while Entries ("/proc/self/task") /= Baseline_Tasks loop
               Check (Ada.Real_Time.Clock < Deadline, "warm-up DNS task deadline"); delay 0.001;
            end loop;
         end;
         Baseline_FD := Entries ("/proc/self/fd");
      else
         declare Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (2);
         begin
            while Entries ("/proc/self/fd") /= Baseline_FD or else
              Entries ("/proc/self/task") /= Baseline_Tasks
            loop
               Check (Ada.Real_Time.Clock < Deadline, "epoch resource cleanup deadline"); delay 0.001;
            end loop;
         end;
      end if;
      Say ("CLEANUP" & Number'Image & " fd=" & Baseline_FD'Image & " tasks=" & Baseline_Tasks'Image);
   end loop;
   Ada.Text_IO.Close (Plan);
   Say ("TASK112_COMPLETE epochs=8");
exception
   when E : others =>
      Say ("TASK112_FAILED " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (1);
end Recovery_Consumer;