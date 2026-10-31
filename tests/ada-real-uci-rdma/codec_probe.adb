with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Text_IO;
with Programs.Oam; use Programs.Oam;
with Service_Codec;
procedure Codec_Probe is
   Documents, Expectations : Ada.Text_IO.File_Type;
   Rejected : Natural := 0;
   procedure Constraint_Probe (Kind : Positive) is
   begin
      case Kind is
         when 1 =>
            declare Item : constant UniversallyUniqueIdentifierType :=
              Create ("12345678-1234-1234-1234-123456789abc"); begin
               raise Program_Error with "invalid UUID model accepted";
            end;
         when 2 =>
            declare Item : constant UCI_SchemaVersionStringType := Create ("002.5"); begin
               raise Program_Error with "invalid version model accepted";
            end;
         when 3 =>
            declare Item : constant IPv4_AddressType := Create ("999.0.0.1"); begin
               raise Program_Error with "invalid IPv4 model accepted";
            end;
         when 4 =>
            declare Item : constant IPv6_AddressType := Create ("not-an-ipv6"); begin
               raise Program_Error with "invalid IPv6 model accepted";
            end;
         when others => raise Program_Error;
      end case;
   exception
      when E : Constraint_Error =>
         declare
            Expected : constant String := (case Kind is
              when 1 => "invalid UUID", when 2 => "invalid schema version",
              when 3 => "invalid structured-ASCII string", when others => "invalid IPv6 profile string");
         begin
            if Ada.Exceptions.Exception_Message (E) /= Expected then
               raise Program_Error with "model rejection reason mismatch: " & Ada.Exceptions.Exception_Message (E);
            end if;
         end;
         Ada.Text_IO.Put_Line ("TASK103_MODEL_CONSTRAINT_REJECTED " & Kind'Image);
   end Constraint_Probe;
begin
   Ada.Text_IO.Open (Documents, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   Ada.Text_IO.Open (Expectations, Ada.Text_IO.In_File, Ada.Command_Line.Argument (2));
   while not Ada.Text_IO.End_Of_File (Documents) loop
      declare
         Document : constant String := Ada.Text_IO.Get_Line (Documents);
         Expected : constant String := Ada.Text_IO.Get_Line (Expectations);
      begin
         declare Item : constant RDMA_InitializeSetupMT := Service_Codec.Decode_RDMA_InitializeSetupMT (Document); begin
            raise Program_Error with "invalid document accepted";
         end;
      exception
         when E : Service_Codec.Codec_Error =>
            if Ada.Exceptions.Exception_Message (E) /= Expected then
               raise Program_Error with "codec reason mismatch: " & Ada.Exceptions.Exception_Message (E);
            end if;
            Rejected := Rejected + 1;
            Ada.Text_IO.Put_Line ("TASK103_CODEC_REJECTED " & Expected);
      end;
   end loop;
   if Rejected /= 9 or else not Ada.Text_IO.End_Of_File (Expectations) then raise Program_Error; end if;
   Ada.Text_IO.Close (Documents);
   Ada.Text_IO.Close (Expectations);
   for Kind in 1 .. 4 loop Constraint_Probe (Kind); end loop;
   Ada.Text_IO.Put_Line ("TASK103_INDEPENDENT_CODEC_COMPLETE rejected=9 model_constraints=4");
end Codec_Probe;