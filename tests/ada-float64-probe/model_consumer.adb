-- Model-only checkpoint. No JSON and no generated codec is exercised here.
with Ada.Assertions;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Interfaces;
with Programs.Oam;
procedure Model_Consumer is
   package Model renames Programs.Oam;
   subtype F64 is Interfaces.IEEE_Float_64;
   use type F64;
   use type Interfaces.IEEE_Float_32;
   Checks, Rejections : Natural := 0;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "model checkpoint failed"; end if;
      Checks := Checks + 1;
   end Check;
   Bounded : Model.FloatPayload_Bounded_Sequence := Model.To_Sequence ((1 => F64'(1.0)));
   Additional : Model.FloatPayload_Unbounded_Additional;
   Item : Model.FloatPayload :=
     (Required => 1.0 + 2.0 ** (-52), Optional => (True, -0.0), Named => 1.25,
      Closed => Model.Create (F64'(-1.0)), Open => Model.Create (F64'(1.0)),
      Bounded => Bounded, Unbounded => (Required => (1.0, 2.0), Additional => Additional),
      NamedRepeated => Model.To_Sequence ((1 => Model.Create (F64'(0.5)))),
      Nested => (Value => 2.0), Choice => (Model.Alternative_Record_Kind, (Value => 3.0)),
      Small => 1.5, Count => 42, Enabled => True, Text => To_Unbounded_String ("model only"));
   Volatile_Zero : F64 := 0.0;
   pragma Volatile (Volatile_Zero);
   Volatile_Two : F64 := 2.0;
   pragma Volatile (Volatile_Two);
begin
   Check (F64'Value_Size = 64 and F64'Object_Size = 64 and F64'Alignment = 8);
   Check (F64' Machine_Radix = 2 and F64' Machine_Mantissa = 53);
   Check (F64' Machine_Emin = -1021 and F64' Machine_Emax = 1024 and F64'Signed_Zeros);
   Check (Item.Required = 1.0 + 2.0 ** (-52));
   Check (Item.Optional.Is_Present and then F64'Copy_Sign (1.0, Item.Optional.Value) = -1.0);
   Item.Optional := (Is_Present => False);
   Check (not Item.Optional.Is_Present);
   Check (F64 (Item.Named) = 1.25);
   Check (Model.Value (Item.Closed) = -1.0 and Model.Value (Item.Open) = 1.0);
   Model.Append (Bounded, 2.0);
   Check (Model.Length (Bounded) = 2 and then Model.Element (Bounded, 2) = 2.0);
   Model.Append (Item.Unbounded.Additional, 3.0);
   Check (Item.Unbounded.Required (2) = 2.0 and then Model.Length (Item.Unbounded.Additional) = 1);
   Check (Model.Element (Item.Unbounded.Additional, 1) = 3.0);
   Check (Model.Value (Model.Element (Item.NamedRepeated, 1)) = 0.5);
   Check (Item.Nested.Value = 2.0 and Item.Choice.Alternative_Record.Value = 3.0);
   Check (Item.Small = 1.5 and Item.Count = 42 and Item.Enabled and To_String (Item.Text) = "model only");
   begin
      Item.Open := Model.Create (Volatile_Zero);
      raise Program_Error with "exclusive lower bound bypassed";
   exception when Ada.Assertions.Assertion_Error => Rejections := Rejections + 1; end;
   begin
      Item.Open := Model.Create (Volatile_Two);
      raise Program_Error with "exclusive upper bound bypassed";
   exception when Ada.Assertions.Assertion_Error => Rejections := Rejections + 1; end;
   begin
      Item.Closed := Model.Create (Volatile_Two);
      raise Program_Error with "inclusive bound bypassed";
   exception when Ada.Assertions.Assertion_Error => Rejections := Rejections + 1; end;
   Put_Line ("TASK096_MODEL_ONLY_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
end Model_Consumer;