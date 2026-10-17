with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;

procedure Repeated_Temporal_Codec_Consumer is
   Good : JSON_Value;
   Checks : Natural := 0;
   Rejections : Natural := 0;
   procedure Check (OK : Boolean; Label : String) is
   begin
      if not OK then raise Program_Error with Label; end if;
      Checks := Checks + 1;
   end Check;
   procedure Reject (Bad : JSON_Value; Label : String) is
      Published : Boolean := False;
   begin
      begin
         declare
            Item : constant TemporalPayload := Decode_TemporalPayload
              (AMS_GRA_OMS_JSON_Guard.Write (Bad));
         begin
            Published := True;
            Check (Length (Item.NamedDates) > 0, Label & " unexpected result");
         end;
      exception
         when Codec_Error =>
            Check (not Published, Label & " partial publication");
            Rejections := Rejections + 1;
            return;
      end;
      raise Program_Error with Label & " accepted malformed input";
   end Reject;
   procedure Bad_Array (Key : String; Child : JSON_Value; Later : Boolean) is
      Bad : JSON_Value := Clone (Good);
      Items : JSON_Array;
   begin
      if Later then Append (Items, Get (Good.Get (Key), 1)); end if;
      Append (Items, Child);
      Bad.Set_Field (Key, Create (Items));
      Reject (Bad, Key & " element" & (if Later then " 2" else " 1"));
   end Bad_Array;
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
      Item : constant TemporalPayload := Decode_TemporalPayload
        (AMS_GRA_OMS_JSON_Guard.Write (Good));
      Again : constant TemporalPayload := Decode_TemporalPayload (Encode_TemporalPayload (Item));
   begin
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_TemporalPayload (Item)) = Good, "encode structural golden");
      Check (Encode_TemporalPayload (Again) = Encode_TemporalPayload (Item), "encode/decode/encode stability");
      Check (Length (Item.DirectDates) = 3, "DirectDates decode length");
      Check (Value (Element (Item.DirectDates, 1)) = "2024-02-29T12:34:56Z", "DirectDates 1 decode");
      Check (Value (Element (Item.DirectDates, 2)) = "2026-10-15T01:02:03+05:30", "DirectDates 2 decode");
      Check (Value (Element (Item.DirectDates, 3)) = "2026-10-15T01:02:03", "DirectDates 3 decode");
      Check (Length (Again.MoreDirectDates) = 2, "MoreDirectDates decode length");
      Check (Value (Element (Again.MoreDirectDates, 2)) = "2026-10-15T01:02:03", "MoreDirectDates 2 decode");
      Check (Length (Item.NamedDates) = 3, "NamedDates decode length");
      Check (Value (Element (Item.NamedDates, 3)) = "2027-01-01T00:00:00Z", "NamedDates 3 decode");
      Check (Value (Item.MoreNamedDates.Required (1)) = "2024-02-29T12:34:56Z", "MoreNamedDates 1 decode");
      Check (Value (Item.MoreNamedDates.Required (2)) = "2026-10-15T01:02:03Z", "MoreNamedDates 2 decode");
      Check (Length (Item.MoreNamedDates.Additional) = 1, "MoreNamedDates additional length");
      Check (Value (Element (Item.MoreNamedDates.Additional, 1)) = "2027-01-01T00:00:00Z", "MoreNamedDates 3 decode");
      Check (Length (Item.DirectSpans) = 3, "DirectSpans decode length");
      Check (Value (Element (Item.DirectSpans, 1)) = "P1D", "DirectSpans 1 decode");
      Check (Value (Element (Item.DirectSpans, 3)) = "P3M", "DirectSpans 3 decode");
      Check (Value (Item.MoreDirectSpans.Required (1)) = "P1D", "MoreDirectSpans 1 decode");
      Check (Value (Item.MoreDirectSpans.Required (2)) = "PT2S", "MoreDirectSpans 2 decode");
      Check (Length (Item.MoreDirectSpans.Additional) = 1, "MoreDirectSpans additional length");
      Check (Value (Element (Item.MoreDirectSpans.Additional, 1)) = "P3M", "MoreDirectSpans 3 decode");
      Check (Length (Item.NamedSpans) = 3, "NamedSpans decode length");
      Check (Value (Element (Item.NamedSpans, 2)) = "PT2S", "NamedSpans 2 decode");
      Check (Length (Item.MoreNamedSpans) = 3, "MoreNamedSpans decode length");
      Check (Value (Element (Item.MoreNamedSpans, 3)) = "P3M", "MoreNamedSpans 3 decode");
   end;
   declare
      File : Ada.Text_IO.File_Type;
      Text : Unbounded_String;
      Minimum : JSON_Value;
   begin
      Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (2));
      while not Ada.Text_IO.End_Of_File (File) loop
         Append (Text, Ada.Text_IO.Get_Line (File));
      end loop;
      Ada.Text_IO.Close (File);
      Minimum := AMS_GRA_OMS_JSON_Guard.Read (To_String (Text));
      declare
         Item : constant TemporalPayload := Decode_TemporalPayload
           (AMS_GRA_OMS_JSON_Guard.Write (Minimum));
         Explicit_Empty : JSON_Value := Clone (Minimum);
         Empty : JSON_Array;
      begin
         Check (Length (Item.DirectDates) = 0, "DirectDates minimum empty");
         Check (Length (Item.MoreDirectDates) = 0, "MoreDirectDates minimum empty");
         Check (Length (Item.NamedSpans) = 0, "NamedSpans minimum empty");
         Check (Length (Item.MoreNamedSpans) = 0, "MoreNamedSpans minimum empty");
         Check (Length (Item.NamedDates) = 1, "NamedDates minimum required");
         Check (Length (Item.DirectSpans) = 1, "DirectSpans minimum required");
         Check (Length (Item.MoreNamedDates.Additional) = 0, "MoreNamedDates minimum Additional empty");
         Check (Length (Item.MoreDirectSpans.Additional) = 0, "MoreDirectSpans minimum Additional empty");
         Check (Value (Item.MoreNamedDates.Required (2)) = "2026-10-15T01:02:03Z", "MoreNamedDates minimum index 2");
         Check (Value (Item.MoreDirectSpans.Required (2)) = "PT2S", "MoreDirectSpans minimum index 2");
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_TemporalPayload (Item)) = Minimum, "minimum structural golden");
         Explicit_Empty.Set_Field ("DirectDates", Create (Empty));
         Explicit_Empty.Set_Field ("MoreDirectDates", Create (Empty));
         Explicit_Empty.Set_Field ("NamedSpans", Create (Empty));
         Explicit_Empty.Set_Field ("MoreNamedSpans", Create (Empty));
         declare
            Other : constant TemporalPayload := Decode_TemporalPayload
              (AMS_GRA_OMS_JSON_Guard.Write (Explicit_Empty));
         begin
            Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_TemporalPayload (Other)) = Minimum, "empty arrays canonical omission");
         end;
      end;
   end;
   for Case_No in 1 .. 6 loop
      declare
         Bad : JSON_Value := Clone (Good);
         Empty : JSON_Array;
      begin
         case Case_No is
            when 1 => Bad.Set_Field ("DirectDates", "not array");
            when 2 => Bad.Set_Field ("DirectDates", Create);
            when 3 => Bad.Set_Field ("NamedDates", Create (Empty));
            when 4 => Bad.Unset_Field ("MoreNamedDates");
            when 5 =>
               for Index in 1 .. 4 loop Append (Empty, Create ("P1D")); end loop;
               Bad.Set_Field ("DirectSpans", Create (Empty));
            when 6 => Bad.Set_Field ("Unknown", True);
            when others => raise Program_Error;
         end case;
         Reject (Bad, "record negative" & Case_No'Image);
      end;
   end loop;
   Bad_Array ("DirectDates", Create, False);
   Bad_Array ("DirectDates", Create (Integer'(1)), False);
   Bad_Array ("DirectDates", Create ("invalid"), False);
   Bad_Array ("DirectDates", Create ("invalid"), True);
   Bad_Array ("DirectDates", Create ("2023-02-29T00:00:00Z"), True);
   Bad_Array ("MoreDirectDates", Create ("2024-02-29T00:00:00+14:01"), True);
   Bad_Array ("NamedDates", Create ("2024-02-29T00:00:00+01:00"), True);
   Bad_Array ("DirectSpans", Create ("P1S2D"), True);
   Bad_Array ("MoreDirectSpans", Create ("PT"), True);
   Bad_Array ("NamedSpans", Create ("P1W"), True);
   Bad_Array ("MoreNamedSpans", Create ("P-1D"), True);
   Ada.Text_IO.Put_Line ("TASK094_CODEC_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
end Repeated_Temporal_Codec_Consumer;