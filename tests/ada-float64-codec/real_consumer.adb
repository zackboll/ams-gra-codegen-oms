with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Programs.Oam;
with Service_Codec;
procedure Real_Consumer is
   Input : File_Type;
   Text : Unbounded_String;
   Output : Unbounded_String := To_Unbounded_String ("sentinel");
begin
   Open (Input, In_File, Ada.Command_Line.Argument (2));
   while not End_Of_File (Input) loop
      Append (Text, Get_Line (Input));
   end loop;
   Close (Input);
   begin
      Output := To_Unbounded_String (Service_Codec.Encode_TASK097_PAYLOAD
        (Service_Codec.Decode_TASK097_PAYLOAD (To_String (Text))));
      if Ada.Command_Line.Argument (1) = "reject" then
         raise Program_Error with "invalid real document accepted";
      end if;
      Put_Line (To_String (Output));
      Put_Line ("TASK097_REAL_ACCEPT_COMPLETE");
   exception
      when Service_Codec.Codec_Error =>
         if Ada.Command_Line.Argument (1) /= "reject" or else Output /= "sentinel" then
            raise Program_Error with "unexpected/non-atomic real rejection";
         end if;
         Put_Line ("TASK097_REAL_REJECT_ATOMIC");
   end;
end Real_Consumer;