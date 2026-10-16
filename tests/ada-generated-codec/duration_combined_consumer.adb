with Ada.Command_Line;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.OAM; use Programs.OAM;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
procedure Duration_Combined_Consumer is
   File : Ada.Text_IO.File_Type;
   Good : JSON_Value;
   Checks : Natural := 0;
   Rejections : Natural := 0;
   type Short_Keys is array (Positive range <>) of String (1 .. 9);
   Scalar_Keys : constant Short_Keys := ["Inherited", "Direct   ", "Optional "];
   type Row_Keys is array (Positive range <>) of String (1 .. 12);
   Repeated_Keys : constant Row_Keys := ["Rows        ", "BoundedRows ", "RequiredRows"];
   procedure Check (OK : Boolean; Context : String) is
   begin
      Checks := Checks + 1;
      if not OK then raise Program_Error with "TASK091R: " & Context; end if;
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
            Check (Item.Pick.Kind = Nested_Kind, "Validate/check-01: Item.Pick.Kind = Nested_Kind");
         end;
      exception
         when Codec_Error =>
            Check (not Published, "Decode/rejection: typed result must not be published");
            Rejections := Rejections + 1; return;
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
      Check (Value (Item.InheritedSpan) = "P001Y02M", "Validate/check-03: Value (Item.InheritedSpan) = ""P001Y02M""; actual=" & Value (Item.InheritedSpan));
      Check (Value (Again.DirectSpan) = "-PT0.000S", "Validate/check-04: Value (Again.DirectSpan) = ""-PT0.000S""; actual=" & Value (Again.DirectSpan));
      Check (Again.OptionalSpan.Is_Present and then Again.OptionalDirectSpan.Is_Present, "Validate/check-05: Again.OptionalSpan.Is_Present and then Again.OptionalDirectSpan.Is_Present");
      Check (Value (Again.OptionalSpan.Value) = "P1DT02H", "Validate/check-06: Value (Again.OptionalSpan.Value) = ""P1DT02H""; actual=" & Value (Again.OptionalSpan.Value));
      Check (Value (Again.OptionalDirectSpan.Value) = "PT01.2300S", "Validate/check-07: Value (Again.OptionalDirectSpan.Value) = ""PT01.2300S""; actual=" & Value (Again.OptionalDirectSpan.Value));
      Check (not Again.AbsentSpan.Is_Present and then not Again.AbsentDirectSpan.Is_Present, "Validate/check-08: not Again.AbsentSpan.Is_Present and then not Again.AbsentDirectSpan.Is_Present");
      Check (Value (Again.Child.Span) = "P02M", "Validate/check-09: Value (Again.Child.Span) = ""P02M""; actual=" & Value (Again.Child.Span));
      Check (Value (Again.Pick.Nested.DirectSpan) = "PT00.1200S", "Validate/check-10: Value (Again.Pick.Nested.DirectSpan) = ""PT00.1200S""; actual=" & Value (Again.Pick.Nested.DirectSpan));
      Check (Length (Again.Rows) = 2, "Validate/check-11: Length (Again.Rows) = 2");
      Check (Value (Element (Again.Rows, 1).Span) = "P02M", "Validate/check-12: Value (Element (Again.Rows, 1).Span) = ""P02M""; actual=" & Value (Element (Again.Rows, 1).Span));
      Check (Value (Element (Again.Rows, 2).Span) = "-P3D", "Validate/check-13: Value (Element (Again.Rows, 2).Span) = ""-P3D""; actual=" & Value (Element (Again.Rows, 2).Span));
      Check (Value (Element (Again.BoundedRows, 1).DirectSpan) = "PT00.1200S", "Validate/check-14: Value (Element (Again.BoundedRows, 1).DirectSpan) = ""PT00.1200S""; actual=" & Value (Element (Again.BoundedRows, 1).DirectSpan));
      Check (Value (Again.RequiredRows.Required (1).Span) = "P02M", "Validate/check-15: Value (Again.RequiredRows.Required (1).Span) = ""P02M""; actual=" & Value (Again.RequiredRows.Required (1).Span));
      Check (Again.Pick.Kind = Nested_Kind, "Validate/check-16: Again.Pick.Kind = Nested_Kind");
      Check (Value (Again.Inherited) = "2026-09-20T12:34:56.1200Z", "Validate/check-17: Value (Again.Inherited) = ""2026-09-20T12:34:56.1200Z""; actual=" & Value (Again.Inherited));
      Check (Value (Again.Direct) = "2026-09-20T12:34:56+05:30", "Validate/check-18: Value (Again.Direct) = ""2026-09-20T12:34:56+05:30""; actual=" & Value (Again.Direct));
      Check (Again.Optional.Is_Present and then Again.NamedOptional.Is_Present, "Validate/check-19: Again.Optional.Is_Present and then Again.NamedOptional.Is_Present");
      Check (Value (Again.Optional.Value) = "2026-09-20T12:34:56-13:30", "Validate/check-20: Value (Again.Optional.Value) = ""2026-09-20T12:34:56-13:30""; actual=" & Value (Again.Optional.Value));
      Check (Length (Again.Values) = 3, "Validate/check-21: Length (Again.Values) = 3");
      Check (Element (Again.Values, 1) = 9 and then Element (Again.Values, 2) = -2
             and then Element (Again.Values, 3) = 8, "Validate/check-22: Element (Again.Values, 1) = 9 and then Element (Again.Values, 2) = -2 and then Element (Again.Values, 3) = 8");
      Check (Value (Element (Again.Rows, 1).Local) = "2026-09-20T12:34:56-00:00", "Validate/check-23: Value (Element (Again.Rows, 1).Local) = ""2026-09-20T12:34:56-00:00""; actual=" & Value (Element (Again.Rows, 1).Local));
      Check (Value (Element (Again.BoundedRows, 1).Observed) = "2026-09-20T12:34:56Z", "Validate/check-24: Value (Element (Again.BoundedRows, 1).Observed) = ""2026-09-20T12:34:56Z""; actual=" & Value (Element (Again.BoundedRows, 1).Observed));
      Check (Value (Again.RequiredRows.Required (1).Observed) = "2026-09-20T12:34:56Z", "Validate/check-25: Value (Again.RequiredRows.Required (1).Observed) = ""2026-09-20T12:34:56Z""; actual=" & Value (Again.RequiredRows.Required (1).Observed));
      Check (Value (Element (Again.RequiredRows.Additional, 1).Local) = "2026-09-21T12:34:56+01:00", "Validate/check-26: Value (Element (Again.RequiredRows.Additional, 1).Local) = ""2026-09-21T12:34:56+01:00""; actual=" & Value (Element (Again.RequiredRows.Additional, 1).Local));
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Again)) = Good, "Validate/check-27: AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Again)) = Good");
   end;
   declare
      Absent : JSON_Value := Clone (Good);
   begin
      Check (Absent.Has_Field ("OptionalSpan") and then Absent.Has_Field ("OptionalDirectSpan"),
             "Validate/absent-setup: golden must contain both present Duration keys");
      Absent.Unset_Field ("OptionalSpan");
      Absent.Unset_Field ("OptionalDirectSpan");
      Check (not Absent.Has_Field ("OptionalSpan") and then not Absent.Has_Field ("OptionalDirectSpan"),
             "Decode/absent-setup: both Duration keys must be removed before decoding");
      Absent.Unset_Field ("Optional");
      Absent.Unset_Field ("NamedOptional");
      declare
         Item : constant CombinedPayload := Decode_CombinedPayload
           (AMS_GRA_OMS_JSON_Guard.Write (Absent));
      begin
         Check (not Item.OptionalSpan.Is_Present and then not Item.OptionalDirectSpan.Is_Present, "Decode/absent-duration: expected presence FALSE/FALSE; actual " & Boolean'Image (Item.OptionalSpan.Is_Present) & "/" & Boolean'Image (Item.OptionalDirectSpan.Is_Present));
         Check (not Item.Optional.Is_Present and then not Item.NamedOptional.Is_Present, "Validate/check-29: not Item.Optional.Is_Present and then not Item.NamedOptional.Is_Present");
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Item)) = Absent, "Validate/check-30: AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Item)) = Absent");
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
   for Key of Repeated_Keys loop
      for Wrong_Kind in Boolean loop
         declare
            Bad : JSON_Value := Clone (Good);
            Rows : JSON_Array;
            Child : JSON_Value := Clone (Good.Get ("Child"));
         begin
            Append (Rows, Clone (Child));
            if Wrong_Kind then Child.Set_Field ("DirectSpan", True);
            else Child.Set_Field ("Span", "P1W"); end if;
            Append (Rows, Child);
            Bad.Set_Field (Ada.Strings.Fixed.Trim (Key, Ada.Strings.Right), Create (Rows));
            Reject (Bad);
         end;
      end loop;
   end loop;
   declare
      Bad : JSON_Value := Clone (Good);
   begin
      Bad.Unset_Field ("InheritedSpan"); Reject (Bad);
   end;
   declare
      Bad : JSON_Value := Clone (Good);
   begin
      Bad.Set_Field ("DirectSpan", "PT"); Reject (Bad);
   end;
   declare
      Bad : JSON_Value := Clone (Good);
      Pick : JSON_Value := Bad.Get ("Pick");
      Child : JSON_Value := Pick.Get ("Nested");
   begin
      Child.Set_Field ("Span", "P-1D");
      Pick.Set_Field ("Nested", Child);
      Bad.Set_Field ("Pick", Pick); Reject (Bad);
   end;
   declare
      Other : JSON_Value := Clone (Good);
      Pick : JSON_Value := Create_Object;
   begin
      Pick.Set_Field ("Enabled", True); Other.Set_Field ("Pick", Pick);
      declare
         Item : constant CombinedPayload := Decode_CombinedPayload
           (AMS_GRA_OMS_JSON_Guard.Write (Other));
      begin
         Check (Item.Pick.Kind = Enabled_Kind and then Item.Pick.Enabled, "Validate/check-31: Item.Pick.Kind = Enabled_Kind and then Item.Pick.Enabled");
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Item)) = Other, "Validate/check-32: AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Item)) = Other");
      end;
   end;
   Ada.Text_IO.Put_Line ("TASK091R_COMBINED_PASS checks=" & Natural'Image (Checks)
                         & " rejections=" & Natural'Image (Rejections));
end Duration_Combined_Consumer;