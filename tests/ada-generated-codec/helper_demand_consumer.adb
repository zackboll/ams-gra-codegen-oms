with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;

procedure Helper_Demand_Consumer is
   File : Ada.Text_IO.File_Type;
   Checks : Natural := 0;
   Content : constant String := "Encode_Hex_Binary ( Decode_Hex_Binary ( Get ( Length ( JSON_Array";
   procedure Check (OK : Boolean) is
   begin
      if not OK then raise Program_Error with "helper-demand regression"; end if;
      Checks := Checks + 1;
   end Check;
begin
   Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   declare
      Text : constant String := Ada.Text_IO.Get_Line (File);
      Item : constant ArraySample := Decode_ArraySample (Text);
      Encoded : constant String := Encode_ArraySample (Item);
      Again : constant ArraySample := Decode_ArraySample (Encoded);
      Target : ArraySample := Item;
      Output : Unbounded_String := To_Unbounded_String ("sentinel");
   begin
      Check (Item.Number = 42);
      Check (To_String (Item.Text) = Content);
      Check (Again.Number = Item.Number);
      Check (To_String (Again.Text) = Content);
      Check (Encode_ArraySample (Again) = Encoded);
      begin
         Target := Decode_ArraySample ("{}");
         Output := To_Unbounded_String (Encode_ArraySample (Target));
         raise Program_Error with "malformed payload admitted";
      exception
         when Codec_Error =>
            Check (Encode_ArraySample (Target) = Encoded);
            Check (Output = "sentinel");
      end;
   end;
   Ada.Text_IO.Close (File);
   if Checks /= 7 then raise Program_Error with "incomplete helper-demand execution"; end if;
   Ada.Text_IO.Put_Line ("TASK093_C4_HELPER_DEMAND_COMPLETE checks=" & Checks'Image);
end Helper_Demand_Consumer;