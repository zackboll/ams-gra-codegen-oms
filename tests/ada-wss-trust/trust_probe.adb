with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Real_Time; use Ada.Real_Time;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with GNAT.Directory_Operations;
with OMS_LACAL;
with OMS_LACAL.Transports; use OMS_LACAL.Transports;
with OMS_LACAL.Transports.WebSocket;
with Test_PEM;

procedure Trust_Probe is
   package WS renames OMS_LACAL.Transports.WebSocket;
   --  This one object survives every command, failed Open and normal Close.
   Client : WS.Client;
   Trusted : constant String := Test_PEM.Read (Ada.Command_Line.Argument (1));
   Unrelated : constant String := Test_PEM.Read (Ada.Command_Line.Argument (2));
   Malformed : constant String := "-----BEGIN CERTIFICATE-----" & ASCII.LF &
     "not a certificate" & ASCII.LF & "-----END CERTIFICATE-----" & ASCII.LF;
   Status : Receive_Status;
   Message : OMS_LACAL.Text;
   Baseline_FDs, Baseline_Tasks : Natural := 0;
   Measured : Boolean := False;

   function Count (Path : String) return Natural is
      Search : GNAT.Directory_Operations.Dir_Type;
      Item : String (1 .. 256);
      Last, N : Natural := 0;
   begin
      GNAT.Directory_Operations.Open (Search, Path);
      loop
         GNAT.Directory_Operations.Read (Search, Item, Last);
         exit when Last = 0;
         if Item (1 .. Last) /= "." and then Item (1 .. Last) /= ".." then
            N := N + 1;
         end if;
      end loop;
      GNAT.Directory_Operations.Close (Search);
      return N;
   end Count;

   procedure Check (OK : Boolean; Why : String) is
   begin
      if not OK then raise Program_Error with Why; end if;
   end Check;

   procedure Resources is
      FDs : constant Natural := Count ("/proc/self/fd");
      Tasks : constant Natural := Count ("/proc/self/task");
   begin
      if Measured then
         Check (FDs = Baseline_FDs, "descriptor growth");
         Check (Tasks = Baseline_Tasks, "task growth");
      end if;
      Put_Line ("RESOURCES fd=" & FDs'Image & " tasks=" & Tasks'Image);
   end Resources;

   procedure Closed is
   begin
      Check (Client.Protocol = "", "usable protocol after failure/Close");
      Client.Poll (Status, Message);
      Check (Status = Peer_Closed, "not closed after failure/Close");
      begin
         Client.Send ("INIT {}");
         raise Program_Error with "Send accepted after failure/Close";
      exception
         when WS.Transport_Error => null;
      end;
      Resources;
   end Closed;

   procedure Attempt (Mode, URL : String) is
      Start : constant Time := Clock;
      Rejected : Boolean := False;
      Positive : constant Boolean := Mode = "pem" or else Mode = "path";
   begin
      begin
         if Mode = "path" then
            Client.Open (URL, CA_Path => Ada.Command_Line.Argument (1));
         else
            Client.Open
              (URL, CA_Path => (if Mode = "both" then Ada.Command_Line.Argument (1) else ""),
               CA_PEM => (if Mode = "untrusted" or else Mode = "untrusted-close" then Unrelated
                          elsif Mode = "malformed" then Malformed else Trusted));
         end if;
      exception
         when E : WS.Transport_Error =>
            Rejected := True;
            Put_Line ("REJECTED category=Transport_Error detail=" & Ada.Exceptions.Exception_Message (E));
      end;
      Check (Clock - Start < Seconds (8), "Open exceeded deadline");
      Check (Rejected /= Positive, "unexpected Open outcome");
      if Positive then
         Check (Client.Protocol = "owp", "not exactly owp");
         Client.Send ("INIT {}");
         declare
            Deadline : constant Time := Clock + Seconds (3);
         begin
            loop
               Client.Poll (Status, Message);
               exit when Status = Frame_Ready;
               Check (Status /= Peer_Closed and then Clock < Deadline, "exchange deadline/EOF");
               delay 0.001;
            end loop;
         end;
         Check (To_String (Message) = "INFO {}", "application exchange mismatch");
         Client.Close;
      elsif Mode = "untrusted-close" then
         Client.Close;
         Client.Close;
      end if;
      --  Negative cases check cleanup BEFORE an explicit Close, except the
      --  separately named explicit-Close control.
      Closed;
      Put_Line ("DONE " & Mode);
      Flush;
   end Attempt;
begin
   Put_Line ("READY"); Flush;
   loop
      declare
         Line : constant String := Get_Line;
         Space : constant Natural := Ada.Strings.Fixed.Index (Line, " ");
      begin
         if Line = "quit" then exit;
         elsif Line = "baseline" then
            Baseline_FDs := Count ("/proc/self/fd");
            Baseline_Tasks := Count ("/proc/self/task");
            Measured := True;
            Resources;
            Put_Line ("DONE baseline"); Flush;
         else
            Check (Space > 1, "bad harness command");
            Attempt (Line (1 .. Space - 1), Line (Space + 1 .. Line'Last));
         end if;
      end;
   end loop;
   Client.Close;
   Closed;
   Put_Line ("COMPLETE");
exception
   when E : others =>
      Put_Line ("HARNESS_FAILURE " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (1);
end Trust_Probe;