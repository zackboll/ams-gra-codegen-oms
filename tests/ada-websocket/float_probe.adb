with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Interfaces;
with Service_Codec;
with TASK100_PACKAGE;
with OMS_LACAL;
with OMS_LACAL.Transports;
with OMS_LACAL.Transports.WebSocket;

procedure Float_Probe is
   use type Ada.Real_Time.Time;
   use type Interfaces.IEEE_Float_64;
   use OMS_LACAL.Transports;
   package WS renames OMS_LACAL.Transports.WebSocket;
   Wire : WS.Client;
   Status : Receive_Status;
   Message : OMS_LACAL.Text;
   type Values is array (Positive range <>) of Interfaces.IEEE_Float_64;
   Expected : constant Values := (-0.0, 2.0 ** (-1074),
     (2.0 - 2.0 ** (-52)) * 2.0 ** 1023, 1.0 + 2.0 ** (-52));
begin
   WS.Open (Wire, Ada.Command_Line.Argument (1));
   WS.Send (Wire, "FLOAT64_TRANSPORT");
   for I in Expected'Range loop
      declare
         Deadline : constant Ada.Real_Time.Time := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (4);
      begin
         loop
            WS.Poll (Wire, Status, Message);
            exit when Status = Frame_Ready;
            if Status = Peer_Closed or else Ada.Real_Time.Clock > Deadline then
               raise Program_Error with "Float64 receive failed";
            end if;
            delay 0.001;
         end loop;
         declare
            Value : constant TASK100_PACKAGE.SharedPayload :=
              Service_Codec.Decode_SharedPayload (To_String (Message));
            Encoded : constant String := Service_Codec.Encode_SharedPayload (Value);
         begin
            if Value.Sample /= Expected (I) then raise Program_Error with "Float64 changed"; end if;
            if I = 1 and then Interfaces.IEEE_Float_64'Copy_Sign (1.0, Value.Sample) /= -1.0 then
               raise Program_Error with "negative zero lost";
            end if;
            WS.Send (Wire, To_String (Message));
            if Service_Codec.Decode_SharedPayload (Encoded).Sample /= Expected (I) then
               raise Program_Error with "Float64 codec round trip changed";
            end if;
         end;
      end;
   end loop;
   WS.Close (Wire);
   Ada.Text_IO.Put_Line ("TASK100_FLOAT64_TRANSPORT_COMPLETE values=4 exact_bytes=4");
exception when E : others =>
   Ada.Text_IO.Put_Line (Ada.Exceptions.Exception_Information (E));
   Ada.Command_Line.Set_Exit_Status (1);
end Float_Probe;