with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Interfaces; use type Interfaces.Unsigned_8;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;

procedure Hexbinary_Combined_Consumer is
   Good : JSON_Value;
   Checks, Rejections : Natural := 0;
   procedure Check (OK : Boolean; Label_Text : String) is
   begin
      if not OK then raise Program_Error with Label_Text; end if;
      Checks := Checks + 1;
   end Check;
   function Changed (Key : String; Value : JSON_Value) return JSON_Value is
      Doc : constant JSON_Value := Clone (Good);
      Picks : JSON_Array := Get (Doc.Get ("Picks"));
      Pick : constant JSON_Value := Get (Picks, 3);
      Nested : constant JSON_Value := Pick.Get ("Nested");
   begin
      Nested.Set_Field (Key, Value);
      Pick.Set_Field ("Nested", Nested);
      Set_Element (Picks, 3, Pick);
      Doc.Set_Field ("Picks", Create (Picks));
      return Doc;
   end Changed;
   procedure Reject (Doc : JSON_Value) is
      Published : Boolean := False;
   begin
      begin
         declare
            Item : constant Combined := Decode_Combined (AMS_GRA_OMS_JSON_Guard.Write (Doc));
            pragma Unreferenced (Item);
         begin
            Published := True;
         end;
      exception
         when Error : Codec_Error =>
            Check (not Published, "failure-atomic typed publication");
            Check (Ada.Strings.Fixed.Index (Ada.Exceptions.Exception_Message (Error), "Picks item 3") > 0,
                   "outer Choice index");
            Rejections := Rejections + 1;
            return;
      end;
      raise Program_Error with "invalid combined message accepted";
   end Reject;
begin
   declare
      File : Ada.Text_IO.File_Type;
      Text : Unbounded_String;
   begin
      Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
      while not Ada.Text_IO.End_Of_File (File) loop
         Append (Text, Ada.Text_IO.Get_Line (File));
      end loop;
      Ada.Text_IO.Close (File);
      Good := AMS_GRA_OMS_JSON_Guard.Read (To_String (Text));
   end;
   declare
      Item : constant Combined := Decode_Combined (AMS_GRA_OMS_JSON_Guard.Write (Good));
      Again : constant Combined := Decode_Combined (Encode_Combined (Item));
      First : constant Selection := Element (Again.Picks, 1);
      Last : constant Selection := Element (Again.Picks, 3);
   begin
      Check (Length (Item.Picks) = 3, "initial Choice length");
      Check (Length (Again.Picks) = 3, "round-trip Choice length");
      Check (First.Kind = Nested_Kind, "first discriminant");
      Check (Element (Again.Picks, 2).Kind = Enabled_Kind, "second discriminant");
      Check (not Element (Again.Picks, 2).Enabled, "boolean alternative");
      Check (Last.Kind = Nested_Kind, "last discriminant");
      Check (Length (First.Nested.Dates) = 3, "DateTime length");
      Check (Value (Element (First.Nested.Dates, 1)) = "2024-02-29T12:34:56Z", "DateTime 1");
      Check (Value (Element (First.Nested.Dates, 2)) = "2026-10-15T01:02:03+05:30", "DateTime 2");
      Check (Value (Element (First.Nested.Dates, 3)) = "2026-10-15T01:02:03", "DateTime 3");
      Check (Length (First.Nested.Spans) = 3, "Duration length");
      Check (Value (Element (First.Nested.Spans, 1)) = "P1D", "Duration 1");
      Check (Value (Element (First.Nested.Spans, 2)) = "PT01.2300S", "Duration 2");
      Check (Value (Element (First.Nested.Spans, 3)) = "-P3M", "Duration 3");
      Check (Natural (First.Nested.Data.Length) = 3, "Binary octet length");
      Check (First.Nested.Data.Element (0) = 0, "zero octet");
      Check (First.Nested.Data.Element (1) = 16#80#, "high-bit octet");
      Check (First.Nested.Data.Element (2) = 16#FF#, "maximum octet");
      Check (First.Nested.Hash.Is_Present, "named optional present");
      Check (Natural (Value (First.Nested.Hash.Value).Length) = 2, "named constrained length");
      Check (Value (First.Nested.Hash.Value).Element (0) = 16#A1# and then
             Value (First.Nested.Hash.Value).Element (1) = 16#FE#, "named octets");
      Check (Length (Last.Nested.Dates) = 0, "empty dates");
      Check (Length (Last.Nested.Spans) = 0, "empty spans");
      Check (Natural (Last.Nested.Data.Length) = 0, "empty Binary admitted");
      Check (not Last.Nested.Hash.Is_Present, "optional absent");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_Combined (Item)) = Good, "independent canonical golden");
      Check (Encode_Combined (Again) = Encode_Combined (Item), "canonical round trip");
   end;
   declare
      Lower : constant JSON_Value := Changed ("Data", Create (" " & Character'Val (9) & "0080ff" & Character'Val (10)));
      Item : constant Combined := Decode_Combined (AMS_GRA_OMS_JSON_Guard.Write (Lower));
      Encoded : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Encode_Combined (Item));
      Picks : constant JSON_Array := Get (Encoded.Get ("Picks"));
      Nested : constant JSON_Value := Get (Picks, 3).Get ("Nested");
   begin
      Check (String'(Get (Nested, "Data")) = "0080FF", "lowercase and XML whitespace canonicalized");
      Check (Element (Item.Picks, 3).Nested.Data.Element (1) = 16#80#, "lowercase typed octet");
   end;
   declare
      Present : constant JSON_Value := Changed ("Hash", Create ("a1fe"));
      Item : constant Combined := Decode_Combined (AMS_GRA_OMS_JSON_Guard.Write (Present));
      Encoded : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (Encode_Combined (Item));
      Picks : constant JSON_Array := Get (Encoded.Get ("Picks"));
      Nested : constant JSON_Value := Get (Picks, 3).Get ("Nested");
   begin
      Check (Element (Item.Picks, 3).Nested.Hash.Is_Present, "optional made present");
      Check (String'(Get (Nested, "Hash")) = "A1FE", "named lowercase canonicalized");
   end;
   for Temporal in Boolean loop
      declare
         Values : JSON_Array;
      begin
         Append (Values, Create (if Temporal then "2024-02-29T00:00:00Z" else "P1D"));
         Append (Values, Create (if Temporal then "2023-02-29T00:00:00Z" else "P1W"));
         Reject (Changed ((if Temporal then "Dates" else "Spans"), Create (Values)));
      end;
   end loop;
   Reject (Changed ("Data", Create ("0")));
   Reject (Changed ("Data", Create ("0G")));
   Reject (Changed ("Data", Create ("AA BB")));
   Reject (Changed ("Hash", Create ("")));
   Reject (Changed ("Hash", Create ("00")));
   Reject (Changed ("Hash", Create ("000000")));
   Reject (Changed ("Hash", Create ("GG00")));
   Ada.Text_IO.Put_Line ("TASK093_COMBINED_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
end Hexbinary_Combined_Consumer;