with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Interfaces;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;

procedure Repeated_Codec_Consumer is
   use type Interfaces.Unsigned_64;
   Passed : Natural := 0;
   function Read_File (Path : String) return String is
      File : File_Type;
      Text : Unbounded_String;
   begin
      Open (File, In_File, Path);
      while not End_Of_File (File) loop Append (Text, Get_Line (File)); end loop;
      Close (File);
      return To_String (Text);
   end Read_File;
   Golden : constant String := Read_File (Ada.Command_Line.Argument (1));
   Zero : constant String := Read_File (Ada.Command_Line.Argument (2));
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "repeated codec check failed after" & Passed'Image; end if;
      Passed := Passed + 1;
   end Check;
   procedure Reject (Text : String) is
   begin
      declare
         Item : constant RepeatedPayload := Decode_RepeatedPayload (Text);
      begin
         raise Program_Error with "invalid payload accepted" & Length (Item.Span)'Image;
      end;
   exception
      when Codec_Error => Passed := Passed + 1;
   end Reject;
   procedure Wrong (Key : String; Value : JSON_Value) is
      Object_Value : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Golden);
   begin
      Object_Value.Set_Field (Key, Value);
      Reject (AMS_GRA_OMS_JSON_Guard.Write (Object_Value));
   end Wrong;
   procedure Count (Key : String; Number : Natural; Child : JSON_Value;
                    Valid : Boolean) is
      Object_Value : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Golden);
      Items : JSON_Array;
   begin
      for Index in 1 .. Number loop Append (Items, Child); end loop;
      Object_Value.Set_Field (Key, Create (Items));
      if Valid then
         declare
            Text : constant String := AMS_GRA_OMS_JSON_Guard.Write (Object_Value);
            Item : constant RepeatedPayload := Decode_RepeatedPayload (Text);
            Encoded : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Encode_RepeatedPayload (Item));
         begin
            if Number = 0 then Object_Value.Unset_Field (Key); end if;
            Check (Encoded = AMS_GRA_OMS_JSON_Guard.Read
              (AMS_GRA_OMS_JSON_Guard.Write (Object_Value)));
         end;
      else
         Reject (AMS_GRA_OMS_JSON_Guard.Write (Object_Value));
      end if;
   end Count;
   Item : constant RepeatedPayload := Decode_RepeatedPayload (Golden);
begin
   Check (Length (Item.Field_begin) = 3
     and then Value (Element (Item.Field_begin, 1)) = "one"
     and then Value (Element (Item.Field_begin, 3)) = "three");
   Check (Length (Item.Span) = 3 and then Value (Element (Item.Span, 2)) = "span second");
   Check (Length (Item.Pair) = 2 and then Value (Element (Item.Pair, 2).Label) = "pair second");
   Check (Length (Item.Words) = 5 and then To_String (Element (Item.Words, 1)) = ""
     and then To_String (Element (Item.Words, 5)) = "four");
   Check (Value (Item.RequiredLabels.Required (1)) = "required first"
     and then Value (Item.RequiredLabels.Required (2)) = "required second"
     and then Length (Item.RequiredLabels.Additional) = 2
     and then Value (Element (Item.RequiredLabels.Additional, 2)) = "additional fourth");
   Check (Length (Item.Signals) = 2 and then Element (Item.Signals, 1) = Value_5G);
   Check (Length (Item.Signed) = 3 and then Element (Item.Signed, 1) = -128
     and then Element (Item.Signed, 3) = 127);
   Check (Length (Item.Unsigned) = 2 and then Element (Item.Unsigned, 2) = 255);
   Check (Length (Item.Flags) = 3 and then not Element (Item.Flags, 2));
   Check (Length (Item.Details) = 2 and then Value (Element (Item.Details, 2).Label) = "nested second");
   Check (Length (Item.Inherited) = 2 and then Value (Element (Item.Inherited, 1)) = "base first");
   Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_RepeatedPayload (Item)) = AMS_GRA_OMS_JSON_Guard.Read (Golden));
   Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_RepeatedPayload (Decode_RepeatedPayload (Encode_RepeatedPayload (Item)))) = AMS_GRA_OMS_JSON_Guard.Read (Golden));
   declare
      Empty : constant RepeatedPayload := Decode_RepeatedPayload (Zero);
   begin
      Check (Length (Empty.Field_begin) = 0 and then Length (Empty.Words) = 0
        and then Length (Empty.Details) = 0 and then Length (Empty.RequiredLabels.Additional) = 0);
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_RepeatedPayload (Empty)) = AMS_GRA_OMS_JSON_Guard.Read (Zero));
   end;
   for Number in 0 .. 4 loop
      Count ("begin", Number, Create ("valid"), Number <= 3);
      Count ("Span", Number, Create ("valid"), Number >= 2 and Number <= 3);
      Count ("Pair", Number, AMS_GRA_OMS_JSON_Guard.Read ("{""Label"":""valid""}"), Number = 2);
      Count ("RequiredLabels", Number, Create ("valid"), Number >= 2);
      Count ("Inherited", Number, Create ("valid"), Number <= 3);
      Count ("Signals", Number, Create ("NORMAL"), Number <= 3);
      Count ("Signed", Number, Create (Long_Long_Integer'(0)), Number <= 3);
      Count ("Unsigned", Number, Create (Long_Long_Integer'(0)), Number <= 3);
   end loop;
   Count ("RequiredLabels", 12, Create ("valid"), True);
   Count ("Words", 0, Create ("valid"), True);
   Count ("Words", 1, Create ("valid"), True);
   Count ("Words", 12, Create ("valid"), True);
   Count ("Details", 12, AMS_GRA_OMS_JSON_Guard.Read ("{""Label"":""valid""}"), True);
   declare
      Keys : constant JSON_Array := Get (AMS_GRA_OMS_JSON_Guard.Read
        ("[""begin"",""Span"",""Pair"",""Words"",""RequiredLabels"",""Signals"",""Signed"",""Unsigned"",""Flags"",""Details"",""Inherited""]"));
   begin
      for Index in 1 .. Length (Keys) loop
         declare
            Key : constant String := Get (Get (Keys, Index));
            Object_Value : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Golden);
         begin
            Wrong (Key, Create);
            Wrong (Key, Create ("scalar"));
            Object_Value.Unset_Field (Key);
            if Key = "Span" or else Key = "Pair" or else Key = "RequiredLabels" then
               Reject (AMS_GRA_OMS_JSON_Guard.Write (Object_Value));
            else
               declare
                  Missing : constant RepeatedPayload := Decode_RepeatedPayload (AMS_GRA_OMS_JSON_Guard.Write (Object_Value));
               begin
                  Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_RepeatedPayload (Missing)) =
                    AMS_GRA_OMS_JSON_Guard.Read (AMS_GRA_OMS_JSON_Guard.Write (Object_Value)));
               end;
            end if;
         end;
      end loop;
   end;
   Wrong ("begin", AMS_GRA_OMS_JSON_Guard.Read ("[""valid"",null]"));
   Wrong ("Span", AMS_GRA_OMS_JSON_Guard.Read ("[""valid"",""""]"));
   Wrong ("RequiredLabels", AMS_GRA_OMS_JSON_Guard.Read ("[""first"",""second"",""valid"",false]"));
   Wrong ("Words", AMS_GRA_OMS_JSON_Guard.Read ("[""valid"",true]"));
   Wrong ("Flags", AMS_GRA_OMS_JSON_Guard.Read ("[true,1]"));
   Wrong ("Signals", AMS_GRA_OMS_JSON_Guard.Read ("[""NORMAL"",""normal""]"));
   Wrong ("Signed", AMS_GRA_OMS_JSON_Guard.Read ("[0,128]"));
   Wrong ("Signed", AMS_GRA_OMS_JSON_Guard.Read ("[0,-129]"));
   Wrong ("Signed", AMS_GRA_OMS_JSON_Guard.Read ("[0,1.5]"));
   Wrong ("Unsigned", AMS_GRA_OMS_JSON_Guard.Read ("[0,-1]"));
   Wrong ("Unsigned", AMS_GRA_OMS_JSON_Guard.Read ("[0,256]"));
   Wrong ("Details", AMS_GRA_OMS_JSON_Guard.Read ("[{""Label"":""valid""},{""Label"":""valid"",""Unknown"":true}]"));
   Wrong ("Details", AMS_GRA_OMS_JSON_Guard.Read ("[{""Label"":""valid""},{}]"));
   Wrong ("Details", AMS_GRA_OMS_JSON_Guard.Read ("[{""Label"":""valid""},null]"));
   Wrong ("Unknown", Create (True));
   Wrong ("Signed", AMS_GRA_OMS_JSON_Guard.Read ("[0,""1""]"));
   Wrong ("Unsigned", AMS_GRA_OMS_JSON_Guard.Read ("[0,true]"));
   Wrong ("Signals", AMS_GRA_OMS_JSON_Guard.Read ("[""NORMAL"",true]"));
   Wrong ("Inherited", AMS_GRA_OMS_JSON_Guard.Read ("[""valid"",""""]"));
   Wrong ("Words", AMS_GRA_OMS_JSON_Guard.Read ("[""valid"",null]"));
   Wrong ("Flags", AMS_GRA_OMS_JSON_Guard.Read ("[true,null]"));
   Wrong ("Signals", AMS_GRA_OMS_JSON_Guard.Read ("[""NORMAL"",null]"));
   Wrong ("Signed", AMS_GRA_OMS_JSON_Guard.Read ("[0,null]"));
   Wrong ("Unsigned", AMS_GRA_OMS_JSON_Guard.Read ("[0,null]"));
   Wrong ("RequiredLabels", AMS_GRA_OMS_JSON_Guard.Read ("[""first"",""second"",null]"));
   Wrong ("Pair", AMS_GRA_OMS_JSON_Guard.Read ("[{""Label"":""first""},null]"));
   Reject ("{");
   Reject ("{""Words"":[""\uD800""]}");
   Put_Line ("TASK089_CODEC_COMPLETE cases=" & Passed'Image);
end Repeated_Codec_Consumer;