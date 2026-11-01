with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with OMS_LACAL.Envelopes;

package body OMS_LACAL.Typed is
   overriding function Is_For
     (Self : Binding; Core : not null access Connections.Connection) return Boolean is
     (Self.Core = Core);

   function Subscribe
     (Self : in out Binding; Namespace, Name, Topic : String;
      Has_Group : Boolean; Group : String;
      Receiver : not null access Handler'Class) return Subscription_ID
   is
      --  Checked *before* sending or registering anything. A receiver declared
      --  in a deeper master than this generic instance raises Program_Error.
      Safe : constant Handler_Access := Handler_Access (Receiver);
      I : Natural := 0;
      SID : Subscription_ID;
      Expected : constant Text := To_Unbounded_String (QName (Namespace, Name));
   begin
      if Self.Dispatching then raise Runtime_Error with "reentrant binding operation"; end if;
      for J in Self.Table'Range loop
         if Self.Table (J).SID = No_Subscription or else
           not Connections.Is_Active (Self.Core.all, Self.Table (J).SID)
         then
            --  Do not discard a pending entry awaiting its SUB acknowledgment.
            if not Connections.Busy (Self.Core.all) then I := J; exit; end if;
         end if;
      end loop;
      if I = 0 then raise Runtime_Error with "binding busy or full"; end if;
      SID := Connections.Subscribe (Self.Core.all, Namespace, Name, Topic, Has_Group, Group);
      Self.Table (I) := (SID, Safe, Expected);
      return SID;
   end Subscribe;

   procedure Dispatch (Self : in out Binding; Message : Connections.Delivery) is
   begin
      if Self.Dispatching then raise Runtime_Error with "reentrant binding dispatch"; end if;
      if not Message.Available then return; end if;
      for E of Self.Table loop
         if E.SID = Message.SID and E.Receiver /= null then
            if not Connections.Is_Active (Self.Core.all, E.SID) then return; end if;
            declare
               procedure Process is
               begin
                  Self.Dispatching := True;
                  begin
                     declare
                        Document : constant String := Envelopes.Extract
                          (To_String (E.Name), To_String (Message.Document));
                        Value : constant Payload := Decode (Document);
                     begin
                        begin
                           Handle (E.Receiver.all, Value);
                        exception
                           when Error : others =>
                              Connections.Report (Self.Core.all, Handler_Error, E.SID,
                                Ada.Exceptions.Exception_Information (Error));
                        end;
                     end;
                  exception
                     when Error : Envelope_Error =>
                        Connections.Report (Self.Core.all, Decode_Error, E.SID,
                          Ada.Exceptions.Exception_Message (Error));
                     when Error : others =>
                        if Is_Codec_Error (Error) then
                           Connections.Report (Self.Core.all, Decode_Error, E.SID,
                             Ada.Exceptions.Exception_Message (Error));
                        else
                           raise;
                        end if;
                  end;
                  Self.Dispatching := False;
               exception
                  when others => Self.Dispatching := False; raise;
               end Process;
            begin
               Connections.Dispatch (Self.Core.all, E.SID, Process'Access);
            end;
            return;
         end if;
      end loop;
   end Dispatch;

   procedure Unsubscribe (Self : in out Binding; SID : Subscription_ID) is
   begin
      for E of Self.Table loop
         --  A shared connection's SID is not authority to remove another
         --  binding's subscription. Empty slots never establish ownership.
         if E.SID = SID and E.Receiver /= null then
            Connections.Unsubscribe (Self.Core.all, SID);
            E := (others => <>);
            return;
         end if;
      end loop;
      raise Runtime_Error with "subscription not owned by binding";
   end Unsubscribe;

   procedure Close (Self : in out Binding) is
   begin
      if Self.Dispatching then raise Runtime_Error with "reentrant binding close"; end if;
      --  The connection owns wire shutdown. Binding scope exit only removes
      --  its local mappings; it cannot block or call application code.
      for E of Self.Table loop
         if E.SID /= No_Subscription then Connections.Forget (Self.Core.all, E.SID); end if;
         E := (others => <>);
      end loop;
   end Close;

   overriding procedure Finalize (Self : in out Binding) is
   begin
      Close (Self);
   end Finalize;

   function Publish
     (Core : in out Connections.Connection;
      Namespace, Name, Topic : String; Value : Payload) return Boolean
   is
      Document : constant String := Encode (Value);
   begin
      Connections.Publish (Core, Namespace, Name, Topic, Document);
      return True; -- Sent, never an assertion of server acceptance.
   end Publish;
end OMS_LACAL.Typed;