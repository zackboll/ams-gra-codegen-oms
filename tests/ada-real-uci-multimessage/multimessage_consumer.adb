with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with GNAT.Directory_Operations;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Interfaces; use type Interfaces.Unsigned_8;
with AHT.Bytes;
with AHT.Random;
with Programs.Oam;
with Service_API;
with Service_Codec;
with OMS_LACAL; use OMS_LACAL;
with OMS_LACAL.Connections;
with OMS_LACAL.Transports.WebSocket;
with OMS_LACAL.Typed;

procedure Multimessage_Consumer is
   use type Ada.Real_Time.Time;
   package Model renames Programs.Oam;
   package C renames OMS_LACAL.Connections;
   package WS renames OMS_LACAL.Transports.WebSocket;
   package Guard renames AMS_GRA_OMS_JSON_Guard;
   package LI renames Service_API.Function_Log.Exchange_Receive;
   package RI renames Service_API.Function_Rdma.Exchange_Receive;
   package SI renames Service_API.Function_Survivability.Exchange_Receive;
   Wire : aliased WS.Client;
   Core : aliased C.Connection (Wire'Access);
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
   LB : L.Binding (Core'Access);
   RB : R.Binding (Core'Access);
   SB : S.Binding (Core'Access);
   L_SID, R_SID, S_SID : Subscription_ID := No_Subscription;
   L_Calls, R_Calls, S_Calls, Errors, Init_Count : Natural := 0;
   Step_Number : Natural := 0;
   Expected : JSON_Value;
   Expected_Error, Expected_Function : Unbounded_String;
   Last_SID : Subscription_ID := No_Subscription;
   Last_Name : Unbounded_String;
   type Log_Receiver is new LI.Handler with null record;
   type Rdma_Receiver is new RI.Handler with null record;
   type Risk_Receiver is new SI.Handler with null record;
   overriding procedure Handle (Self : in out Log_Receiver; Message : LI.Payload);
   overriding procedure Handle (Self : in out Rdma_Receiver; Message : RI.Payload);
   overriding procedure Handle (Self : in out Risk_Receiver; Message : SI.Payload);

   procedure Check (OK : Boolean; Detail : String) is
   begin
      if not OK then raise Program_Error with Detail; end if;
   end Check;

   procedure Owners (Value : Model.SecurityInformationType) is
      use Model;
      Items : SecurityInformationType_OwnerProducer_Sequence renames Value.OwnerProducer;
      Want : constant JSON_Array := Get (Expected.Get ("SecurityInformation").Get ("OwnerProducer"));
   begin
      Check (Items.Required (1).Kind = GovernmentIdentifier_Kind and then
             Items.Required (1).GovernmentIdentifier = USA, "required OwnerProducer Choice");
      Check (Model.Length (Items.Additional) = GNATCOLL.JSON.Length (Want) - 1, "OwnerProducer length");
      for J in 1 .. Model.Length (Items.Additional) loop
         declare
            Item : constant OwnerProducerChoiceType := Element (Items.Additional, J);
            Doc : constant JSON_Value := Get (Want, J + 1);
         begin
            if Doc.Has_Field ("NATO_SpecialWord") then
               Check (Item.Kind = NATO_SpecialWord_Kind and then
                 Model.Value (Item.NATO_SpecialWord) = String'(Get (Doc.Get ("NATO_SpecialWord"))), "NATO owner order/value");
            else
               Check (Item.Kind = GovernmentIdentifier_Kind and then Item.GovernmentIdentifier = CAN,
                      "government owner order/value");
            end if;
         end;
      end loop;
   end Owners;

   procedure Header (Value : Model.HeaderType) is
      use Model;
   begin
      Check (Model.Value (Value.SystemID.UUID) = "12345678-1234-1234-8234-123456789abc", "header UUID");
      Check (Model.Value (Value.SchemaVersion) = "002.5.0", "authentic release");
      Check (Model.Value (Value.Timestamp) = "2026-01-02T03:04:05Z", "header DateTime");
   end Header;

   overriding procedure Handle (Self : in out Log_Receiver; Message : LI.Payload) is
      pragma Unreferenced (Self);
      use Model;
      Doc : constant JSON_Value := Expected.Get ("MessageData");
   begin
      Check (Expected_Function = "log" and then Last_SID = L_SID and then Last_Name = "Log", "Log cross-message dispatch");
      Check (Guard.Read (Service_Codec.Encode_LogMT (Message)) = Expected, "Log independent oracle");
      Header (Message.MessageHeader); Owners (Message.SecurityInformation);
      Check (Model.Value (Message.MessageData.Timestamp) = "2026-01-02T03:04:06Z", "Log DateTime");
      Check (Message.MessageData.ServiceUpTime.Is_Present and then
             Model.Value (Message.MessageData.ServiceUpTime.Value) = "PT01.2300S", "Log Duration");
      Check (Message.MessageData.Data.Is_Present = Doc.Has_Field ("Data"), "optional binary presence");
      if Message.MessageData.Data.Is_Present then
         declare
            Bytes : Binary_Vectors.Vector renames Message.MessageData.Data.Value;
            Hex : constant String := Get (Doc.Get ("Data"));
         begin
            Check (Natural (Bytes.Length) = Hex'Length / 2, "binary length");
            if Hex = "0080FF" then
               Check (Bytes.Element (0) = 0 and then Bytes.Element (1) = 16#80# and then
                      Bytes.Element (2) = 16#FF#, "zero/high octets");
            elsif Hex = "FF008001007F" then
               Check (Bytes.Element (0) = 16#FF# and then Bytes.Element (1) = 0 and then
                 Bytes.Element (2) = 16#80# and then Bytes.Element (3) = 1 and then
                 Bytes.Element (4) = 0 and then Bytes.Element (5) = 16#7F#, "ordered populated octets");
            else
               Check (Hex = "" and then Bytes.Is_Empty, "empty octets");
            end if;
         end;
      end if;
      L_Calls := L_Calls + 1;
      Ada.Text_IO.Put_Line ("TASK106_HANDLER log" & L_Calls'Image);
   end Handle;

   overriding procedure Handle (Self : in out Rdma_Receiver; Message : RI.Payload) is
      pragma Unreferenced (Self);
      use Model;
      Doc : constant JSON_Value := Expected.Get ("MessageData");
      Want : constant JSON_Array := (if Doc.Has_Field ("Endpoint") then Get (Doc.Get ("Endpoint")) else Empty_Array);
   begin
      Check (Expected_Function = "rdma" and then Last_SID = R_SID and then Last_Name = "RDMA_InitializeSetup", "RDMA cross-message dispatch");
      Check (Guard.Read (Service_Codec.Encode_RDMA_InitializeSetupMT (Message)) = Expected, "RDMA independent oracle");
      Header (Message.MessageHeader); Owners (Message.SecurityInformation);
      Check (Model.Value (Message.MessageData.RDMA_InitializeSetupID.UUID) = "12345678-1234-1234-8234-123456789abc", "setup UUID");
      Check (Message.MessageData.RDMA_Provider = TCP, "required provider");
      Check (Model.Length (Message.MessageData.Endpoint) = GNATCOLL.JSON.Length (Want), "endpoint count");
      for J in 1 .. Model.Length (Message.MessageData.Endpoint) loop
         declare
            Item : constant NetworkEndpointType := Element (Message.MessageData.Endpoint, J);
            Expected_Item : constant JSON_Value := Get (Want, J);
         begin
            if Expected_Item.Has_Field ("IPv4_Endpoint") then
               Check (Item.Kind = IPv4_Endpoint_Kind and then Model.Value (Item.IPv4_Endpoint.IPv4_Address) =
                 String'(Get (Expected_Item.Get ("IPv4_Endpoint").Get ("IPv4_Address"))), "IPv4 discriminant/address/order");
            else
               Check (Item.Kind = IPv6_Endpoint_Kind and then Model.Value (Item.IPv6_Endpoint.IPv6_Address) =
                 String'(Get (Expected_Item.Get ("IPv6_Endpoint").Get ("IPv6_Address"))), "IPv6 discriminant/address/order");
            end if;
         end;
      end loop;
      R_Calls := R_Calls + 1;
      Ada.Text_IO.Put_Line ("TASK106_HANDLER rdma" & R_Calls'Image);
   end Handle;

   overriding procedure Handle (Self : in out Risk_Receiver; Message : SI.Payload) is
      pragma Unreferenced (Self);
      use Model;
      Want : constant JSON_Array := Get (Expected.Get ("MessageData").Get ("RiskSetting"));
   begin
      Check (Expected_Function = "survivability" and then Last_SID = S_SID and then
             Last_Name = "SurvivabilityRiskLevel", "Survivability cross-message dispatch");
      Check (Guard.Read (Service_Codec.Encode_SurvivabilityRiskLevelMT (Message)) = Expected, "risk independent oracle");
      Header (Message.MessageHeader); Owners (Message.SecurityInformation);
      Check (Model.Value (Message.MessageData.SurvivabilityRiskLevelID.UUID) =
             "12345678-1234-1234-8234-123456789abc", "risk UUID");
      Check (Model.Length (Message.MessageData.RiskSetting) = GNATCOLL.JSON.Length (Want), "settings count");
      for J in 1 .. Model.Length (Message.MessageData.RiskSetting) loop
         declare
            Item : constant SurvivabilityRiskSettingType := Element (Message.MessageData.RiskSetting, J);
            Doc : constant JSON_Value := Get (Want, J);
            Levels : constant JSON_Array := Get (Doc.Get ("RiskLevel"));
            Priority : constant String := Get (Doc.Get ("RiskPrioritization"));
         begin
            Check (Item.RiskPrioritization = RiskPrioritizationEnum'Value (Priority), "priority/order");
            Check (Model.Length (Item.RiskLevel) = GNATCOLL.JSON.Length (Levels), "nested levels count");
            for K in 1 .. Model.Length (Item.RiskLevel) loop
               declare
                  Risk : constant RiskSettingType := Element (Item.RiskLevel, K);
                  D : constant JSON_Value := Get (Levels, K);
                  Allowed : constant String := Get (D.Get ("AllowedRisk"));
               begin
                  Check (Risk.AllowedRisk = (if Allowed = "ALL" then Value_ALL else
                    VulnerabilityTypeEnum'Value (Allowed)), "allowed risk/order");
                  Check (Risk.MaximumContinuousExposure.Is_Present = D.Has_Field ("MaximumContinuousExposure"), "exposure presence");
                  if Risk.MaximumContinuousExposure.Is_Present then
                     Check (Model.Value (Risk.MaximumContinuousExposure.Value.CompositeExposureTime) = "PT1S"
                       and then Model.Value (Risk.MaximumContinuousExposure.Value.CumulativeExposureTime) = "PT2S", "exposure durations");
                  end if;
               end;
            end loop;
         end;
      end loop;
      S_Calls := S_Calls + 1;
      Ada.Text_IO.Put_Line ("TASK106_HANDLER survivability" & S_Calls'Image);
   end Handle;
   LH : aliased Log_Receiver;
   RH : aliased Rdma_Receiver;
   SH : aliased Risk_Receiver;
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
      C.Poll (Core, Delivery);
      if Delivery.Available then
         Last_SID := Delivery.SID; Last_Name := Delivery.Name;
         Ada.Text_IO.Put_Line ("TASK106_DELIVERY" & Delivery.SID'Image & " " &
           To_String (Delivery.Name) & " " & To_String (Delivery.Document));
      end if;
      --  One delivery is borrowed by all bindings. Only its SID owner decodes.
      L.Dispatch (LB, Delivery); R.Dispatch (RB, Delivery); S.Dispatch (SB, Delivery);
      loop
         C.Next_Event (Core, Available, E);
         exit when not Available;
         Ada.Text_IO.Put_Line ("TASK106_EVENT " & E.Kind'Image & E.SID'Image & " " & To_String (E.Detail));
         case E.Kind is
            when Initialized => Init_Count := Init_Count + 1;
            when Command_Accepted => null;
            when Decode_Error =>
               Check (Length (Expected_Error) > 0 and then E.SID = Last_SID and then
                 Ada.Strings.Fixed.Index (To_String (E.Detail), To_String (Expected_Error)) /= 0, "unexpected codec rejection");
               Check ((Expected_Function = "log" and then E.SID = L_SID and then Last_Name = "Log") or else
                 (Expected_Function = "rdma" and then E.SID = R_SID and then Last_Name = "RDMA_InitializeSetup") or else
                 (Expected_Function = "survivability" and then E.SID = S_SID and then Last_Name = "SurvivabilityRiskLevel"), "decode error SID/name");
               Errors := Errors + 1;
               Ada.Text_IO.Put_Line ("TASK106_DECODE_ERROR" & E.SID'Image & " " & To_String (Last_Name));
            when others => raise Program_Error with "unexpected event " & E.Kind'Image;
         end case;
      end loop;
      Check (C.State (Core) /= Closed and then C.Lost_Events (Core) = 0, "connection failed or lost events");
   end Step;
   procedure Await_Command is
      Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
   begin
      while C.Busy (Core) loop
         Step; Check (Ada.Real_Time.Clock < Deadline, "command timeout"); delay 0.001;
      end loop;
   end Await_Command;
   function Count_Entries (Path : String) return Natural is
      use GNAT.Directory_Operations;
      Dir : Dir_Type;
      Name : String (1 .. 1024);
      Last, Count : Natural := 0;
   begin
      Open (Dir, Path);
      loop Read (Dir, Name, Last); exit when Last = 0; Count := Count + 1; end loop;
      Close (Dir); return Count;
   end Count_Entries;
   Before_FD, Before_Tasks : Natural;
   Plan : Ada.Text_IO.File_Type;
begin
   --  The inherited entropy provider retains a process-owned descriptor. Warm
   --  it directly, NOT by opening a second WebSocket as single-message tests do.
   declare Nonce : AHT.Bytes.Byte_Array (1 .. 16);
   begin AHT.Random.Fill (Nonce); end;
   Before_FD := Count_Entries ("/proc/self/fd");
   Before_Tasks := Count_Entries ("/proc/self/task");
   WS.Open (Wire, Ada.Command_Line.Argument (1));
   Check (WS.Protocol (Wire) = "owp", "OWP subprotocol");
   Ada.Text_IO.Put_Line ("TASK106_UPGRADE_OWP_COMPLETE");
   C.Initialize (Core, "002.5.0", Service_API.Service_Name, True);
   Await_Command;
   declare Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
   begin
      while C.State (Core) /= Active loop
         Step; Check (Ada.Real_Time.Clock < Deadline, "INFO timeout"); delay 0.001;
      end loop;
   end;
   Check (Init_Count = 1, "exactly one INIT/INFO");
   L_SID := LS.Subscribe (LH'Access); Await_Command;
   R_SID := RS.Subscribe (RH'Access); Await_Command;
   S_SID := SS.Subscribe (SH'Access); Await_Command;
   Check (L_SID < R_SID and then R_SID < S_SID and then L_SID /= No_Subscription, "global monotonic SIDs");
   Check (C.Is_Active (Core, L_SID) and then C.Is_Active (Core, R_SID) and then C.Is_Active (Core, S_SID), "three accepted subscriptions");
   Ada.Text_IO.Put_Line ("TASK106_SIDS" & L_SID'Image & R_SID'Image & S_SID'Image);
   for Attempt in 1 .. 9 loop
      begin
         case Attempt is
            when 1 => L.Unsubscribe (LB, R_SID);
            when 2 => L.Unsubscribe (LB, S_SID);
            when 3 => R.Unsubscribe (RB, L_SID);
            when 4 => R.Unsubscribe (RB, S_SID);
            when 5 => S.Unsubscribe (SB, L_SID);
            when 6 => S.Unsubscribe (SB, R_SID);
            when 7 => L.Unsubscribe (LB, S_SID + 100);
            when 8 => R.Unsubscribe (RB, S_SID + 100);
            when 9 => S.Unsubscribe (SB, S_SID + 100);
         end case;
         raise Program_Error with "foreign/unknown unsubscribe accepted";
      exception when Runtime_Error =>
         Check (not C.Busy (Core), "rejected unsubscribe sent wire command");
         Ada.Text_IO.Put_Line ("TASK106_OWNERSHIP_REJECTED" & Attempt'Image);
      end;
   end loop;
   Ada.Text_IO.Put_Line ("TASK106_SUBSCRIBED"); Ada.Text_IO.Flush;
   Ada.Text_IO.Open (Plan, Ada.Text_IO.In_File, Ada.Command_Line.Argument (2));
   while not Ada.Text_IO.End_Of_File (Plan) loop
      declare
         Item : constant JSON_Value := Guard.Read (Ada.Text_IO.Get_Line (Plan));
         Action : constant String := Get (Item.Get ("action"));
         Family : constant String := Get (Item.Get ("function"));
         Was_L : constant Natural := L_Calls;
         Was_R : constant Natural := R_Calls;
         Was_S : constant Natural := S_Calls;
         Was_E : constant Natural := Errors;
         Sent : Boolean := False;
         Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (10);
      begin
         Step_Number := Step_Number + 1;
         Expected_Function := To_Unbounded_String (Family);
         Expected_Error := To_Unbounded_String (if Item.Has_Field ("error") then String'(Get (Item.Get ("error"))) else "");
         Expected := (if Item.Has_Field ("expected") then Item.Get ("expected") else JSON_Null);
         Ada.Text_IO.Put_Line ("TASK106_READY" & Step_Number'Image); Ada.Text_IO.Flush;
         if Action = "remove" then
            L.Unsubscribe (LB, L_SID); Await_Command;
            Check (not C.Is_Active (Core, L_SID) and then C.Is_Active (Core, R_SID) and then
                   C.Is_Active (Core, S_SID), "unsubscribe isolation");
         else
            if Action = "publish" or else Action = "publish-retired" then
               declare Document : constant String := Guard.Write (Item.Get ("document"));
               begin
                  if Family = "log" then Sent := LP.Publish (Service_Codec.Decode_LogMT (Document));
                  elsif Family = "rdma" then Sent := RP.Publish (Service_Codec.Decode_RDMA_InitializeSetupMT (Document));
                  else Sent := SP.Publish (Service_Codec.Decode_SurvivabilityRiskLevelMT (Document)); end if;
               end;
               Check (Sent, "failed generated Publish"); Await_Command;
            end if;
            if Action = "publish-retired" then
               declare Until_Time : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Milliseconds (200);
               begin while Ada.Real_Time.Clock < Until_Time loop Step; delay 0.001; end loop; end;
            else
               loop
                  Step;
                  exit when (if Item.Has_Field ("error") then Errors = Was_E + 1 else
                    L_Calls + R_Calls + S_Calls = Was_L + Was_R + Was_S + 1);
                  Check (Ada.Real_Time.Clock < Deadline, "delivery timeout"); delay 0.001;
               end loop;
            end if;
         end if;
         Check (L_Calls = Was_L + (if Item.Has_Field ("expected") and Family = "log" then 1 else 0)
           and then R_Calls = Was_R + (if Item.Has_Field ("expected") and Family = "rdma" then 1 else 0)
           and then S_Calls = Was_S + (if Item.Has_Field ("expected") and Family = "survivability" then 1 else 0)
           and then Errors = Was_E + (if Item.Has_Field ("error") then 1 else 0), "cross-message count isolation");
         Ada.Text_IO.Put_Line ("TASK106_DONE" & Step_Number'Image & " counts" & L_Calls'Image & R_Calls'Image & S_Calls'Image & " errors" & Errors'Image);
         Ada.Text_IO.Flush;
      end;
   end loop;
   Ada.Text_IO.Close (Plan);
   R.Unsubscribe (RB, R_SID); Await_Command;
   S.Unsubscribe (SB, S_SID); Await_Command;
   L.Close (LB); R.Close (RB); S.Close (SB);
   Check (C.Lost_Events (Core) = 0 and then not C.Busy (Core), "unclean event/command state");
   C.Close (Core);
   declare Available : Boolean; E : Event;
   begin
      C.Next_Event (Core, Available, E);
      Check (Available and then E.Kind = Connection_Closed, "close event missing");
      C.Next_Event (Core, Available, E);
      Check (not Available and then C.Lost_Events (Core) = 0, "unconsumed/lost events at close");
   end;
   Check (C.State (Core) = Closed and then not C.Is_Active (Core, R_SID) and then
     not C.Is_Active (Core, S_SID) and then Count_Entries ("/proc/self/fd") = Before_FD and then
     Count_Entries ("/proc/self/task") = Before_Tasks, "resource/subscription cleanup");
   Ada.Text_IO.Put_Line ("TASK106_CLEANUP_COMPLETE lost_events=0");
   Ada.Text_IO.Put_Line ("TASK106_COMPLETE counts" & L_Calls'Image & R_Calls'Image & S_Calls'Image & " errors" & Errors'Image);
exception
   when E : others =>
      C.Close (Core);
      Ada.Text_IO.Put_Line ("TASK106_FAILED " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (1);
end Multimessage_Consumer;