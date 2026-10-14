with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Programs.Oam; use Programs.Oam;
with Service_API;
with Service_Codec; use Service_Codec;

procedure Codec_Consumer is
   Golden : Unbounded_String;
   File : File_Type;
   Passed : Natural := 0;
   Unicode : constant String := "Unicode " & Character'Val (195) & Character'Val (169)
     & " " & Character'Val (240) & Character'Val (159) & Character'Val (152) & Character'Val (128);
   Item : constant AdaPayload :=
     (Enabled => True, Level => -7, Name => To_Unbounded_String (Unicode),
      Signal => Value_5G, Label => Create ("checked label"),
      Details => (Text => To_Unbounded_String ("nested record")),
      Field_begin => To_Unbounded_String ("wire name"),
      Note => (Is_Present => True, Value => To_Unbounded_String ("present note")),
      OptionalLabel => (Is_Present => True, Value => Create ("present label")));
   procedure Check (Condition : Boolean; Label_Text : String) is
   begin
      if not Condition then raise Program_Error with Label_Text; end if;
      Passed := Passed + 1;
      Put_Line ("PASS " & Label_Text);
   end Check;
   procedure Equal (Left, Right : AdaPayload) is
   begin
      Check (Left.Enabled = Right.Enabled and then Left.Level = Right.Level
        and then Left.Name = Right.Name and then Left.Signal = Right.Signal
        and then Value (Left.Label) = Value (Right.Label)
        and then Left.Details.Text = Right.Details.Text
        and then Left.Field_begin = Right.Field_begin
        and then Left.Note.Is_Present = Right.Note.Is_Present
        and then Left.OptionalLabel.Is_Present = Right.OptionalLabel.Is_Present,
        "typed required and optional discriminant equality");
      if Left.Note.Is_Present then
         Check (Left.Note.Value = Right.Note.Value, "optional direct String equality");
      end if;
      if Left.OptionalLabel.Is_Present then
         Check (Value (Left.OptionalLabel.Value) = Value (Right.OptionalLabel.Value),
                "optional checked String equality");
      end if;
   end Equal;
   procedure Reject (Text, Label_Text : String) is
   begin
      declare
         Decoded : constant AdaPayload := Decode_AdaPayload (Text);
      begin
         raise Program_Error with "accepted invalid payload: " & Label_Text & Value (Decoded.Label);
      end;
   exception
      when Codec_Error => Check (True, "rejected " & Label_Text);
   end Reject;
   procedure Wrong (Key : String; Replacement : JSON_Value) is
      Object_Value : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden));
   begin
      Object_Value.Set_Field (Key, Replacement);
      Reject (AMS_GRA_OMS_JSON_Guard.Write (Object_Value), Key & " wrong value");
   end Wrong;
begin
   Open (File, In_File, Ada.Command_Line.Argument (1));
   while not End_Of_File (File) loop
      Append (Golden, Get_Line (File));
   end loop;
   Close (File);
   Check (Service_API.Function_Codec.Exchange_Notice.Message_Name = "AdaNotice",
          "generated wrapper linked");
   Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_AdaPayload (Item)) =
          AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden)), "independent golden structural equality");
   Equal (Item, Decode_AdaPayload (To_String (Golden)));
   Equal (Item, Decode_AdaPayload (Encode_AdaPayload (Item)));
   declare
      Object_Value : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Encode_AdaPayload (Item));
      Absent : constant AdaPayload :=
        (Enabled => Item.Enabled, Level => Item.Level, Name => Item.Name, Signal => Item.Signal,
         Label => Item.Label, Details => Item.Details, Field_begin => Item.Field_begin,
         Note => (Is_Present => False), OptionalLabel => (Is_Present => False));
      Encoded : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Encode_AdaPayload (Absent));
   begin
      Check (String'(Get (Object_Value, "Signal")) = "5G"
        and then String'(Get (Object_Value, "begin")) = "wire name"
        and then not Object_Value.Has_Field ("Field_begin"), "enum and remapped wire spelling");
      Check (String'(Get (Object_Value, "Name")) = Unicode, "Unicode bytes preserved");
      Check (not Encoded.Has_Field ("Note") and then not Encoded.Has_Field ("OptionalLabel"),
             "absent optionals omitted");
      Equal (Absent, Decode_AdaPayload (Encode_AdaPayload (Absent)));
      Object_Value.Unset_Field ("Enabled");
      Reject (AMS_GRA_OMS_JSON_Guard.Write (Object_Value), "missing required field");
   end;
   --  Authored order differs from the golden and provider serialization order.
   Equal (Item, Decode_AdaPayload
     ("{""OptionalLabel"":""present label"",""Note"":""present note"",""begin"":""wire name"","
      & """Details"":{""Text"":""nested record""},""Label"":""checked label"","
      & """Signal"":""5G"",""Name"":""" & Unicode & """,""Level"":-7,""Enabled"":true}"));
   Reject ("{", "malformed JSON");
   Reject ("[]", "wrong root");
   Wrong ("Unknown", GNATCOLL.JSON.Create (True));
   Wrong ("Enabled", GNATCOLL.JSON.Create ("true"));
   Wrong ("Name", GNATCOLL.JSON.Create (False));
   Wrong ("Level", GNATCOLL.JSON.Create ("-7"));
   Wrong ("Level", GNATCOLL.JSON.Create (Long_Long_Integer'(11)));
   Wrong ("Level", GNATCOLL.JSON.Create (Long_Long_Integer'(-11)));
   Wrong ("Signal", GNATCOLL.JSON.Create ("Value_5G"));
   Wrong ("Label", GNATCOLL.JSON.Create (""));
   Wrong ("OptionalLabel", GNATCOLL.JSON.Create (""));
   Wrong ("Note", JSON_Null);
   Wrong ("Label", JSON_Null);
   Wrong ("Details", GNATCOLL.JSON.Create ("not a Record"));
   Wrong ("Details", Create_Object);
   Reject ("{""Name"":""\uD800""}", "malformed surrogate");
   Reject ("{""Name"":""" & Character'Val (255) & """}", "invalid UTF8");
   Reject ((1 .. AMS_GRA_OMS_JSON_Guard.Max_Document_Bytes + 1 => ' '), "oversized document");
   Reject ((1 .. 129 => '[') & "0" & (1 .. 129 => ']'), "excessive nesting");
   Reject ("{""Level"":9223372036854775808}", "signed overflow");
   Wrong ("Level", AMS_GRA_OMS_JSON_Guard.Read ("1.0"));
   Wrong ("Level", AMS_GRA_OMS_JSON_Guard.Read ("1e0"));
   --  Guard/provider policy is deliberately last-value-wins.
   Equal (Item, Decode_AdaPayload ("{""Enabled"":false," & To_String (Golden) (2 .. Length (Golden))));
   Put_Line ("TASK087B_CODEC_COMPLETE cases=" & Passed'Image);
end Codec_Consumer;