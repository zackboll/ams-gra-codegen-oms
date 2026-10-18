with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Ada.Unchecked_Conversion;
with Interfaces; use type Interfaces.Unsigned_8; use type Interfaces.Unsigned_32;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;

procedure Hexbinary_Float32_Consumer is
   use type Interfaces.IEEE_Float_32;
   function Bits is new Ada.Unchecked_Conversion (Interfaces.IEEE_Float_32, Interfaces.Unsigned_32);
   Text : Unbounded_String;
   Checks, Rejections : Natural := 0;
   procedure Check (OK : Boolean; Label_Text : String) is
   begin
      if not OK then raise Program_Error with Label_Text; end if;
      Checks := Checks + 1;
   end Check;
   function Replaced (Input, Old, New_Text : String) return String is
      At_Pos : constant Natural := Ada.Strings.Fixed.Index (Input, Old);
   begin
      if At_Pos = 0 then raise Program_Error with "missing mutation target: " & Old; end if;
      return Input (Input'First .. At_Pos - 1) & New_Text & Input (At_Pos + Old'Length .. Input'Last);
   end Replaced;
   procedure Reject (Input : String; Indexed : Boolean := False) is
      Target : Mixed := Decode_Mixed (To_String (Text));
      Before : constant String := Encode_Mixed (Target);
      Serialized : Unbounded_String := To_Unbounded_String ("sentinel");
   begin
      begin
         Target := Decode_Mixed (Input);
         Serialized := To_Unbounded_String (Encode_Mixed (Target));
      exception
         when Error : Codec_Error =>
            Check (Encode_Mixed (Target) = Before, "typed result unchanged");
            Check (Serialized = "sentinel", "serialized result unchanged");
            if Indexed then
               Check (Ada.Strings.Fixed.Index (Ada.Exceptions.Exception_Message (Error), "Picks item 3") > 0,
                      "outer indexed attribution");
            end if;
            Rejections := Rejections + 1;
            return;
      end;
      raise Program_Error with "invalid mixed payload accepted";
   end Reject;
begin
   declare
      File : Ada.Text_IO.File_Type;
   begin
      Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
      while not Ada.Text_IO.End_Of_File (File) loop
         Append (Text, Ada.Text_IO.Get_Line (File));
      end loop;
      Ada.Text_IO.Close (File);
   end;
   declare
      Item : constant Mixed := Decode_Mixed (To_String (Text));
      Again : constant Mixed := Decode_Mixed (Encode_Mixed (Item));
      First : constant PathLength := Element (Again.Picks, 1).Nested;
      Last : constant PathLength := Element (Again.Picks, 3).Nested;
      Encoded : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Encode_Mixed (Item));
      Picks : constant JSON_Array := Get (Encoded.Get ("Picks"));
   begin
      Check (Length (Again.Picks) = 3, "Choice ordering length");
      Check (Element (Again.Picks, 1).Kind = Nested_Kind, "first discriminant");
      Check (Element (Again.Picks, 2).Kind = Enabled_Kind and then not Element (Again.Picks, 2).Enabled, "middle alternative");
      Check (Element (Again.Picks, 3).Kind = Nested_Kind, "last discriminant");
      Check (Natural (First.Data.Length) = 3, "direct octet count");
      Check (First.Data.Element (0) = 0 and then First.Data.Element (1) = 16#80# and then First.Data.Element (2) = 16#FF#, "direct octets");
      Check (Natural (Last.Data.Length) = 0, "empty direct Binary");
      Check (First.Optional.Is_Present and then Natural (First.Optional.Value.Length) = 0, "present empty optional");
      Check (not Last.Optional.Is_Present, "optional absent");
      Check (Natural (First.Blob.Value.Length) = 0, "unconstrained named empty");
      Check (Last.Blob.Value.Element (0) = 16#FF# and then Last.Blob.Value.Element (1) = 0, "named octets");
      Check (First.Hash.Is_Present and then Natural (Value (First.Hash.Value).Length) = 2, "checked lower bound");
      Check (Value (First.Hash.Value).Element (0) = 16#A1# and then Value (First.Hash.Value).Element (1) = 16#FE#, "checked octets");
      Check (String'(Get (Get (Picks, 1).Get ("Nested"), "Data")) = "0080FF", "uppercase canonical direct");
      Check (String'(Get (Get (Picks, 1).Get ("Nested"), "Hash")) = "A1FE", "uppercase canonical checked");
      Check (Bits (First.Required) = 16#3F80_0001#, "original midpoint lexeme");
      Check (Bits (Last.Required) = 16#8000_0000#, "integer negative zero");
      Check (Value (First.Checked) = 1.0 and then Value (Last.Checked) = -1.0, "checked Float32 boundaries");
      Check (Length (First.Samples) = 3, "repeated Float32 length");
      Check (Bits (Element (First.Samples, 1)) = 16#8000_0000#, "array signed zero");
      Check (Bits (Element (First.Samples, 2)) = 1, "array minimum subnormal");
      Check (Bits (Element (First.Samples, 3)) = 16#3F80_0000#, "array lower midpoint");
      Check (Length (First.Dates) = 2 and then Value (Element (First.Dates, 1)) = "2024-02-29T12:34:56Z" and then
             Value (Element (First.Dates, 2)) = "2026-10-15T01:02:03+05:30", "DateTime order");
      Check (Length (First.Spans) = 2 and then Value (Element (First.Spans, 1)) = "P1D" and then
             Value (Element (First.Spans, 2)) = "-PT0.5S", "Duration order");
      Check (Encode_Mixed (Again) = Encode_Mixed (Item), "canonical round trip");
      declare
         File : Ada.Text_IO.File_Type;
      begin
         Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (2));
         declare
            Expected : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Ada.Text_IO.Get_Line (File));
         begin
            if Encoded /= Expected then
               Ada.Text_IO.Put_Line ("ACTUAL " & AMS_GRA_OMS_JSON_Guard.Write (Encoded));
               Ada.Text_IO.Put_Line ("EXPECTED " & AMS_GRA_OMS_JSON_Guard.Write (Expected));
            end if;
            Check (Encoded = Expected, "independent canonical golden");
         end;
         Ada.Text_IO.Close (File);
      end;
   end;
   declare
      Item : constant Mixed := Decode_Mixed (Replaced (To_String (Text), "a1fe", "a1fe00"));
   begin
      Check (Natural (Value (Element (Item.Picks, 1).Nested.Hash.Value).Length) = 3, "checked upper bound");
   end;
   declare
      Item : constant Mixed := Decode_Mixed (Replaced (To_String (Text), "0080ff", " \t0080ff\r\n"));
   begin Check (Element (Item.Picks, 1).Nested.Data.Element (1) = 16#80#, "XML whitespace"); end;
   -- Duplicate decoded keys replace whole semantic values before model construction.
   for Escaped in Boolean loop
      declare
         Key : constant String := (if Escaped then "Da\u0074a" else "Data");
         Good : constant String := """" & Key & """:""0080ff""";
         Bad : constant String := """Data"":""GG""";
         Original : constant String := """Data"":""0080ff""";
         Item : constant Mixed := Decode_Mixed (Replaced (To_String (Text), Original, Bad & "," & Good));
      begin
         Check (Element (Item.Picks, 1).Nested.Data.Element (1) = 16#80#, "last decoded Binary wins");
         Reject (Replaced (To_String (Text), Original, Good & "," & Bad));
      end;
   end loop;
   declare
      Input : constant String := To_String (Text);
      Old : constant String := """Nested"":{";
      Item : constant Mixed := Decode_Mixed (Replaced (Input, Old,
        """Nested"":{""Data"":""GG"",""Required"":1e1000},""Nes\u0074ed"":{"));
   begin
      Check (Bits (Element (Item.Picks, 1).Nested.Required) = 16#3F80_0001#, "duplicate subtree owns Binary and original Float32");
      Reject (Replaced (Input, Old, """Nested"":{""Data"":""GG"",""Required"":1e+},""Nested"":{"));
   end;
   for Kind in 0 .. 10 loop
      declare
         Input : constant String := To_String (Text);
      begin
         case Kind is
            when 0 => Reject (Replaced (Input, """Data"":""""", """Data"":""0"""), True);
            when 1 => Reject (Replaced (Input, """Data"":""""", """Data"":""0G"""), True);
            when 2 => Reject (Replaced (Input, """Data"":""""", """Data"":""AA BB"""), True);
            when 3 => Reject (Replaced (Input, """Data"":""""", """Data"":null"), True);
            when 4 => Reject (Replaced (Input, """Data"":""""", """Data"":[]"), True);
            when 5 => Reject (Replaced (Input, """Data"":""""", """Data"":1"), True);
            when 6 => Reject (Replaced (Input, """Required"":-0", """Required"":1e1000"), True);
            when 7 => Reject (Replaced (Input, """Required"":-0", """Required"":""nan"""), True);
            when 8 => Reject (Replaced (Input, """Checked"":-1", """Checked"":2"), True);
            when 9 => Reject (Replaced (Input, """Required"":-0", """Unknown"":0,""Required"":-0"), True);
            when 10 => Reject (Replaced (Input, """Data"":""""", """Data"":true"), True);
            when others => raise Program_Error;
         end case;
      end;
   end loop;
   Reject (Replaced (To_String (Text), "a1fe", ""));
   Reject (Replaced (To_String (Text), "a1fe", "00"));
   Reject (Replaced (To_String (Text), "a1fe", "00000000"));
   Reject (Replaced (To_String (Text), "-0,1e-45", "-0,1e1000"));
   Reject (Replaced (To_String (Text), "-PT0.5S", "P1W"));
   Reject (Replaced (To_String (Text), "2026-10-15T01:02:03+05:30", "2023-02-29T00:00:00Z"));
   Reject ("{""Picks"":[]}");
   Reject ("{""Picks"":[{""Enabled"":true},{""Enabled"":true},{""Enabled"":true},{""Enabled"":true}]}");
   Reject (Replaced (To_String (Text), """Enabled"":false", """Enabled"":false,""Nested"":{}"));
   Reject (Replaced (To_String (Text), "0080ff", "\uD800"));
   declare
      Large : constant String (1 .. 200_000) := [others => 'A'];
      Item : constant Mixed := Decode_Mixed (Replaced (To_String (Text), "0080ff", Large));
   begin Check (Natural (Element (Item.Picks, 1).Nested.Data.Length) = 100_000, "unconstrained Binary not given Log octet limit"); end;
   declare
      Oversize : constant String (1 .. 1_048_577) := [others => 'A'];
   begin Reject (Replaced (To_String (Text), "0080ff", Oversize)); end;
   declare
      Deep : Unbounded_String;
   begin
      for I in 1 .. 130 loop Append (Deep, "["); end loop;
      Append (Deep, "0");
      for I in 1 .. 130 loop Append (Deep, "]"); end loop;
      Reject (Replaced (To_String (Text), """Data"":""""", """Data"":" & To_String (Deep)));
   end;
   Ada.Text_IO.Put_Line ("TASK093_C3_MIXED_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
end Hexbinary_Float32_Consumer;