with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Text_IO;
with Interfaces.C;
with Programs.Oam;
with Service_Codec;
with AMS_GRA_OMS_JSON_Float32;
procedure Float32_Rounding is
   package Model renames Programs.Oam;
   package F renames AMS_GRA_OMS_JSON_Float32;
   use type Interfaces.C.unsigned;
   use type Interfaces.C.int;
   use type F.Float32;
   function Save return Interfaces.C.int with Import, Convention => C, External_Name => "task095_rounding_save";
   function Restore return Interfaces.C.int with Import, Convention => C, External_Name => "task095_rounding_restore";
   function Pair (X87, SSE : Interfaces.C.unsigned) return Interfaces.C.int with Import, Convention => C, External_Name => "task095_rounding_pair";
   function X87 return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_x87";
   function MXCSR return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_mxcsr";
   function Bits (Value : Interfaces.C.C_float) return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_bits";
   function From_Bits (Value : Interfaces.C.unsigned) return Interfaces.C.C_float with Import, Convention => C, External_Name => "task095_from_bits";
   Selected_X : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (2));
   Selected_S : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (3));
   Original_X : constant Interfaces.C.unsigned := X87;
   Original_S : constant Interfaces.C.unsigned := MXCSR;
   Saved : Boolean := False;
   Supported : constant Boolean := Selected_X = 0 and Selected_S = 0;
   Checks, Rejections : Natural := 0;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "rounding regression"; end if;
      Checks := Checks + 1;
   end;
   type Patterns is array (Positive range <>) of Interfaces.C.unsigned;
   Values : constant Patterns := [16#3F80_0000#,16#3F80_0001#,16#BF80_0001#,0,16#8000_0000#,
     1,16#8000_0001#,16#007F_FFFF#,16#0080_0000#];
   Input : Ada.Text_IO.File_Type;
begin
   Check (Save = 0); Saved := True;
   Check (Pair (0, 0) = 0);
   Check ((MXCSR and 16#8040#) = 0);
   Ada.Text_IO.Open (Input, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   declare
      Text : constant String := Ada.Text_IO.Get_Line (Input);
      Seed : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload (Text);
   begin
      Ada.Text_IO.Close (Input);
      Check (Model.Value (Seed.Closed) = 1.0 and then Model.Value (Seed.Open) = 1.0 + 2.0 ** (-23));
      Check (F.Number ("1.000000059604644775390624") = 1.0);
      Check (F.Number ("1.000000059604644775390626") = 1.0 + 2.0 ** (-23));
      Check (F.Number ("1.000000178813934326171874") = 1.0 + 2.0 ** (-23));
      for Pattern of Values loop
         Check (Pair (0, 0) = 0);
         declare
            Value : Model.FloatPayload := Seed;
            Target : Model.FloatPayload := Seed;
            Encoded_Baseline : constant String := Service_Codec.Encode_FloatPayload (Seed);
            X : constant F.Float32 := F.Float32 (From_Bits (Pattern));
            Number_Target : F.Float32 := 123.0;
         begin
            Value.Required := X;
            for Operation in 0 .. 4 loop
               declare
                  Published, Rejected : Boolean := False;
                  Before_X, Before_S, After_X, After_S : Interfaces.C.unsigned;
                  Output_Bits : Interfaces.C.unsigned := 0;
               begin
                  Check (Pair (Selected_X, Selected_S) = 0);
                  Before_X := X87; Before_S := MXCSR;
                  begin
                     case Operation is
                        when 0 =>
                           declare
                              JSON : constant String := Service_Codec.Encode_FloatPayload (Value);
                           begin
                              After_X := X87; After_S := MXCSR;
                              Check (Pair (0, 0) = 0);
                              Target := Service_Codec.Decode_FloatPayload (JSON);
                              Output_Bits := Bits (Interfaces.C.C_float (Target.Required));
                           end;
                        when 1 =>
                           Target := Service_Codec.Decode_FloatPayload (Encoded_Baseline);
                           After_X := X87; After_S := MXCSR;
                        when 2 => F.Require_Platform; After_X := X87; After_S := MXCSR;
                        when 3 => Number_Target := F.Number ("1e-45"); After_X := X87; After_S := MXCSR;
                        when 4 => Number_Target := F.Special ("Infinity"); After_X := X87; After_S := MXCSR;
                     end case;
                     Published := True;
                  exception when F.Unsupported_Platform =>
                     After_X := X87; After_S := MXCSR; Rejected := True;
                  end;
                  Check (Pair (0, 0) = 0);
                  Check (Before_X = After_X and then (Before_S and 16#FFC0#) = (After_S and 16#FFC0#));
                  Check ((Supported and Published and not Rejected) or else (not Supported and Rejected and not Published));
                  if Rejected then
                     Rejections := Rejections + 1;
                     Check (Bits (Interfaces.C.C_float (Target.Required)) = Bits (Interfaces.C.C_float (Seed.Required)));
                     Check (Bits (Interfaces.C.C_float (Number_Target)) = 16#42F6_0000#);
                  elsif Operation = 0 then Check (Output_Bits = Pattern);
                  end if;
               end;
            end loop;
         end;
      end loop;
      Check (Pair (0, 0) = 0);
      declare
         Again : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload (Service_Codec.Encode_FloatPayload (Seed));
      begin Check (Again.Required = Seed.Required); end;
   end;
   Check (Restore = 0); Saved := False;
   Check (X87 = Original_X and then MXCSR = Original_S);
   if Checks /= (if Supported then 218 else 290) or else Rejections /= (if Supported then 0 else 45) then
      raise Program_Error with "incomplete rounding assertions";
   end if;
   Ada.Text_IO.Put_Line ("TASK095_C3_ADA_ROUNDING_COMPLETE x87=" & Selected_X'Image &
     " mxcsr=" & Selected_S'Image & " checks=" & Checks'Image & " rejections=" & Rejections'Image);
exception when E : others =>
   if Saved then
      declare Status : constant Interfaces.C.int := Restore; pragma Unreferenced (Status);
      begin null; end;
   end if;
   Ada.Text_IO.Put_Line (Ada.Exceptions.Exception_Information (E));
   Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Float32_Rounding;