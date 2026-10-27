with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Service_API;
with Service_Codec;
with TASK099_PACKAGE;
with OMS_LACAL; use OMS_LACAL;
with OMS_LACAL.Connections;
with OMS_LACAL.Envelopes;
with OMS_LACAL.Transports.Mock;
with OMS_LACAL.Typed;

procedure Runtime_Consumer is
   package Model renames TASK099_PACKAGE;
   package C renames OMS_LACAL.Connections;
   package M renames OMS_LACAL.Transports.Mock;
   package IA renames Service_API.Function_Io.Exchange_In_A;
   package IB renames Service_API.Function_Io.Exchange_In_B;
   Namespace : constant String := IA.Message_Namespace;
   Wire : aliased M.Mock_Transport;
   Core : aliased C.Connection (Wire'Access);
   Count : Natural := 0;
   procedure Check (Condition : Boolean; Label_Text : String) is
   begin
      if not Condition then raise Program_Error with Label_Text; end if;
      Count := Count + 1;
      Ada.Text_IO.Put_Line ("PASS " & Label_Text);
   end Check;
   function Is_Codec (Error : Ada.Exceptions.Exception_Occurrence) return Boolean is
      use type Ada.Exceptions.Exception_Id;
   begin return Ada.Exceptions.Exception_Identity (Error) = Service_Codec.Codec_Error'Identity; end;
   package A is new OMS_LACAL.Typed
     (Model.SharedPayload, IA.Handler, IA.Handle,
      Service_Codec.Encode_SharedPayload, Service_Codec.Decode_SharedPayload, Is_Codec);
   package B is new OMS_LACAL.Typed
     (Model.SharedPayload, IB.Handler, IB.Handle,
      Service_Codec.Encode_SharedPayload, Service_Codec.Decode_SharedPayload, Is_Codec);
   BA : A.Binding (Core'Access);
   BB : B.Binding (Core'Access);
   Calls_A, Calls_B : Natural := 0;
   Last : Text;
   Throw, Reenter : Boolean := False;
   type Receiver_A is new IA.Handler with null record;
   overriding procedure Handle (Self : in out Receiver_A; Message : IA.Payload);
   type Receiver_B is new IB.Handler with null record;
   overriding procedure Handle (Self : in out Receiver_B; Message : IB.Payload);
   overriding procedure Handle (Self : in out Receiver_A; Message : IA.Payload) is
      pragma Unreferenced (Self);
      D : C.Delivery;
   begin
      if Reenter then
         begin
            C.Poll (Core, D);
            raise Program_Error with "reentrant poll accepted";
         exception when Runtime_Error => null; end;
         begin
            C.Forget (Core, 1);
            raise Program_Error with "reentrant mapping removal accepted";
         exception when Runtime_Error => null; end;
         begin
            C.Close (Core);
            raise Program_Error with "reentrant connection close accepted";
         exception when Runtime_Error => null; end;
         begin
            A.Close (BA);
            raise Program_Error with "reentrant binding close accepted";
         exception when Runtime_Error => null; end;
      end if;
      if Throw then raise Constraint_Error with "handler test"; end if;
      Calls_A := Calls_A + 1;
      Last := To_Unbounded_String (Service_Codec.Encode_SharedPayload (Message));
   end Handle;
   overriding procedure Handle (Self : in out Receiver_B; Message : IB.Payload) is
      pragma Unreferenced (Self);
   begin
      Calls_B := Calls_B + 1;
      Last := To_Unbounded_String (Service_Codec.Encode_SharedPayload (Message));
   end Handle;
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
   function Key (Local : String) return String is (QName (Namespace, Local));
   function Body_Text (Number : String := "42"; Clock : String := "12:34:56.5000Z") return String is
     ("{""" & Key ("Count") & """:" & Number & ",""" & Key ("At") & """:""" & Clock &
      """,""" & Key ("Label") & """:""test""}");
   function Env (Name : String; Body_Value : String) return String is
     ("{""" & Key (Name) & """:" & Body_Value & "}");
   function Img (SID : Subscription_ID) return String is
     (Ada.Strings.Fixed.Trim (SID'Image, Ada.Strings.Both));
   procedure Receive (Frame : String) is
      D : C.Delivery;
   begin
      M.Enqueue (Wire, Frame);
      C.Poll (Core, D);
      A.Dispatch (BA, D);
      B.Dispatch (BB, D);
   end Receive;
   procedure Ack is begin Receive ("+OK"); end Ack;
   function Event_Count (Kind : Event_Kind) return Natural is
      Found : Natural := 0;
      Available : Boolean;
      Item : Event;
   begin
      loop
         C.Next_Event (Core, Available, Item);
         exit when not Available;
         if Item.Kind = Kind then Found := Found + 1; end if;
      end loop;
      return Found;
   end Event_Count;
   Info : constant String := "INFO {""version"":""1.0"",""server_id"":""mock""," &
     """uuids"":{""system"":""550e8400-e29b-41d4-a716-446655440000""," &
     """service"":""550e8400-e29b-41d4-a716-446655440001""},""system_label"":""test""}";
   SID_A, SID_B, Rejected : Subscription_ID;
   N : Natural;
   Value : constant Model.SharedPayload := Service_Codec.Decode_SharedPayload (Body_Text);
   Encoded : constant String := Service_Codec.Encode_SharedPayload (Value);
   Result : Boolean;
   procedure Boundary_Tests is
      W : aliased M.Mock_Transport;
      X : aliased C.Connection (W'Access);
      XA : A.Binding (X'Access);
      S : Subscription_ID;
      D : C.Delivery;
      procedure Step (Frame : String) is
      begin
         M.Enqueue (W, Frame);
         C.Poll (X, D);
         A.Dispatch (XA, D);
      end Step;
      procedure Bad_Peer (Frame : String; After_Init : Boolean := False) is
         Bad_Wire : aliased M.Mock_Transport;
         Bad_Core : C.Connection (Bad_Wire'Access);
         Item : C.Delivery;
      begin
         C.Initialize (Bad_Core, "2.5", "test-service", False);
         if After_Init then
            M.Enqueue (Bad_Wire, Info);
            C.Poll (Bad_Core, Item);
         end if;
         M.Enqueue (Bad_Wire, Frame);
         C.Poll (Bad_Core, Item);
         Check (C.State (Bad_Core) = Closed and M.Is_Closed (Bad_Wire),
                "invalid peer frame closes connection");
      end Bad_Peer;
   begin
      begin
         C.Initialize (X, "2.5", "bad service");
         raise Program_Error with "invalid INIT service accepted";
      exception when Runtime_Error => null; end;
      Check (M.Sent_Count (W) = 0 and C.State (X) = New_Connection,
             "invalid INIT rejected before send");
      begin
         C.Publish (X, Namespace, "MessageA", "topic", Body_Text);
         raise Program_Error with "publish before INFO accepted";
      exception when Runtime_Error => null; end;
      Check (M.Sent_Count (W) = 0, "pre-initialization publish rejected");
      C.Initialize (X, "2.5", "test-service", False);
      Step (Info);
      Check (C.State (X) = Active and not C.Busy (X), "nonverbose INIT INFO");
      S := A.Subscribe (XA, Namespace, "MessageA", "topic", False, "", RA'Access);
      Check (C.Is_Active (X, S), "nonverbose SUB locally active acceptance unknown");
      Step ("-ERR" & ASCII.HT & "Illegal-State" & ASCII.HT & "uncorrelated");
      Check (C.Is_Active (X, S), "nonverbose error does not invent SID correlation");
      N := Calls_A;
      Step (ASCII.HT & "MSG" & ASCII.HT & " 1 " & ASCII.HT & Env ("MessageA", Body_Text));
      Check (Calls_A = N + 1, "OWP space tab repeated delimiters");
      N := M.Sent_Count (W);
      declare
         --  The model's unrestricted string can contain invalid UTF-8 bytes;
         --  the real generated JSON encoder must reject before transport Send.
         Large : Model.SharedPayload := Value;
      begin
         Large.Label := To_Unbounded_String (String'(1 => Character'Val (255)));
         begin
            Result := A.Publish (X, Namespace, "MessageA", "topic", Large);
            raise Program_Error with "invalid UTF-8 encoded payload published";
         exception when Service_Codec.Codec_Error => null; end;
      end;
      Check (M.Sent_Count (W) = N and C.State (X) = Active,
             "real generated encoding failure no partial publication");
      declare
         Big : constant String (1 .. Max_Frame_Bytes) := (others => ' ');
      begin
         begin
            C.Publish (X, Namespace, "MessageA", "topic", "0" & Big);
            raise Program_Error with "oversized PUB accepted";
         exception when Envelope_Error => null; end;
      end;
      Check (M.Sent_Count (W) = N, "oversized PUB no partial send");
      A.Close (XA);
      Step ("MSG 1 " & Env ("MessageA", Body_Text));
      Check (not C.Is_Active (X, S) and not D.Available, "binding close removes mapping");
      S := A.Subscribe (XA, Namespace, "MessageA", "topic", False, "", RA'Access);
      M.Fail_Next_Send (W);
      begin
         C.Publish (X, Namespace, "MessageA", "topic", Body_Text);
         raise Program_Error with "failed send reported success";
      exception when Runtime_Error => null; end;
      Check (M.Sent_Count (W) = N + 1 and C.State (X) = Closed and not C.Is_Active (X, S),
             "send failure closes clears mappings no partial frame");
      Bad_Peer ("+OK");
      Bad_Peer ("MSG 1 {} ");
      Bad_Peer ("INFO {""version"":""9.0""}");
      Bad_Peer ("MSG bad/sid {}", True);
      Bad_Peer ("MSG 1", True);
      Bad_Peer ("-ERR Unknown-Code", True);
      Bad_Peer ("PUB topic {}", True);
      Bad_Peer (String'(1 .. Max_Frame_Bytes + 1 => 'X'), True);
      declare
         Final_Wire : aliased M.Mock_Transport;
      begin
         declare Final_Core : C.Connection (Final_Wire'Access);
         begin C.Initialize (Final_Core, "2.5", "scope-service", False); end;
         Check (M.Is_Closed (Final_Wire), "connection scope finalizes transport");
      end;
      declare
         Event_Wire : aliased M.Mock_Transport;
         Event_Core : C.Connection (Event_Wire'Access);
         Available : Boolean;
         Item : Event;
         Seen : Natural := 0;
      begin
         for I in 1 .. Max_Events + 2 loop C.Report (Event_Core, Information, 0, "bounded"); end loop;
         loop
            C.Next_Event (Event_Core, Available, Item);
            exit when not Available;
            Seen := Seen + 1;
         end loop;
         Check (Seen = Max_Events and C.Lost_Events (Event_Core) = 2, "bounded event queue explicit loss count");
      end;
   end Boundary_Tests;
begin
   C.Initialize (Core, "2.5", "test-service");
   Check (C.State (Core) = Initializing and C.Busy (Core), "INIT pending");
   Check (Ada.Strings.Fixed.Index (M.Sent (Wire, 1), """versions"":[""1.0""]") > 0,
          "INIT OWP 1.0");
   Ack;
   Receive (Info);
   Check (C.State (Core) = Active, "INFO initializes");
   Result := PA.Publish (Value);
   Check (Result and M.Sent (Wire, 2) = "PUB common-topic " & Env ("MessageA", Encoded),
          "generated PUB A exact envelope");
   Ack;
   Result := PB.Publish (Value);
   Check (Result and M.Sent (Wire, 3) = "PUB common-topic " & Env ("MessageB", Encoded),
          "same payload different global identity");
   Ack;
   SID_A := SA.Subscribe (RA'Access);
   Check (not C.Is_Active (Core, SID_A), "SUB not active before acceptance");
   Receive ("MSG 1 " & Env ("MessageA", Body_Text));
   if Calls_A /= 0 then raise Program_Error with "pending SUB dispatched"; end if;
   Check (M.Sent (Wire, 4) = "SUB 1 " & Key ("MessageA") & " common-topic", "SUB no group");
   Ack;
   SID_B := SB.Subscribe (RB'Access);
   Check (M.Sent (Wire, 5) = "SUB 2 " & Key ("MessageB") & " common-topic pool-7", "SUB group");
   Check (SID_A /= SID_B, "runtime unique SIDs");
   Ack;
   Check (C.Is_Active (Core, SID_A) and C.Is_Active (Core, SID_B) and not C.Busy (Core),
          "ownership regression both SUB acknowledged");
   N := M.Sent_Count (Wire);
   begin
      B.Unsubscribe (BB, SID_A);
      raise Program_Error with "foreign binding UNSUB accepted";
   exception when Runtime_Error =>
      Check (True, "foreign binding UNSUB rejected");
   end;
   Check (M.Sent_Count (Wire) = N and C.State (Core) = Active and not C.Busy (Core)
          and C.Is_Active (Core, SID_A) and C.Is_Active (Core, SID_B),
          "foreign UNSUB no frame or subscription state change");
   declare
      Unknown_SIDs : constant array (1 .. 2) of Subscription_ID :=
        (No_Subscription, Subscription_ID'Last);
   begin
      for SID of Unknown_SIDs loop
         begin
            B.Unsubscribe (BB, SID);
            raise Program_Error with "unknown binding UNSUB accepted";
         exception when Runtime_Error =>
            Check (True, "unknown binding UNSUB rejected" & SID'Image);
         end;
      end loop;
   end;
   Check (M.Sent_Count (Wire) = N and C.State (Core) = Active and not C.Busy (Core)
          and C.Is_Active (Core, SID_A) and C.Is_Active (Core, SID_B),
          "unknown UNSUB no frame or subscription state change");
   Receive ("MSG " & Img (SID_A) & " " & Env ("MessageA", Body_Text));
   Receive ("MSG " & Img (SID_B) & " " & Env ("MessageB", Body_Text));
   Check (Calls_A = 1 and Calls_B = 1 and To_String (Last) = Encoded, "typed dispatch and values");
   N := Event_Count (Decode_Error);
   Receive ("MSG 1 " & Env ("MessageB", Body_Text));
   Receive ("MSG 1 {""" & Key ("MessageA") & """:" & Body_Text &
            ",""" & Key ("MessageA") & """:" & Body_Text & "}");
   Receive ("MSG 1 {");
   Receive ("MSG 1 []");
   Receive ("MSG 1 {}");
   Receive ("MSG 1 " & Env ("MessageA", Body_Text) & " trailing");
   Receive ("MSG 1 " & Env ("MessageA", Body_Text (Clock => "12:00:00+01:00")));
   Check (Calls_A = 1 and Event_Count (Decode_Error) = 7 and C.Is_Active (Core, SID_A),
          "wrong duplicate malformed TimeZulu reject without removing subscription");
   Receive ("MSG 1 " & Env ("MessageA", Body_Text));
   Check (Calls_A = 2, "valid delivery after invalid documents");
   Receive ("MSG foreign-sid " & Env ("MessageA", Body_Text));
   Check (Event_Count (Unknown_Subscription) = 1 and Calls_A = 2, "unknown SID no dispatch");
   N := M.Sent_Count (Wire);
   begin
      Rejected := Sub_A (Namespace, "MessageA", "bad topic", False, "", RA'Access);
      raise Program_Error with "invalid topic accepted";
   exception when Runtime_Error => null; end;
   begin
      Rejected := Sub_A (Namespace, "MessageA", "topic", True, "bad group", RA'Access);
      raise Program_Error with "invalid group accepted";
   exception when Runtime_Error => null; end;
   Check (M.Sent_Count (Wire) = N, "invalid identifiers rejected before send");
   declare
      Inner : aliased Receiver_A;
   begin
      Rejected := SA.Subscribe (Inner'Access);
      raise Constraint_Error with "short-lived receiver retained";
   exception when Program_Error =>
      Check (M.Sent_Count (Wire) = N, "checked receiver lifetime before SUB");
   end;
   Rejected := SA.Subscribe (RA'Access);
   Receive ("-ERR Illegal-Argument synthetic rejection");
   Receive ("MSG " & Img (Rejected) & " " & Env ("MessageA", Body_Text));
   Check (not C.Is_Active (Core, Rejected) and Calls_A = 2, "rejected SUB no active mapping");
   SID_B := SB.Subscribe (RB'Access);
   Ack;
   Check (SID_B > Rejected, "rejected SID never reused");
   Throw := True;
   Receive ("MSG 1 " & Env ("MessageA", Body_Text));
   Check (Event_Count (Handler_Error) = 1 and Calls_A = 2, "handler exception observable");
   Throw := False;
   Reenter := True;
   Receive ("MSG 1 " & Env ("MessageA", Body_Text));
   Check (Calls_A = 3 and Event_Count (Handler_Error) = 0, "reentrant operations rejected safely");
   Reenter := False;
   N := M.Sent_Count (Wire);
   begin
      C.Publish (Core, Namespace, "MessageA", "topic", "{invalid");
      raise Program_Error with "invalid encoding sent";
   exception when Envelope_Error => null; end;
   Check (M.Sent_Count (Wire) = N, "invalid encoded document no partial publication");
   N := M.Sent_Count (Wire);
   A.Unsubscribe (BA, SID_A);
   Check (M.Sent_Count (Wire) = N + 1 and M.Sent (Wire, N + 1) = "UNSUB " & Img (SID_A)
          and not C.Is_Active (Core, SID_A) and C.Is_Active (Core, SID_B) and C.Busy (Core),
          "owning binding UNSUB sends exact frame removes only owned subscription");
   Ack;
   N := M.Sent_Count (Wire);
   begin
      A.Unsubscribe (BA, SID => SID_A);
      raise Program_Error with "retired binding UNSUB accepted";
   exception when Runtime_Error =>
      Check (True, "retired binding UNSUB rejected");
   end;
   Check (M.Sent_Count (Wire) = N and not C.Busy (Core)
          and not C.Is_Active (Core, SID_A) and C.Is_Active (Core, SID_B),
          "retired UNSUB no frame or subscription state change");
   Receive ("MSG 1 " & Env ("MessageA", Body_Text));
   Check (Calls_A = 3 and Event_Count (Unknown_Subscription) = 1, "UNSUB mapping removed");
   Receive ("-ERR Invalid-Message late error");
   Check (Event_Count (Server_Error) = 1 and C.State (Core) = Active, "late uncorrelated error event");
   Check (OMS_LACAL.Envelopes.Extract ("x", "{""x"":-0}") = "-0" and then
     OMS_LACAL.Envelopes.Extract ("x", "{""x"":4.9406564584124654e-324}") = "4.9406564584124654e-324"
     and then OMS_LACAL.Envelopes.Extract ("x", "{""x"":1.7976931348623157e308}") = "1.7976931348623157e308",
     "numeric lexemes untouched");
   M.Disconnect (Wire);
   declare D : C.Delivery; begin C.Poll (Core, D); end;
   Check (C.State (Core) = Closed and M.Is_Closed (Wire) and not C.Is_Active (Core, SID_B),
          "connection loss deterministic cleanup");
   A.Close (BA);
   B.Close (BB);
   C.Close (Core);
   Boundary_Tests;
   Ada.Text_IO.Put_Line ("TASK099_CONSUMER_COMPLETE checks=" & Count'Image);
end Runtime_Consumer;