with Ada.Finalization;
with OMS_LACAL.Transports;

package OMS_LACAL.Connections is
   type Connection (Wire : not null access Transports.Transport'Class) is
     new Ada.Finalization.Limited_Controlled with private;
   procedure Initialize
     (Self : in out Connection; Schema, Service_ID : String;
      Verbose : Boolean := True);
   function State (Self : Connection) return Connection_State;
   function Busy (Self : Connection) return Boolean;
   procedure Publish
     (Self : in out Connection; Namespace, Name, Topic, Payload : String);
   function Subscribe
     (Self : in out Connection; Namespace, Name, Topic : String;
      Has_Group : Boolean; Group : String) return Subscription_ID;
   procedure Unsubscribe (Self : in out Connection; SID : Subscription_ID);
   function Is_Active (Self : Connection; SID : Subscription_ID) return Boolean;
   --  Poll never invokes a callback or retains an access to a handler.
   type Delivery is record
      Available : Boolean := False;
      SID : Subscription_ID := No_Subscription;
      Name, Document : Text;
   end record;
   procedure Poll (Self : in out Connection; Message : out Delivery);
   procedure Close (Self : in out Connection);
   procedure Next_Event (Self : in out Connection; Available : out Boolean; Item : out Event);
   function Lost_Events (Self : Connection) return Natural;
   procedure Report
     (Self : in out Connection; Kind : Event_Kind; SID : Subscription_ID; Detail : String);
   --  Synchronous borrowed callback. Never retained. The guard is released
   --  internally, including exception unwinding; callers cannot release it.
   procedure Dispatch
     (Self : in out Connection; SID : Subscription_ID;
      Process : not null access procedure);
   --  Lifetime cleanup only, no wire operation and no user code.
   procedure Forget (Self : in out Connection; SID : Subscription_ID);
private
   type Entry_State is (Unused, Pending, Registered);
   type Subscription_Entry is record
      Status : Entry_State := Unused;
      SID : Subscription_ID := No_Subscription;
      Name : Text;
   end record;
   type Entries is array (1 .. Max_Subscriptions) of Subscription_Entry;
   type Events is array (1 .. Max_Events) of Event;
   type Pending_Kind is (None, Init_Command, Publish_Command, Subscribe_Command, Unsubscribe_Command);
   type Connection (Wire : not null access Transports.Transport'Class) is
     new Ada.Finalization.Limited_Controlled with record
      Current : Connection_State := New_Connection;
      Verbose : Boolean := True;
      Waiting : Pending_Kind := None;
      Pending_SID : Subscription_ID := No_Subscription;
      Next_SID : Subscription_ID := 1;
      Exhausted, Dispatching, Polling : Boolean := False;
      Table : Entries;
      Queue : Events;
      Head : Positive := 1;
      Count, Dropped : Natural := 0;
   end record;
   overriding procedure Finalize (Self : in out Connection);
end OMS_LACAL.Connections;