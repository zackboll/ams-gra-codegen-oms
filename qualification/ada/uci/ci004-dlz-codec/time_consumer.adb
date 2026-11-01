--  CI-SELECT-004-A2: retain the registered codec execution contract.
--  Task 098: batch consumer over a production-generated service codec.
--
--  The harness substitutes Programs.Oam and DLZ_MT. Each input
--  line is one complete OMS JSON payload document. For each line it prints
--  exactly one result line:
--
--    ACCEPT<TAB><encoded document>   decode, encode, re-decode, re-encode
--                                    succeeded and both encodings agree
--    REJECT<TAB><Codec_Error message> Service_Codec.Codec_Error only
--
--  Every other exception (including Constraint_Error escaping a checked
--  constructor) is deliberately NOT handled: it terminates the process with a
--  nonzero status, which the harness treats as a qualification failure.
with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Programs.Oam;
with Service_API;  --  Compile the generated service API as well as the codec.
with Service_Codec;

procedure Time_Consumer is
   Input : File_Type;
   Count, Accepted, Rejected : Natural := 0;
begin
   Open (Input, In_File, Ada.Command_Line.Argument (1));
   while not End_Of_File (Input) loop
      declare
         Text : constant String := Get_Line (Input);
         Published : Unbounded_String := To_Unbounded_String ("sentinel");
      begin
         Count := Count + 1;
         declare
            Item : constant Programs.Oam.DLZ_MT :=
              Service_Codec.Decode_DLZ_MT (Text);
         begin
            Published := To_Unbounded_String (Service_Codec.Encode_DLZ_MT (Item));
         end;
         --  Outside the Codec_Error scope of the original decode: an encoder
         --  output the decoder rejects is a defect, not an expected rejection.
         declare
            Again : constant Programs.Oam.DLZ_MT :=
              Service_Codec.Decode_DLZ_MT (To_String (Published));
         begin
            if Service_Codec.Encode_DLZ_MT (Again) /= To_String (Published) then
               raise Program_Error with "encode/decode/encode instability at line" & Count'Image;
            end if;
         exception
            when Service_Codec.Codec_Error =>
               raise Program_Error with "encoder output rejected at line" & Count'Image;
         end;
         Put_Line ("ACCEPT" & ASCII.HT & To_String (Published));
         Accepted := Accepted + 1;
      exception
         when E : Service_Codec.Codec_Error =>
            if Published /= "sentinel" then
               raise Program_Error with "non-atomic rejection at line" & Count'Image;
            end if;
            Put_Line ("REJECT" & ASCII.HT & Ada.Exceptions.Exception_Message (E));
            Rejected := Rejected + 1;
      end;
   end loop;
   Close (Input);
   Put_Line ("TASK098_CONSUMER_COMPLETE count=" & Count'Image & " accepted=" & Accepted'Image
             & " rejected=" & Rejected'Image);
end Time_Consumer;
