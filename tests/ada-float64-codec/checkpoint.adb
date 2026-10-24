with Ada.Command_Line;
with Ada.Text_IO;
with Ada.Unchecked_Conversion;
with Interfaces;
with Programs.Oam;
with Service_Codec;
procedure Checkpoint is
   use type Interfaces.Unsigned_64;
   package Model renames Programs.Oam;
   function Bits is new Ada.Unchecked_Conversion
     (Interfaces.IEEE_Float_64, Interfaces.Unsigned_64);
   function Read_All return String is
      File : Ada.Text_IO.File_Type;
   begin
      Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
      declare
         Text : constant String := Ada.Text_IO.Get_Line (File);
      begin
         Ada.Text_IO.Close (File);
         return Text;
      end;
   end Read_All;
   Text : constant String := Read_All;
   Item : constant Model.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (Text);
begin
   if Bits (Item.Field_Record.Direct) /= 16#3FF0000000000001# or else
     Bits (Item.Field_Record.Optional.Value) /= 16#8000000000000000# or else
     Bits (Interfaces.IEEE_Float_64 (Item.Field_Record.Named)) /= 1
   then raise Program_Error with "Float64 checkpoint bits"; end if;
   Ada.Text_IO.Put_Line (Service_Codec.Encode_LosslessEnvelope (Item));
   Ada.Text_IO.Put_Line ("TASK097_GENERATED_CHECKPOINT_COMPLETE");
end Checkpoint;