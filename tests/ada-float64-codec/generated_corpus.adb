with Ada.Command_Line;
with Ada.Strings.Fixed;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Unchecked_Conversion;
with Interfaces;
with Programs.Oam;
with Service_Codec;
procedure Generated_Corpus is
   package Model renames Programs.Oam;
   subtype F64 is Interfaces.IEEE_Float_64;
   use type Interfaces.Unsigned_64;
   use type Interfaces.IEEE_Float_32;
   function To_Bits is new Ada.Unchecked_Conversion (F64, Interfaces.Unsigned_64);
   function From_Bits is new Ada.Unchecked_Conversion (Interfaces.Unsigned_64, F64);
   function Bits (Item : F64) return String is
      Hex : constant String := "0123456789ABCDEF";
      N : Interfaces.Unsigned_64 := To_Bits (Item);
      Result : String (1 .. 16);
   begin
      for J in reverse Result'Range loop
         Result (J) := Hex (Integer (N and 15) + 1);
         N := Interfaces.Shift_Right (N, 4);
      end loop;
      return Result;
   end Bits;
   function Key (Local : String) return String is (Local);
   function Record_Text (Token : String) return String is
     ("{""" & Key ("Direct") & """:" & Token &
      ",""" & Key ("Optional") & """:" & Token &
      ",""" & Key ("Named") & """:" & Token &
      ",""" & Key ("Checked") & """:0.5,""" & Key ("Data") & """:""80FF"",""" &
      Key ("Single") & """:1.00000005960464477539062500000000001,""" &
      Key ("Samples") & """: [" & Token & "," & Token & "],""" &
      Key ("NamedSamples") & """: [0.5,-1]}");
   function Document (Token : String) return String is
      R : constant String := Record_Text (Token);
   begin
      return "{""" & Key ("Record") & """:" & R &
        ",""" & Key ("Pick") & """:{""" & Key ("Nested") & """:" & R &
        "},""" & Key ("Picks") & """:[{""" & Key ("Nested") & """:" & R & "}]}";
   end Document;
   Input : File_Type;
   Count : Natural := 0;
   Mode : constant String := Ada.Command_Line.Argument (1);
   Item : Model.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (Document ("0"));
   procedure Check_Record (R : Model.PathLength; Expected : String) is
   begin
      if Bits (R.Direct) /= Expected or else not R.Optional.Is_Present or else
        Bits (R.Optional.Value) /= Expected or else
        Bits (F64 (R.Named)) /= Expected or else Model.Length (R.Samples) /= 2 or else
        Bits (Model.Element (R.Samples, 1)) /= Expected or else
        Bits (Model.Element (R.Samples, 2)) /= Expected or else
        R.Single /= 1.0 + 2.0 ** (-23)
      then raise Program_Error with "generated conversion width/position mismatch"; end if;
   end Check_Record;
begin
   Open (Input, In_File, Ada.Command_Line.Argument (2));
   while not End_Of_File (Input) loop
      declare
         Line : constant String := Get_Line (Input);
      begin
         if Mode = "convert" then
            declare
               First : constant Natural := Ada.Strings.Fixed.Index (Line, ASCII.HT & "");
               Last : constant Natural := Ada.Strings.Fixed.Index (Line, ASCII.HT & "", First + 1);
               Expected : constant String := Line (First + 1 .. Last - 1);
               Token : constant String := Line (Last + 1 .. Line'Last);
               Before : constant String := Service_Codec.Encode_LosslessEnvelope (Item);
            begin
               begin
                  Item := Service_Codec.Decode_LosslessEnvelope (Document (Token));
                  if Expected = "OVERFLOW" then raise Program_Error with "overflow accepted"; end if;
                  Check_Record (Item.Field_Record, Expected);
                  Check_Record (Item.Pick.Nested, Expected);
                  Check_Record (Model.Element (Item.Picks, 1).Nested, Expected);
               exception
                  when Service_Codec.Codec_Error =>
                     if Expected /= "OVERFLOW" or else
                       Service_Codec.Encode_LosslessEnvelope (Item) /= Before
                     then raise Program_Error with "unexpected/non-atomic rejection"; end if;
               end;
            end;
         elsif Mode = "encode" then
            declare
               Number : constant F64 := From_Bits (Interfaces.Unsigned_64'Value ("16#" & Line & "#"));
               R : Model.PathLength := Item.Field_Record;
            begin
               R.Direct := Number;
               R.Optional := (True, Number);
               R.Named := Model.Array_Encode_Hex_Binary (Number);
               R.Samples := Model.To_Sequence ((Number, Number));
               Item.Field_Record := R;
               Item.Pick := (Kind => Model.Nested_Kind, Nested => R);
               Item.Picks := Model.To_Sequence ((1 => Item.Pick));
               Put_Line (Line & ASCII.HT & Service_Codec.Encode_LosslessEnvelope (Item));
            end;
         else
            raise Program_Error with "unknown corpus mode";
         end if;
         Count := Count + 1;
      end;
   end loop;
   Close (Input);
   Put_Line ("TASK097_GENERATED_CORPUS_COMPLETE mode=" & Mode & " count=" & Count'Image);
end Generated_Corpus;