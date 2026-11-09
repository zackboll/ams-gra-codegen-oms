with Ada.Real_Time;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with UCI_Client.Policy;

procedure Policy_Tests is
   package P renames UCI_Client.Policy;
   use type P.Arguments;
   use type Ada.Real_Time.Time;
   Base : constant Ada.Real_Time.Time := Ada.Real_Time.Clock;
   Deadline : constant Ada.Real_Time.Time := Base + Ada.Real_Time.Seconds (1);
   Remaining : Natural := 2;
   Options : P.Options;
   Checks : Natural := 0;
   function T (Value : String) return Unbounded_String renames To_Unbounded_String;
   procedure Check (OK : Boolean) is
   begin
      pragma Assert (OK);
      Checks := Checks + 1;
   end Check;
   procedure Reject (Args : P.Arguments) is
   begin
      Options := P.Parse (Args);
      raise Program_Error with "invalid arguments accepted";
   exception
      when P.Invalid_Arguments => Checks := Checks + 1;
   end Reject;
   Valid : constant P.Arguments := (T ("--url"), T ("wss://localhost:9443/"),
                                   T ("--ca-path"), T ("ca.pem"));
begin
   Options := P.Parse (Valid);
   Check (Options.Reconnect_Attempts = 0 and then Options.Duration_Seconds = 30 and then
          Options.Reconnect_Delay_MS = 250 and then not Options.Help);
   Options := P.Parse ((1 => T ("--help")));
   Check (Options.Help);
   Reject ((1 => T ("--unknown")));
   Reject ((1 => T ("--url")));
   Reject ((1 => T ("--help"), 2 => T ("--help")));
   Reject (Valid & (T ("--url"), T ("wss://localhost:9443/")));
   Reject (Valid & (T ("--ca-pem-file"), T ("ca.pem")));
   Reject ((T ("--url"), T ("ws://localhost:9443/"), T ("--ca-path"), T ("ca.pem")));
   Reject ((T ("--url"), T ("wss://localhost/"), T ("--ca-path"), T ("ca.pem")));
   Reject ((T ("--url"), T ("wss://localhost:0/"), T ("--ca-path"), T ("ca.pem")));
   Reject ((T ("--url"), T ("wss://localhost:65536/"), T ("--ca-path"), T ("ca.pem")));
   Reject ((T ("--url"), T ("wss://user@localhost:9443/"), T ("--ca-path"), T ("ca.pem")));
   for N in 1 .. 6 loop
      declare
         Option : constant String := (case N is
           when 1 | 2 => "--duration-seconds",
           when 3 | 4 => "--reconnect-attempts",
           when others => "--reconnect-delay-ms");
         Value : constant String := (case N is
           when 1 => "0", when 2 => "3601", when 3 => "-1", when 4 => "9",
           when 5 => "49", when others => "2001");
      begin
         Reject (Valid & (T (Option), T (Value)));
      end;
   end loop;
   Reject (Valid & (T ("--duration-seconds"), T ("1_0")));
   Reject (Valid & (T ("--duration-seconds"), T ("+1")));
   Reject (Valid & (T ("--duration-seconds"), T ("9999999999")));
   Check (not P.Take_Attempt (P.Peer_Loss, False, Remaining, Base, Deadline));
   Check (Remaining = 2);
   Check (not P.Take_Attempt (P.Terminal_Failure, True, Remaining, Base, Deadline));
   Check (not P.Take_Attempt (P.Completed, True, Remaining, Base, Deadline));
   Check (not P.Take_Attempt (P.Peer_Loss, True, Remaining, Deadline, Deadline));
   Check (not P.Take_Attempt (P.Peer_Loss, True, Remaining, Deadline + Ada.Real_Time.Seconds (1), Deadline));
   Check (Remaining = 2);
   Check (P.Take_Attempt (P.Peer_Loss, True, Remaining, Base, Deadline));
   Check (Remaining = 1);
   Check (P.Take_Attempt (P.Peer_Loss, True, Remaining, Base, Deadline));
   Check (Remaining = 0);
   Check (not P.Take_Attempt (P.Peer_Loss, True, Remaining, Base, Deadline));
   Check (P.Wake_Time (Base, Deadline, 2000) = Deadline);
   Check (P.Wake_Time (Base, Deadline, 50) = Base + Ada.Real_Time.Milliseconds (50));
   Ada.Text_IO.Put_Line ("POLICY_TESTS_PASS checks=" & Checks'Image);
end Policy_Tests;