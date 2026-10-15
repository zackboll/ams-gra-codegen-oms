with Ada.Command_Line;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.OAM; use Programs.OAM;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
procedure Datetime_Combined_Consumer is
   File : Ada.Text_IO.File_Type;
   Good : JSON_Value;
   type Short_Keys is array (Positive range <>) of String (1 .. 9);
   Scalar_Keys : constant Short_Keys := ["Inherited", "Direct   ", "Optional "];
   type Row_Keys is array (Positive range <>) of String (1 .. 12);
   Repeated_Keys : constant Row_Keys := ["Rows        ", "BoundedRows ", "RequiredRows"];
   procedure Check (OK : Boolean) is
   begin
      if not OK then raise Program_Error with "DateTime integration regression"; end if;
   end Check;
   procedure Reject (Bad : JSON_Value) is
      Published : Boolean := False;
   begin
      begin
         declare
            Item : constant CombinedPayload := Decode_CombinedPayload
              (AMS_GRA_OMS_JSON_Guard.Write (Bad));
         begin
            Published := True;
            Check (Item.Pick.Kind = Nested_Kind);
         end;
      exception
         when Codec_Error => Check (not Published); return;
      end;
      raise Program_Error with "published malformed typed value";
   end Reject;
begin
   Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   declare
      Text : Ada.Strings.Unbounded.Unbounded_String;
   begin
      while not Ada.Text_IO.End_Of_File (File) loop
         Ada.Strings.Unbounded.Append (Text, Ada.Text_IO.Get_Line (File));
      end loop;
      Good := AMS_GRA_OMS_JSON_Guard.Read (Ada.Strings.Unbounded.To_String (Text));
   end;
   Ada.Text_IO.Close (File);
   declare
      Item : constant CombinedPayload := Decode_CombinedPayload
        (AMS_GRA_OMS_JSON_Guard.Write (Good));
      Again : constant CombinedPayload := Decode_CombinedPayload (Encode_CombinedPayload (Item));
   begin
      Check (Again.Pick.Kind = Nested_Kind);
      Check (Value (Again.Inherited) = "2026-09-20T12:34:56.1200Z");
      Check (Value (Again.Direct) = "2026-09-20T12:34:56+05:30");
      Check (Again.Optional.Is_Present and then Again.NamedOptional.Is_Present);
      Check (Value (Again.Optional.Value) = "2026-09-20T12:34:56-13:30");
      Check (Length (Again.Values) = 3);
      Check (Element (Again.Values, 1) = 9 and then Element (Again.Values, 2) = -2
             and then Element (Again.Values, 3) = 8);
      Check (Value (Element (Again.Rows, 1).Local) = "2026-09-20T12:34:56-00:00");
      Check (Value (Element (Again.BoundedRows, 1).Observed) = "2026-09-20T12:34:56Z");
      Check (Value (Again.RequiredRows.Required (1).Observed) = "2026-09-20T12:34:56Z");
      Check (Value (Element (Again.RequiredRows.Additional, 1).Local) = "2026-09-21T12:34:56+01:00");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Again)) = Good);
   end;
   declare
      Absent : JSON_Value := Clone (Good);
   begin
      Absent.Unset_Field ("Optional");
      Absent.Unset_Field ("NamedOptional");
      declare
         Item : constant CombinedPayload := Decode_CombinedPayload
           (AMS_GRA_OMS_JSON_Guard.Write (Absent));
      begin
         Check (not Item.Optional.Is_Present and then not Item.NamedOptional.Is_Present);
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Item)) = Absent);
      end;
   end;
   declare
      Bad : JSON_Value := Clone (Good);
   begin
      Bad.Set_Field ("Unknown", True); Reject (Bad);
   end;
   for Key of Scalar_Keys loop
      declare
         Bad : JSON_Value := Clone (Good);
      begin
         Bad.Set_Field (Ada.Strings.Fixed.Trim (Key, Ada.Strings.Right), "not-a-date");
         Reject (Bad);
      end;
   end loop;
   declare
      Bad : JSON_Value := Clone (Good);
   begin
      Bad.Set_Field ("Inherited", "2026-09-20T12:34:56+00:00"); Reject (Bad);
   end;
   declare
      Bad : JSON_Value := Clone (Good);
      Child : JSON_Value := Bad.Get ("Child");
   begin
      Child.Set_Field ("Observed", "2026-02-30T00:00:00Z");
      Bad.Set_Field ("Child", Child); Reject (Bad);
   end;
   declare
      Bad : JSON_Value := Clone (Good);
      Pick : JSON_Value := Bad.Get ("Pick");
      Child : JSON_Value := Pick.Get ("Nested");
   begin
      Child.Set_Field ("Local", "2026-09-20T12:34:56+14:01");
      Pick.Set_Field ("Nested", Child);
      Bad.Set_Field ("Pick", Pick); Reject (Bad);
   end;
   declare
      Bad : JSON_Value := Clone (Good);
      Numbers : JSON_Array;
   begin
      Append (Numbers, Create (Integer'(1)));
      Append (Numbers, Create (Integer'(2)));
      Append (Numbers, Create (Integer'(128)));
      Bad.Set_Field ("Values", Create (Numbers)); Reject (Bad);
   end;
   for Key of Repeated_Keys loop
      declare
         Bad : JSON_Value := Clone (Good);
         Rows : JSON_Array;
         Child : JSON_Value := Clone (Good.Get ("Child"));
      begin
         Append (Rows, Clone (Child));
         Child.Set_Field ("Local", "2026-09-20T12:34:56+14:01");
         Append (Rows, Child);
         Bad.Set_Field (Ada.Strings.Fixed.Trim (Key, Ada.Strings.Right), Create (Rows));
         Reject (Bad);
      end;
   end loop;
   Ada.Text_IO.Put_Line ("TASK090R_COMBINED_PASS");
end Datetime_Combined_Consumer;