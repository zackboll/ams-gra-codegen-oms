with Ada.Command_Line; with Ada.Text_IO; with Ada.Strings.Unbounded; with Programs.OAM; use Programs.OAM;
with Service_Codec; use Service_Codec; with GNATCOLL.JSON; use GNATCOLL.JSON; with AMS_GRA_OMS_JSON_Guard;
procedure Codec_Consumer is
File : Ada.Text_IO.File_Type; Text : Ada.Strings.Unbounded.Unbounded_String;
Checks : Natural := 0;
procedure Check (OK : Boolean) is begin if not OK then raise Program_Error with "real UCI check"; end if; Checks := Checks + 1; end Check;
begin
Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
while not Ada.Text_IO.End_Of_File (File) loop Ada.Strings.Unbounded.Append (Text, Ada.Text_IO.Get_Line (File)); end loop;
Ada.Text_IO.Close (File);
declare
Good : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Ada.Strings.Unbounded.To_String (Text));
Item : constant SurvivabilityRiskLevelMT := Decode_SurvivabilityRiskLevelMT (AMS_GRA_OMS_JSON_Guard.Write (Good));
Again : constant SurvivabilityRiskLevelMT := Decode_SurvivabilityRiskLevelMT (Encode_SurvivabilityRiskLevelMT (Item));
begin
Check (Again.SecurityInformation.OwnerProducer.Required (1).Kind = GovernmentIdentifier_Kind);
Check (Length (Again.SecurityInformation.OwnerProducer.Additional) = 1);
Check (Element (Again.SecurityInformation.OwnerProducer.Additional, 1).Kind = NATO_SpecialWord_Kind);
Check (Value (Element (Again.SecurityInformation.OwnerProducer.Additional, 1).NATO_SpecialWord) = "NATO:EXAMPLE");
Check (Length (Again.MessageData.RiskSetting) = 1);
Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_SurvivabilityRiskLevelMT (Again)) = Good);
for Case_Number in 1 .. 2 loop
   declare
      Bad : JSON_Value := Clone (Good);
      Header : JSON_Value := Bad.Get ("MessageHeader");
      ID : JSON_Value := Header.Get ("SystemID");
      Published : Boolean := False;
   begin
      if Case_Number = 1 then
         ID.Set_Field ("UUID", "12345678-1234-1234-1234-123456789abc");
         Header.Set_Field ("SystemID", ID);
      else Header.Set_Field ("SchemaVersion", "002.6"); end if;
      Bad.Set_Field ("MessageHeader", Header);
      begin
         declare Invalid_Item : constant SurvivabilityRiskLevelMT := Decode_SurvivabilityRiskLevelMT (AMS_GRA_OMS_JSON_Guard.Write (Bad));
         begin Published := True; Check (False); end;
      exception when Codec_Error => Check (not Published); end;
   end;
end loop;
end;
Ada.Text_IO.Put_Line ("TASK105_CODEC_PASS checks=" & Checks'Image);
end Codec_Consumer;
