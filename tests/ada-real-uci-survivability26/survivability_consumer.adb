with Ada.Command_Line;
with GNAT.Directory_Operations;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with AMS_GRA_OMS_JSON_Guard;
with GNATCOLL.JSON; use type GNATCOLL.JSON.JSON_Value;
with Programs.Oam;
with Service_API;
with Service_Codec;
with OMS_LACAL; use OMS_LACAL;
with OMS_LACAL.Connections;
with OMS_LACAL.Transports.WebSocket;
with OMS_LACAL.Typed;

procedure Survivability_Consumer is
   use type Ada.Real_Time.Time;
   package Model renames Programs.Oam;
   package C renames OMS_LACAL.Connections;
   package WS renames OMS_LACAL.Transports.WebSocket;
   package Input renames Service_API.Function_Survivability.Exchange_Receive;
   package Output renames Service_API.Function_Survivability.Exchange_Publish;
   Wire : aliased WS.Client;
   Core : aliased C.Connection (Wire'Access);
   function Is_Codec (E : Ada.Exceptions.Exception_Occurrence) return Boolean is
      use type Ada.Exceptions.Exception_Id;
   begin
      return Ada.Exceptions.Exception_Identity (E) = Service_Codec.Codec_Error'Identity;
   end Is_Codec;
   package Typed is new OMS_LACAL.Typed
     (Model.SurvivabilityRiskLevelMT, Input.Handler, Input.Handle,
      Service_Codec.Encode_SurvivabilityRiskLevelMT, Service_Codec.Decode_SurvivabilityRiskLevelMT, Is_Codec);
   Binding : Typed.Binding (Core'Access);
   Calls, Witness_Calls, Rejected : Natural := 0;
   Expected : Unbounded_String;
   type Receiver is new Input.Handler with record
      Witness : Boolean := False;
   end record;
   overriding procedure Handle (Self : in out Receiver; Message : Input.Payload);
   Sequence_Case : Natural := 0;
   procedure Check_Model (Message : Model.SurvivabilityRiskLevelMT) is
      use Model;
      Settings : SurvivabilityRiskLevelMDT_RiskSetting_Sequence renames Message.MessageData.RiskSetting;
      Owners : SecurityInformationType_OwnerProducer_Sequence renames Message.SecurityInformation.OwnerProducer;
      procedure Check (OK : Boolean; Detail : String) is
      begin
         if not OK then raise Program_Error with Detail; end if;
      end Check;
      procedure Risk (Index : Positive; Priority : RiskPrioritizationEnum;
                      Allowed : VulnerabilityTypeEnum; Count : Positive := 1) is
         Setting : constant SurvivabilityRiskSettingType := Element (Settings, Index);
         Level : constant RiskSettingType := Element (Setting.RiskLevel, 1);
      begin
         Check (Setting.RiskPrioritization = Priority, "priority/order");
         Check (Length (Setting.RiskLevel) = Count, "risk levels length");
         Check (Level.AllowedRisk = Allowed, "allowed risk");
         Check (Level.MaximumContinuousExposure.Is_Present =
                (Sequence_Case = 4 and Index = 1), "optional exposure presence");
         if Sequence_Case = 4 then
            declare
               Expected_Risks : constant array (Positive range 1 .. 5) of VulnerabilityTypeEnum :=
                 (ACQUISITION, TRACK, LAUNCH, INTERCEPT, KILL);
            begin
               for J in Expected_Risks'Range loop
                  declare
                     Item : constant RiskSettingType := Element (Setting.RiskLevel, J);
                  begin
                     Check (Item.AllowedRisk = Expected_Risks (J), "nested risk order/value");
                     if Index = 1 and J = 1 then
                        Check (Value (Item.MaximumContinuousExposure.Value.CompositeExposureTime) = "PT1S",
                               "composite exposure duration");
                        Check (Value (Item.MaximumContinuousExposure.Value.CumulativeExposureTime) = "PT2S",
                               "cumulative exposure duration");
                     else
                        Check (not Item.MaximumContinuousExposure.Is_Present, "unexpected exposure");
                     end if;
                  end;
               end loop;
            end;
         end if;
         Ada.Text_IO.Put_Line ("TASK105_RISK_OBSERVED setting=" & Index'Image &
           " priority=" & Priority'Image & " allowed=" & Allowed'Image & " levels=" & Count'Image);
      end Risk;
   begin
      Check (Value (Message.MessageHeader.SystemID.UUID) = "12345678-1234-1234-8234-123456789abc", "header UUID");
      Check (Value (Message.MessageData.SurvivabilityRiskLevelID.UUID) = "12345678-1234-1234-8234-123456789abc", "risk UUID");
      Check (Value (Message.MessageHeader.SchemaVersion) = Ada.Command_Line.Argument (2), "schema version");
      Check (Message.SecurityInformation.DeclassDate.Is_Present = (Sequence_Case = 4), "declass presence");
      Check (Message.SecurityInformation.CUI_DecontrolDate.Is_Present = (Sequence_Case = 4), "decontrol presence");
      Check (Message.SecurityInformation.ClassifiedBy.Is_Present = (Sequence_Case = 4), "authority presence");
      Check (Length (Message.SecurityInformation.SAR_Identifier) = (if Sequence_Case = 4 then 1 else 0), "SAR length");
      if Sequence_Case = 4 then
         Check (Value (Message.SecurityInformation.DeclassDate.Value) = "2026-01-02T03:04:05Z", "named declass UTC");
         Check (Value (Message.SecurityInformation.CUI_DecontrolDate.Value) = "2026-01-03T03:04:05Z", "named decontrol UTC");
         Check (Value (Message.SecurityInformation.ClassifiedBy.Value) = "  Task105  authority  ", "preserved authority whitespace");
         Check (Value (Element (Message.SecurityInformation.SAR_Identifier, 1)) = "Task105 marking", "SAR value");
      end if;
      Check (Owners.Required (1).Kind = GovernmentIdentifier_Kind, "required owner discriminant");
      Check (Owners.Required (1).GovernmentIdentifier = USA, "required owner value");
      Check (Length (Owners.Additional) = (if Sequence_Case = 4 then 3 else 1), "additional owners length");
      Check (Element (Owners.Additional, 1).Kind = NATO_SpecialWord_Kind, "additional owner discriminant");
      Check (Value (Element (Owners.Additional, 1).NATO_SpecialWord) = "NATO:EXAMPLE", "additional owner value");
      if Sequence_Case = 4 then
         Check (Element (Owners.Additional, 2).Kind = GovernmentIdentifier_Kind,
                "extra government owner discriminant/order");
         Check (Element (Owners.Additional, 2).GovernmentIdentifier = CAN,
                "extra government owner value");
         Check (Element (Owners.Additional, 3).Kind = NATO_SpecialWord_Kind,
                "extra NATO owner discriminant/order");
         Check (Value (Element (Owners.Additional, 3).NATO_SpecialWord) = "NATO:MORE",
                "extra NATO owner value");
      end if;
      Check (Length (Message.MessageData.AppliesToSystemID) = 0, "optional system sequence");
      Check (Length (Settings) = (case Sequence_Case is when 3 => 2, when 4 => 4, when others => 1),
             "risk settings length");
      case Sequence_Case is
         when 0 | 5 => Risk (1, HIGH, VulnerabilityTypeEnum'(NONE));
         when 1 => Risk (1, MEDIUM, Value_ALL);
         when 2 => Risk (1, LOW, TRACK);
         when 3 => Risk (1, RiskPrioritizationEnum'(NONE), VulnerabilityTypeEnum'(NONE));
                   Risk (2, HIGH, Value_ALL);
         when 4 => Risk (1, HIGH, ACQUISITION, 5); Risk (2, MEDIUM, ACQUISITION, 5);
                   Risk (3, LOW, ACQUISITION, 5); Risk (4, RiskPrioritizationEnum'(NONE), ACQUISITION, 5);
         when others => raise Program_Error with "unknown independent model expectation";
      end case;
      Check (Service_Codec.Encode_SurvivabilityRiskLevelMT
        (Service_Codec.Decode_SurvivabilityRiskLevelMT (Service_Codec.Encode_SurvivabilityRiskLevelMT (Message))) =
        Service_Codec.Encode_SurvivabilityRiskLevelMT (Message), "exact generated round trip");
   end Check_Model;
   overriding procedure Handle (Self : in out Receiver; Message : Input.Payload) is
   begin
      if Service_Codec.Encode_SurvivabilityRiskLevelMT (Message) /= To_String (Expected) then
         raise Program_Error with "typed payload or routing mismatch";
      end if;
      Check_Model (Message);
      Ada.Text_IO.Put_Line ("TASK105_MODEL_CHECK_COMPLETE case=" & Sequence_Case'Image);
      if Self.Witness then
         Witness_Calls := Witness_Calls + 1;
         Ada.Text_IO.Put_Line ("TASK105_WITNESS_HANDLER " & Witness_Calls'Image);
      else
         Calls := Calls + 1;
         Ada.Text_IO.Put_Line ("TASK105_TYPED_HANDLER " & Calls'Image);
      end if;
   end Handle;
   Handler : aliased Receiver;
   Witness_Handler : aliased Receiver := (Input.Handler with Witness => True);
   function Subscribe
     (Message_Namespace, Message_Name, Topic : String;
      Has_Subscription_Group : Boolean; Subscription_Group : String;
      Receiver : not null access Input.Handler'Class) return Subscription_ID is
     (Typed.Subscribe (Binding, Message_Namespace, Message_Name, Topic,
                       Has_Subscription_Group, Subscription_Group, Receiver));
   function Publish
     (Message_Namespace, Message_Name, Topic : String; Value : Model.SurvivabilityRiskLevelMT) return Boolean is
     (Typed.Publish (Core, Message_Namespace, Message_Name, Topic, Value));
   package Subscriber is new Input.Subscriber (Subscription_ID, Subscribe);
   package Publisher is new Output.Publisher (Boolean, Publish);
   procedure Step is
      Delivery : C.Delivery;
      Available : Boolean;
      Item : Event;
   begin
      C.Poll (Core, Delivery);
      if Delivery.Available then
         Ada.Text_IO.Put_Line ("TASK105_WIRE_RECEIVED " & To_String (Delivery.Document));
      end if;
      Typed.Dispatch (Binding, Delivery);
      loop
         C.Next_Event (Core, Available, Item);
         exit when not Available;
         Ada.Text_IO.Put_Line ("TASK105_EVENT " & Item.Kind'Image & " " & To_String (Item.Detail));
         if Item.Kind = Decode_Error then
            if To_String (Item.Detail) /= "invalid named DateTime" or else Calls /= 0 then
               raise Program_Error with "unexpected codec rejection or handler execution";
            end if;
            Rejected := Rejected + 1;
         elsif Item.Kind in Server_Error | Protocol_Error | Handler_Error | Unknown_Subscription then
            raise Program_Error with "unexpected OWP error: " & To_String (Item.Detail);
         end if;
      end loop;
   end Step;
   procedure Await_Command is
      Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
   begin
      loop
         Step;
         exit when not C.Busy (Core);
         if Ada.Real_Time.Clock > Deadline then raise Program_Error with "acceptance timeout"; end if;
         delay 0.001;
      end loop;
   end Await_Command;
   procedure Await_Calls (Count : Natural) is
      Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (10);
   begin
      loop
         Step;
         exit when Calls = Count;
         if Calls > Count then raise Program_Error with "extra handler"; end if;
         if Ada.Real_Time.Clock > Deadline then raise Program_Error with "delivery timeout"; end if;
         delay 0.001;
      end loop;
   end Await_Calls;
   function Count_Entries (Directory : String) return Natural is
      Search : GNAT.Directory_Operations.Dir_Type;
      Item : String (1 .. 1024);
      Last : Natural;
      Count : Natural := 0;
   begin
      GNAT.Directory_Operations.Open (Search, Directory);
      loop
         GNAT.Directory_Operations.Read (Search, Item, Last);
         exit when Last = 0;
         if Item (1 .. Last) not in "." | ".." then Count := Count + 1; end if;
      end loop;
      GNAT.Directory_Operations.Close (Search);
      return Count;
   end Count_Entries;
   Documents, Expectations : Ada.Text_IO.File_Type;
   SID, Witness_SID : Subscription_ID;
   Sent : Boolean;
   Before_FD, Before_Tasks : Natural;
begin
   -- Warm the existing entropy source before measuring descriptor ownership.
   WS.Open (Wire, Ada.Command_Line.Argument (1));
   WS.Close (Wire);
   Before_FD := Count_Entries ("/proc/self/fd");
   Before_Tasks := Count_Entries ("/proc/self/task");
   WS.Open (Wire, Ada.Command_Line.Argument (1));
   if WS.Protocol (Wire) /= "owp" then raise Program_Error with "wrong subprotocol"; end if;
   Ada.Text_IO.Put_Line ("TASK105_UPGRADE_OWP_COMPLETE");
   C.Initialize (Core, Ada.Command_Line.Argument (2), Service_API.Service_Name, True);
   Await_Command;
   declare
      Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
   begin
      while C.State (Core) = Initializing loop
         Step;
         if Ada.Real_Time.Clock > Deadline then raise Program_Error with "INFO timeout"; end if;
         delay 0.001;
      end loop;
   end;
   if C.State (Core) /= Active then raise Program_Error with "INFO not validated"; end if;
   Ada.Text_IO.Put_Line ("TASK105_INFO_VALIDATED");
   Ada.Text_IO.Open (Documents, Ada.Text_IO.In_File, Ada.Command_Line.Argument (3));
   Ada.Text_IO.Open (Expectations, Ada.Text_IO.In_File, Ada.Command_Line.Argument (4));
   declare
      Base : constant Model.SurvivabilityRiskLevelMT := Service_Codec.Decode_SurvivabilityRiskLevelMT (Ada.Text_IO.Get_Line (Documents));
   begin
      Expected := To_Unbounded_String (Service_Codec.Encode_SurvivabilityRiskLevelMT (Base));

      if AMS_GRA_OMS_JSON_Guard.Read (To_String (Expected)) /=
        AMS_GRA_OMS_JSON_Guard.Read (Ada.Text_IO.Get_Line (Expectations))
      then raise Program_Error with "golden JSON expectation mismatch"; end if;
      Check_Model (Base);
      SID := Subscriber.Subscribe (Handler'Access);
      Await_Command;
      if not C.Is_Active (Core, SID) then raise Program_Error with "SUB not accepted"; end if;
      Ada.Text_IO.Put_Line ("TASK105_SUB_ACCEPTED SurvivabilityRiskLevel task105-survivability");
      begin
         Typed.Unsubscribe (Binding, SID + 100);
         raise Program_Error with "unknown binding SID accepted";
      exception
         when Runtime_Error => Ada.Text_IO.Put_Line ("TASK105_UNKNOWN_BINDING_SID_REJECTED");
      end;
      Ada.Text_IO.Put_Line ("TASK105_SUBSCRIBED");
      Ada.Text_IO.Flush;
      Await_Calls (1);
      if Rejected /= 1 then raise Program_Error with "missing decode rejection before recovery"; end if;
      Ada.Text_IO.Put_Line ("TASK105_DECODE_RECOVERY_COMPLETE");
      Sent := Publisher.Publish (Base);
      if not Sent then raise Program_Error with "not sent"; end if;
      Await_Command;
      Await_Calls (2);
      Ada.Text_IO.Put_Line ("TASK105_PUBLISHED " & To_String (Expected));
      while not Ada.Text_IO.End_Of_File (Documents) loop
         declare
            Value : constant Model.SurvivabilityRiskLevelMT := Service_Codec.Decode_SurvivabilityRiskLevelMT (Ada.Text_IO.Get_Line (Documents));
            Want : constant Natural := Calls + 1;
         begin
            Sequence_Case := Sequence_Case + 1;
            Expected := To_Unbounded_String (Service_Codec.Encode_SurvivabilityRiskLevelMT (Value));
            if AMS_GRA_OMS_JSON_Guard.Read (To_String (Expected)) /=
              AMS_GRA_OMS_JSON_Guard.Read (Ada.Text_IO.Get_Line (Expectations))
            then raise Program_Error with "independent JSON expectation mismatch"; end if;
            Check_Model (Value);
            Ada.Text_IO.Put_Line ("TASK105_EXTERNAL_READY " & Sequence_Case'Image);
            Ada.Text_IO.Flush;
            Await_Calls (Want);
            Sent := Publisher.Publish (Value);
            if not Sent then raise Program_Error with "not sent"; end if;
            Ada.Text_IO.Put_Line ("TASK105_PUBLISHED " & To_String (Expected));
            Await_Command;
            Await_Calls (Want + 1);
         end;
      end loop;
      Ada.Text_IO.Close (Documents);
      Ada.Text_IO.Close (Expectations);
      Witness_SID := Subscriber.Subscribe (Witness_Handler'Access);
      Await_Command;
      if not C.Is_Active (Core, Witness_SID) then raise Program_Error with "witness SUB not accepted"; end if;
      Typed.Unsubscribe (Binding, SID);
      Await_Command;
      if C.Is_Active (Core, SID) then raise Program_Error with "UNSUB not accepted"; end if;
      declare
         Was : constant Natural := Calls;
         Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (5);
      begin
         Sequence_Case := 0;
         Expected := To_Unbounded_String (Service_Codec.Encode_SurvivabilityRiskLevelMT (Base));
         Sent := Publisher.Publish (Base);
         if not Sent then raise Program_Error; end if;
         Await_Command;
         while Witness_Calls = 0 loop
            Step;
            if Ada.Real_Time.Clock > Deadline then raise Program_Error with "witness delivery timeout"; end if;
            delay 0.001;
         end loop;
         if Calls /= Was then raise Program_Error with "delivery after unsubscribe"; end if;
      end;
      Typed.Unsubscribe (Binding, Witness_SID);
      Await_Command;
      Ada.Text_IO.Put_Line ("TASK105_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1");
      Typed.Close (Binding);
      C.Close (Core);
      if C.State (Core) /= Closed or else Count_Entries ("/proc/self/fd") /= Before_FD
        or else Count_Entries ("/proc/self/task") /= Before_Tasks
      then raise Program_Error with "transport cleanup failed"; end if;
      Ada.Text_IO.Put_Line ("TASK105_CLEANUP_COMPLETE");
      Ada.Text_IO.Put_Line ("TASK105_ADA_REAL_SURVIVABILITY_COMPLETE handlers=" & Calls'Image & " rejected=" & Rejected'Image);
   end;
exception
   when E : others =>
      C.Close (Core);
      Ada.Text_IO.Put_Line ("TASK105_FAILED " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (1);
end Survivability_Consumer;
