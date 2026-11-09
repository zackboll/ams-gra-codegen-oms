with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Real_Time; use Ada.Real_Time;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with GNAT.Directory_Operations;
with OMS_LACAL;
with OMS_LACAL.Transports; use OMS_LACAL.Transports;
with OMS_LACAL.Transports.WebSocket;

procedure Deadline_Probe is
   package WS renames OMS_LACAL.Transports.WebSocket;
   Mode : constant String := Ada.Command_Line.Argument (3);
   Millis : constant Integer := Integer'Value (Ada.Command_Line.Argument (4));
   Start : constant Time := Clock;
   Deadline : constant Time := Start + Milliseconds (Millis);
   Baseline_FD, Baseline_Tasks : Natural;
   procedure Say (Value : String) is
   begin
      Ada.Text_IO.Put_Line (Value); Ada.Text_IO.Flush;
   end Say;
   procedure Check (Value : Boolean; Detail : String) is
   begin
      if not Value then raise Program_Error with Detail; end if;
   end Check;
   function Entries (Path : String) return Natural is
      use GNAT.Directory_Operations;
      Dir : Dir_Type;
      Name : String (1 .. 1024);
      Last, Count : Natural := 0;
   begin
      Open (Dir, Path);
      loop
         Read (Dir, Name, Last); exit when Last = 0;
         if Name (1 .. Last) /= "." and then Name (1 .. Last) /= ".." then Count := Count + 1; end if;
      end loop;
      Close (Dir); return Count;
   end Entries;
   procedure Session is
      Wire : WS.Client;
      Status : Receive_Status;
      Message : OMS_LACAL.Text;
      Failed : Boolean := False;
      procedure Closed is
      begin
         Check (WS.Protocol (Wire) = "", "failed client still upgraded");
         Wire.Poll (Status, Message);
         Check (Status = Peer_Closed, "failed client still usable");
         Wire.Close; Wire.Close;
         begin
            Wire.Send ("FORBIDDEN_REPLAY");
            raise Program_Error with "retired Send accepted";
         exception when WS.Transport_Error => null; end;
      end Closed;
   begin
      if Mode /= "legacy-handshake" and then Mode /= "legacy-normal" and then Mode /= "legacy-close" then
         WS.Set_Absolute_Deadline (Wire, Deadline);
      end if;
      begin
         Wire.Open (Ada.Command_Line.Argument (1), CA_Path => Ada.Command_Line.Argument (2));
      exception
         when E : WS.Transport_Error =>
            Say ("OPEN_ERROR " & Ada.Exceptions.Exception_Message (E));
            Failed := True;
      end;
      if Mode in "handshake" | "upgrade" | "trickle-upgrade" | "cumulative-upgrade" |
        "legacy-handshake" | "expired-open" | "short-open" | "long-handshake"
      then
         Check (Failed, "stalled/expired Open succeeded");
         Closed;
         if Mode = "expired-open" then
            --  Drop/Close must not silently clear the configured deadline.
            begin
               Wire.Open (Ada.Command_Line.Argument (1), CA_Path => Ada.Command_Line.Argument (2));
               raise Program_Error with "deadline reset on cleanup";
            exception when WS.Transport_Error => null; end;
         end if;
         return;
      end if;
      Check (not Failed and then WS.Protocol (Wire) = "owp", "verified Open failed");
      Say ("VERIFIED_OWP");
      if Mode = "write" or else Mode = "queued-write" then
         if Mode = "queued-write" then
            --  The peer sends a Ping before the data barrier. Exactly one Poll
            --  consumes it and queues the bounded Pong; Send must include it.
            delay until Start + Milliseconds (150);
            Wire.Poll (Status, Message);
            Say ("POLLED_CONTROL");
         end if;
         begin
            for I in 1 .. 64 loop
               Wire.Send (I'Image & ":" & String'(1 .. OMS_LACAL.Max_Frame_Bytes - 16 => 'x'));
               Say ("COMMITTED" & I'Image);
            end loop;
            raise Program_Error with "stalled writes did not fail";
         exception when WS.Transport_Error => Say ("WRITE_ERROR"); end;
         Closed;
      elsif Mode = "expired-send" then
         delay until Deadline;
         begin
            Wire.Send ("FORBIDDEN_EXPIRED");
            raise Program_Error with "expired publication accepted";
         exception when WS.Transport_Error => Say ("EXPIRED_SEND"); end;
         Closed;
      elsif Mode = "expired-close" then
         delay until Deadline;
         Wire.Close;
         Say ("EXPIRED_CLOSE");
         Closed;
      elsif Mode = "close" or else Mode = "legacy-close" then
         if Mode = "close" then delay until Deadline - Milliseconds (100); end if;
         Wire.Close;
         Say ("CLOSE_COMPLETE");
         Closed;
      elsif Mode = "normal" or else Mode = "legacy-normal" or else Mode = "reuse" then
         Wire.Send ("Hello");
         loop
            Wire.Poll (Status, Message);
            exit when Status = Frame_Ready;
            Check (Clock < Start + Seconds (5), "normal receive timeout");
            delay 0.001;
         end loop;
         Check (To_String (Message) = "Hello", "verified echo");
         Wire.Close;
         Closed;
         if Mode = "reuse" then
            WS.Set_Absolute_Deadline (Wire, Clock + Seconds (1));
            Wire.Open (Ada.Command_Line.Argument (1), CA_Path => Ada.Command_Line.Argument (2));
            Check (WS.Protocol (Wire) = "owp", "explicit replacement deadline on reuse");
            Wire.Close; Closed;
         end if;
      else raise Program_Error with "unknown mode";
      end if;
   end Session;
begin
   Baseline_FD := Entries ("/proc/self/fd");
   Baseline_Tasks := Entries ("/proc/self/task");
   Session;
   declare Limit : constant Time := Clock + Seconds (1); begin
      while Entries ("/proc/self/task") /= Baseline_Tasks loop
         Check (Clock < Limit, "resolver worker did not quiesce"); delay 0.001;
      end loop;
   end;
   Say ("RESOURCES fd=" & Entries ("/proc/self/fd")'Image & " tasks=" & Entries ("/proc/self/task")'Image);
   --  TLS initialization can retain one process-owned entropy FD on first use.
   Check (Entries ("/proc/self/fd") <= Baseline_FD + 1, "descriptor growth after finalization");
   Say ("DEADLINE_PROBE_PASS " & Mode);
exception
   when E : others =>
      Say ("DEADLINE_PROBE_FAILED " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (1);
end Deadline_Probe;