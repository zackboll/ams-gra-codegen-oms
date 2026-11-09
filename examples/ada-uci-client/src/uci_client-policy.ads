with Ada.Real_Time;
with Ada.Strings.Unbounded;

package UCI_Client.Policy is
   Invalid_Arguments : exception;
   type Options is record
      URL, CA_Path, CA_PEM_File : Ada.Strings.Unbounded.Unbounded_String;
      Duration_Seconds : Positive range 1 .. 3600 := 30;
      Reconnect_Attempts : Natural range 0 .. 8 := 0;
      Reconnect_Delay_MS : Positive range 50 .. 2000 := 250;
      Help : Boolean := False;
   end record;
   type Arguments is array (Positive range <>) of
     Ada.Strings.Unbounded.Unbounded_String;
   function Parse (Args : Arguments) return Options;
   type Outcome is (Completed, Peer_Loss, Terminal_Failure);
   --  Only Peer_Loss from a fully established epoch authorizes recovery.
   --  Exceptions, including ambiguous transport exceptions, fail closed.
   function Take_Attempt
     (Result : Outcome; Established : Boolean;
      Remaining : in out Natural; Now, Deadline : Ada.Real_Time.Time)
      return Boolean;
   function Wake_Time
     (Now, Deadline : Ada.Real_Time.Time; Delay_MS : Positive)
      return Ada.Real_Time.Time;
end UCI_Client.Policy;