with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;

procedure Repeated_Choice_Temporal_Consumer is
   Good : JSON_Value;
   Checks, Rejections : Natural := 0;
   procedure Check (OK : Boolean; Label : String) is
   begin
      if not OK then raise Program_Error with Label; end if;
      Checks := Checks + 1;
   end Check;
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
      Check (Length (Item.Picks) = 3, "initial Choice sequence length");
      Check (Length (Again.Picks) = 3, "round-trip Choice sequence length");
      Check (First.Kind = Nested_Kind, "first discriminant");
      Check (Element (Again.Picks, 2).Kind = Enabled_Kind, "second discriminant");
      Check (not Element (Again.Picks, 2).Enabled, "second alternative value");
      Check (Last.Kind = Nested_Kind, "last discriminant");
      Check (Length (First.Nested.Dates) = 3, "nested DateTime length");
      Check (Value (Element (First.Nested.Dates, 1)) = "2024-02-29T12:34:56Z", "DateTime 1");
      Check (Value (Element (First.Nested.Dates, 2)) = "2026-10-15T01:02:03+05:30", "DateTime 2");
      Check (Value (Element (First.Nested.Dates, 3)) = "2026-10-15T01:02:03", "DateTime 3");
      Check (Length (First.Nested.Spans) = 3, "nested Duration length");
      Check (Value (Element (First.Nested.Spans, 1)) = "P1D", "Duration 1");
      Check (Value (Element (First.Nested.Spans, 2)) = "PT01.2300S", "Duration 2 lexical preservation");
      Check (Value (Element (First.Nested.Spans, 3)) = "-P3M", "Duration 3");
      Check (Length (Last.Nested.Dates) = 0, "empty DateTime sequence");
      Check (Length (Last.Nested.Spans) = 0, "empty Duration sequence");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_Combined (Item)) = Good, "independent canonical JSON");
      Check (Encode_Combined (Again) = Encode_Combined (Item), "round-trip canonical stability");
   end;
   declare
      Explicit_Empty : JSON_Value := Clone (Good);
      Picks : JSON_Array := Get (Explicit_Empty.Get ("Picks"));
      Pick, Nested : JSON_Value := Create_Object;
      Empty : JSON_Array;
   begin
      Nested.Set_Field ("Dates", Create (Empty));
      Nested.Set_Field ("Spans", Create (Empty));
      Pick.Set_Field ("Nested", Nested);
      Set_Element (Picks, 3, Pick);
      Explicit_Empty.Set_Field ("Picks", Create (Picks));
      declare
         Item : constant Combined := Decode_Combined (AMS_GRA_OMS_JSON_Guard.Write (Explicit_Empty));
      begin
         Check (Length (Element (Item.Picks, 3).Nested.Dates) = 0, "explicit empty DateTime");
         Check (Length (Element (Item.Picks, 3).Nested.Spans) = 0, "explicit empty Duration");
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_Combined (Item)) = Good, "canonical empty omission");
      end;
   end;
   declare
      Item : constant Combined := Decode_Combined ("{}");
   begin
      Check (Length (Item.Picks) = 0, "empty outer Choice sequence");
   end;
   for Case_No in 1 .. 2 loop
      declare
         Bad : JSON_Value := Clone (Good);
         Picks : JSON_Array := Get (Bad.Get ("Picks"));
         Pick, Nested : JSON_Value := Create_Object;
         Values : JSON_Array;
         Published : Boolean := False;
         Key : constant String := (if Case_No = 1 then "Dates" else "Spans");
      begin
         Append (Values, Create (if Case_No = 1 then "2024-02-29T12:34:56Z" else "P1D"));
         Append (Values, Create (if Case_No = 1 then "2023-02-29T00:00:00Z" else "P1W"));
         Nested.Set_Field (Key, Create (Values));
         Pick.Set_Field ("Nested", Nested);
         Set_Element (Picks, 3, Pick);
         Bad.Set_Field ("Picks", Create (Picks));
         begin
            declare
               Item : constant Combined := Decode_Combined (AMS_GRA_OMS_JSON_Guard.Write (Bad));
            begin
               Published := True;
               raise Program_Error with "invalid later temporal published" & Length (Item.Picks)'Image;
            end;
         exception
            when Error : Codec_Error =>
               Check (not Published, "failure-atomic decode");
               Check (Ada.Strings.Fixed.Index (Ada.Exceptions.Exception_Message (Error), "Picks item 3") > 0,
                      "outer Choice index attribution");
               Rejections := Rejections + 1;
         end;
      end;
   end loop;
   Ada.Text_IO.Put_Line ("TASK094_COMBINED_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
end Repeated_Choice_Temporal_Consumer;