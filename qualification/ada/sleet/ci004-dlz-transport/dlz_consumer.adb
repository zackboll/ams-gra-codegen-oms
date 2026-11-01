with Ada.Command_Line;
with GNAT.Directory_Operations;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.Oam;
with Service_API;
with Service_Codec;
with OMS_LACAL; use OMS_LACAL;
with OMS_LACAL.Connections;
with OMS_LACAL.Transports.WebSocket;
with OMS_LACAL.Typed;

procedure DLZ_Consumer is
   use type Ada.Real_Time.Time;
   package Model renames Programs.Oam;
   package C renames OMS_LACAL.Connections;
   package WS renames OMS_LACAL.Transports.WebSocket;
   package Input renames Service_API.Function_F.Exchange_E;
   package Output renames Service_API.Function_F.Exchange_Publish;
   Wire : aliased WS.Client;
   Core : aliased C.Connection (Wire'Access);
   function Is_Codec (E : Ada.Exceptions.Exception_Occurrence) return Boolean is
      use type Ada.Exceptions.Exception_Id;
   begin
      return Ada.Exceptions.Exception_Identity (E) = Service_Codec.Codec_Error'Identity;
   end Is_Codec;
   package Typed is new OMS_LACAL.Typed
     (Model.DLZ_MT, Input.Handler, Input.Handle,
      Service_Codec.Encode_DLZ_MT, Service_Codec.Decode_DLZ_MT, Is_Codec);
   Binding : Typed.Binding (Core'Access);
   Calls, Witness_Calls, Rejected : Natural := 0;
   Expected_Reject : Unbounded_String;
   Expected : Unbounded_String;
   type Receiver is new Input.Handler with record
      Witness : Boolean := False;
   end record;
   overriding procedure Handle (Self : in out Receiver; Message : Input.Payload);
   overriding procedure Handle (Self : in out Receiver; Message : Input.Payload) is
   begin
      if Service_Codec.Encode_DLZ_MT (Message) /= To_String (Expected) then
         raise Program_Error with "typed payload or routing mismatch";
      end if;
      if Self.Witness then
         Witness_Calls := Witness_Calls + 1;
         Ada.Text_IO.Put_Line ("TASK101_WITNESS_HANDLER " & Witness_Calls'Image);
      else
         Calls := Calls + 1;
         Ada.Text_IO.Put_Line ("TASK101_TYPED_HANDLER " & Calls'Image);
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
     (Message_Namespace, Message_Name, Topic : String; Value : Model.DLZ_MT) return Boolean is
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
         Ada.Text_IO.Put_Line ("TASK101_WIRE_RECEIVED " & To_String (Delivery.Document));
      end if;
      Typed.Dispatch (Binding, Delivery);
      loop
         C.Next_Event (Core, Available, Item);
         exit when not Available;
         Ada.Text_IO.Put_Line ("TASK101_EVENT " & Item.Kind'Image & " " & To_String (Item.Detail));
         if Item.Kind = Decode_Error then
            if To_String (Item.Detail) /= To_String (Expected_Reject) then
               raise Program_Error with "unexpected codec rejection: " & To_String (Item.Detail);
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
   Ada.Text_IO.Put_Line ("TASK101_UPGRADE_OWP_COMPLETE");
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
   Ada.Text_IO.Put_Line ("TASK101_INFO_VALIDATED");
   Ada.Text_IO.Open (Documents, Ada.Text_IO.In_File, Ada.Command_Line.Argument (3));
   Ada.Text_IO.Open (Expectations, Ada.Text_IO.In_File, Ada.Command_Line.Argument (4));
   declare
      Base : constant Model.DLZ_MT := Service_Codec.Decode_DLZ_MT (Ada.Text_IO.Get_Line (Documents));
   begin
      Expected := To_Unbounded_String (Service_Codec.Encode_DLZ_MT (Base));
      Expected_Reject := To_Unbounded_String ("invalid named Time");
      SID := Subscriber.Subscribe (Handler'Access);
      Await_Command;
      if not C.Is_Active (Core, SID) then raise Program_Error with "SUB not accepted"; end if;
      Ada.Text_IO.Put_Line ("TASK101_SUB_ACCEPTED DLZ task098-dlz");
      begin
         Typed.Unsubscribe (Binding, SID + 100);
         raise Program_Error with "unknown binding SID accepted";
      exception
         when Runtime_Error => Ada.Text_IO.Put_Line ("TASK101_UNKNOWN_BINDING_SID_REJECTED");
      end;
      Ada.Text_IO.Put_Line ("TASK101_SUBSCRIBED");
      Ada.Text_IO.Flush;
      Await_Calls (1);
      if Rejected /= 1 then raise Program_Error with "missing decode rejection before recovery"; end if;
      Ada.Text_IO.Put_Line ("TASK101_DECODE_RECOVERY_COMPLETE");
      Sent := Publisher.Publish (Base);
      if not Sent then raise Program_Error with "not sent"; end if;
      Await_Command;
      Await_Calls (2);
      Ada.Text_IO.Put_Line ("TASK101_PUBLISHED " & To_String (Expected));
      while not Ada.Text_IO.End_Of_File (Documents) loop
         declare
            Value : constant Model.DLZ_MT := Service_Codec.Decode_DLZ_MT (Ada.Text_IO.Get_Line (Documents));
            Want : constant Natural := Calls + 1;
         begin
            Expected_Reject := To_Unbounded_String (Ada.Text_IO.Get_Line (Expectations));
            if Expected_Reject /= "" then
               raise Program_Error with "positive expectation missing";
            end if;
            Expected := To_Unbounded_String (Service_Codec.Encode_DLZ_MT (Value));
            Sent := Publisher.Publish (Value);
            if not Sent then raise Program_Error with "not sent"; end if;
            Ada.Text_IO.Put_Line ("TASK101_PUBLISHED " & To_String (Expected));
            Await_Command;
            Await_Calls (Want);
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
         Expected := To_Unbounded_String (Service_Codec.Encode_DLZ_MT (Base));
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
      Ada.Text_IO.Put_Line ("TASK101_UNSUB_WITNESS_COMPLETE primary_unchanged witness=1");
      Typed.Close (Binding);
      C.Close (Core);
      if C.State (Core) /= Closed or else Count_Entries ("/proc/self/fd") /= Before_FD
        or else Count_Entries ("/proc/self/task") /= Before_Tasks
      then raise Program_Error with "transport cleanup failed"; end if;
      Ada.Text_IO.Put_Line ("TASK101_CLEANUP_COMPLETE");
      Ada.Text_IO.Put_Line ("TASK101_ADA_REAL_DLZ_COMPLETE handlers=" & Calls'Image & " rejected=" & Rejected'Image);
   end;
exception
   when E : others =>
      C.Close (Core);
      Ada.Text_IO.Put_Line ("TASK101_FAILED " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (1);
end DLZ_Consumer;