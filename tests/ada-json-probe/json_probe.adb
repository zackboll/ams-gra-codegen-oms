with Ada.Text_IO; use Ada.Text_IO;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with GNATCOLL.Buffer;
with JSON_Guard;

procedure Json_Probe is
   procedure Dump (Label_Text, Text : String) is
      Hex : constant String := "0123456789ABCDEF";
   begin
      Put (Label_Text & " HEX");
      for C of Text loop
         Put (" " & Hex (Character'Pos (C) / 16 + 1) &
              Hex (Character'Pos (C) mod 16 + 1));
      end loop;
      New_Line;
   end Dump;
   procedure Check (Condition : Boolean; Label_Text : String) is
   begin
      if not Condition then
         raise Program_Error with Label_Text;
      end if;
      Put_Line ("PASS " & Label_Text);
   end Check;
   procedure Observe (Text : String) is
      Result : constant Read_Result := Read (Text);
   begin
      if Result.Success then
         Put_Line ("ACCEPT " & Text & " => " & Write (Result.Value));
      else
         Put_Line ("REJECT " & Text & " => " & Format_Parsing_Error (Result.Error));
      end if;
   exception
      when others => Put_Line ("EXCEPTION " & Text);
   end Observe;
   Object_Value : constant JSON_Value := Create_Object;
   Text_Value : constant String := "quote""slash\" & Character'Val (195) & Character'Val (169);
   procedure Guarded_Corpus is
      Passed : Natural := 0;
      function Bytes (Hex : String) return String is
         Result : String (1 .. Hex'Length / 2);
      begin
         for J in Result'Range loop
            Result (J) := Character'Val
              (Natural'Value ("16#" & Hex (2 * J - 1 .. 2 * J) & "#"));
         end loop;
         return Result;
      end Bytes;

      function Quoted (Text : String) return String is ("""" & Text & """");

      procedure Positive (Label_Text, Lexical, Expected : String) is
         Value : constant JSON_Value := JSON_Guard.Read (Quoted (Lexical));
         Object_Value : constant JSON_Value := JSON_Guard.Read
           ("{" & Quoted (Lexical) & ":" & Quoted (Lexical) & "}");
         procedure Member (Name : String; Item : JSON_Value) is
         begin
            Check (Name = Expected and then String'(Get (Item)) = Expected,
                   Label_Text & " key bytes");
         end Member;
      begin
         Check (String'(Get (Value)) = Expected, Label_Text & " decoded bytes");
         Check (String'(Get (JSON_Guard.Read (JSON_Guard.Write (Value)))) = Expected,
                Label_Text & " guarded round trip");
         Check (String'(Get (JSON_Guard.Read
           (JSON_Guard.Write (Create (Expected))))) = Expected,
           Label_Text & " directly constructed writer bytes");
         Map_JSON_Object (Object_Value, Member'Access);
         Map_JSON_Object (JSON_Guard.Read (JSON_Guard.Write (Object_Value)),
                          Member'Access);
         Dump (Label_Text & " expected", Expected);
         Passed := Passed + 1;
      end Positive;

      procedure Bad_Document (Label_Text, Text : String) is
      begin
         declare
            Value : constant JSON_Value := JSON_Guard.Read (Text);
         begin
            raise Program_Error with "accepted invalid document: " & Label_Text &
              Value.Kind'Image;
         end;
      exception
         when JSON_Guard.Validation_Error =>
            Put_Line ("PASS rejected document " & Label_Text);
            Passed := Passed + 1;
      end Bad_Document;

      procedure Negative (Label_Text, Lexical : String) is
      begin
         Bad_Document (Label_Text & " value", Quoted (Lexical));
         Bad_Document (Label_Text & " key", "{" & Quoted (Lexical) & ":1}");
      end Negative;

      procedure Bad_Tree (Label_Text : String; Value : JSON_Value) is
      begin
         declare
            Output : constant String := JSON_Guard.Write (Value);
         begin
            raise Program_Error with "writer accepted " & Label_Text & Output;
         end;
      exception
         when JSON_Guard.Validation_Error =>
            Put_Line ("PASS rejected writer " & Label_Text);
            Passed := Passed + 1;
      end Bad_Tree;

      procedure Invalid_Bytes (Label_Text, Hex : String) is
         Raw : constant String := Bytes (Hex);
         Object_Value : constant JSON_Value := Create_Object;
         Items : JSON_Array;
      begin
         Negative (Label_Text, Raw);
         Bad_Tree (Label_Text & " scalar", Create (Raw));
         Object_Value.Set_Field (Raw, Create (True));
         Bad_Tree (Label_Text & " key", Object_Value);
         Append (Items, Create (Raw));
         Object_Value.Unset_Field (Raw);
         Object_Value.Set_Field ("nested", Create (Items));
         Bad_Tree (Label_Text & " nested array", Object_Value);
      end Invalid_Bytes;

      procedure Integer_Case (Text : String; Expected : Long_Long_Integer) is
         Value : constant JSON_Value := JSON_Guard.Read (Text);
         Again : constant JSON_Value := JSON_Guard.Read (JSON_Guard.Write (Value));
      begin
         Check (Value.Kind = JSON_Int_Type and then
                Long_Long_Integer'(Get (Value)) = Expected and then
                Long_Long_Integer'(Get (Again)) = Expected, "exact integer " & Text);
         Passed := Passed + 1;
      end Integer_Case;

      --  SAX investigation only: provider events, not a duplicate checker.
      procedure SAX_Keys is
         Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String
           ("{""a"":1,""\u0061"":2}");
         Parser : JSON_Parser;
         Event : JSON_Parser_Event;
         Keys : Natural := 0;
      begin
         loop
            Event := Parse_Next (Parser, Data);
            exit when Event.Kind = DOC_END;
            if Event.Kind = STRING_VALUE then
               Check (Decode_As_String (Event, Data) = "a", "SAX decoded key a");
               Keys := Keys + 1;
            end if;
         end loop;
         Check (Keys = 2, "SAX exposes both equivalent keys");
         Check (Integer'(Get (JSON_Guard.Read
           ("{""a"":1,""\u0061"":2}"), "a")) = 2,
           "guard explicitly retains last-value-wins");
         Passed := Passed + 1;
      end SAX_Keys;
      procedure Bounds_And_Limits is
         Text : constant String (7 .. 9) := """x""";
         Deep : JSON_Value := Create (True);
      begin
         Check (String'(Get (JSON_Guard.Read (Text))) = "x",
                "non-one-based input");
         Bad_Document ("empty document", "");
         --  Every prefix ending inside an escape must fail controllably.
         declare
            Escape_Text : constant String := """\uD800\uDC00""";
         begin
            for J in 1 .. Escape_Text'Length - 1 loop
               Bad_Document ("escape prefix" & J'Image, Escape_Text (1 .. J));
            end loop;
         end;
         for J in 1 .. 130 loop
            declare
               Items : JSON_Array;
            begin
               Append (Items, Deep);
               Deep := Create (Items);
            end;
         end loop;
         Bad_Tree ("depth limit", Deep);
         declare
            Items : JSON_Array;
         begin
            for J in 1 .. 100_000 loop
               Append (Items, Create (False));
            end loop;
            Bad_Tree ("node limit", Create (Items));
         end;
      end Bounds_And_Limits;
   begin
      Positive ("ASCII", "plain ASCII", "plain ASCII");
      Positive ("empty", "", "");
      Positive ("quote backslash", "a\""b\\c", "a""b\c");
      Positive ("all short escapes", "\""\\\/\b\f\n\r\t",
                """\/" & ASCII.BS & ASCII.FF & ASCII.LF & ASCII.CR & ASCII.HT);
      Positive ("raw UTF8", Bytes ("C3A9E282ACF09F9880"), Bytes ("C3A9E282ACF09F9880"));
      Positive ("escaped BMP", "\u00e9\u20ac", Bytes ("C3A9E282AC"));
      Positive ("supplementary", "\uD83D\uDE00", Bytes ("F09F9880"));
      Positive ("NUL", "\u0000", Bytes ("00"));
      Positive ("D7FF", "\uD7FF", Bytes ("ED9FBF"));
      Positive ("E000", "\uE000", Bytes ("EE8080"));
      Positive ("10000", "\uD800\uDC00", Bytes ("F0908080"));
      Positive ("10FFFF", "\uDBFF\uDFFF", Bytes ("F48FBFBF"));
      Positive ("raw boundaries", Bytes ("ED9FBFEE8080F0908080F48FBFBF"),
                Bytes ("ED9FBFEE8080F0908080F48FBFBF"));
      Positive ("combining", "e\u0301", Bytes ("65CC81"));
      Positive ("2028 2029", "\u2028\u2029", Bytes ("E280A8E280A9"));
      Positive ("literal Unicode text", "\\uD800", "\uD800");
      Negative ("high", "\uD800");
      Negative ("low", "\uDC00");
      Negative ("high nonlow", "\uD800\u0041");
      Negative ("high high", "\uD800\uD800");
      Negative ("escaped backslash", "\uD800\\uDC00");
      Negative ("truncated Unicode", "\u123");
      Negative ("nonhex", "\uZZZZ");
      Negative ("unknown escape", "\q");
      Negative ("uppercase U", "\U0041");
      Bad_Document ("truncated escape", """\");
      Bad_Document ("truncated key escape", "{""\");
      Invalid_Bytes ("continuation", "80");
      Invalid_Bytes ("invalid lead", "FF");
      Invalid_Bytes ("bad second", "C241");
      Invalid_Bytes ("bad third", "E28241");
      Invalid_Bytes ("bad fourth", "F0908041");
      Invalid_Bytes ("overlong two", "C080");
      Invalid_Bytes ("overlong three", "E08080");
      Invalid_Bytes ("overlong four", "F0808080");
      Invalid_Bytes ("raw surrogate", "EDA080");
      Invalid_Bytes ("above maximum", "F4908080");
      Invalid_Bytes ("five byte lead", "F888808080");
      Invalid_Bytes ("truncated two", "C2");
      Invalid_Bytes ("truncated three", "E282");
      Invalid_Bytes ("truncated four", "F09080");
      for C in 0 .. 31 loop
         Negative ("control" & C'Image, (1 => Character'Val (C)));
      end loop;
      Bad_Document ("invalid outside string", Bytes ("FF") & "{}");
      Bad_Document ("grammar", "{""a"":}");
      Bad_Document ("root truncated", "{");
      Bad_Document ("trailing document", "{} {}");
      Integer_Case ("-9223372036854775808", Long_Long_Integer'First);
      Integer_Case ("9223372036854775807", Long_Long_Integer'Last);
      Integer_Case ("9007199254740993", 9_007_199_254_740_993);
      Integer_Case ("0", 0);
      Integer_Case ("-0", 0);
      Bad_Document ("positive overflow", "9223372036854775808");
      Bad_Document ("negative overflow", "-9223372036854775809");
      Bad_Document ("uint64 maximum", "18446744073709551615");
      SAX_Keys;
      Bounds_And_Limits;
      Put_Line ("GUARDED CORPUS PASSED cases=" & Passed'Image);
   end Guarded_Corpus;
begin
   declare
      Bad : constant String := """\uD800\u0041""";
      Parsed : constant Read_Result := Read (Bad);
      Invalid : constant String := (1 => Character'Val (16#FF#));
   begin
      Check (Parsed.Success, "upstream surrogate defect reproduced");
      Dump ("surrogate input", Bad);
      Dump ("surrogate decoded", String'(Get (Parsed.Value)));
      Dump ("surrogate reencoded", String'(Write (Parsed.Value)));
      Dump ("writer invalid input", Invalid);
      Dump ("writer substituted output", String'(Write (Create (Invalid))));
      Check (String'(Write (Create (Invalid))) = """\u0000""",
             "upstream writer substitution reproduced");
   end;
   Object_Value.Set_Field ("text", Text_Value);
   Object_Value.Set_Field ("bool", True);
   Object_Value.Set_Field ("int", Create (Long_Long_Integer'Last));
   Object_Value.Set_Field ("empty", "");
   Object_Value.Set_Field ("null", Create);
   declare
      Encoded : constant String := Write (Object_Value);
      Parsed : constant Read_Result := Read (Encoded);
   begin
      Put_Line (Encoded);
      Check (Parsed.Success, "object round trip");
      Check (String'(Get (Parsed.Value, "text")) = Text_Value, "quotes backslash UTF8");
      Check (Boolean'(Get (Parsed.Value, "bool")), "boolean");
      Check (Long_Long_Integer'(Get (Get (Parsed.Value, "int"))) = Long_Long_Integer'Last, "int64 exact");
      Check (not Has_Field (Parsed.Value, "missing"), "missing");
      Check (Get (Parsed.Value, "null").Kind = JSON_Null_Type, "null");
      Check (String'(Get (Parsed.Value, "empty")) = "", "authored empty");
   end;
   declare
      Parsed : constant Read_Result := Read ("""\u00e9\uD83D\uDE00""");
   begin
      Check (Parsed.Success, "Unicode escapes parse");
      Check (String'(Get (Parsed.Value)) =
        Character'Val (195) & Character'Val (169) & Character'Val (240) &
        Character'Val (159) & Character'Val (152) & Character'Val (128), "Unicode BMP and surrogate pair exact");
   end;
   Observe ("{""k"":1,""k"":2}");
   Observe ("[true,false,null,"""",1,1.25]");
   Observe ("9223372036854775807");
   Observe ("-9223372036854775808");
   Observe ("9223372036854775808");
   Observe ("18446744073709551615");
   Observe ("9007199254740993");
   Observe ("1.234567890123456789012345");
   Observe ("{");
   Observe ("{""k"":}");
   Observe ("{} trailing");
   Observe ("""\q""");
   Observe ("""\uZZZZ""");
   Observe ("""\uD800""");
   Observe ("""\uDC00""");
   Observe ("""\uD800\u0041""");
   Observe ("""truncated");
   Guarded_Corpus;
end Json_Probe;
