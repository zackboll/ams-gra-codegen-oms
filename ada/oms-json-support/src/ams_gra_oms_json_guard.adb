with GNATCOLL.JSON; use GNATCOLL.JSON;
with GNATCOLL.Buffer;

package body AMS_GRA_OMS_JSON_Guard is
   --  Offsets, rather than Text'Last + 1, also handle non-one-based strings
   --  without overflowing at Integer'Last. Every lookahead is length-checked.
   procedure Validate (Text : String; Document : Boolean) is
      Position : Natural := 0;
      In_String : Boolean := False;
      Nesting : Natural := 0;

      procedure Fail (Reason : String) is
      begin
         raise Validation_Error with
           "byte offset" & Position'Image & ": " & Reason;
      end Fail;

      function Byte (Offset : Natural := 0) return Natural is
      begin
         if Offset >= Text'Length - Position then
            Fail ("truncated encoding");
         end if;
         return Character'Pos (Text (Text'First + Position + Offset));
      end Byte;

      function Unit_16 (Offset : Natural) return Natural is
         Result : Natural := 0;
         Digit : Natural;
      begin
         for J in Offset .. Offset + 3 loop
            Digit := Byte (J);
            case Digit is
               when 48 .. 57 => Digit := Digit - 48;
               when 65 .. 70 => Digit := Digit - 65 + 10;
               when 97 .. 102 => Digit := Digit - 97 + 10;
               when others => Fail ("nonhex Unicode escape");
            end case;
            Result := Result * 16 + Digit;
         end loop;
         return Result;
      end Unit_16;

      procedure UTF8_Sequence is
         Lead : constant Natural := Byte;
         Width : Natural;
         Second_Min : Natural := 16#80#;
         Second_Max : Natural := 16#BF#;
      begin
         case Lead is
            when 16#C2# .. 16#DF# => Width := 2;
            when 16#E0# .. 16#EF# =>
               Width := 3;
               if Lead = 16#E0# then
                  Second_Min := 16#A0#;
               elsif Lead = 16#ED# then
                  Second_Max := 16#9F#;
               end if;
            when 16#F0# .. 16#F4# =>
               Width := 4;
               if Lead = 16#F0# then
                  Second_Min := 16#90#;
               elsif Lead = 16#F4# then
                  Second_Max := 16#8F#;
               end if;
            when others => Fail ("invalid UTF-8 lead");
         end case;
         if Byte (1) not in Second_Min .. Second_Max then
            Fail ("invalid UTF-8 second byte");
         end if;
         for J in 2 .. Width - 1 loop
            if Byte (J) not in 16#80# .. 16#BF# then
               Fail ("invalid UTF-8 continuation");
            end if;
         end loop;
         Position := Position + Width;
      end UTF8_Sequence;
   begin
      while Position < Text'Length loop
         if Byte >= 16#80# then
            UTF8_Sequence;
         elsif not Document then
            --  Decoded strings may contain every ASCII byte, including NUL.
            Position := Position + 1;
         elsif not In_String then
            if Byte = 34 then
               In_String := True;
            elsif Byte = 123 or else Byte = 91 then
               if Nesting = Max_Depth then
                  Fail ("JSON pre-parse nesting limit");
               end if;
               Nesting := Nesting + 1;
            elsif Byte = 125 or else Byte = 93 then
               --  Matching delimiters and JSON grammar remain the parser's job.
               if Nesting > 0 then
                  Nesting := Nesting - 1;
               end if;
            end if;
            Position := Position + 1;
         elsif Byte = 34 then
            In_String := False;
            Position := Position + 1;
         elsif Byte < 32 then
            Fail ("unescaped string control");
         elsif Byte = 92 then
            case Byte (1) is
               when 34 | 92 | 47 | 98 | 102 | 110 | 114 | 116 =>
                  Position := Position + 2;
               when 117 =>
                  declare
                     Lead : constant Natural := Unit_16 (2);
                  begin
                     if Lead in 16#DC00# .. 16#DFFF# then
                        Fail ("lone low surrogate");
                     elsif Lead in 16#D800# .. 16#DBFF# then
                        if Byte (6) /= 92 or else Byte (7) /= 117 then
                           Fail ("missing low surrogate escape");
                        end if;
                        if Unit_16 (8) not in 16#DC00# .. 16#DFFF# then
                           Fail ("invalid low surrogate");
                        end if;
                        Position := Position + 12;
                     else
                        Position := Position + 6;
                     end if;
                  end;
               when others => Fail ("invalid escape");
            end case;
         else
            Position := Position + 1;
         end if;
      end loop;
      if Document and then In_String then
         Fail ("unterminated string");
      end if;
   end Validate;

   procedure Validate_String (Text : String) is
   begin
      Validate (Text, Document => False);
   end Validate_String;

   procedure Validate_Encoded_Document (Text : String) is
      --  This is event bookkeeping, not a second grammar parser. Parse_Next
      --  checks punctuation, token grammar, matching delimiters and DOC_END.
      --  STRING_VALUE also denotes object keys, which must not count as nodes.
      type Frame is record
         Is_Object : Boolean;
         Expect_Key : Boolean;
      end record;
      type Frames is array (Positive range 1 .. Max_Depth) of Frame;
      Stack : Frames := (others => (Is_Object => False, Expect_Key => False));
      Level : Natural range 0 .. Max_Depth := 0;
      Nodes : Natural range 0 .. Max_Nodes := 0;
   begin
      if Text'Length > Max_Document_Bytes then
         raise Validation_Error with "JSON output document size limit";
      end if;
      --  Keep the established UTF-8, escape, surrogate, control and lexical
      --  nesting checks. Its offset arithmetic accepts arbitrary String bounds.
      Validate (Text, Document => True);
      declare
         --  Open_String normalizes bounds and explicitly copies at most 1 MiB.
         --  Both limited-controlled objects own their storage for this scope;
         --  no controlled value-result adjustment is involved.
         Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (Text);
         Parser : JSON_Parser;
         Event : JSON_Parser_Event;
      begin
         loop
            Event := Parse_Next (Parser, Data);
            case Event.Kind is
               when DOC_END =>
                  if Nodes = 0 or else Level /= 0 then
                     raise Validation_Error with "incomplete JSON output";
                  end if;
                  exit;
               when ARRAY_END | OBJECT_END =>
                  if Level = 0 then
                     raise Validation_Error with "unmatched JSON output end";
                  end if;
                  Level := Level - 1;
               when STRING_VALUE .. OBJECT_START =>
                  if Level > 0 and then Stack (Level).Is_Object
                    and then Stack (Level).Expect_Key
                  then
                     --  The parser guarantees that an object key is a string.
                     if Event.Kind /= STRING_VALUE then
                        raise Validation_Error with "expected JSON output key";
                     end if;
                     Stack (Level).Expect_Key := False;
                  else
                     if Nodes = Max_Nodes then
                        raise Validation_Error with "JSON output node limit";
                     end if;
                     Nodes := Nodes + 1;
                     if Level > 0 and then Stack (Level).Is_Object then
                        Stack (Level).Expect_Key := True;
                     end if;
                     if Event.Kind in ARRAY_START | OBJECT_START then
                        if Level = Max_Depth then
                           raise Validation_Error with "JSON output nesting limit";
                        end if;
                        Level := Level + 1;
                        Stack (Level) :=
                          (Is_Object => Event.Kind = OBJECT_START,
                           Expect_Key => Event.Kind = OBJECT_START);
                     end if;
                  end if;
               when VALUE_SEP | NAME_SEP =>
                  --  These events are internal to the pinned SAX parser and
                  --  must never be returned through its public interface.
                  raise Program_Error with "unexpected SAX separator event";
            end case;
         end loop;
      end;
   exception
      when Invalid_JSON_Stream =>
         raise Validation_Error with "invalid encoded JSON document";
   end Validate_Encoded_Document;

   procedure Validate_Tree (Value : JSON_Value) is
      Nodes : Natural := 0;
      procedure Visit (Item : JSON_Value; Depth : Natural) is
         procedure Member (Name : String; Child : JSON_Value) is
         begin
            Validate (Name, Document => False);
            Visit (Child, Depth + 1);
         end Member;
      begin
         if Depth > Max_Depth or else Nodes = Max_Nodes then
            raise Validation_Error with "JSON tree traversal limit";
         end if;
         Nodes := Nodes + 1;
         case Item.Kind is
            when JSON_String_Type =>
               Validate (String'(Get (Item)), Document => False);
            when JSON_Object_Type =>
               Map_JSON_Object (Item, Member'Access);
            when JSON_Array_Type =>
               declare
                  Items : constant JSON_Array := Get (Item);
               begin
                  for J in 1 .. Length (Items) loop
                     Visit (Get (Items, J), Depth + 1);
                  end loop;
               end;
            when others => null;
         end case;
      end Visit;
   begin
      Visit (Value, 0);
   end Validate_Tree;

   function Read (Text : String) return JSON_Value is
   begin
      if Text'Length > Max_Document_Bytes then
         raise Validation_Error with "JSON input document size limit";
      end if;
      Validate (Text, Document => True);
      declare
         Result : constant Read_Result := GNATCOLL.JSON.Read (Text);
      begin
         if not Result.Success then
            raise Validation_Error with Format_Parsing_Error (Result.Error);
         end if;
         Validate_Tree (Result.Value);
         return Result.Value;
      end;
   end Read;

   function Write (Value : JSON_Value) return String is
   begin
      Validate_Tree (Value);
      declare
         Text : constant String := GNATCOLL.JSON.Write (Value);
      begin
         Validate (Text, Document => True);
         return Text;
      end;
   end Write;
end AMS_GRA_OMS_JSON_Guard;
