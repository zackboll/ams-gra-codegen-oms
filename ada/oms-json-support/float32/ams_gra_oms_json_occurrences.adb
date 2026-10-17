with Ada.Exceptions;
with GNATCOLL.Buffer;
with AMS_GRA_OMS_JSON_Guard;
package body AMS_GRA_OMS_JSON_Occurrences is
   use GNATCOLL.JSON;
   function Kind (Item : Value) return JSON_Value_Type is
     (JSON_Value_Type'Val (Integer'(Get (Get (Item.Node, "kind")))));
   function First (Item : Value) return Long_Long_Integer is
     (Get (Get (Item.Node, "first")));
   function Last (Item : Value) return Long_Long_Integer is
     (Get (Get (Item.Node, "last")));
   procedure Require_Kind (Item : Value; Expected : JSON_Value_Type) is
   begin
      if Kind (Item) /= Expected then
         raise Validation_Error with "wrong JSON kind at byte" & First (Item)'Image;
      end if;
   end;
   function Has_Field (Item : Value; Name : String) return Boolean is
   begin
      Require_Kind (Item, JSON_Object_Type);
      return Has_Field (Get (Item.Node, "data"), Name);
   end;
   function Get (Item : Value; Name : String) return Value is
   begin
      Require_Kind (Item, JSON_Object_Type);
      if not Has_Field (Item, Name) then
         raise Validation_Error with "missing member: " & Name;
      end if;
      return (Node => Get (Get (Item.Node, "data"), Name));
   end;
   function Get (Item : Value) return Boolean is
   begin
      Require_Kind (Item, JSON_Boolean_Type); return Get (Get (Item.Node, "data"));
   end;
   function Get (Item : Value) return String is
   begin
      Require_Kind (Item, JSON_String_Type); return Get (Get (Item.Node, "data"));
   end;
   function Get (Item : Value) return Long_Long_Integer is
   begin
      Require_Kind (Item, JSON_Int_Type);
      begin
         return Long_Long_Integer'Value (String'(Get (Get (Item.Node, "data"))));
      exception
         when Constraint_Error => raise Validation_Error with "integer outside signed-64 domain at byte" & First (Item)'Image;
      end;
   end;
   function Get (Item : Value) return Value_Array is
   begin
      Require_Kind (Item, JSON_Array_Type); return (Items => Get (Get (Item.Node, "data")));
   end;
   function Get (Items : Value_Array; Index : Positive) return Value is
     (Node => Get (Items.Items, Index));
   function Length (Items : Value_Array) return Natural is (Length (Items.Items));
   function Floating (Item : Value) return AMS_GRA_OMS_JSON_Float32.Float32 is
   begin
      if Kind (Item) not in JSON_Int_Type | JSON_Float_Type | JSON_String_Type then
         raise Validation_Error with "expected Float32 at byte" & First (Item)'Image;
      end if;
      declare
         Text : constant String := Get (Get (Item.Node, "data"));
      begin
      case Kind (Item) is
         when JSON_Int_Type | JSON_Float_Type => return AMS_GRA_OMS_JSON_Float32.Number (Text);
         when JSON_String_Type => return AMS_GRA_OMS_JSON_Float32.Special (Text);
         when others => raise Validation_Error with "expected Float32 at byte" & First (Item)'Image;
      end case;
      end;
   end;
   procedure Map_JSON_Object
     (Item : Value; Process : not null access procedure (Name : String; Child : Value)) is
      procedure Visit (Name : String; Child : JSON_Value) is
      begin Process (Name, (Node => Child)); end;
   begin
      Require_Kind (Item, JSON_Object_Type);
      Map_JSON_Object (Get (Item.Node, "data"), Visit'Access);
   end;
   function Read (Text : String) return Value is
      function Bounded_Copy return String is
      begin
         if Text'Length > AMS_GRA_OMS_JSON_Guard.Max_Document_Bytes then
            raise Validation_Error with "JSON input document size limit";
         end if;
         return Text;
      end;
      Shadow : String := Bounded_Copy;
      Nodes : Natural := 0;
   begin
      AMS_GRA_OMS_JSON_Float32.Require_Platform;
      -- First pass validates ORIGINAL grammar and identifies only numeric
      -- spans. No decoding or replacement happens in strings or keys.
      declare
         Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (Text);
         Parser : JSON_Parser;
         Event : JSON_Parser_Event;
         Depth, Events : Natural := 0;
      begin
         loop
            Event := Parse_Next (Parser, Data);
            exit when Event.Kind = DOC_END;
            Events := Events + 1;
            if Events > 3 * AMS_GRA_OMS_JSON_Guard.Max_Nodes then
               raise Validation_Error with "JSON event limit";
            end if;
            if Event.Kind in OBJECT_START | ARRAY_START then
               Depth := Depth + 1;
               if Depth > AMS_GRA_OMS_JSON_Guard.Max_Depth then raise Validation_Error with "JSON depth limit"; end if;
            elsif Event.Kind in OBJECT_END | ARRAY_END then Depth := Depth - 1;
            elsif Event.Kind in INTEGER_VALUE | NUMBER_VALUE then
               declare
                  Start : constant Integer := Text'First + Integer (Event.First) - 1;
                  Finish : constant Integer := Text'First + Integer (Event.Last) - 1;
               begin
                  Shadow (Start .. Finish) := [others => ' ']; Shadow (Start) := '0';
               end;
            end if;
         end loop;
         if Depth /= 0 then raise Validation_Error with "incomplete JSON document"; end if;
      end;
      -- Validation-only shadow: same length/positions, original Unicode and
      -- structural characters. Its substituted numbers NEVER become data.
      declare
         Checked : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Shadow);
         pragma Unreferenced (Checked);
      begin null; end;
      declare
         Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (Text);
         Parser : JSON_Parser;
         Event : JSON_Parser_Event;
         function Parse_Value (Depth : Natural) return Value is
            Node : constant JSON_Value := Create_Object;
            K : JSON_Value_Type;
            Payload : JSON_Value;
         begin
            if Depth > AMS_GRA_OMS_JSON_Guard.Max_Depth or else Nodes = AMS_GRA_OMS_JSON_Guard.Max_Nodes then
               raise Validation_Error with "JSON occurrence limit";
            end if;
            Nodes := Nodes + 1;
            declare
               Finish : constant Long_Long_Integer := GNATCOLL.Buffer.Current_Position (Data);
               Start : constant Long_Long_Integer :=
                 (case Event.Kind is
                    when INTEGER_VALUE | NUMBER_VALUE | STRING_VALUE => Event.First,
                    when TRUE_VALUE | NULL_VALUE => Finish - 3,
                    when FALSE_VALUE => Finish - 4,
                    when others => Finish);
            begin
               Set_Field (Node, "first", Create (Start));
               Set_Field (Node, "last", Create (Finish));
            end;
            case Event.Kind is
               when INTEGER_VALUE | NUMBER_VALUE =>
                  K := (if Event.Kind = INTEGER_VALUE then JSON_Int_Type else JSON_Float_Type);
                  Payload := Create (GNATCOLL.Buffer.Token (Data, Event.First, Event.Last));
               when STRING_VALUE => K := JSON_String_Type; Payload := Create (Decode_As_String (Event, Data));
               when TRUE_VALUE | FALSE_VALUE => K := JSON_Boolean_Type; Payload := Create (Event.Kind = TRUE_VALUE);
               when NULL_VALUE => K := JSON_Null_Type; Payload := Create;
               when OBJECT_START =>
                  K := JSON_Object_Type; Payload := Create_Object;
                  loop
                     Event := Parse_Next (Parser, Data); exit when Event.Kind = OBJECT_END;
                     if Event.Kind /= STRING_VALUE then raise Validation_Error with "expected object key"; end if;
                     declare
                        Key : constant String := Decode_As_String (Event, Data);
                     begin
                        Event := Parse_Next (Parser, Data);
                        declare
                           Child : constant Value := Parse_Value (Depth + 1);
                        begin
                           -- Provider storage replaces the complete previous
                           -- child for this DECODED key, including raw token.
                           Set_Field (Payload, Key, Child.Node);
                        end;
                     end;
                  end loop;
                  Set_Field (Node, "last", Create (GNATCOLL.Buffer.Current_Position (Data)));
               when ARRAY_START =>
                  K := JSON_Array_Type;
                  declare
                     Items : JSON_Array;
                  begin
                     loop
                        Event := Parse_Next (Parser, Data); exit when Event.Kind = ARRAY_END;
                        declare Child : constant Value := Parse_Value (Depth + 1);
                        begin Append (Items, Child.Node); end;
                     end loop;
                     Payload := Create (Items);
                  end;
                  Set_Field (Node, "last", Create (GNATCOLL.Buffer.Current_Position (Data)));
               when others => raise Validation_Error with "expected JSON value";
            end case;
            Set_Field (Node, "kind", Integer (JSON_Value_Type'Pos (K))); Set_Field (Node, "data", Payload);
            return (Node => Node);
         end;
      begin
         Event := Parse_Next (Parser, Data);
         declare Root : constant Value := Parse_Value (0);
         begin
            Event := Parse_Next (Parser, Data);
            if Event.Kind /= DOC_END then raise Validation_Error with "trailing JSON document"; end if;
            return Root;
         end;
      end;
   exception
      when E : Invalid_JSON_Stream | AMS_GRA_OMS_JSON_Guard.Validation_Error =>
         raise Validation_Error with Ada.Exceptions.Exception_Message (E);
   end Read;
end AMS_GRA_OMS_JSON_Occurrences;