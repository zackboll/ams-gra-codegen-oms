with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Interfaces.C;
with Programs.Oam;
with Service_Codec;
with AMS_GRA_OMS_JSON_Float32;

procedure Hexbinary_Float32_Environment is
   use type Interfaces.C.int;
   use type Interfaces.C.unsigned;
   function Save return Interfaces.C.int with Import, Convention => C, External_Name => "task095_rounding_save";
   function Restore return Interfaces.C.int with Import, Convention => C, External_Name => "task095_rounding_restore";
   function Pair (X, S : Interfaces.C.unsigned) return Interfaces.C.int with Import, Convention => C, External_Name => "task095_rounding_pair";
   function X87 return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_x87";
   function MXCSR return Interfaces.C.unsigned with Import, Convention => C, External_Name => "task095_mxcsr";
   procedure Mode (Value : Interfaces.C.unsigned) with Import, Convention => C, External_Name => "task095_mode";
   Saved : Boolean := False;
   File : Ada.Text_IO.File_Type;
   Text : Unbounded_String;
   Checks, Rejections : Natural := 0;
   procedure Check (OK : Boolean) is
   begin
      if not OK then raise Program_Error with "mixed environment regression"; end if;
      Checks := Checks + 1;
   end Check;
begin
   Check (Save = 0); Saved := True;
   Check (Pair (0, 0) = 0); Mode (0);
   Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   Text := To_Unbounded_String (Ada.Text_IO.Get_Line (File));
   Ada.Text_IO.Close (File);
   declare
      Seed : constant Programs.Oam.Mixed := Service_Codec.Decode_Mixed (To_String (Text));
      Baseline : constant String := Service_Codec.Encode_Mixed (Seed);
   begin
      for Daz_Ftz in 0 .. 3 loop
         for X in 0 .. 3 loop
            for S in 0 .. 3 loop
               for Encode in Boolean loop
                  declare
                     Target : Programs.Oam.Mixed := Seed;
                     Output : Unbounded_String := To_Unbounded_String ("sentinel");
                     Rejected : Boolean := False;
                     Before_X, Before_S, After_X, After_S : Interfaces.C.unsigned;
                     Supported : constant Boolean := Daz_Ftz = 0 and X = 0 and S = 0;
                  begin
                     Check (Pair (Interfaces.C.unsigned (X), Interfaces.C.unsigned (S)) = 0);
                     Mode (Interfaces.C.unsigned (Daz_Ftz));
                     Before_X := X87; Before_S := MXCSR;
                     begin
                        if Encode then
                           Output := To_Unbounded_String (Service_Codec.Encode_Mixed (Seed));
                        else
                           Target := Service_Codec.Decode_Mixed (To_String (Text));
                        end if;
                     exception
                        when AMS_GRA_OMS_JSON_Float32.Unsupported_Platform => Rejected := True;
                     end;
                     After_X := X87; After_S := MXCSR;
                     Check (Pair (0, 0) = 0); Mode (0);
                     Check (Before_X = After_X and then (Before_S and 16#FFC0#) = (After_S and 16#FFC0#));
                     Check (Rejected /= Supported);
                     if Rejected then
                        Rejections := Rejections + 1;
                        Check (Output = "sentinel");
                        Check (Service_Codec.Encode_Mixed (Target) = Baseline);
                     elsif Encode then
                        Check (To_String (Output) = Baseline);
                     else
                        Check (Service_Codec.Encode_Mixed (Target) = Baseline);
                     end if;
                  end;
               end loop;
            end loop;
         end loop;
      end loop;
      Check (Service_Codec.Encode_Mixed (Service_Codec.Decode_Mixed (To_String (Text))) = Baseline);
   end;
   Check (Restore = 0); Saved := False;
   if Checks /= 770 or Rejections /= 126 then raise Program_Error with "incomplete mixed environment controls"; end if;
   Ada.Text_IO.Put_Line ("TASK093_C3_ENVIRONMENT_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
exception
   when Error : others =>
      if Saved then
         declare Status : constant Interfaces.C.int := Restore; pragma Unreferenced (Status);
         begin null; end;
      end if;
      Ada.Text_IO.Put_Line (Ada.Exceptions.Exception_Information (Error));
      Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Hexbinary_Float32_Environment;