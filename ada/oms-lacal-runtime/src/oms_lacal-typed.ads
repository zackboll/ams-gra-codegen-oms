with Ada.Finalization;
with Ada.Exceptions;
with OMS_LACAL.Connections;
with OMS_LACAL.Dispatchers;

--  Instantiate at the receiver's owning scope, once for each generated input
--  Handler interface. The named receiver access type enforces that scope.
generic
   type Payload is private;
   type Handler is limited interface;
   with procedure Handle (Self : in out Handler; Message : Payload) is abstract;
   with function Encode (Item : Payload) return String;
   with function Decode (Document : String) return Payload;
   with function Is_Codec_Error
     (Error : Ada.Exceptions.Exception_Occurrence) return Boolean;
package OMS_LACAL.Typed is
   type Binding (Core : not null access Connections.Connection) is
     new Ada.Finalization.Limited_Controlled and Dispatchers.Participant with private;
   overriding function Is_For
     (Self : Binding; Core : not null access Connections.Connection) return Boolean;
   function Subscribe
     (Self : in out Binding; Namespace, Name, Topic : String;
      Has_Group : Boolean; Group : String;
      Receiver : not null access Handler'Class) return Subscription_ID;
   overriding procedure Dispatch (Self : in out Binding; Message : Connections.Delivery);
   --  Only a SID registered by this binding may be removed. Unknown/foreign
   --  SIDs raise Runtime_Error before wire traffic or subscription changes.
   procedure Unsubscribe (Self : in out Binding; SID : Subscription_ID);
   procedure Close (Self : in out Binding);
   function Publish
     (Core : in out Connections.Connection;
      Namespace, Name, Topic : String; Value : Payload) return Boolean;
private
   type Handler_Access is access all Handler'Class;
   type Slot is record
      SID : Subscription_ID := No_Subscription;
      Receiver : Handler_Access;
      Name : Text;
   end record;
   type Slots is array (1 .. Max_Subscriptions) of Slot;
   type Binding (Core : not null access Connections.Connection) is
     new Ada.Finalization.Limited_Controlled and Dispatchers.Participant with record
      Table : Slots;
      Dispatching : Boolean := False;
   end record;
   overriding procedure Finalize (Self : in out Binding);
end OMS_LACAL.Typed;