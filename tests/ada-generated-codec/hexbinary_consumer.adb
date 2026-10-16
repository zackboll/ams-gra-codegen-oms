with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Interfaces; use type Interfaces.Unsigned_8;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;
procedure Hexbinary_Consumer is
   use type Binary_Vectors.Vector;
   Passed : Natural := 0;
   Golden : Unbounded_String;
   File : File_Type;
   procedure Check (Condition : Boolean; Label_Text : String) is
   begin
      if not Condition then raise Program_Error with Label_Text; end if;
      Passed := Passed + 1;
   end Check;
   function Document return JSON_Value is
     (AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden)));
   procedure Reject (Doc : JSON_Value; Label_Text : String) is
      Published : Boolean := False;
   begin
      begin
         declare
            Item : constant BinaryPayload := Decode_BinaryPayload
              (AMS_GRA_OMS_JSON_Guard.Write (Doc));
            pragma Unreferenced (Item);
         begin
            Published := True;
         end;
      exception
         when Codec_Error => Check (not Published, Label_Text & " atomic"); return;
      end;
      raise Program_Error with "accepted " & Label_Text;
   end Reject;
   procedure Bad (Key : String; Value : JSON_Value; Label_Text : String) is
      Doc : constant JSON_Value := Document;
   begin
      Doc.Set_Field (Key, Value);
      Reject (Doc, Label_Text);
   end Bad;
   procedure Lexical (Text, Canonical : String) is
      Doc : constant JSON_Value := Document;
   begin
      Doc.Set_Field ("Direct", Create (Text));
      declare
         Item : constant BinaryPayload := Decode_BinaryPayload
           (AMS_GRA_OMS_JSON_Guard.Write (Doc));
         Encoded : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read
           (Encode_BinaryPayload (Item));
      begin
         Check (String'(Get (Encoded, "Direct")) = Canonical, "canonical lexical");
      end;
   end Lexical;
begin
   Open (File, In_File, Ada.Command_Line.Argument (1));
   while not End_Of_File (File) loop Append (Golden, Get_Line (File)); end loop;
   Close (File);
   declare
      Item : constant BinaryPayload := Decode_BinaryPayload (To_String (Golden));
      Again : constant BinaryPayload := Decode_BinaryPayload (Encode_BinaryPayload (Item));
   begin
      Check (Natural (Item.Direct.Length) = 3, "direct length");
      Check (Item.Direct.Element (0) = 0 and then Item.Direct.Element (1) = 16#A1#
        and then Item.Direct.Element (2) = 16#FF#, "direct octets");
      Check (Natural (Item.Named.Value.Length) = 4 and then
        Item.Named.Value.Element (0) = 16#DE#, "unconstrained named");
      Check (Value (Item.ExactValue).Element (1) = 16#80#, "high bit");
      Check (Natural (Value (Item.Empty).Length) = 0 and then
        Natural (Item.Inherited.Length) = 0, "empty octets");
      Check (Item.OptionalDirect.Is_Present and then Item.OptionalNamed.Is_Present,
        "optional presence");
      Check (Again.Direct = Item.Direct, "round trip octets");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_BinaryPayload (Item)) = Document,
        "independent structural golden");
   end;
   declare
      Doc : constant JSON_Value := Document;
   begin
      Doc.Unset_Field ("OptionalDirect"); Doc.Unset_Field ("OptionalNamed");
      declare
         Item : constant BinaryPayload := Decode_BinaryPayload
           (AMS_GRA_OMS_JSON_Guard.Write (Doc));
      begin
         Check (not Item.OptionalDirect.Is_Present and then not Item.OptionalNamed.Is_Present,
           "optional absence");
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_BinaryPayload (Item)) = Doc,
           "optional omission");
      end;
   end;
   Lexical ("", ""); Lexical ("00", "00");
   Lexical ("00a1Ff", "00A1FF"); Lexical ("deAdbEEf", "DEADBEEF");
   Lexical (Character'Val (9) & " 0080ff" & Character'Val (13) & Character'Val (10), "0080FF");
   Lexical (" " & Character'Val (9), "");
   Bad ("Direct", Create ("0"), "odd digits");
   Bad ("Direct", Create ("GG"), "invalid digit");
   Bad ("Direct", Create ("AA BB"), "internal space");
   Bad ("Direct", Create ("AA" & Character'Val (9) & "BB"), "internal tab");
   Bad ("Direct", JSON_Null, "null");
   Bad ("Direct", Create (Empty_Array), "array");
   Bad ("Direct", Create_Object, "object");
   Bad ("Direct", Create (Integer'(1)), "number");
   Bad ("Direct", Create (True), "boolean");
   Bad ("ExactValue", Create ("0000"), "exact too short");
   Bad ("BoundedValue", Create (""), "min octets");
   Bad ("BoundedValue", Create ("0000000000"), "max octets");
   Bad ("Empty", Create ("00"), "zero octets");
   Bad ("Unknown", Create (""), "unknown member");
   declare
      Doc : constant JSON_Value := Document;
      Nested : constant JSON_Value := Create_Object;
   begin
      Doc.Unset_Field ("Direct"); Reject (Doc, "missing required");
      Nested.Set_Field ("Nested", Create ("0Z"));
      Bad ("Details", Nested, "invalid nested");
      Bad ("LaterDetails", Nested, "invalid later nested");
   end;
   Put_Line ("TASK093_CODEC_COMPLETE cases=" & Natural'Image (Passed));
end Hexbinary_Consumer;