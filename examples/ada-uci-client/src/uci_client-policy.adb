with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;

package body UCI_Client.Policy is
   use type Ada.Real_Time.Time;
   function Parse (Args : Arguments) return Options is
      Result : Options;
      Seen : array (1 .. 6) of Boolean := (others => False);
      I : Natural := Args'First;
      function Number (Value : String; Low, High : Natural) return Natural is
         N : Natural := 0;
      begin
         if Value'Length = 0 or else Value'Length > 10 then
            raise Invalid_Arguments with "expected decimal integer";
         end if;
         for C of Value loop
            if C not in '0' .. '9' then
               raise Invalid_Arguments with "expected decimal integer";
            end if;
            if N > High / 10 then
               raise Invalid_Arguments with "integer out of range";
            end if;
            N := N * 10 + Character'Pos (C) - Character'Pos ('0');
            if N > High then raise Invalid_Arguments with "integer out of range"; end if;
         end loop;
         if N < Low then raise Invalid_Arguments with "integer out of range"; end if;
         return N;
      end Number;
   begin
      while I <= Args'Last loop
         declare
            Key : constant String := To_String (Args (I));
            Index : Natural := 0;
         begin
            if Key = "--help" then
               if Args'Length /= 1 then
                  raise Invalid_Arguments with "--help must be used alone";
               end if;
               Result.Help := True;
               return Result;
            elsif Key = "--url" then Index := 1;
            elsif Key = "--ca-path" then Index := 2;
            elsif Key = "--ca-pem-file" then Index := 3;
            elsif Key = "--duration-seconds" then Index := 4;
            elsif Key = "--reconnect-attempts" then Index := 5;
            elsif Key = "--reconnect-delay-ms" then Index := 6;
            else raise Invalid_Arguments with "unknown option";
            end if;
            if Seen (Index) or else I = Args'Last or else Length (Args (I + 1)) = 0 then
               raise Invalid_Arguments with "duplicate option or missing value";
            end if;
            Seen (Index) := True;
            I := I + 1;
            declare V : constant String := To_String (Args (I)); begin
               if V'Length >= 2 and then V (V'First .. V'First + 1) = "--" then
                  raise Invalid_Arguments with "missing option value";
               end if;
               case Index is
                  when 1 => Result.URL := Args (I);
                  when 2 => Result.CA_Path := Args (I);
                  when 3 => Result.CA_PEM_File := Args (I);
                  when 4 => Result.Duration_Seconds := Number (V, 1, 3600);
                  when 5 => Result.Reconnect_Attempts := Number (V, 0, 8);
                  when 6 => Result.Reconnect_Delay_MS := Number (V, 50, 2000);
                  when others => raise Program_Error;
               end case;
            end;
         end;
         I := I + 1;
      end loop;
      if not Seen (1) or else Seen (2) = Seen (3) then
         raise Invalid_Arguments with "require --url and exactly one explicit CA option";
      end if;
      declare
         URL : constant String := To_String (Result.URL);
         use Ada.Strings.Fixed;
         Colon, Slash : Natural;
      begin
         if URL'Length < 9 or else URL'Length > 2048 or else URL (1 .. 6) /= "wss://" then
            raise Invalid_Arguments with "require wss://host:port/path";
         end if;
         for C of URL loop
            if C < '!' or else C > '~' or else C in '#' | '@' | '[' | ']' then
               raise Invalid_Arguments with "invalid URL character or unsupported IPv6 literal";
            end if;
         end loop;
         Colon := Index (URL (7 .. URL'Last), ":");
         if Colon <= 7 then raise Invalid_Arguments with "explicit host and port required"; end if;
         Slash := Index (URL (Colon + 1 .. URL'Last), "/");
         if Slash = 0 then Slash := URL'Last + 1; end if;
         if Index (URL (7 .. Colon - 1), "/") /= 0 then
            raise Invalid_Arguments with "invalid host";
         end if;
         declare Port : constant Natural := Number (URL (Colon + 1 .. Slash - 1), 1, 65535);
         begin pragma Unreferenced (Port); end;
      end;
      return Result;
   end Parse;

   function Take_Attempt
     (Result : Outcome; Established : Boolean;
      Remaining : in out Natural; Now, Deadline : Ada.Real_Time.Time)
      return Boolean is
   begin
      if Result /= Peer_Loss or else not Established or else
        Remaining = 0 or else Now >= Deadline
      then return False; end if;
      Remaining := Remaining - 1;
      return True;
   end Take_Attempt;

   function Wake_Time
     (Now, Deadline : Ada.Real_Time.Time; Delay_MS : Positive)
      return Ada.Real_Time.Time is
     (if Now + Ada.Real_Time.Milliseconds (Delay_MS) < Deadline then
         Now + Ada.Real_Time.Milliseconds (Delay_MS) else Deadline);
end UCI_Client.Policy;