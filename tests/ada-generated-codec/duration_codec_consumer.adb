with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Programs.Oam; use Programs.Oam;
with Service_API;
with Service_Codec; use Service_Codec;
procedure Duration_Codec_Consumer is
   Passed : Natural := 0;
   Golden : Unbounded_String;
   File : File_Type;
   type String_Access is access constant String;
   type String_Array is array (Positive range <>) of String_Access;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "Duration check failed"; end if;
      Passed := Passed + 1;
   end Check;
   procedure Reject (Text : String) is
   begin
      declare
         Item : constant DurationPayload := Decode_DurationPayload (Text);
      begin
         raise Program_Error with "accepted invalid Duration: " & Value (Item.Named);
      end;
   exception
      when Codec_Error => Check (True);
   end Reject;
   procedure Corpus (Input, Expected : String; Valid : Boolean) is
      Document : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden));
      procedure Test (Key : String) is
      begin
         Document.Set_Field (Key, GNATCOLL.JSON.Create (Input));
         if Valid then
            declare
               Item : constant DurationPayload := Decode_DurationPayload
                 (AMS_GRA_OMS_JSON_Guard.Write (Document));
               Encoded : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read
                 (Encode_DurationPayload (Item));
            begin
               Check (String'(Get (Encoded, Key)) = Expected);
            end;
         else
            Reject (AMS_GRA_OMS_JSON_Guard.Write (Document));
         end if;
         Document.Set_Field (Key, JSON_Value'(AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden)).Get (Key)));
      end Test;
   begin
      Test ("Named"); Test ("OtherNamed"); Test ("DirectDuration");
      Test ("OptionalNamed"); Test ("OptionalDirect"); Test ("InheritedDuration");
   end Corpus;
   procedure Frozen_Corpus is separate;
   procedure Reordered_Golden is separate;
begin
   Open (File, In_File, Ada.Command_Line.Argument (1));
   while not End_Of_File (File) loop Append (Golden, Get_Line (File)); end loop;
   Close (File);
   declare
      Item : constant DurationPayload := Decode_DurationPayload (To_String (Golden));
      Absent : constant DurationPayload :=
        (InheritedDuration => Item.InheritedDuration, Named => Item.Named,
         OtherNamed => Item.OtherNamed, DirectDuration => Item.DirectDuration,
         OptionalNamed => (Is_Present => False), OptionalDirect => (Is_Present => False),
         Details => Item.Details, Label => Item.Label);
      Expected : Unbounded_String;
   begin
      Check (Value (Item.Named) = "P12M" and then Value (Item.OtherNamed) = "P1Y"
        and then Value (Item.DirectDuration) = "PT6.2500S"
        and then Value (Item.OptionalNamed.Value) = "-P0D"
        and then Value (Item.OptionalDirect.Value) = "-PT0.0S"
        and then Value (Item.Details.NestedDuration) = "P3DT4H5M6S"
        and then Value (Item.Details.NestedDirect) = "PT0S"
        and then Value (Item.InheritedDuration) = "P999999999999999999999999Y");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_DurationPayload (Item)) =
        AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden)));
      Open (File, In_File, Ada.Command_Line.Argument (2));
      while not End_Of_File (File) loop Append (Expected, Get_Line (File)); end loop;
      Close (File);
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_DurationPayload (Absent)) =
        AMS_GRA_OMS_JSON_Guard.Read (To_String (Expected)));
      declare
         Decoded : constant DurationPayload := Decode_DurationPayload (To_String (Expected));
      begin
         Check (not Decoded.OptionalNamed.Is_Present and then not Decoded.OptionalDirect.Is_Present);
      end;
   end;
   --  Generated corpus calls are supplied as a separate Ada subunit body.
   Frozen_Corpus;
   Reordered_Golden;
   Corpus ("PT1.23000S", "PT1.23000S", True);
   Corpus ("P" & String'(1 .. 8_192 => '9') & "Y",
           "P" & String'(1 .. 8_192 => '9') & "Y", True);
   for Key of String_Array'(1 => new String'("Named"),
                            2 => new String'("DirectDuration"),
                            3 => new String'("OptionalNamed"),
                            4 => new String'("OptionalDirect")) loop
      for Kind in 1 .. 6 loop
         declare
            Document : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden));
            Replacement : JSON_Value;
         begin
            case Kind is
               when 1 => Replacement := JSON_Null;
               when 2 => Replacement := GNATCOLL.JSON.Create (Long_Long_Integer'(1));
               when 3 => Replacement := GNATCOLL.JSON.Create (True);
               when 4 => Replacement := GNATCOLL.JSON.Create (Empty_Array);
               when 5 => Replacement := Create_Object;
               when others => Replacement := GNATCOLL.JSON.Create ("P");
            end case;
            Document.Set_Field (Key.all, Replacement);
            Reject (AMS_GRA_OMS_JSON_Guard.Write (Document));
         end;
      end loop;
   end loop;
   declare
      Document : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden));
   begin
      Document.Unset_Field ("Named");
      Reject (AMS_GRA_OMS_JSON_Guard.Write (Document));
   end;
   declare
      Document : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden));
   begin
      Document.Set_Field ("Unknown", JSON_Null);
      Reject (AMS_GRA_OMS_JSON_Guard.Write (Document));
   end;
   declare
      Document : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden));
      Nested : constant JSON_Value := Document.Get ("Details");
   begin
      Nested.Set_Field ("NestedDirect", GNATCOLL.JSON.Create ("PT"));
      Document.Set_Field ("Details", Nested);
      Reject (AMS_GRA_OMS_JSON_Guard.Write (Document));
   end;
   Reject ("[]"); Reject ("null"); Reject ("{");
   Reject ("{""Named"":""\uD800""}");
   Reject (String'(1 .. AMS_GRA_OMS_JSON_Guard.Max_Document_Bytes + 1 => ' '));
   Reject (String'(1 .. 129 => '[') & "0" & String'(1 .. 129 => ']'));
   Put_Line ("TASK091_CODEC_COMPLETE cases=" & Natural'Image (Passed));
end Duration_Codec_Consumer;