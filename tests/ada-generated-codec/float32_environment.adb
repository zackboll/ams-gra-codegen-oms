with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Text_IO;
with Interfaces.C;
with Programs.Oam;
with Service_Codec;
with AMS_GRA_OMS_JSON_Float32;
procedure Float32_Environment is
   package Model renames Programs.Oam;
   package F renames AMS_GRA_OMS_JSON_Float32;
   use type Interfaces.C.unsigned;
   use type F.Float32;
   function Controls return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_controls";
   procedure Restore (Value : Interfaces.C.unsigned) with Import, Convention => C, External_Name => "task095_restore";
   procedure Mode (Value : Interfaces.C.unsigned) with Import, Convention => C, External_Name => "task095_mode";
   function Bits (Value : Interfaces.C.C_float) return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_bits";
   function From_Bits (Value : Interfaces.C.unsigned) return Interfaces.C.C_float with Import, Convention => C, External_Name => "task095_from_bits";
   Saved : constant Interfaces.C.unsigned := Controls;
   Selected : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (2));
   Red : constant Boolean := Ada.Command_Line.Argument_Count > 2;
   Checks, Rejections : Natural := 0;
   Control_Mask : constant Interfaces.C.unsigned := 16#FFC0#; -- excludes sticky status bits 0..5
   type Patterns is array (Positive range <>) of Interfaces.C.unsigned;
   Values : constant Patterns := [1,16#8000_0001#,16#0001_16C2#,16#8001_16C2#,16#007F_FFFF#,16#0080_0000#,0,16#8000_0000#];
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "environment regression"; end if;
      Checks := Checks + 1;
   end;
   Input : Ada.Text_IO.File_Type;
begin
   Mode (0);
   Ada.Text_IO.Open (Input, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   declare
      Text : constant String := Ada.Text_IO.Get_Line (Input);
      Seed : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload (Text);
   begin
      Ada.Text_IO.Close (Input);
      for Pattern of Values loop
         Mode (0);
         declare
            Value : Model.FloatPayload := Seed;
            Result : Model.FloatPayload := Seed;
            X : constant F.Float32 := F.Float32 (From_Bits (Pattern));
            Before : Interfaces.C.unsigned;
            After : Interfaces.C.unsigned;
            Published : Boolean := False;
            Rejected : Boolean := False;
         begin
            Value.Required := X;
            -- Construct data only in supported mode, then alter only DAZ/FTZ.
            Mode (Selected); Before := Controls;
            begin
               declare
                  Encoded : constant String := Service_Codec.Encode_FloatPayload (Value);
               begin
                  After := Controls;
                  Mode (0);
                  Result := Service_Codec.Decode_FloatPayload (Encoded);
                  Published := True;
               end;
            exception when F.Unsupported_Platform => After := Controls; Rejected := True;
            end;
            Mode (0);
            if Red then
               Ada.Text_IO.Put_Line ("OBSERVE mode=" & Selected'Image & " input_bits=" & Pattern'Image &
                 " encoded=" & Published'Image & " rejected=" & Rejected'Image &
                 " result_bits=" & Bits (Interfaces.C.C_float (Result.Required))'Image);
            else
               Check ((Before and Control_Mask) = (After and Control_Mask));
               if Selected = 0 then
                  Check (Published and not Rejected);
                  Check (Bits (Interfaces.C.C_float (Result.Required)) = Pattern);
               else
                  Check (Rejected and not Published); Rejections := Rejections + 1;
               end if;
            end if;
            -- Generated decode must also reject without publishing a typed value.
            Published := False; Rejected := False;
            Result := Seed;
            Mode (Selected); Before := Controls;
            begin
               Result := Service_Codec.Decode_FloatPayload (Text);
               After := Controls; Published := True;
            exception when F.Unsupported_Platform => After := Controls; Rejected := True;
            end;
            Mode (0);
            if Red then
               Ada.Text_IO.Put_Line ("DECODE mode=" & Selected'Image & " published=" & Published'Image & " rejected=" & Rejected'Image);
            else
               Check ((Before and Control_Mask) = (After and Control_Mask));
               Check ((Selected = 0 and Published and not Rejected) or else (Selected /= 0 and Rejected and not Published));
               if Rejected then
                  Rejections := Rejections + 1;
                  if Result.Required /= Seed.Required then raise Program_Error with "typed output changed on rejection"; end if;
               end if;
            end if;
         end;
      end loop;
      if not Red then
         for Operation in 0 .. 3 loop
            declare
               Output : F.Float32 := 123.0;
               Published, Rejected : Boolean := False;
               Before, After : Interfaces.C.unsigned;
            begin
               Mode (Selected); Before := Controls;
               begin
                  case Operation is
                     when 0 => F.Require_Platform;
                     when 1 => Output := F.Number ("1e-40");
                     when 2 => Output := F.Special ("NaN");
                     when 3 =>
                        declare
                           Encoded : constant String := Service_Codec.Encode_FloatPayload (Seed);
                           pragma Unreferenced (Encoded);
                        begin null; end;
                  end case;
                  After := Controls; Published := True;
               exception when F.Unsupported_Platform => After := Controls; Rejected := True;
               end;
               Mode (0);
               Check ((Before and Control_Mask) = (After and Control_Mask));
               Check ((Selected = 0 and Published and not Rejected) or else (Selected /= 0 and Rejected and not Published));
               if Selected /= 0 and Operation in 1 .. 2 then Check (Output = 123.0); end if;
            end;
         end loop;
      end if;
      Mode (0);
      declare
         Result : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload (Service_Codec.Encode_FloatPayload (Seed));
      begin
         Check (Result.Required = Seed.Required); -- approval must not be cached
      end;
   end;
   Restore (Saved);
   if not Red then
      if Checks /= (if Selected = 0 then 49 else 43) or else Rejections /= (if Selected = 0 then 0 else 16) then
         raise Program_Error with "incomplete environment execution";
      end if;
   end if;
   Ada.Text_IO.Put_Line ("TASK095_C2_ENVIRONMENT_COMPLETE mode=" & Selected'Image & " checks=" & Checks'Image & " rejections=" & Rejections'Image);
exception when E : others =>
   Restore (Saved);
   Ada.Text_IO.Put_Line (Ada.Exceptions.Exception_Information (E));
   Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Float32_Environment;