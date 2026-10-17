with Ada.Command_Line;
with Ada.Strings.Unbounded;
with Ada.Text_IO;
with Interfaces;
with Programs.Oam;
with Service_Codec;
procedure Float32_Identifiers is
   package Model renames Programs.Oam;
   use type Interfaces.IEEE_Float_32;
   use Ada.Strings.Unbounded;
   Checks : Natural := 0;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "identifier regression"; end if;
      Checks := Checks + 1;
   end;
   procedure Verify (Item : Model.ArraySample) is
   begin
      Check (Item.Reading = 0.0 and then Interfaces.IEEE_Float_32'Copy_Sign (1.0, Item.Reading) < 0.0);
      Check (Interfaces.IEEE_Float_32 (Item.Detail.Measurement) = 1.0 + 2.0 ** (-23));
      Check (To_String (Item.Detail.Content) = "Get ( Length ( JSON_Array");
      Check (Model.Length (Item.More) = 2);
      Check (Interfaces.IEEE_Float_32 (Model.Element (Item.More, 1)) = 0.100000001490116119384765625);
      Check (Interfaces.IEEE_Float_32 (Model.Element (Item.More, 2)) = Interfaces.IEEE_Float_32'Scaling (1.0, -149));
   end;
   Input : Ada.Text_IO.File_Type;
begin
   Ada.Text_IO.Open (Input, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   declare
      Item : constant Model.ArraySample := Service_Codec.Decode_ArraySample (Ada.Text_IO.Get_Line (Input));
   begin
      Verify (Item);
      Verify (Service_Codec.Decode_ArraySample (Service_Codec.Encode_ArraySample (Item)));
   end;
   Ada.Text_IO.Close (Input);
   if Checks /= 12 then raise Program_Error with "incomplete identifier checks"; end if;
   Ada.Text_IO.Put_Line ("TASK095_C2_IDENTIFIERS_COMPLETE checks=" & Checks'Image);
end Float32_Identifiers;