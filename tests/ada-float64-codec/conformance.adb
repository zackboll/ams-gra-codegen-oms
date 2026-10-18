with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Allocation_Faults;
with Programs.Oam;
with Service_Codec;
procedure Conformance is
   package Model renames Programs.Oam;
   function Read_File (Name : String) return String is
      Input : File_Type;
      Text : Unbounded_String;
   begin
      Open (Input, In_File, Name);
      while not End_Of_File (Input) loop Append (Text, Get_Line (Input)); end loop;
      Close (Input);
      return To_String (Text);
   end Read_File;
   Mode : constant String := Ada.Command_Line.Argument (1);
   Valid : constant String := Read_File (Ada.Command_Line.Argument (2));
   Item : Model.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (Valid);
   Before : constant String := Service_Codec.Encode_LosslessEnvelope (Item);
   Output : Unbounded_String := To_Unbounded_String ("sentinel");
   Checks : Natural := 0;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "conformance contract"; end if;
      Checks := Checks + 1;
   end Check;
begin
   if Mode = "allocation" then
      declare
         Encode_Failures, Decode_Failures : Natural := 0;
      begin
         for Budget in 0 .. 100 loop
            Put_Line (Standard_Error, "TASK097_FAULT_BUDGET encode" & Budget'Image);
            Output := To_Unbounded_String ("sentinel");
            Allocation_Faults.Arm (Budget);
            begin
               Output := To_Unbounded_String (Service_Codec.Encode_LosslessEnvelope (Item));
               Allocation_Faults.Disarm;
               Check (To_String (Output) = Before);
            exception
               when Storage_Error =>
                  Allocation_Faults.Disarm;
                  Check (Output = "sentinel");
                  Encode_Failures := Encode_Failures + 1;
            end;
            Put_Line (Standard_Error, "TASK097_FAULT_BUDGET decode" & Budget'Image);
            Allocation_Faults.Arm (Budget);
            begin
               Item := Service_Codec.Decode_LosslessEnvelope (Valid);
               Allocation_Faults.Disarm;
            exception
               when Storage_Error =>
                  Allocation_Faults.Disarm;
                  Decode_Failures := Decode_Failures + 1;
            end;
            Check (Service_Codec.Encode_LosslessEnvelope (Item) = Before);
         end loop;
         Check (Encode_Failures > 0 and Decode_Failures > 0);
         Put_Line ("TASK097_ALLOCATION_ATOMIC_COMPLETE encode_failures=" & Encode_Failures'Image &
                   " decode_failures=" & Decode_Failures'Image & " checks=" & Checks'Image);
      end;
   elsif Mode = "reject" then
      begin
         Item := Service_Codec.Decode_LosslessEnvelope (Read_File (Ada.Command_Line.Argument (3)));
         raise Program_Error with "invalid conformance payload accepted";
      exception
         when E : Service_Codec.Codec_Error =>
            Check (Service_Codec.Encode_LosslessEnvelope (Item) = Before);
            Put_Line ("TASK097_REJECT_ATOMIC " & Ada.Exceptions.Exception_Message (E));
      end;
   elsif Mode = "encode-invalid" then
      Item.Field_Record.Text := To_Unbounded_String ("bad" & Character'Val (16#FF#));
      begin
         Output := To_Unbounded_String (Service_Codec.Encode_LosslessEnvelope (Item));
         raise Program_Error with "invalid output accepted";
      exception
         when Service_Codec.Codec_Error =>
            Check (Output = "sentinel");
            Put_Line ("TASK097_ENCODE_REJECT_ATOMIC");
      end;
   else
      Item := Service_Codec.Decode_LosslessEnvelope (Read_File (Ada.Command_Line.Argument (3)));
      Put_Line (Service_Codec.Encode_LosslessEnvelope (Item));
      Put_Line ("TASK097_CONFORMANCE_ACCEPT_COMPLETE");
   end if;
exception
   when others => Allocation_Faults.Disarm; raise;
end Conformance;