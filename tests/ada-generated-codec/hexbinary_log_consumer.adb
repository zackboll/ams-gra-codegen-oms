with Ada.Command_Line;
with Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Interfaces; use type Interfaces.Unsigned_8;
with Programs.OAM; use Programs.OAM;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;

procedure Hexbinary_Log_Consumer is
   File : Ada.Text_IO.File_Type;
   Text : Unbounded_String;
   Good : JSON_Value;
   Checks, Rejections : Natural := 0;
   procedure Check (OK : Boolean; Label_Text : String) is
   begin
      if not OK then raise Program_Error with Label_Text; end if;
      Checks := Checks + 1;
   end Check;
   function Octets (Item : LogMT) return Binary_Vectors.Vector is
     (Item.MessageData.Data.Value);
   procedure Reject (Data : JSON_Value) is
      Bad : constant JSON_Value := Clone (Good);
      Message_Data : constant JSON_Value := Bad.Get ("MessageData");
      Published : Boolean := False;
   begin
      Message_Data.Set_Field ("Data", Data);
      Bad.Set_Field ("MessageData", Message_Data);
      begin
         declare
            Item : constant LogMT := Decode_LogMT (AMS_GRA_OMS_JSON_Guard.Write (Bad));
            pragma Unreferenced (Item);
         begin
            Published := True;
         end;
      exception
         when Codec_Error =>
            Check (not Published, "real Log failure atomicity");
            Rejections := Rejections + 1;
            return;
      end;
      raise Program_Error with "invalid real Log Binary accepted";
   end Reject;
begin
   Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   while not Ada.Text_IO.End_Of_File (File) loop Append (Text, Ada.Text_IO.Get_Line (File)); end loop;
   Ada.Text_IO.Close (File);
   Good := AMS_GRA_OMS_JSON_Guard.Read (To_String (Text));
   declare
      Item : constant LogMT := Decode_LogMT (To_String (Text));
      Again : constant LogMT := Decode_LogMT (Encode_LogMT (Item));
      Bytes : constant Binary_Vectors.Vector := Octets (Again);
   begin
      Check (Item.MessageData.Data.Is_Present, "real optional present");
      Check (Natural (Bytes.Length) = 3, "real octet length");
      Check (Bytes.Element (0) = 0, "zero byte");
      Check (Bytes.Element (1) = 16#80#, "high-bit byte");
      Check (Bytes.Element (2) = 16#FF#, "maximum byte");
      Check (Again.SecurityInformation.OwnerProducer.Required (1).Kind = GovernmentIdentifier_Kind,
             "required repeated Choice");
      Check (Length (Again.SecurityInformation.OwnerProducer.Additional) = 1, "additional Choice length");
      Check (Element (Again.SecurityInformation.OwnerProducer.Additional, 1).Kind = NATO_SpecialWord_Kind,
             "additional Choice discriminant");
      Check (Value (Element (Again.SecurityInformation.OwnerProducer.Additional, 1).NATO_SpecialWord) =
             "NATO:EXAMPLE", "additional Choice value");
      Check (Value (Again.MessageData.Timestamp) = "2026-01-02T03:04:06Z", "real DateTime");
      Check (Again.MessageData.ServiceUpTime.Is_Present and then
             Value (Again.MessageData.ServiceUpTime.Value) = "PT01.2300S", "real Duration");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_LogMT (Item)) = Good, "independent real golden");
      Check (Encode_LogMT (Again) = Encode_LogMT (Item), "canonical round trip");
   end;
   for Present in Boolean loop
      declare
         Doc : constant JSON_Value := Clone (Good);
         Data : constant JSON_Value := Doc.Get ("MessageData");
      begin
         if Present then Data.Set_Field ("Data", ""); else Data.Unset_Field ("Data"); end if;
         Doc.Set_Field ("MessageData", Data);
         declare
            Item : constant LogMT := Decode_LogMT (AMS_GRA_OMS_JSON_Guard.Write (Doc));
         begin
            Check (Item.MessageData.Data.Is_Present = Present, "real optionality");
            if Present then Check (Natural (Octets (Item).Length) = 0, "real empty Binary"); end if;
            Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_LogMT (Item)) = Doc, "optional round trip");
         end;
      end;
   end loop;
   declare
      Doc : constant JSON_Value := Clone (Good);
      Data : constant JSON_Value := Doc.Get ("MessageData");
   begin
      Data.Set_Field ("Data", " " & Character'Val (9) & "0080ff" & Character'Val (13) & Character'Val (10));
      Doc.Set_Field ("MessageData", Data);
      declare
         Item : constant LogMT := Decode_LogMT (AMS_GRA_OMS_JSON_Guard.Write (Doc));
      begin
         Check (Octets (Item).Element (1) = 16#80#, "real lowercase typed");
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_LogMT (Item)) = Good, "real uppercase canonical");
      end;
   end;
   Reject (Create ("0"));
   Reject (Create ("000"));
   Reject (Create ("0G"));
   Reject (Create ("AA BB"));
   Reject (Create ("AA" & Character'Val (9) & "BB"));
   Reject (JSON_Null);
   Reject (Create (Integer'(1)));
   Reject (Create (Empty_Array));
   Reject (Create_Object);
   Ada.Text_IO.Put_Line ("TASK093_LOG_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
end Hexbinary_Log_Consumer;