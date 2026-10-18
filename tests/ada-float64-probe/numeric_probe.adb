with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Unchecked_Conversion;
with GNATCOLL.Buffer;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with Interfaces;
with Interfaces.C;
procedure Numeric_Probe is
   subtype F64 is Interfaces.IEEE_Float_64;
   use type F64;
   use type Interfaces.C.int;
   use type Interfaces.C.size_t;
   use type Interfaces.Unsigned_64;
   function To_Bits is new Ada.Unchecked_Conversion (F64, Interfaces.Unsigned_64);
   function From_Bits is new Ada.Unchecked_Conversion (Interfaces.Unsigned_64, F64);
   function Parse (Text : Interfaces.C.char_array; Length, Capacity : Interfaces.C.size_t;
                   Value : out Interfaces.C.double; Consumed : out Interfaces.C.size_t;
                   Error : out Interfaces.C.int) return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_parse";
   function Format (Value : access Interfaces.C.double; Text : out Interfaces.C.char_array;
                    Capacity : Interfaces.C.size_t) return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_format";
   function ABI (Input : Interfaces.C.double; Positive, Negative_Zero : out Interfaces.C.double)
     return Interfaces.C.int with Import, Convention => C, External_Name => "task096_abi";
   function Bits (Value : F64) return String is
      Hex : constant String := "0123456789ABCDEF";
      N : Interfaces.Unsigned_64 := To_Bits (Value);
      Result : String (1 .. 16);
   begin
      for J in reverse Result'Range loop
         Result (J) := Hex (Integer (N and 15) + 1); N := Interfaces.Shift_Right (N, 4);
      end loop;
      return Result;
   end Bits;
   function Classification (Value : F64) return String is
     (if Value > F64'Last or else Value < -F64'Last then "OVERFLOW" else Bits (Value));
   Input : File_Type;
   Mode : constant String := Ada.Command_Line.Argument (1);
   Count : Natural := 0;
   Positive, Negative_Zero : Interfaces.C.double;
   function Locale_Enter return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_locale_enter";
   function Locale_Leave return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_locale_leave";
   Locale_Active : Boolean := False;
begin
   if Ada.Command_Line.Argument_Count = 3 then
      if Locale_Enter /= 0 then raise Program_Error with "locale setup"; end if;
      Locale_Active := True;
   end if;
   if F64'Object_Size /= 64 or else F64'Value_Size /= 64 or else
     F64'Alignment /= 8 or else F64' Machine_Radix /= 2 or else
     F64' Machine_Mantissa /= 53 or else F64' Machine_Emin /= -1021 or else
     F64' Machine_Emax /= 1024 or else not F64'Signed_Zeros or else
     Interfaces.C.double'Size /= 64 or else Interfaces.C.double'Machine_Mantissa /= 53 or else
     Long_Float'Size /= 64 or else Long_Float'Machine_Mantissa /= 53
   then raise Program_Error with "unsupported Ada representation"; end if;
   if ABI (Interfaces.C.double (1.0 + 2.0 ** (-52)), Positive, Negative_Zero) /= 0 or else
     Bits (F64 (Positive)) /= "3FF0000000000001" or else Bits (F64 (Negative_Zero)) /= "8000000000000000"
   then raise Program_Error with "C/Ada ABI mismatch"; end if;
   Open (Input, In_File, Ada.Command_Line.Argument (2));
   while not End_Of_File (Input) loop
      declare
         Line : constant String := Get_Line (Input);
      begin
         Count := Count + 1;
         if Mode = "convert" then
            declare
               T1 : constant Natural := Ada.Strings.Fixed.Index (Line, ASCII.HT & "");
               T2 : constant Natural := Ada.Strings.Fixed.Index (Line, ASCII.HT & "", T1 + 1);
               Want : constant String := Line (T1 + 1 .. T2 - 1);
               Original : constant String := Line (T2 + 1 .. Line'Last);
               Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (Original);
               Parser : JSON_Parser;
               Event : JSON_Parser_Event := Parse_Next (Parser, Data);
               Kind_Text : constant String := Event.Kind'Image;
               Token : constant String := GNATCOLL.Buffer.Token (Data, Event.First, Event.Last);
               Value : Interfaces.C.double := 42.0;
               Consumed : Interfaces.C.size_t := 0;
               Error : Interfaces.C.int := 0;
               Status : Interfaces.C.int;
               procedure Row (Candidate, Actual, Detail : String) is
               begin
                  Put_Line (Candidate & ASCII.HT & Want & ASCII.HT & Actual & ASCII.HT &
                            Kind_Text & ASCII.HT & Detail & ASCII.HT & Original);
               end Row;
            begin
               if Event.Kind not in INTEGER_VALUE | NUMBER_VALUE or else Token /= Original then
                  raise Program_Error with "SAX token attribution failed";
               end if;
               Event := Parse_Next (Parser, Data);
               if Event.Kind /= DOC_END then raise Program_Error with "incomplete token"; end if;
               Status := Parse (Interfaces.C.To_C (Token), Token'Length, Token'Length + 1, Value, Consumed, Error);
               Row ("strtod_l", (if Status = 0 then Bits (F64 (Value)) else "REJECT"),
                    Status'Image & ":" & Consumed'Image & ":" & Error'Image & ":" & Bits (F64 (Value)));
               begin
                  Row ("IEEE_Value", Classification (F64'Value (Token)), "complete");
               exception
                  when E : Constraint_Error => Row ("IEEE_Value", "EXCEPTION", Ada.Exceptions.Exception_Name (E));
               end;
               begin
                  declare
                     Result : constant Read_Result := Read (Original);
                     Number : F64;
                  begin
                     if not Result.Success then Row ("DOM", "REJECT", "Read_Result");
                     else
                        if Result.Value.Kind = JSON_Int_Type then
                           Number := F64 (Long_Long_Integer'(Get (Result.Value)));
                        else Number := F64 (Get_Long_Float (Result.Value)); end if;
                        Row ("DOM", Classification (Number), Result.Value.Kind'Image);
                     end if;
                  end;
               exception
                  when E : Constraint_Error => Row ("DOM", "EXCEPTION", Ada.Exceptions.Exception_Name (E));
               end;
            end;
         elsif Mode = "encode" then
            declare
               Value : constant F64 := From_Bits (Interfaces.Unsigned_64'Value ("16#" & Line & "#"));
               C_Value : aliased Interfaces.C.double := Interfaces.C.double (Value);
               Buffer : Interfaces.C.char_array (0 .. 31);
               Status : Interfaces.C.int;
            begin
               Put_Line ("writer" & ASCII.HT & Line & ASCII.HT & Write (Create (Long_Float (Value))));
               Status := Format (C_Value'Access, Buffer, Buffer'Length);
               if Status /= 0 then raise Program_Error with "formatter rejected finite value"; end if;
               Put_Line ("format17" & ASCII.HT & Line & ASCII.HT & Interfaces.C.To_Ada (Buffer));
            end;
         else raise Program_Error with "unknown mode"; end if;
      end;
   end loop;
   Close (Input);
   if Locale_Active then
      if Locale_Leave /= 0 then raise Program_Error with "locale restoration"; end if;
      Locale_Active := False;
   end if;
   Put_Line ("TASK096_NUMERIC_COMPLETE mode=" & Mode & " count=" & Count'Image);
exception
   when E : others =>
      if Locale_Active then
         declare Status : constant Interfaces.C.int := Locale_Leave;
         begin if Status /= 0 then Put_Line ("LOCALE_RESTORATION_FAILED"); end if; end;
      end if;
      Put_Line ("INCOMPLETE " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Numeric_Probe;