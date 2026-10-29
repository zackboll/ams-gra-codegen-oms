with Ada.Finalization;
with Ada.Strings.Unbounded;
with GNAT.Sockets;
with AHT.Bytes;
with AHT.Websocket.Frames;

package OMS_LACAL.Transports.WebSocket is
   --  Explicitly driven Linux loopback transport. No task or application
   --  callback. Poll performs at most one 4 KiB read and one complete-frame
   --  consumption; bounded codec/copy work is O(Max_Frame_Bytes).
   --  Open/Send have two-second I/O deadlines; Close waits at most 250 ms
   --  after its bounded write. Finalization releases without waiting.
   --  CI-SELECT-002-FINAL: preserve these bounds in both compiler campaigns.
   Transport_Error : exception;
   type Client is new Ada.Finalization.Limited_Controlled and Transport with private;
   procedure Open (Self : in out Client; URL : String);
   overriding procedure Send (Self : in out Client; Frame : String);
   overriding procedure Poll
     (Self : in out Client; Status : out Receive_Status;
      Frame : out Text);
   overriding procedure Close (Self : in out Client);
   function Protocol (Self : Client) return String;
private
   type Client is new Ada.Finalization.Limited_Controlled and Transport with record
      Socket : GNAT.Sockets.Socket_Type := GNAT.Sockets.No_Socket;
      Input, Fragments : AHT.Bytes.Buffer;
      Fragment : AHT.Websocket.Frames.Fragment_State :=
        AHT.Websocket.Frames.No_Message;
      Upgraded : Boolean := False;
      Close_Sent : Boolean := False;
   end record;
   overriding procedure Finalize (Self : in out Client);
end OMS_LACAL.Transports.WebSocket;