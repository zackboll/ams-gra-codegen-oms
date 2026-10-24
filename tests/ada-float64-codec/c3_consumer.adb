with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Allocation_Faults;
with Programs.Oam;
with Service_Codec;
procedure C3_Consumer is
   function Read_File (Name : String) return String is
      F : File_Type;
      S : Unbounded_String;
   begin
      Open (F, In_File, Name);
      while not End_Of_File (F) loop Append (S, Get_Line (F)); end loop;
      Close (F); return To_String (S);
   end Read_File;
   Mode : constant String := Ada.Command_Line.Argument (1);
   Text : constant String := Read_File (Ada.Command_Line.Argument (2));
   Item : Programs.Oam.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (Text);
   Before : constant String := Service_Codec.Encode_LosslessEnvelope (Item);
   Output : Unbounded_String := To_Unbounded_String ("sentinel");
begin
   if Mode = "accept" then
      declare
         Roundtrip : constant Programs.Oam.LosslessEnvelope :=
           Service_Codec.Decode_LosslessEnvelope (Before);
      begin
         if Service_Codec.Encode_LosslessEnvelope (Roundtrip) /= Before then
            raise Program_Error with "typed roundtrip mismatch";
         end if;
      end;
      Put_Line (Before);
      Put_Line ("C3_TYPED_ROUNDTRIP_COMPLETE");
   elsif Mode = "reject" then
      begin
         Item := Service_Codec.Decode_LosslessEnvelope
           (Read_File (Ada.Command_Line.Argument (3)));
         raise Program_Error with "malformed input accepted";
      exception
         when Service_Codec.Codec_Error =>
            if Service_Codec.Encode_LosslessEnvelope (Item) /= Before then
               raise Program_Error with "decode publication";
            end if;
      end;
      Put_Line ("C3_REJECT_ATOMIC_COMPLETE");
   elsif Mode = "encode" then
      Allocation_Faults.Arm (Natural'Value (Ada.Command_Line.Argument (3)));
      begin
         Output := To_Unbounded_String (Service_Codec.Encode_LosslessEnvelope (Item));
         Allocation_Faults.Disarm;
         if To_String (Output) /= Before then raise Program_Error with "output mismatch"; end if;
         Put_Line ("C3_ENCODE_SUCCESS");
      exception
         when E : others =>
            Allocation_Faults.Disarm;
            if Output /= "sentinel" or else Service_Codec.Encode_LosslessEnvelope (Item) /= Before then
               raise Program_Error with "encode publication";
            end if;
            Put_Line ("C3_ENCODE_EXCEPTION " & Ada.Exceptions.Exception_Name (E));
      end;
      if Service_Codec.Encode_LosslessEnvelope (Item) /= Before then
         raise Program_Error with "recovery mismatch";
      end if;
      Put_Line ("C3_ENCODE_RECOVERY_COMPLETE attempts=" & Allocation_Faults.Attempts'Image &
                " failures=" & Allocation_Faults.Failures'Image &
                " size=" & Allocation_Faults.Failed_Size'Image);
   else
      raise Program_Error with "unknown mode";
   end if;
exception when others => Allocation_Faults.Disarm; raise;
end C3_Consumer;