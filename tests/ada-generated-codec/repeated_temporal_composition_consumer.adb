with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
procedure Repeated_Temporal_Composition_Consumer is
   Good : JSON_Value;
   Checks : Natural := 0;
   Rejections : Natural := 0;
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
      Item : constant Composition := Decode_Composition (AMS_GRA_OMS_JSON_Guard.Write (Good));
      Again : constant Composition := Decode_Composition (Encode_Composition (Item));
   begin
      Check (Length (Item.InheritedDates) = 2, "InheritedDates length");
      Check (Value (Element (Item.InheritedDates, 2)) = "2026-10-15T01:02:03Z", "InheritedDates 2 decode");
      Check (Value (Item.Date) = "2026-10-15T01:02:03+05:30", "single Date unchanged");
      Check (Value (Item.Duration) = "PT01.2300S", "single Duration lexical unchanged");
      Check (Length (Item.Child.Dates) = 2, "Child Dates length");
      Check (Value (Element (Item.Child.Dates, 2)) = "2024-02-29T12:34:56Z", "Child Dates 2 order");
      Check (Value (Item.Child.Spans.Required (2)) = "PT2S", "Child Spans 2 required");
      Check (Value (Element (Item.Child.Spans.Additional, 1)) = "P3M", "Child Spans 3 additional");
      Check (Item.Pick.Kind = Nested_Kind, "Pick discriminant");
      Check (Value (Element (Item.Pick.Nested.Dates, 1)) = "2024-02-29T12:34:56Z", "Pick Nested Dates 1");
      Check (Value (Item.Pick.Nested.Spans.Required (2)) = "PT2S", "Pick Nested Spans 2");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_Composition (Item)) = Good, "composition independent golden");
      Check (Encode_Composition (Again) = Encode_Composition (Item), "composition encode stability");
   end;
   for Case_No in 1 .. 3 loop
      declare
         Bad : JSON_Value := Clone (Good);
         Child : JSON_Value := Clone (Good.Get ("Child"));
         Values : JSON_Array;
         Published : Boolean := False;
      begin
         Append (Values, Create ("2024-02-29T12:34:56Z"));
         Append (Values, Create ("invalid"));
         if Case_No = 1 then
            Bad.Set_Field ("InheritedDates", Create (Values));
         elsif Case_No = 2 then
            Child.Set_Field ("Dates", Create (Values));
            Bad.Set_Field ("Child", Child);
         else
            declare
               Pick : JSON_Value := Create_Object;
            begin
               Child.Set_Field ("Dates", Create (Values));
               Pick.Set_Field ("Nested", Child);
               Bad.Set_Field ("Pick", Pick);
            end;
         end if;
         begin
            declare
               Item : constant Composition := Decode_Composition (AMS_GRA_OMS_JSON_Guard.Write (Bad));
            begin
               Published := True;
               Check (Item.Pick.Kind = Nested_Kind, "malformed result unexpectedly published");
            end;
            raise Program_Error with "invalid later temporal accepted";
         exception
            when Error : Codec_Error =>
               Check (not Published, "composition rejection atomic" & Case_No'Image);
               Check (Ada.Strings.Fixed.Index (Ada.Exceptions.Exception_Message (Error), "item 2") > 0,
                      "composition later element indexed" & Case_No'Image);
               Rejections := Rejections + 1;
         end;
      end;
   end loop;
   Ada.Text_IO.Put_Line ("TASK094_COMPOSITION_COMPLETE checks=" & Checks'Image & " rejections=" & Rejections'Image);
end Repeated_Temporal_Composition_Consumer;