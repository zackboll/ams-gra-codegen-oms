with OMS_LACAL.Connections;

package OMS_LACAL.Dispatchers is
   type Participant is limited interface;
   function Is_For
     (Self : Participant; Core : not null access Connections.Connection)
      return Boolean is abstract;
   procedure Dispatch
     (Self : in out Participant; Message : Connections.Delivery) is abstract;

   --  Access discriminants borrow even locally declared typed bindings without
   --  bypassing accessibility checks. Neither the list nor its elements escape.
   type Reference (Value : access Participant'Class := null) is limited null record;
   type Binding_List is array (Positive range <>) of Reference;

   --  Validate the entire list (including null/duplicate/foreign entries) before
   --  exactly one Poll. Runtime_Error leaves queued frames untouched on failure.
   procedure Poll_And_Dispatch
     (Core : not null access Connections.Connection; Bindings : Binding_List);
end OMS_LACAL.Dispatchers;