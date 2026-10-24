--  Task 097 production gate consumer, shared by every generated fixture.
--  Ordinary resources only: forced allocator exhaustion is outside the
--  qualified production scope (docs/task-097-ada-float64-production.md).
with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Interfaces.C;
with Programs.Oam;
with Service_Codec;
procedure Conformance is
   use type Interfaces.C.int;
   package Model renames Programs.Oam;
   --  Test-only per-thread German numeric locale (tests/ada-float64-probe).
   function Locale_Enter return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_locale_enter";
   function Locale_Leave return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_locale_leave";
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
   if Mode = "locale" then
      --  The production C boundary must use its own C numeric locale and
      --  restore this thread's exact German handle after every operation.
      Check (Locale_Enter = 0);
      declare
         Decoded : constant Model.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (Valid);
      begin
         Check (Service_Codec.Encode_LosslessEnvelope (Decoded) = Before);
         begin
            Item := Service_Codec.Decode_LosslessEnvelope ("{""malformed""");
            raise Program_Error with "malformed document accepted in German locale";
         exception
            when Service_Codec.Codec_Error =>
               Check (Service_Codec.Encode_LosslessEnvelope (Item) = Before);
         end;
      end;
      Check (Locale_Leave = 0);
      Check (Service_Codec.Encode_LosslessEnvelope (Service_Codec.Decode_LosslessEnvelope (Valid)) = Before);
      Put_Line ("TASK097_LOCALE_COMPLETE checks=" & Checks'Image);
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
   elsif Mode = "accept" then
      Item := Service_Codec.Decode_LosslessEnvelope (Read_File (Ada.Command_Line.Argument (3)));
      declare
         First : constant String := Service_Codec.Encode_LosslessEnvelope (Item);
         Roundtrip : constant Model.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (First);
      begin
         --  Deterministic repeated output and typed round trip.
         Check (Service_Codec.Encode_LosslessEnvelope (Item) = First);
         Check (Service_Codec.Encode_LosslessEnvelope (Roundtrip) = First);
         Put_Line (First);
      end;
      Put_Line ("TASK097_CONFORMANCE_ACCEPT_COMPLETE");
   else
      raise Program_Error with "unknown conformance mode";
   end if;
end Conformance;