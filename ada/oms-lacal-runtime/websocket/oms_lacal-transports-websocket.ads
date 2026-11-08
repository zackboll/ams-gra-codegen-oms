with Ada.Finalization;
with Ada.Strings.Unbounded;
with GNAT.Sockets;
with AHT.Bytes;
with AHT.Websocket.Frames;
with AHT.Transport.TLS;

package OMS_LACAL.Transports.WebSocket is
   --  Explicitly driven Linux transport (plaintext restricted to loopback).
   --  No task or application
   --  callback. Poll performs at most one 4 KiB read and one complete-frame
   --  consumption; bounded codec/copy work is O(Max_Frame_Bytes).
   --  Open/Send have two-second I/O deadlines; Close waits at most 250 ms
   --  after its bounded write. Finalization releases without waiting.
   Transport_Error : exception;
   type Client is new Ada.Finalization.Limited_Controlled and Transport with private;
   procedure Open (Self : in out Client; URL : String);
   --  WSS always verifies the original hostname and certificate chain.
   --  Empty CA inputs select backend defaults; there is no insecure mode.
   --  AHT rejects simultaneous CA_Path and CA_PEM. Setup may block within
   --  separate two-second resolve/connect/handshake/upgrade budgets.
   procedure Open
     (Self : in out Client; URL : String; CA_Path : String; CA_PEM : String := "");
   overriding procedure Send (Self : in out Client; Frame : String);
   overriding procedure Poll
     (Self : in out Client; Status : out Receive_Status;
      Frame : out Text);
   overriding procedure Close (Self : in out Client);
   function Protocol (Self : Client) return String;
private
   type TLS_Access is access AHT.Transport.TLS.TLS_Transport;
   type Client is new Ada.Finalization.Limited_Controlled and Transport with record
      Socket : GNAT.Sockets.Socket_Type := GNAT.Sockets.No_Socket;
      TLS : TLS_Access := null;
      --  At most one masked control frame (131 bytes). Backpressure pauses
      --  frame consumption; TLS-generated output is separately capped at 64 KiB.
      Output : AHT.Bytes.Buffer;
      Output_Pos : Positive := 1;
      Peer_Closing : Boolean := False;
      Input, Fragments : AHT.Bytes.Buffer;
      Fragment : AHT.Websocket.Frames.Fragment_State :=
        AHT.Websocket.Frames.No_Message;
      Upgraded : Boolean := False;
      Close_Sent : Boolean := False;
   end record;
   overriding procedure Finalize (Self : in out Client);
end OMS_LACAL.Transports.WebSocket;