with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Text_IO;
with Interfaces;
with Programs.Oam;
with Service_Codec;
procedure Float32_Consumer is
   package Model renames Programs.Oam;
   subtype F32 is Interfaces.IEEE_Float_32;
   use type F32;
   use type Interfaces.Unsigned_32;
   use type Model.Selection_Kind;
   use type Model.FloatPayload;
   Checks, Rejections : Natural := 0;
   function From_Bits (B : Interfaces.Unsigned_32) return F32 is
      E : constant Integer := Integer (Interfaces.Shift_Right (B, 23) and 255);
      M : constant Interfaces.Unsigned_32 := B and 16#7F_FFFF#;
      X : F32;
   begin
      X := (if E = 0 then F32' Scaling (F32 (M), -149)
            else F32' Scaling (F32 (M + 16#80_0000#), E - 150));
      return (if (B and 16#8000_0000#) /= 0 then -X else X);
   end;
   procedure Check (Condition : Boolean; Label : String) is
   begin
      if not Condition then raise Program_Error with Label; end if;
      Checks := Checks + 1;
   end;
   procedure Bits (Value : F32; Expected : Interfaces.Unsigned_32) is
      Want : constant F32 := From_Bits (Expected);
   begin
      Check (Value = Want and then
        (Value /= 0.0 or else F32'Copy_Sign (1.0, Value) = F32'Copy_Sign (1.0, Want)),
        "incorrect Float32 bits");
   end;
   Input : Ada.Text_IO.File_Type;
begin
   Ada.Text_IO.Open (Input, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   declare
      Text : constant String := Ada.Text_IO.Get_Line (Input);
      Initial : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload (Text);
      Value : Model.FloatPayload := Initial;
      procedure Verify (Item : Model.FloatPayload) is
      begin
         Bits (Item.Required, 16#3F80_0001#);
         Check (Item.Optional.Is_Present, "optional missing");
         Bits (Item.Optional.Value, 16#8000_0000#);
         Bits (F32 (Item.Named), 16#BF80_0001#);
         Bits (Model.Value (Item.Closed), 16#3F80_0000#);
         Bits (Model.Value (Item.Open), 16#3F80_0001#);
         Check (Model.Value (Item.Lower) > F32'Last, "lower-only infinity");
         Check (Model.Length (Item.Bounded) = 3, "bounded length");
         Bits (Model.Element (Item.Bounded, 1), 16#3DCC_CCCD#);
         Bits (Model.Element (Item.Bounded, 2), 16#8000_0000#);
         Bits (Model.Element (Item.Bounded, 3), 1);
         Bits (Item.Unbounded.Required (1), 16#0080_0000#);
         Bits (Item.Unbounded.Required (2), 16#7F7F_FFFF#);
         Check (Model.Length (Item.Unbounded.Additional) = 1, "additional length");
         Bits (Model.Element (Item.Unbounded.Additional, 1), 16#8000_0000#);
         Check (Model.Length (Item.NamedRepeated) = 3, "named sequence length");
         Bits (Model.Value (Model.Element (Item.NamedRepeated, 1)), 16#BF80_0000#);
         Bits (Model.Value (Model.Element (Item.NamedRepeated, 2)), 0);
         Bits (Model.Value (Model.Element (Item.NamedRepeated, 3)), 16#3F80_0000#);
         Bits (Item.Nested.Value, 16#3F80_0000#);
         Check (Item.Choice.Kind = Model.Alternative_Record_Kind, "choice kind");
         Bits (Item.Choice.Alternative_Record.Value, 16#8000_0000#);
         Check (Item.Count = 42 and Item.Enabled, "mixed fields");
         Check (Model.Value (Item.Observed) = "2026-10-17T01:02:03Z", "dateTime");
         Check (Model.Value (Item.Elapsed) = "PT1.5S", "duration");
      end;
      function Replace (Original, Old, New_Text : String) return String is
         At_Index : constant Natural := Ada.Strings.Fixed.Index (Original, Old);
      begin
         if At_Index = 0 then raise Program_Error with "missing test replacement"; end if;
         return Original (Original'First .. At_Index - 1) & New_Text &
           Original (At_Index + Old'Length .. Original'Last);
      end;
      procedure Reject (Bad : String; Diagnostic : String := "") is
      begin
         begin
            Value := Service_Codec.Decode_FloatPayload (Bad);
         exception when E : Service_Codec.Codec_Error =>
            Rejections := Rejections + 1;
            Check (Value = Initial, "failure was not atomic");
            if Diagnostic /= "" then
               Check (Ada.Strings.Fixed.Index (Ada.Exceptions.Exception_Message (E), Diagnostic) /= 0,
                      "missing indexed diagnostic");
            end if;
            return;
         end;
         raise Program_Error with "accepted invalid document";
      end;
   begin
      Verify (Initial);
      Verify (Service_Codec.Decode_FloatPayload (Service_Codec.Encode_FloatPayload (Initial)));
      Verify (Service_Codec.Decode_FloatPayload (Replace (Text, "{""Required"":", "{""Required"":[],""Requ\u0069red"":")));
      Reject (Replace (Text, "1.000000059604644775390626", "null"));
      Reject (Replace (Text, "1.000000059604644775390626", "1e9999"));
      Reject (Replace (Text, "1.000000059604644775390626", """+Infinity"""));
      Reject (Replace (Text, """Closed"":1", """Closed"":1.00000011920928955078125"));
      Reject (Replace (Text, """Closed"":1", """Closed"":""NaN"""));
      Reject (Replace (Text, """Closed"":1", """Closed"":-1.00000011920928955078125"));
      Reject (Replace (Text, """Lower"":""Infinity""", """Lower"":""-Infinity"""));
      Reject (Replace (Text, """Open"":1.000000178813934326171874", """Open"":0"));
      Reject (Replace (Text, """Open"":1.000000178813934326171874", """Open"":2"));
      Reject (Replace (Text, "[0.1,-0.0,1e-45]", "[0.1,-0.0,null]"), "3");
      Reject (Replace (Text, "[0.1,-0.0,1e-45]", "[0,1,2,3]"));
      Reject (Replace (Text, """Required"":1.000000059604644775390626,", ""));
      Reject (Replace (Text, """Count"":42", """Unknown"":0,""Count"":42"));
      Reject (Text & "{}");
      declare
         Empty : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload
           (Replace (Replace (Text, """Optional"":-0,", ""), "[0.1,-0.0,1e-45]", "[]"));
      begin
         Check (not Empty.Optional.Is_Present and Model.Length (Empty.Bounded) = 0, "empty/absent");
         Check (Ada.Strings.Fixed.Index (Service_Codec.Encode_FloatPayload (Empty), """Optional""") = 0,
                "absent optional was encoded");
      end;
      declare
         Specials : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload
           (Replace (Replace (Text, "1.000000059604644775390626", """NaN"""),
                     """Named"":-1.000000059604644775390626", """Named"":""-Infinity"""));
         Stable : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload
           (Service_Codec.Encode_FloatPayload (Specials));
      begin
         Check (Specials.Required /= Specials.Required and then Stable.Required /= Stable.Required, "NaN stability");
         Check (F32 (Specials.Named) < -F32'Last and then F32 (Stable.Named) < -F32'Last, "negative infinity stability");
      end;
      declare
         Negative_Zero : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload
           (Replace (Text, "1.000000059604644775390626", "-0"));
         Stable : constant Model.FloatPayload := Service_Codec.Decode_FloatPayload
           (Service_Codec.Encode_FloatPayload (Negative_Zero));
      begin
         Bits (Negative_Zero.Required, 16#8000_0000#);
         Bits (Stable.Required, 16#8000_0000#);
      end;
   end;
   Ada.Text_IO.Close (Input);
   Ada.Text_IO.Put_Line ("TASK095_GENERATED_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
exception when E : others =>
   Ada.Text_IO.Put_Line ("INCOMPLETE " & Ada.Exceptions.Exception_Information (E));
   Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Float32_Consumer;