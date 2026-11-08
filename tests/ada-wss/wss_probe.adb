with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Environment_Variables;
with Ada.Real_Time; use Ada.Real_Time;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with GNAT.Directory_Operations;
with OMS_LACAL;
with OMS_LACAL.Transports; use OMS_LACAL.Transports;
with OMS_LACAL.Transports.WebSocket;

procedure WSS_Probe is
   package WS renames OMS_LACAL.Transports.WebSocket;
   C : WS.Client;
   Status : Receive_Status;
   Message : OMS_LACAL.Text;
   Mode : constant String := Ada.Command_Line.Argument (3);
   Deadline, Start : Time;
   Messages, Polls : Natural := 0;
   function FDs return Natural is
      Search : GNAT.Directory_Operations.Dir_Type;
      Item : String (1 .. 256);
      Last, Count : Natural := 0;
   begin
      GNAT.Directory_Operations.Open (Search, "/proc/self/fd");
      loop
         GNAT.Directory_Operations.Read (Search, Item, Last);
         exit when Last = 0;
         Count := Count + 1;
      end loop;
      GNAT.Directory_Operations.Close (Search);
      return Count;
   end FDs;
   procedure Step is
   begin
      Start := Clock;
      C.Poll (Status, Message);
      if Clock - Start > Milliseconds (200) then
         raise Program_Error with "Poll waited for network";
      end if;
      Polls := Polls + 1;
   end Step;
begin
   if Mode = "failed-handshake" then
      declare
         Baseline : Natural := 0;
      begin
         for Cycle in 1 .. 20 loop
            begin
               C.Open (Ada.Command_Line.Argument (1), CA_Path => Ada.Command_Line.Argument (2));
               raise Program_Error with "invalid handshake accepted";
            exception
               when WS.Transport_Error => null;
            end;
            C.Close;
            C.Close;
            if Cycle = 1 then Baseline := FDs;
            elsif FDs /= Baseline then raise Program_Error with "failed handshake FD leak";
            end if;
         end loop;
      end;
      Put_Line ("FAILED_HANDSHAKE_CLEANUP_COMPLETE cycles=20 fd_growth=0");
      return;
   end if;
   C.Open (Ada.Command_Line.Argument (1), CA_Path => Ada.Command_Line.Argument (2));
   Put_Line ("VERIFIED_OWP_UPGRADE");
   if Mode = "backpressure" or else Mode = "burst-backpressure" then
      Ada.Environment_Variables.Set ("OMS_WSS_ARM_OUTPUT", "1");
   end if;
   if Mode = "deaf" then
      begin
         for I in 1 .. 32 loop
            C.Send (String'(1 .. OMS_LACAL.Max_Frame_Bytes => 's'));
         end loop;
         raise Program_Error with "incomplete publication reported success";
      exception
         when WS.Transport_Error =>
            Step;
            if Status /= Peer_Closed then raise Program_Error; end if;
            C.Close;
            C.Close;
            Put_Line ("WSS_DEAF_PUBLICATION_FAILED_CLOSED");
            return;
      end;
   end if;
   Deadline := Clock + Seconds (8);
   loop
      Step;
      if Status = Frame_Ready then
         Messages := Messages + 1;
         if Mode = "records" then
            if To_String (Message) /= "Hello" & Messages'Image then raise Program_Error; end if;
         elsif To_String (Message) /= "Hello" then raise Program_Error; end if;
         Put_Line ("MESSAGE " & To_String (Message));
      end if;
      exit when Status = Peer_Closed;
      if Clock > Deadline then raise Program_Error with "WSS progress timeout"; end if;
      delay 0.001;
   end loop;
   if Messages /= (if Mode = "records" then 4 else 0) then
      raise Program_Error with "message accounting";
   end if;
   C.Close;
   C.Close;
   Put_Line ("WSS_PROBE_COMPLETE polls=" & Polls'Image);
exception
   when E : others =>
      Put_Line ("WSS_PROBE_FAILED " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (1);
end WSS_Probe;