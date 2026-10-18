with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Allocation_Faults;
with Programs.Oam;
with Service_Codec;
procedure C1_Generated_Probe is
   function Read_File return String is
      Input : File_Type;
      Text : Unbounded_String;
   begin
      Open (Input, In_File, Ada.Command_Line.Argument (3));
      while not End_Of_File (Input) loop Append (Text, Get_Line (Input)); end loop;
      Close (Input);
      return To_String (Text);
   end Read_File;
   Text : constant String := Read_File;
   Item : Programs.Oam.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (Text);
   Before : constant String := Service_Codec.Encode_LosslessEnvelope (Item);
   Output : Unbounded_String := To_Unbounded_String ("sentinel");
   Mode : constant String := Ada.Command_Line.Argument (1);
   Budget : constant Natural := Natural'Value (Ada.Command_Line.Argument (2));
begin
   Allocation_Faults.Arm (Budget);
   begin
      if Mode = "decode" then
         Item := Service_Codec.Decode_LosslessEnvelope (Text);
      elsif Mode = "encode" then
         Output := To_Unbounded_String (Service_Codec.Encode_LosslessEnvelope (Item));
      else
         raise Program_Error with "bad probe operation";
      end if;
      Allocation_Faults.Disarm;
      Put_Line ("TASK097_C1_GENERATED_SUCCESS");
   exception
      when E : others =>
         Allocation_Faults.Disarm;
         Put_Line ("TASK097_C1_GENERATED_EXCEPTION " & Ada.Exceptions.Exception_Name (E));
         Put_Line (Ada.Exceptions.Exception_Message (E));
         if Output /= "sentinel" or else Service_Codec.Encode_LosslessEnvelope (Item) /= Before then
            raise Program_Error with "published/retained result changed on failure";
         end if;
   end;
   Item := Service_Codec.Decode_LosslessEnvelope (Text);
   if Service_Codec.Encode_LosslessEnvelope (Item) /= Before then
      raise Program_Error with "generated recovery mismatch";
   end if;
   Put_Line ("TASK097_C1_GENERATED_RECOVERY_COMPLETE");
end C1_Generated_Probe;