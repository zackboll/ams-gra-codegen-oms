with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Interfaces;
with Interfaces.C;
with Programs.Oam;
with Service_Codec;
with AMS_GRA_OMS_JSON_Float64;
procedure Environment is
   use type Interfaces.C.int;
   use type Interfaces.C.unsigned;
   package Model renames Programs.Oam;
   function X87 return Interfaces.C.unsigned
     with Import, Convention => C, External_Name => "task095_x87";
   function MXCSR return Interfaces.C.unsigned
     with Import, Convention => C, External_Name => "task095_mxcsr";
   function Save return Interfaces.C.int
     with Import, Convention => C, External_Name => "task095_rounding_save";
   function Restore return Interfaces.C.int
     with Import, Convention => C, External_Name => "task095_rounding_restore";
   function Pair (A, B : Interfaces.C.unsigned) return Interfaces.C.int
     with Import, Convention => C, External_Name => "task095_rounding_pair";
   procedure Mode (Flushing : Interfaces.C.unsigned)
     with Import, Convention => C, External_Name => "task095_mode";
   function Key (Local : String) return String is (Local);
   function Document return String is
     ("{""" & Key ("Record") & """:{""" & Key ("Direct") & """:-0,""" &
      Key ("Named") & """:5e-324,""" & Key ("Checked") & """:0.5,""" &
      Key ("Data") & """:""80FF"",""" & Key ("Single") & """:1,""" &
      Key ("NamedSamples") & """:[0.5]},""" & Key ("Pick") & """:{""" &
      Key ("Enabled") & """:true},""" & Key ("Picks") & """:[{""" &
      Key ("Enabled") & """:false}]}");
   Item : Model.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (Document);
   Original : constant String := Service_Codec.Encode_LosslessEnvelope (Item);
   Output : Unbounded_String := To_Unbounded_String ("sentinel");
   A : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (1));
   B : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (2));
   F : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (3));
   Supported : constant Boolean := A = 0 and then B = 0 and then F = 0;
   Original_X87 : constant Interfaces.C.unsigned := X87;
   Original_MXCSR : constant Interfaces.C.unsigned := MXCSR;
   Before_X87, Before_MXCSR : Interfaces.C.unsigned;
   Saved : Boolean := False;
   Rejections : Natural := 0;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "environment contract"; end if;
   end Check;
begin
   Check (Save = 0);
   Saved := True;
   Check (Pair (A, B) = 0);
   Mode (F);
   Before_X87 := X87; Before_MXCSR := MXCSR;
   begin
      Output := To_Unbounded_String (Service_Codec.Encode_LosslessEnvelope (Item));
      Check (Supported);
   exception
      when AMS_GRA_OMS_JSON_Float64.Unsupported_Platform =>
         Check (not Supported and then Output = "sentinel");
         Rejections := Rejections + 1;
   end;
   begin
      Item := Service_Codec.Decode_LosslessEnvelope ("malformed before work");
      raise Program_Error with "malformed decoded";
   exception
      when AMS_GRA_OMS_JSON_Float64.Unsupported_Platform =>
         Check (not Supported); Rejections := Rejections + 1;
      when Service_Codec.Codec_Error => Check (Supported);
   end;
   Check (X87 = Before_X87 and then (MXCSR and not 63) = (Before_MXCSR and not 63));
   Check (Restore = 0); Saved := False;
   Check (X87 = Original_X87 and then MXCSR = Original_MXCSR);
   Check (Service_Codec.Encode_LosslessEnvelope (Item) = Original);
   Item := Service_Codec.Decode_LosslessEnvelope (Document);
   Check (Service_Codec.Encode_LosslessEnvelope (Item) = Original);
   Check (Rejections = (if Supported then 0 else 2));
   Ada.Text_IO.Put_Line ("TASK097_GENERATED_ENVIRONMENT_COMPLETE x87=" & A'Image &
     " mxcsr=" & B'Image & " flushing=" & F'Image & " rejections=" & Rejections'Image);
exception
   when others =>
      if Saved then
         if Restore /= 0 then raise Program_Error with "TEST harness restoration failed"; end if;
      end if;
      raise;
end Environment;