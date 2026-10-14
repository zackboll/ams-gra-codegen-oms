with Ada.Text_IO;
with Interfaces; use type Interfaces.Unsigned_64;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;

--  Additional in-subset generated model: inherited fields, direct bounded
--  integers, named unsigned/Boolean, and optional nested Records.
procedure Subset_Consumer is
   Item : constant AdaPayload :=
     (Inherited => To_Unbounded_String ("base"),
      Enabled => True, Level => -7, Name => To_Unbounded_String ("plain"),
      Signal => Value_5G, Label => Create ("checked"),
      Details => (Text => To_Unbounded_String ("nested")),
      Field_begin => To_Unbounded_String ("wire"),
      Note => (Is_Present => False), OptionalLabel => (Is_Present => False),
      Signed => -5, Unsigned => 255, NamedUnsigned => 9,
      NamedBoolean => True,
      OptionalDetails => (Is_Present => True, Value => (Text => To_Unbounded_String ("optional nested"))));
   Decoded : constant AdaPayload := Decode_AdaPayload (Encode_AdaPayload (Item));
   Passed : Natural := 0;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "subset check"; end if;
      Passed := Passed + 1;
   end Check;
begin
   Check (Decoded.Inherited = Item.Inherited);
   Check (Decoded.Signed = Item.Signed);
   Check (Decoded.Unsigned = Item.Unsigned);
   Check (Decoded.NamedUnsigned = Item.NamedUnsigned);
   Check (Decoded.NamedBoolean = Item.NamedBoolean);
   Check (Decoded.OptionalDetails.Is_Present and then
          Decoded.OptionalDetails.Value.Text = Item.OptionalDetails.Value.Text);
   declare
      Absent : AdaPayload := Item;
   begin
      Absent.OptionalDetails := (Is_Present => False);
      Check (not Decode_AdaPayload (Encode_AdaPayload (Absent)).OptionalDetails.Is_Present);
   end;
   Ada.Text_IO.Put_Line ("TASK087B_SUBSET_COMPLETE cases=" & Passed'Image);
end Subset_Consumer;