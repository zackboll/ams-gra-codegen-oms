with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;

package body OMS_LACAL.Transports.Mock is
   overriding procedure Send (Self : in out Mock_Transport; Frame : String) is
      Prepared : Text;
   begin
      if Self.Closed or Self.Lost or Self.Fail_Send then
         Self.Fail_Send := False;
         raise Runtime_Error with "mock send failure";
      elsif Frame'Length > Max_Frame_Bytes or Self.Written = Capacity then
         raise Runtime_Error with "mock output capacity";
      end if;
      Prepared := To_Unbounded_String (Frame);
      Self.Output (Self.Written + 1) := Prepared;
      Self.Written := Self.Written + 1;
   end Send;

   overriding procedure Poll
     (Self : in out Mock_Transport; Status : out Receive_Status; Frame : out Text) is
   begin
      Frame := Null_Unbounded_String;
      if Self.Closed or Self.Lost then Status := Peer_Closed;
      elsif Self.Count = 0 then Status := Idle;
      else
         Status := Frame_Ready;
         Frame := Self.Input (Self.Head);
         Self.Input (Self.Head) := Null_Unbounded_String;
         Self.Head := Self.Head mod Capacity + 1;
         Self.Count := Self.Count - 1;
      end if;
   end Poll;

   overriding procedure Close (Self : in out Mock_Transport) is
   begin
      Self.Closed := True;
      Self.Input := (others => Null_Unbounded_String);
      Self.Count := 0;
      --  Sent frames remain as bounded test evidence, never production state.
   end Close;

   procedure Enqueue (Self : in out Mock_Transport; Frame : String) is
   begin
      if Self.Count = Capacity or Self.Closed then
         raise Runtime_Error with "mock input capacity/closed";
      end if;
      Self.Input ((Self.Head - 1 + Self.Count) mod Capacity + 1) := To_Unbounded_String (Frame);
      Self.Count := Self.Count + 1;
   end Enqueue;
   procedure Disconnect (Self : in out Mock_Transport) is
   begin Self.Lost := True; end Disconnect;
   procedure Fail_Next_Send (Self : in out Mock_Transport) is
   begin Self.Fail_Send := True; end Fail_Next_Send;
   function Sent_Count (Self : Mock_Transport) return Natural is (Self.Written);
   function Sent (Self : Mock_Transport; Index : Positive) return String is
   begin
      if Index > Self.Written then raise Constraint_Error with "unsent frame"; end if;
      return To_String (Self.Output (Index));
   end Sent;
   function Is_Closed (Self : Mock_Transport) return Boolean is (Self.Closed);
end OMS_LACAL.Transports.Mock;