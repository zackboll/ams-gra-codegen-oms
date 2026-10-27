with Ada.Strings.Unbounded;

package OMS_LACAL is
   subtype Text is Ada.Strings.Unbounded.Unbounded_String;
   Max_Frame_Bytes : constant := 1_048_576;
   Max_Subscriptions : constant := 128;
   Max_Events : constant := 128;
   Runtime_Error : exception;
   Envelope_Error : exception;

   type Subscription_ID is range 0 .. Long_Long_Integer'Last;
   No_Subscription : constant Subscription_ID := 0;
   type Connection_State is (New_Connection, Initializing, Active, Closed);
   type Event_Kind is
     (Initialized, Information, Command_Accepted, Server_Error,
      Protocol_Error, Decode_Error, Handler_Error, Unknown_Subscription,
      Connection_Closed);
   type Event is record
      Kind : Event_Kind := Information;
      SID : Subscription_ID := No_Subscription;
      Detail : Text;
   end record;
   --  No URI normalization: this is global-element identity, not payload type.
   function QName (Namespace, Local_Name : String) return String;
   function Valid_Identifier (Value : String) return Boolean;
end OMS_LACAL;