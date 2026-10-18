-- Raw candidate observations, deliberately NOT a guarded production operation.
-- No unsupported-environment candidate is certified by these observations.
with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Unchecked_Conversion;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with Interfaces;
with Interfaces.C;
procedure Candidate_Environment is
   subtype F64 is Interfaces.IEEE_Float_64;
   use type Interfaces.C.int;
   use type Interfaces.C.unsigned;
   use type Interfaces.Unsigned_64;
   function To_Bits is new Ada.Unchecked_Conversion (F64, Interfaces.Unsigned_64);
   function From_Bits is new Ada.Unchecked_Conversion (Interfaces.Unsigned_64, F64);
   type Pattern_Array is array (Positive range <>) of Interfaces.Unsigned_64;
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
   function Save return Interfaces.C.int with Import, Convention => C, External_Name => "task095_rounding_save";
   function Restore return Interfaces.C.int with Import, Convention => C, External_Name => "task095_rounding_restore";
   function Pair (X87, SSE : Interfaces.C.unsigned) return Interfaces.C.int
     with Import, Convention => C, External_Name => "task095_rounding_pair";
   procedure Mode (Value : Interfaces.C.unsigned)
     with Import, Convention => C, External_Name => "task095_mode";
   function X87_Control return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_x87";
   function SSE_Control return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_mxcsr";
   X87 : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (1));
   SSE : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (2));
   Flushing : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (3));
   Original_X87 : constant Interfaces.C.unsigned := X87_Control;
   Original_SSE : constant Interfaces.C.unsigned := SSE_Control;
   Saved : Boolean := False;
   File : File_Type;
   Count : Natural := 0;
begin
   Open (File, In_File, Ada.Command_Line.Argument (4));
   if Save /= 0 then raise Program_Error; end if;
   Saved := True;
   if Pair (X87, SSE) /= 0 then raise Program_Error; end if;
   Mode (Flushing);
   while not End_Of_File (File) loop
      declare
         Text : constant String := Get_Line (File);
         function IEEE return String is
         begin return Bits (F64'Value (Text));
         exception when E : Constraint_Error => return Ada.Exceptions.Exception_Name (E); end;
         function DOM return String is
            Result : constant Read_Result := Read (Text);
         begin
            if not Result.Success then return "REJECT"; end if;
            if Result.Value.Kind = JSON_Int_Type then
               return Bits (F64 (Long_Long_Integer'(Get (Result.Value))));
            else return Bits (F64 (Get_Long_Float (Result.Value))); end if;
         exception when E : Constraint_Error => return Ada.Exceptions.Exception_Name (E); end;
         First : constant String := IEEE;
         Second : constant String := DOM;
      begin
         Count := Count + 1;
         Put_Line (Text & ASCII.HT & First & ASCII.HT & Second);
      end;
   end loop;
   for Pattern of Pattern_Array'(16#3FF0_0000_0000_0001#, 16#0000_0000_0000_0001#) loop
      declare
         Value : constant F64 := From_Bits (Pattern);
      begin
         Put_Line ("WRITER" & ASCII.HT & Bits (Value) & ASCII.HT & Write (Create (Long_Float (Value))));
      end;
   end loop;
   if Restore /= 0 then raise Program_Error; end if;
   Saved := False;
   if Original_X87 /= X87_Control or Original_SSE /= SSE_Control then raise Program_Error; end if;
   Close (File);
   Put_Line ("TASK096_RAW_CANDIDATE_ENVIRONMENT_COMPLETE count=" & Count'Image);
exception
   when E : others =>
      if Saved then
         declare Status : constant Interfaces.C.int := Restore;
         begin if Status /= 0 then Put_Line ("RESTORATION_FAILED"); end if; end;
      end if;
      Put_Line ("INCOMPLETE " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Candidate_Environment;