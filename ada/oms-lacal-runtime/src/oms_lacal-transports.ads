package OMS_LACAL.Transports is
   type Transport is limited interface;
   type Receive_Status is (Idle, Frame_Ready, Peer_Closed);
   --  One complete OWP text frame per Send/Poll. Send must not publish any
   --  bytes on validation failure. An uncertain write must raise and close.
   procedure Send (Self : in out Transport; Frame : String) is abstract;
   procedure Poll
     (Self : in out Transport; Status : out Receive_Status; Frame : out Text)
      is abstract;
   procedure Close (Self : in out Transport) is abstract;
end OMS_LACAL.Transports;