with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.OAM; use Programs.OAM;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
procedure Repeated_Choice_Consumer is
   File : Ada.Text_IO.File_Type;
   Good : JSON_Value;
   Checks, Rejections : Natural := 0;
   procedure Check (OK : Boolean) is
   begin
      if not OK then raise Program_Error with "Task 092 check"; end if;
      Checks := Checks + 1;
   end Check;
   procedure Reject (Bad : JSON_Value; Indexed : Boolean := False) is
      Published : Boolean := False;
   begin
      begin
         declare
            Item : constant RepeatedPayload := Decode_RepeatedPayload
              (AMS_GRA_OMS_JSON_Guard.Write (Bad));
         begin
            Published := True;
            Check (Item.Single.Kind = Enabled_Kind);
         end;
      exception
         when Error : Codec_Error =>
            Check (not Published);
            if Indexed then
               Check (Ada.Strings.Fixed.Index
                 (Ada.Exceptions.Exception_Message (Error), "Bounded item 2") > 0);
            end if;
            Rejections := Rejections + 1;
            return;
      end;
      raise Program_Error with "malformed result published";
   end Reject;
begin
   Ada.Text_IO.Open (File, Ada.Text_IO.In_File, Ada.Command_Line.Argument (1));
   declare
      Text : Ada.Strings.Unbounded.Unbounded_String;
   begin
      while not Ada.Text_IO.End_Of_File (File) loop
         Ada.Strings.Unbounded.Append (Text, Ada.Text_IO.Get_Line (File));
      end loop;
      Good := AMS_GRA_OMS_JSON_Guard.Read
        (Ada.Strings.Unbounded.To_String (Text)).Get ("RepeatedNotice");
   end;
   Ada.Text_IO.Close (File);
   declare
      Item : constant RepeatedPayload := Decode_RepeatedPayload
        (AMS_GRA_OMS_JSON_Guard.Write (Good));
      Again : constant RepeatedPayload := Decode_RepeatedPayload
        (Encode_RepeatedPayload (Item));
   begin
      Check (Length (Again.Bounded) = 3);
      Check (Element (Again.Bounded, 1).Kind = Enabled_Kind);
      Check (Element (Again.Bounded, 1).Enabled);
      Check (Element (Again.Bounded, 2).Kind = Nested_Kind);
      Check (Value (Element (Again.Bounded, 2).Nested.Observed) = "2026-01-02T03:04:05Z");
      Check (Value (Element (Again.Bounded, 2).Nested.Elapsed) = "PT2S");
      Check (Element (Again.Bounded, 3).State = online);
      Check (Element (Again.PositiveBounded, 1).State = offline);
      Check (not Element (Again.PositiveBounded, 2).Enabled);
      Check (Length (Again.Unbounded) = 3);
      Check (Element (Again.Unbounded, 3).State = offline);
      Check (Again.PositiveUnbounded.Required (1).Enabled);
      Check (Again.PositiveUnbounded.Required (2).State = online);
      Check (Length (Again.PositiveUnbounded.Additional) = 1);
      Check (not Element (Again.PositiveUnbounded.Additional, 1).Enabled);
      Check (not Again.Single.Enabled);
      Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_RepeatedPayload (Again)) = Good);
   end;
   for Count in 0 .. 3 loop
      declare
         Other : JSON_Value := Clone (Good);
         Items : JSON_Array;
         Original : constant JSON_Array := Get (Good.Get ("Bounded"));
      begin
         for Index in 1 .. Count loop Append (Items, Get (Original, Index)); end loop;
         Other.Set_Field ("Bounded", Create (Items));
         Other.Set_Field ("Unbounded", Create (Items));
         if Count > 0 then Other.Set_Field ("PositiveBounded", Create (Items)); end if;
         declare
            Item : constant RepeatedPayload := Decode_RepeatedPayload
              (AMS_GRA_OMS_JSON_Guard.Write (Other));
            Encoded : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read
              (Encode_RepeatedPayload (Item));
         begin
            Check (Length (Item.Bounded) = Count);
            Check (Length (Item.Unbounded) = Count);
            if Count = 0 then
               Check (not Encoded.Has_Field ("Bounded"));
               Check (not Encoded.Has_Field ("Unbounded"));
            else
               Check (Length (Item.PositiveBounded) = Count);
               Check (Encoded = Other);
            end if;
         end;
      end;
   end loop;
   declare
      Absent : JSON_Value := Clone (Good);
      Required_Items : JSON_Array;
   begin
      Absent.Unset_Field ("Bounded");
      Absent.Unset_Field ("Unbounded");
      Append (Required_Items, Get (Get (Good.Get ("PositiveUnbounded")), 1));
      Append (Required_Items, Get (Get (Good.Get ("PositiveUnbounded")), 2));
      Absent.Set_Field ("PositiveUnbounded", Create (Required_Items));
      declare
         Item : constant RepeatedPayload := Decode_RepeatedPayload
           (AMS_GRA_OMS_JSON_Guard.Write (Absent));
      begin
         Check (Length (Item.Bounded) = 0);
         Check (Length (Item.Unbounded) = 0);
         Check (Length (Item.PositiveUnbounded.Additional) = 0);
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_RepeatedPayload (Item)) = Absent);
      end;
   end;
   for Case_Number in 1 .. 17 loop
      declare
         Bad : JSON_Value := Clone (Good);
         Items : JSON_Array;
         Child : JSON_Value := Create_Object;
      begin
         case Case_Number is
            when 1 => Bad.Set_Field ("Bounded", True);
            when 2 => Bad.Set_Field ("Bounded", Create_Object);
            when 3 => Bad.Set_Field ("Bounded", JSON_Null);
            when 4 .. 12 =>
               Append (Items, Get (Get (Good.Get ("Bounded")), 1));
               case Case_Number is
                  when 4 => Child := JSON_Null;
                  when 5 => null;
                  when 6 => Child.Set_Field ("Enabled", True); Child.Set_Field ("State", "online");
                  when 7 => Child.Set_Field ("Unknown", True);
                  when 8 => Child.Set_Field ("Enabled", "true");
                  when 9 => Child.Set_Field ("Nested", Create_Object);
                  when 10 .. 12 =>
                     Child := Clone (Get (Get (Good.Get ("Bounded")), 2));
                     declare
                        Nested : JSON_Value := Child.Get ("Nested");
                     begin
                        if Case_Number = 10 then Nested.Set_Field ("Observed", "bad");
                        elsif Case_Number = 11 then Nested.Set_Field ("Elapsed", "P1W");
                        else Nested.Set_Field ("Elapsed", True); end if;
                        Child.Set_Field ("Nested", Nested);
                     end;
                  when others => raise Program_Error;
               end case;
               Append (Items, Child); Bad.Set_Field ("Bounded", Create (Items));
            when 13 => Bad.Unset_Field ("PositiveBounded");
            when 14 => Bad.Unset_Field ("PositiveUnbounded");
            when 15 => Bad.Set_Field ("PositiveBounded", Create (Items));
            when 16 =>
               Append (Items, Get (Get (Good.Get ("Bounded")), 1));
               Bad.Set_Field ("PositiveUnbounded", Create (Items));
            when 17 =>
               for Index in 1 .. 4 loop Append (Items, Get (Get (Good.Get ("Bounded")), 1)); end loop;
               Bad.Set_Field ("Bounded", Create (Items));
            when others => raise Program_Error;
         end case;
         Reject (Bad, Case_Number in 4 .. 12);
      end;
   end loop;
   Ada.Text_IO.Put_Line ("TASK092_CODEC_PASS checks=" & Checks'Image
                        & " rejections=" & Rejections'Image);
end Repeated_Choice_Consumer;