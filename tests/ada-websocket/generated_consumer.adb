with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Service_API;
with Service_Codec;
with TASK100_PACKAGE;
with OMS_LACAL; use OMS_LACAL;
with OMS_LACAL.Connections;
with OMS_LACAL.Transports.WebSocket;
with OMS_LACAL.Typed;

procedure Generated_Consumer is
   use type Ada.Real_Time.Time;
   package Model renames TASK100_PACKAGE;
   package C renames OMS_LACAL.Connections;
   package WS renames OMS_LACAL.Transports.WebSocket;
   package IA renames Service_API.Function_Io.Exchange_In_A;
   package IB renames Service_API.Function_Io.Exchange_In_B;
   Wire : aliased WS.Client;
   Core : aliased C.Connection (Wire'Access);
   function Is_Codec (E : Ada.Exceptions.Exception_Occurrence) return Boolean is
      use type Ada.Exceptions.Exception_Id;
   begin return Ada.Exceptions.Exception_Identity (E) = Service_Codec.Codec_Error'Identity; end;
   package A is new OMS_LACAL.Typed
     (Model.SharedPayload, IA.Handler, IA.Handle,
      Service_Codec.Encode_SharedPayload, Service_Codec.Decode_SharedPayload, Is_Codec);
   package B is new OMS_LACAL.Typed
     (Model.SharedPayload, IB.Handler, IB.Handle,
      Service_Codec.Encode_SharedPayload, Service_Codec.Decode_SharedPayload, Is_Codec);
   BA : A.Binding (Core'Access);
   BB : B.Binding (Core'Access);
   Calls_A, Calls_B, Errors : Natural := 0;
   Invalids : Natural := 0;
   Safety : constant Boolean := Ada.Command_Line.Argument_Count > 1 and then
     Ada.Command_Line.Argument (2) = "generated";
   type Receiver_A is new IA.Handler with null record;
   overriding procedure Handle (Self : in out Receiver_A; Message : IA.Payload);
   type Receiver_B is new IB.Handler with null record;
   overriding procedure Handle (Self : in out Receiver_B; Message : IB.Payload);
   procedure Validate (Message : Model.SharedPayload) is
   begin
      if Message.Count /= 42 or else To_String (Message.Label) /= "task100" then
         raise Program_Error with "typed value changed";
      end if;
   end Validate;
   overriding procedure Handle (Self : in out Receiver_A; Message : IA.Payload) is
      pragma Unreferenced (Self);
   begin Validate (Message); Calls_A := Calls_A + 1; end Handle;
   overriding procedure Handle (Self : in out Receiver_B; Message : IB.Payload) is
      pragma Unreferenced (Self);
   begin Validate (Message); Calls_B := Calls_B + 1; end Handle;
   RA : aliased Receiver_A;
   RB : aliased Receiver_B;
   function Sub_A (Message_Namespace, Message_Name, Topic : String;
                   Has_Subscription_Group : Boolean; Subscription_Group : String;
                   Receiver : not null access IA.Handler'Class) return Subscription_ID is
     (A.Subscribe (BA, Message_Namespace, Message_Name, Topic,
       Has_Subscription_Group, Subscription_Group, Receiver));
   function Sub_B (Message_Namespace, Message_Name, Topic : String;
                   Has_Subscription_Group : Boolean; Subscription_Group : String;
                   Receiver : not null access IB.Handler'Class) return Subscription_ID is
     (B.Subscribe (BB, Message_Namespace, Message_Name, Topic,
       Has_Subscription_Group, Subscription_Group, Receiver));
   function Pub (Message_Namespace, Message_Name, Topic : String;
                 Value : Model.SharedPayload) return Boolean is
     (A.Publish (Core, Message_Namespace, Message_Name, Topic, Value));
   package SA is new IA.Subscriber (Subscription_ID, Sub_A);
   package SB is new IB.Subscriber (Subscription_ID, Sub_B);
   package PA is new Service_API.Function_Io.Exchange_Out_A.Publisher (Boolean, Pub);
   package PB is new Service_API.Function_Io.Exchange_Out_B.Publisher (Boolean, Pub);
   procedure Step is
      D : C.Delivery;
      Available : Boolean;
      E : Event;
   begin
      C.Poll (Core, D);
      if D.Available then
         Ada.Text_IO.Put_Line ("ORIGINAL_ENVELOPE " & To_String (D.Document));
      end if;
      A.Dispatch (BA, D); B.Dispatch (BB, D);
      loop
         C.Next_Event (Core, Available, E);
         exit when not Available;
         Ada.Text_IO.Put_Line ("OWP_EVENT " & E.Kind'Image & " " & To_String (E.Detail));
         if E.Kind = Server_Error then Errors := Errors + 1;
         elsif Safety and then E.Kind in Protocol_Error | Decode_Error then
            Invalids := Invalids + 1;
         elsif E.Kind in Protocol_Error | Decode_Error | Handler_Error then
            raise Program_Error with "unexpected runtime error " & To_String (E.Detail);
         end if;
      end loop;
   end Step;
   procedure Await_Command is
      Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (4);
   begin
      loop
         Step;
         exit when not C.Busy (Core);
         if Ada.Real_Time.Clock > Deadline then raise Program_Error with "OWP acceptance timeout"; end if;
         delay 0.001;
      end loop;
   end Await_Command;
   procedure Await_Delivery (Want_A, Want_B : Natural) is
      Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (4);
   begin
      loop
         Step;
         exit when Calls_A = Want_A and then Calls_B = Want_B;
         if Calls_A > Want_A or else Calls_B > Want_B then raise Program_Error with "wrong routing"; end if;
         if Ada.Real_Time.Clock > Deadline then raise Program_Error with "typed delivery timeout"; end if;
         delay 0.001;
      end loop;
   end Await_Delivery;
   SID_A, SID_B : Subscription_ID;
   function Key (Name : String) return String is
     (QName (IA.Message_Namespace, Name));
   function Body_Text return String is
     ("{""" & Key ("Count") & """:42,""" & Key ("At") &
      """:""12:34:56.5000Z"",""" & Key ("Label") & """:""task100""}");
   Value : constant Model.SharedPayload := Service_Codec.Decode_SharedPayload
     (Body_Text);
   Sent : Boolean;
begin
   WS.Open (Wire, Ada.Command_Line.Argument (1));
   if WS.Protocol (Wire) /= "owp" then raise Program_Error; end if;
   Ada.Text_IO.Put_Line ("GENERATED_UPGRADE_OWP_COMPLETE");
   C.Initialize (Core, "000.1.0", "task100", True);
   declare
      Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (4);
   begin
      while C.State (Core) /= Active loop
         Step;
         if Ada.Real_Time.Clock > Deadline then raise Program_Error with "INFO timeout"; end if;
         delay 0.001;
      end loop;
   end;
   if C.State (Core) /= Active then raise Program_Error with "INIT not accepted"; end if;
   SID_A := SA.Subscribe (RA'Access); Await_Command;
   SID_B := SB.Subscribe (RB'Access); Await_Command;
   begin
      B.Unsubscribe (BB, SID_A);
      raise Program_Error with "foreign binding UNSUB accepted";
   exception when Runtime_Error => null; end;
   if not C.Is_Active (Core, SID_A) or else not C.Is_Active (Core, SID_B) or else C.Busy (Core) then
      raise Program_Error with "foreign binding changed ownership";
   end if;
   Ada.Text_IO.Put_Line ("OWNERSHIP_WEBSOCKET_COMPLETE");
   Sent := PA.Publish (Value);
   if not Sent then raise Program_Error with "transmission failed"; end if;
   Await_Command; Await_Delivery (1, 0);
   if Safety and then Invalids /= 4 then
      raise Program_Error with "invalid payloads did not fail before valid delivery";
   end if;
   Sent := PB.Publish (Value);
   if not Sent then raise Program_Error; end if;
   Await_Command; Await_Delivery (1, 1);
   A.Unsubscribe (BA, SID_A); Await_Command;
   Sent := PA.Publish (Value);
   if not Sent then raise Program_Error; end if;
   Await_Command;
   declare
      Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Milliseconds (150);
   begin
      while Ada.Real_Time.Clock < Deadline loop Step; delay 0.001; end loop;
   end;
   if Calls_A /= 1 or else Calls_B /= 1 then raise Program_Error with "delivery after UNSUB"; end if;
   B.Unsubscribe (BB, SID_B); Await_Command;
   if Errors /= 0 then raise Program_Error with "valid commands rejected"; end if;
   C.Publish (Core, IA.Message_Namespace, IA.Message_Name, "rejected-topic",
              Service_Codec.Encode_SharedPayload (Value));
   Ada.Text_IO.Put_Line ("REJECTED_TOPIC_TRANSMITTED_NOT_ACCEPTED");
   Await_Command;
   if Errors /= 1 then raise Program_Error with "missing rejected topic OWP error"; end if;
   C.Close (Core);
   Ada.Text_IO.Put_Line ("TASK100_GENERATED_CONSUMER_COMPLETE typed_A=1 typed_B=1 rejected=1");
exception when E : others =>
   Ada.Text_IO.Put_Line ("GENERATED_CONSUMER_FAILED " & Ada.Exceptions.Exception_Information (E));
   Ada.Command_Line.Set_Exit_Status (1);
end Generated_Consumer;