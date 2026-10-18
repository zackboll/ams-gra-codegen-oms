with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Allocation_Faults;
with GNATCOLL.Buffer;
with GNATCOLL.JSON;
procedure C1_Provider_Probe is
   use GNATCOLL.JSON;
   --  The same escaped string as the generated conformance fixture, with a
   --  heap-sized decoded result. Input/parser/logging are outside injection.
   Text : constant String := """Unicode \ud83d\udef0\ufe0f \u00e9 \"" \\ \u0000""";
   Expected : constant String := "Unicode " & Character'Val (16#F0#) &
     Character'Val (16#9F#) & Character'Val (16#9B#) & Character'Val (16#B0#) &
     Character'Val (16#EF#) & Character'Val (16#B8#) & Character'Val (16#8F#) &
     " " & Character'Val (16#C3#) & Character'Val (16#A9#) & " "" \ " & ASCII.NUL;
   Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (Text);
   Parser : JSON_Parser;
   Event : constant JSON_Parser_Event := Parse_Next (Parser, Data);
   Budget : constant Natural := Natural'Value (Ada.Command_Line.Argument (1));
   Result : Unbounded_String := To_Unbounded_String ("sentinel");
begin
   Allocation_Faults.Arm (Budget);
   begin
      Result := To_Unbounded_String (Decode_As_String (Event, Data));
      Allocation_Faults.Disarm;
      if To_String (Result) /= Expected then raise Program_Error with "provider wrong result"; end if;
      Ada.Text_IO.Put_Line ("TASK097_C1_PROVIDER_SUCCESS");
   exception
      when E : others =>
         Allocation_Faults.Disarm;
         if Result /= "sentinel" then raise Program_Error with "provider published on failure"; end if;
         Ada.Text_IO.Put_Line ("TASK097_C1_PROVIDER_EXCEPTION " & Ada.Exceptions.Exception_Name (E));
         Ada.Text_IO.Put_Line (Ada.Exceptions.Exception_Message (E));
   end;
   --  Prove recovery or detect retained provider damage in the same process.
   if Decode_As_String (Event, Data) /= Expected then
      raise Program_Error with "provider post-failure recovery";
   end if;
   Ada.Text_IO.Put_Line ("TASK097_C1_PROVIDER_RECOVERY_COMPLETE");
end C1_Provider_Probe;