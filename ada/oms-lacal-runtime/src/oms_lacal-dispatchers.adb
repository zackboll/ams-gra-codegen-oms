package body OMS_LACAL.Dispatchers is
   procedure Poll_And_Dispatch
     (Core : not null access Connections.Connection; Bindings : Binding_List)
   is
      Message : Connections.Delivery;
   begin
      for I in Bindings'Range loop
         if Bindings (I).Value = null then
            raise Runtime_Error with "null dispatch binding";
         elsif not Is_For (Bindings (I).Value.all, Core) then
            raise Runtime_Error with "foreign dispatch connection";
         end if;
         for J in Bindings'First .. I - 1 loop
            if Bindings (I).Value = Bindings (J).Value then
               raise Runtime_Error with "duplicate dispatch binding";
            end if;
         end loop;
      end loop;
      Connections.Poll (Core.all, Message);
      for Item of Bindings loop
         Dispatch (Item.Value.all, Message);
      end loop;
   end Poll_And_Dispatch;
end OMS_LACAL.Dispatchers;