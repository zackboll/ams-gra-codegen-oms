with Ada.Text_IO; use Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Urn.Choice; use Urn.Choice;
with Service_Codec; use Service_Codec;
procedure Qualified_Choice_Consumer is
   A : constant ChoicePayload := (Pick => (Kind => Alpha_Kind, Alpha => -7));
   B : constant ChoicePayload := (Pick => (Kind => Beta_Kind, Beta => To_Unbounded_String ("text")));
   Passed : Natural := 0;
   procedure Test (Item : ChoicePayload; Golden : String) is
      Decoded : constant ChoicePayload := Decode_ChoicePayload (Golden);
   begin
      if Decoded /= Item or else Decode_ChoicePayload (Encode_ChoicePayload (Item)) /= Item
        or else AMS_GRA_OMS_JSON_Guard.Read (Encode_ChoicePayload (Item)) /=
          AMS_GRA_OMS_JSON_Guard.Read (Golden)
      then raise Program_Error; end if;
      Passed := Passed + 1;
   end Test;
begin
   Test (A, "{""{urn:choice}Pick"":{""{urn:choice}Alpha"":-7}}");
   Test (B, "{""{urn:choice}Pick"":{""{urn:choice}Beta"":""text""}}");
   Put_Line ("TASK088_QUALIFIED_COMPLETE cases=" & Passed'Image);
end Qualified_Choice_Consumer;