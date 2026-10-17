with Ada.Text_IO; use Ada.Text_IO;
with Interfaces; use type Interfaces.Unsigned_8;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;
procedure Hexbinary_Real_Consumer is
   Passed : Natural := 0;
   -- Independent 32-octet golden, not produced by the codec.
   Hash : constant String := "0080FF0102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D";
   function Document return JSON_Value is
      Result : constant JSON_Value := Create_Object;
   begin
      Result.Set_Field ("Data", Create ("00A1FF"));
      Result.Set_Field ("SHA_2_Hash", Create (Hash));
      return Result;
   end Document;
   procedure Check (Condition : Boolean; Label_Text : String) is
   begin
      if not Condition then raise Program_Error with Label_Text; end if;
      Passed := Passed + 1;
   end Check;
   procedure Reject (Key, Text : String) is
      Doc : constant JSON_Value := Document;
      Published : Boolean := False;
   begin
      Doc.Set_Field (Key, Create (Text));
      begin
         declare
            Item : constant BinaryProbe := Decode_BinaryProbe
              (AMS_GRA_OMS_JSON_Guard.Write (Doc));
            pragma Unreferenced (Item);
         begin
            Published := True;
         end;
      exception
         when Codec_Error => Check (not Published, "real decode atomic " & Key); return;
      end;
      raise Program_Error with "accepted invalid real Binary " & Key;
   end Reject;
begin
   declare
      Item : constant BinaryProbe := Decode_BinaryProbe
        (AMS_GRA_OMS_JSON_Guard.Write (Document));
      Octets : constant Binary_Vectors.Vector := Item.Data.Value;
      Hash_Octets : constant Binary_Vectors.Vector := Value (Item.SHA_2_Hash.Value);
   begin
      Check (Item.Data.Is_Present and then Item.SHA_2_Hash.Is_Present, "real optional present");
      Check (Natural (Octets.Length) = 3 and then Octets.Element (0) = 0
        and then Octets.Element (1) = 16#A1# and then Octets.Element (2) = 16#FF#, "real Data octets");
      Check (Natural (Hash_Octets.Length) = 32 and then Hash_Octets.Element (1) = 16#80#
        and then Hash_Octets.Element (2) = 16#FF#, "real hash octets");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_BinaryProbe (Item)) = Document,
        "real independent canonical golden");
   end;
   declare
      Doc : constant JSON_Value := Document;
   begin
      Doc.Set_Field ("Data", Create (Character'Val (9) & " 00a1fF " & Character'Val (13)));
      declare
         Item : constant BinaryProbe := Decode_BinaryProbe (AMS_GRA_OMS_JSON_Guard.Write (Doc));
      begin
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_BinaryProbe (Item)) = Document,
           "real mixed case whitespace canonical");
      end;
      Doc.Set_Field ("Data", Create (""));
      declare
         Item : constant BinaryProbe := Decode_BinaryProbe (AMS_GRA_OMS_JSON_Guard.Write (Doc));
      begin
         Check (Item.Data.Is_Present, "real empty present");
         Check (String'(Get (AMS_GRA_OMS_JSON_Guard.Read (Encode_BinaryProbe (Item)), "Data")) = "",
           "real empty octets");
      end;
      Doc.Unset_Field ("Data"); Doc.Unset_Field ("SHA_2_Hash");
      declare
         Item : constant BinaryProbe := Decode_BinaryProbe (AMS_GRA_OMS_JSON_Guard.Write (Doc));
      begin
         Check (not Item.Data.Is_Present and then not Item.SHA_2_Hash.Is_Present, "real absent");
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_BinaryProbe (Item)) = Doc, "real optional omission");
      end;
   end;
   Reject ("Data", "0"); Reject ("Data", "XX"); Reject ("Data", "AA BB");
   Reject ("SHA_2_Hash", Hash (1 .. 62));
   Reject ("SHA_2_Hash", Hash & "00");
   Put_Line ("TASK093_REAL_COMPLETE cases=" & Natural'Image (Passed));
end Hexbinary_Real_Consumer;