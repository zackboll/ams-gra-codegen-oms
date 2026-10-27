package OMS_LACAL.Transports.Mock is
   Capacity : constant := 256;
   type Mock_Transport is new Transport with private;
   overriding procedure Send (Self : in out Mock_Transport; Frame : String);
   overriding procedure Poll
     (Self : in out Mock_Transport; Status : out Receive_Status; Frame : out Text);
   overriding procedure Close (Self : in out Mock_Transport);
   procedure Enqueue (Self : in out Mock_Transport; Frame : String);
   procedure Disconnect (Self : in out Mock_Transport);
   procedure Fail_Next_Send (Self : in out Mock_Transport);
   function Sent_Count (Self : Mock_Transport) return Natural;
   function Sent (Self : Mock_Transport; Index : Positive) return String;
   function Is_Closed (Self : Mock_Transport) return Boolean;
private
   type Frames is array (1 .. Capacity) of Text;
   type Mock_Transport is new Transport with record
      Input, Output : Frames;
      Head : Positive := 1;
      Count, Written : Natural := 0;
      Closed, Lost, Fail_Send : Boolean := False;
   end record;
end OMS_LACAL.Transports.Mock;