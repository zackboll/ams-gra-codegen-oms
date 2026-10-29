with Ada.Command_Line;
with Ada.Exceptions;
with GNAT.Directory_Operations;
with Ada.Real_Time;
with Ada.Strings.Unbounded;
with Ada.Text_IO;
with OMS_LACAL;
with OMS_LACAL.Transports;
with OMS_LACAL.Transports.WebSocket;

procedure Transport_Probe is
   use type Ada.Real_Time.Time;
   use type Ada.Real_Time.Time_Span;
   use OMS_LACAL.Transports;
   package WS renames OMS_LACAL.Transports.WebSocket;
   Client : WS.Client;
   Status : Receive_Status;
   Message : OMS_LACAL.Text;
   Deadline : Ada.Real_Time.Time;
   Mode : constant String := Ada.Command_Line.Argument (2);
   function Entries (Directory : String) return Natural is
      Search : GNAT.Directory_Operations.Dir_Type;
      Item : String (1 .. 256);
      Last : Natural;
      Count : Natural := 0;
   begin
      GNAT.Directory_Operations.Open (Search, Directory);
      loop
         --  Read directory names without stat-following /proc FD symlinks:
         --  stdin may refer to a pipe whose target is not a filesystem path.
         GNAT.Directory_Operations.Read (Search, Item, Last);
         exit when Last = 0;
         if Item (1 .. Last) /= "." and then Item (1 .. Last) /= ".."
         then Count := Count + 1; end if;
      end loop;
      GNAT.Directory_Operations.Close (Search);
      return Count;
   end Entries;
   Tasks : constant Natural := Entries ("/proc/self/task");
   Closed_FDs : Natural := 0;
begin
   for Cycle in 1 .. (if Mode = "lifecycle" then 30 else 1) loop
      WS.Open (Client, Ada.Command_Line.Argument (1));
      if Entries ("/proc/self/task") /= Tasks then
         raise Program_Error with "transport created task";
      end if;
      if WS.Protocol (Client) /= "owp" then raise Program_Error; end if;
      Ada.Text_IO.Put_Line ("UPGRADE_OWP_COMPLETE");
      if Mode = "send-failure" then
         delay 0.1;
         begin
            for I in 1 .. 20 loop
               WS.Send (Client, String'(1 .. OMS_LACAL.Max_Frame_Bytes => 's'));
            end loop;
            raise Program_Error with "send failure not observed";
         exception when WS.Transport_Error =>
            WS.Poll (Client, Status, Message);
            if Status /= Peer_Closed then raise Program_Error; end if;
            Ada.Text_IO.Put_Line ("SEND_FAILURE_CLOSED");
         end;
      else
         if Mode = "large" then
            WS.Send (Client, String'(1 .. OMS_LACAL.Max_Frame_Bytes => 's'));
         elsif Mode = "validation" then
            begin
               WS.Send (Client, String'(1 .. OMS_LACAL.Max_Frame_Bytes + 1 => 'x'));
               raise Program_Error with "oversized send accepted";
            exception when WS.Transport_Error => null; end;
            begin
               WS.Send (Client, "bad" & Character'Val (16#FF#));
               raise Program_Error with "bad UTF8 send accepted";
            exception when WS.Transport_Error => null; end;
            WS.Send (Client, "valid-after-rejection");
         else
            WS.Send (Client, "INIT {} {-0, UTF8: " & Character'Val (16#C2#) & Character'Val (16#A3#) & "}");
         end if;
         Deadline := Ada.Real_Time.Clock + Ada.Real_Time.Seconds (4);
         loop
            declare Start : constant Ada.Real_Time.Time := Ada.Real_Time.Clock; begin
               WS.Poll (Client, Status, Message);
               if Ada.Real_Time.Clock - Start > Ada.Real_Time.Milliseconds (200) then
                  raise Program_Error with "Poll exceeded bounded work allowance";
               end if;
            end;
            exit when Status /= Idle;
            if Ada.Real_Time.Clock > Deadline then raise Program_Error with "receive timeout"; end if;
            delay 0.001;
         end loop;
         if Mode = "closed" then
            if Status /= Peer_Closed then raise Program_Error with "expected closure"; end if;
         else
            if Status /= Frame_Ready then raise Program_Error with "no message"; end if;
            if Mode = "receive-large" then
               if Ada.Strings.Unbounded.Length (Message) /= OMS_LACAL.Max_Frame_Bytes then
                  raise Program_Error with "maximum receive length changed";
               end if;
               Ada.Text_IO.Put_Line ("RECEIVED_MAX_MESSAGE_COMPLETE");
            else
               Ada.Text_IO.Put_Line ("RECEIVED " & Ada.Strings.Unbounded.To_String (Message));
            end if;
            Deadline := Ada.Real_Time.Clock;
            for I in 1 .. 100 loop
               WS.Poll (Client, Status, Message);
               if Status /= Idle then raise Program_Error with "not idle"; end if;
            end loop;
            if Ada.Real_Time.Clock - Deadline > Ada.Real_Time.Milliseconds (200) then
               raise Program_Error with "idle polling blocked";
            end if;
            Ada.Text_IO.Put_Line ("IDLE_POLLS_COMPLETE 100");
         end if;
         if Mode /= "finalize" then WS.Close (Client); end if;
         if Mode = "lifecycle" then
            --  First connection warms AHT's one process-lifetime entropy FD.
            if Cycle = 1 then Closed_FDs := Entries ("/proc/self/fd");
            elsif Entries ("/proc/self/fd") /= Closed_FDs then
               raise Program_Error with "socket descriptor leak";
            end if;
            if Entries ("/proc/self/task") /= Tasks then
               raise Program_Error with "task resource leak";
            end if;
         end if;
      end if;
   end loop;
   if Mode = "lifecycle" then
      Ada.Text_IO.Put_Line ("RESOURCE_LIFECYCLES_COMPLETE cycles=30 fd_growth=0 task_growth=0");
   end if;
   Ada.Text_IO.Put_Line ("TRANSPORT_PROBE_COMPLETE");
exception when E : others =>
   Ada.Text_IO.Put_Line ("PROBE_FAILED " & Ada.Exceptions.Exception_Information (E));
   Ada.Command_Line.Set_Exit_Status (1);
end Transport_Probe;