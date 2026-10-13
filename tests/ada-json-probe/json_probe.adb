with Ada.Text_IO; use Ada.Text_IO;
with GNATCOLL.JSON; use GNATCOLL.JSON;

procedure Json_Probe is
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
begin
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
end Json_Probe;
