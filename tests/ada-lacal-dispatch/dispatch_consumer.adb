with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.Oam;
with Service_API;
with Service_Codec;
with OMS_LACAL; use OMS_LACAL;
with OMS_LACAL.Connections;
with OMS_LACAL.Dispatchers;
with OMS_LACAL.Transports.Mock;
with OMS_LACAL.Typed;

procedure Dispatch_Consumer is
   package C renames OMS_LACAL.Connections;
   package D renames OMS_LACAL.Dispatchers;
   package M renames OMS_LACAL.Transports.Mock;
   package IL renames Service_API.Function_Io.Exchange_Log;
   package IR renames Service_API.Function_Io.Exchange_Rdma;
   package ISK renames Service_API.Function_Io.Exchange_Risk;
   Wire, Foreign_Wire : aliased M.Mock_Transport;
   Core : aliased C.Connection (Wire'Access);
   Foreign_Core : aliased C.Connection (Foreign_Wire'Access);
   Checks : Natural := 0;
   procedure Check (OK : Boolean; Label_Text : String) is
   begin
      if not OK then raise Program_Error with Label_Text; end if;
      Checks := Checks + 1;
      Ada.Text_IO.Put_Line ("PASS " & Label_Text);
   end Check;
   function Is_Codec (Error : Ada.Exceptions.Exception_Occurrence) return Boolean is
      use type Ada.Exceptions.Exception_Id;
   begin
      return Ada.Exceptions.Exception_Identity (Error) = Service_Codec.Codec_Error'Identity;
   end Is_Codec;
   package Log_Binding is new OMS_LACAL.Typed
     (Programs.Oam.LogPayload, IL.Handler, IL.Handle,
      Service_Codec.Encode_LogPayload, Service_Codec.Decode_LogPayload, Is_Codec);
   package Rdma_Binding is new OMS_LACAL.Typed
     (Programs.Oam.RdmaPayload, IR.Handler, IR.Handle,
      Service_Codec.Encode_RdmaPayload, Service_Codec.Decode_RdmaPayload, Is_Codec);
   package Risk_Binding is new OMS_LACAL.Typed
     (Programs.Oam.RiskPayload, ISK.Handler, ISK.Handle,
      Service_Codec.Encode_RiskPayload, Service_Codec.Decode_RiskPayload, Is_Codec);
   LB : aliased Log_Binding.Binding (Core'Access);
   RB : aliased Rdma_Binding.Binding (Core'Access);
   SB : aliased Risk_Binding.Binding (Core'Access);
   FB : aliased Log_Binding.Binding (Foreign_Core'Access);
   type Failing_Participant is new D.Participant with null record;
   overriding function Is_For
     (Self : Failing_Participant; Connection : not null access C.Connection) return Boolean;
   overriding procedure Dispatch (Self : in out Failing_Participant; Message : C.Delivery);
   overriding function Is_For
     (Self : Failing_Participant; Connection : not null access C.Connection) return Boolean is
      pragma Unreferenced (Self);
   begin return Log_Binding.Is_For (LB, Connection); end Is_For;
   overriding procedure Dispatch (Self : in out Failing_Participant; Message : C.Delivery) is
      pragma Unreferenced (Self, Message);
   begin raise Constraint_Error with "unexpected dispatch failure"; end Dispatch;
   Failure : aliased Failing_Participant;
   Calls_L, Calls_R, Calls_S : Natural := 0;
   Order : Text;
   Throw, Reenter : Boolean := False;
   procedure Poll is
   begin
      D.Poll_And_Dispatch
        (Core'Access, ((Value => LB'Access), (Value => RB'Access), (Value => SB'Access)));
   end Poll;
   type Log_Receiver is new IL.Handler with null record;
   overriding procedure Handle (Self : in out Log_Receiver; Message : IL.Payload);
   type Rdma_Receiver is new IR.Handler with null record;
   overriding procedure Handle (Self : in out Rdma_Receiver; Message : IR.Payload);
   type Risk_Receiver is new ISK.Handler with null record;
   overriding procedure Handle (Self : in out Risk_Receiver; Message : ISK.Payload);
   overriding procedure Handle (Self : in out Log_Receiver; Message : IL.Payload) is
      pragma Unreferenced (Self);
   begin
      if Throw then raise Constraint_Error with "intentional handler failure"; end if;
      if Reenter then
         begin
            Poll;
            raise Program_Error with "reentrant dispatch accepted";
         exception when Runtime_Error => null; end;
         begin
            Risk_Binding.Close (SB);
            raise Program_Error with "cross-binding reentrant close accepted";
         exception when Runtime_Error => null; end;
         begin
            Log_Binding.Close (LB);
            raise Program_Error with "reentrant close accepted";
         exception when Runtime_Error => null; end;
      end if;
      if To_String (Message.Text) /= "independent" then
         raise Program_Error with "wrong log payload";
      end if;
      Calls_L := Calls_L + 1;
      Append (Order, "L");
   end Handle;
   overriding procedure Handle (Self : in out Rdma_Receiver; Message : IR.Payload) is
      pragma Unreferenced (Self);
   begin
      if Message.Channel /= 17 then raise Program_Error with "wrong rdma payload"; end if;
      Calls_R := Calls_R + 1;
      Append (Order, "R");
   end Handle;
   overriding procedure Handle (Self : in out Risk_Receiver; Message : ISK.Payload) is
      pragma Unreferenced (Self);
   begin
      if not Message.Allowed then raise Program_Error with "wrong risk payload"; end if;
      Calls_S := Calls_S + 1;
      Append (Order, "S");
   end Handle;
   LR : aliased Log_Receiver;
   RR : aliased Rdma_Receiver;
   SR : aliased Risk_Receiver;
   function Sub_Log (Namespace, Name, Topic : String; Has_Group : Boolean;
                     Group : String; Receiver : not null access IL.Handler'Class)
                     return Subscription_ID is
     (Log_Binding.Subscribe (LB, Namespace, Name, Topic, Has_Group, Group, Receiver));
   function Sub_Rdma (Namespace, Name, Topic : String; Has_Group : Boolean;
                      Group : String; Receiver : not null access IR.Handler'Class)
                      return Subscription_ID is
     (Rdma_Binding.Subscribe (RB, Namespace, Name, Topic, Has_Group, Group, Receiver));
   function Sub_Risk (Namespace, Name, Topic : String; Has_Group : Boolean;
                      Group : String; Receiver : not null access ISK.Handler'Class)
                      return Subscription_ID is
     (Risk_Binding.Subscribe (SB, Namespace, Name, Topic, Has_Group, Group, Receiver));
   package SL is new IL.Subscriber (Subscription_ID, Sub_Log);
   package SRD is new IR.Subscriber (Subscription_ID, Sub_Rdma);
   package SS is new ISK.Subscriber (Subscription_ID, Sub_Risk);
   function Publish_Log (Namespace, Name, Topic : String; Value : IL.Payload) return Boolean is
     (Log_Binding.Publish (Core, Namespace, Name, Topic, Value));
   package PL is new Service_API.Function_Io.Exchange_Publish.Publisher (Boolean, Publish_Log);

   --  Independent documents, not encoder-produced fixtures.
   Log_JSON : constant String := "{""Log"":{""Text"":""independent""}}";
   Rdma_JSON : constant String := "{""Rdma"":{""Channel"":17}}";
   Risk_JSON : constant String := "{""Risk"":{""Allowed"":true}}";
   Info : constant String := "INFO {""version"":""1.0"",""server_id"":""mock""," &
     """uuids"":{""system"":""550e8400-e29b-41d4-a716-446655440000""," &
     """service"":""550e8400-e29b-41d4-a716-446655440001""},""system_label"":""test""}";
   L, R, S : Subscription_ID;
   function Img (SID : Subscription_ID) return String is
     (Ada.Strings.Fixed.Trim (SID'Image, Ada.Strings.Both));
   procedure Queue (SID : Subscription_ID; JSON : String) is
   begin M.Enqueue (Wire, "MSG " & Img (SID) & " " & JSON); end Queue;
   procedure Receive (Frame : String) is
   begin M.Enqueue (Wire, Frame); Poll; end Receive;
   procedure Events (Kind : Event_Kind; SID : Subscription_ID := No_Subscription) is
      Available : Boolean;
      Item : Event;
   begin
      C.Next_Event (Core, Available, Item);
      Check (Available and then Item.Kind = Kind and then Item.SID = SID, "expected " & Kind'Image);
      C.Next_Event (Core, Available, Item);
      Check (not Available and C.Lost_Events (Core) = 0, "no unexpected or lost events");
   end Events;
   N, Before : Natural;
   Result : Boolean;
   procedure Lifetime_And_Empty_Tests is
      W : aliased M.Mock_Transport;
      X : aliased C.Connection (W'Access);
      B : aliased Log_Binding.Binding (X'Access);
      SID : Subscription_ID;
   begin
      C.Initialize (X, "2.5", "pending");
      M.Enqueue (W, "+OK"); M.Enqueue (W, Info);
      D.Poll_And_Dispatch (X'Access, (1 => (Value => B'Access)));
      D.Poll_And_Dispatch (X'Access, (1 => (Value => B'Access)));
      SID := Log_Binding.Subscribe (B, IL.Message_Namespace, IL.Message_Name,
        IL.Topic, False, "", LR'Access);
      M.Enqueue (W, "MSG " & Img (SID) & " " & Log_JSON);
      D.Poll_And_Dispatch (X'Access, (1 => (Value => B'Access)));
      Check (not C.Is_Active (X, SID) and Calls_L = 0, "pending SID never invokes callback");
      M.Enqueue (W, "+OK");
      D.Poll_And_Dispatch (X'Access, (1 => (Value => B'Access)));
      Check (C.Is_Active (X, SID), "pending subscription acknowledged normally");
      M.Enqueue (W, "MSG " & Img (SID) & " " & Log_JSON);
      M.Enqueue (W, "MSG " & Img (SID) & " " & Log_JSON);
      D.Poll_And_Dispatch (X'Access, (1 .. 0 => (Value => null)));
      Check (Calls_L = 0, "empty list polls once without callbacks");
      D.Poll_And_Dispatch (X'Access, (1 => (Value => B'Access)));
      Check (Calls_L = 1, "empty list consumed exactly one queued frame");
      declare
         Inner : aliased Log_Receiver;
      begin
         SID := Log_Binding.Subscribe (B, IL.Message_Namespace, IL.Message_Name,
           IL.Topic, False, "", Inner'Access);
         raise Constraint_Error with "short-lived receiver accepted";
      exception when Program_Error =>
         Check (M.Sent_Count (W) = 2, "receiver accessibility preserved before SUB");
      end;
      Calls_L := 0;
      Order := Null_Unbounded_String;
   end Lifetime_And_Empty_Tests;
begin
   Lifetime_And_Empty_Tests;
   C.Initialize (Core, "2.5", "task107");
   Receive ("+OK");
   Events (Command_Accepted);
   Receive (Info);
   Events (Initialized);
   L := SL.Subscribe (LR'Access); Receive ("+OK"); Events (Command_Accepted, L);
   R := SRD.Subscribe (RR'Access); Receive ("+OK"); Events (Command_Accepted, R);
   S := SS.Subscribe (SR'Access); Receive ("+OK"); Events (Command_Accepted, S);
   Check (M.Sent_Count (Wire) = 4 and C.State (Core) = Active and
     C.Is_Active (Core, L) and C.Is_Active (Core, R) and C.Is_Active (Core, S) and
     L /= R and L /= S and R /= S, "one INIT and three distinct active subscriptions");
   Result := PL.Publish ((Text => To_Unbounded_String ("independent")));
   Check (Result and M.Sent (Wire, 5) = "PUB dispatch " & Log_JSON, "generated publisher uses existing codec");
   Receive ("+OK"); Events (Command_Accepted);

   Queue (R, Rdma_JSON); Queue (L, Log_JSON); Queue (S, Risk_JSON); Queue (R, Rdma_JSON);
   Poll; Check (To_String (Order) = "R", "exactly one frame first poll");
   Poll; Check (To_String (Order) = "RL", "second poll ordered and isolated");
   Poll; Check (To_String (Order) = "RLS", "third concrete handler");
   Poll; Check (To_String (Order) = "RLSR" and Calls_L = 1 and Calls_R = 2 and Calls_S = 1,
     "successive polls preserve interleaved order");
   Queue (L, "{""Log"":{""Text"":17}}"); Poll;
   Check (Calls_L = 1, "malformed payload no successful callback"); Events (Decode_Error, L);
   Queue (L, Log_JSON); Poll; Check (Calls_L = 2, "valid after codec error");
   Queue (L, Rdma_JSON); Poll;
   Check (Calls_R = 2 and Calls_L = 2, "wrong QName never cross-dispatches"); Events (Decode_Error, L);
   Throw := True; Queue (L, Log_JSON); Poll;
   Check (Calls_L = 2, "throwing handler not successful"); Events (Handler_Error, L);
   Throw := False; Queue (S, Risk_JSON); Poll; Check (Calls_S = 2, "other subscription survives handler error");
   Reenter := True; Queue (L, Log_JSON); Queue (S, Risk_JSON); Poll;
   Check (Calls_L = 3 and Calls_S = 2, "reentrancy rejected without consuming next frame");
   Reenter := False; Poll; Check (Calls_S = 3, "guard released and sibling unchanged");

   Queue (999, Log_JSON); Poll; Events (Unknown_Subscription);
   Check (Calls_L = 3 and Calls_R = 2 and Calls_S = 3, "unknown SID no callbacks");
   N := M.Sent_Count (Wire);
   begin Log_Binding.Unsubscribe (LB, R); raise Program_Error;
   exception when Runtime_Error => null; end;
   begin Rdma_Binding.Unsubscribe (RB, S); raise Program_Error;
   exception when Runtime_Error => null; end;
   begin Risk_Binding.Unsubscribe (SB, L); raise Program_Error;
   exception when Runtime_Error => null; end;
   begin Log_Binding.Unsubscribe (LB, 999); raise Program_Error;
   exception when Runtime_Error => null; end;
   Check (M.Sent_Count (Wire) = N and not C.Busy (Core) and C.Is_Active (Core, L)
     and C.Is_Active (Core, R) and C.Is_Active (Core, S), "foreign and unknown UNSUB no authority or traffic");

   Queue (R, Rdma_JSON);
   begin
      D.Poll_And_Dispatch (Core'Access, ((Value => LB'Access), (Value => RB'Access), (Value => LB'Access)));
      raise Program_Error with "duplicate accepted";
   exception when Runtime_Error => null; end;
   Check (Calls_R = 2, "duplicate rejected before callback");
   Poll; Check (Calls_R = 3, "duplicate rejection preserves queued frame");
   Queue (S, Risk_JSON);
   begin
      D.Poll_And_Dispatch (Core'Access, ((Value => LB'Access), (Value => FB'Access)));
      raise Program_Error with "foreign connection accepted";
   exception when Runtime_Error => null; end;
   Check (M.Sent_Count (Foreign_Wire) = 0 and C.State (Foreign_Core) = New_Connection,
     "foreign list rejected without touching either connection");
   Poll; Check (Calls_S = 4, "foreign rejection preserves queued frame");
   Queue (R, Rdma_JSON);
   begin
      D.Poll_And_Dispatch (Core'Access, (1 => (Value => null)));
      raise Program_Error with "null accepted";
   exception when Runtime_Error => null; end;
   D.Poll_And_Dispatch (Core'Access, (7 => (Value => RB'Access)));
   Check (Calls_R = 4, "null rejection and non-one list lower bound");
   begin
      D.Poll_And_Dispatch (Core'Access, (1 => (Value => Failure'Access)));
      raise Program_Error with "unexpected exception swallowed";
   exception when Constraint_Error =>
      Check (True, "unexpected dispatcher exception propagates");
   end;

   Log_Binding.Unsubscribe (LB, L); Receive ("+OK"); Events (Command_Accepted, L);
   Queue (L, Log_JSON); Poll; Events (Unknown_Subscription);
   Queue (R, Rdma_JSON); Queue (S, Risk_JSON); Poll; Poll;
   Check (Calls_L = 3 and Calls_R = 5 and Calls_S = 5, "removing log leaves both siblings operational");
   Rdma_Binding.Close (RB);
   Queue (R, Rdma_JSON); Poll; Events (Unknown_Subscription);
   Queue (S, Risk_JSON); Poll;
   Check (Calls_R = 5 and Calls_S = 6, "explicit binding close clears only its callbacks");
   declare
      Temp : aliased Log_Binding.Binding (Core'Access);
      SID : Subscription_ID;
   begin
      SID := Log_Binding.Subscribe (Temp, IL.Message_Namespace, IL.Message_Name,
        IL.Topic, False, "", LR'Access);
      Receive ("+OK"); Events (Command_Accepted, SID);
      Queue (SID, Log_JSON);
      D.Poll_And_Dispatch (Core'Access, (1 => (Value => Temp'Access)));
      Check (Calls_L = 4, "shorter-lived borrowed binding dispatches safely");
      Queue (SID, Log_JSON);
   end;
   Poll; Events (Unknown_Subscription);
   Queue (S, Risk_JSON); Poll;
   Check (Calls_L = 4 and Calls_S = 7, "finalization forgets local binding with no retained reference");
   Before := Calls_L + Calls_R + Calls_S;
   C.Close (Core); Events (Connection_Closed);
   D.Poll_And_Dispatch (Core'Access, (1 .. 0 => (Value => null)));
   Poll;
   Check (M.Is_Closed (Wire) and not C.Is_Active (Core, S) and
     Calls_L + Calls_R + Calls_S = Before, "closed core and empty list no callbacks");
   Log_Binding.Close (LB); Risk_Binding.Close (SB);
   declare
      W : aliased M.Mock_Transport;
   begin
      declare
         X : aliased C.Connection (W'Access);
         B : aliased Risk_Binding.Binding (X'Access);
         SID : Subscription_ID;
      begin
         C.Initialize (X, "2.5", "finalize", False);
         M.Enqueue (W, Info);
         D.Poll_And_Dispatch (X'Access, (1 => (Value => B'Access)));
         SID := Risk_Binding.Subscribe (B, ISK.Message_Namespace, ISK.Message_Name,
           ISK.Topic, False, "", SR'Access);
         M.Enqueue (W, "MSG " & Img (SID) & " " & Risk_JSON);
      end;
      Check (M.Is_Closed (W) and Calls_L + Calls_R + Calls_S = Before,
        "binding then connection finalization closes without callbacks");
   end;
   Ada.Text_IO.Put_Line ("TASK107_DISPATCH_COMPLETE checks=" & Checks'Image);
end Dispatch_Consumer;