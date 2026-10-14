with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Programs.Oam; use Programs.Oam;
with Service_API;
with Service_Codec; use Service_Codec;
procedure Choice_Consumer is
   Items : constant array (Positive range <>) of Selection :=
     ((Kind => Enabled_Kind, Enabled => True),
      (Kind => Number_Kind, Number => -7),
      (Kind => Alternative_begin_Kind, Alternative_begin => To_Unbounded_String ("text")),
      (Kind => Signal_Kind, Signal => Value_5G),
      (Kind => Checked_Kind, Checked => Create ("checked")),
      (Kind => Nested_Kind, Nested => (Text => To_Unbounded_String ("nested"))),
      (Kind => Other_Kind, Other => (Kind => Flag_Kind, Flag => False)));
   Passed : Natural := 0;
   File : File_Type;
   Text : Unbounded_String;
   procedure Check (OK : Boolean) is
   begin
      if not OK then raise Program_Error with "Choice check failed"; end if;
      Passed := Passed + 1;
   end Check;
   procedure Equal (Left, Right : Selection) is
      OK : Boolean := False;
   begin
      Check (Left.Kind = Right.Kind);
      case Left.Kind is
         when Enabled_Kind => OK := Left.Enabled = Right.Enabled;
         when Number_Kind => OK := Left.Number = Right.Number;
         when Alternative_begin_Kind => OK := Left.Alternative_begin = Right.Alternative_begin;
         when Signal_Kind => OK := Left.Signal = Right.Signal;
         when Checked_Kind => OK := Value (Left.Checked) = Value (Right.Checked);
         when Nested_Kind => OK := Left.Nested.Text = Right.Nested.Text;
         when Other_Kind => OK := Left.Other = Right.Other;
      end case;
      Check (OK);
   end Equal;
   procedure Reject (Bad : String) is
   begin
      declare
         Item : constant Selection := Decode_Selection (Bad);
      begin
         raise Program_Error with "accepted malformed Choice " & Item.Kind'Image;
      end;
   exception
      when Codec_Error => Check (True);
   end Reject;
begin
   Open (File, In_File, Ada.Command_Line.Argument (1));
   while not End_Of_File (File) loop Append (Text, Get_Line (File)); end loop;
   Close (File);
   declare
      Golden : constant JSON_Array := Get (AMS_GRA_OMS_JSON_Guard.Read (To_String (Text)));
   begin
      Check (Length (Golden) = Items'Length);
      for I in Items'Range loop
         Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_Selection (Items (I))) = Get (Golden, I));
         Equal (Items (I), Decode_Selection (AMS_GRA_OMS_JSON_Guard.Write (Get (Golden, I))));
         for Round in 1 .. 3 loop
            Equal (Items (I), Decode_Selection (Encode_Selection (Items (I))));
         end loop;
         declare
            Present : constant ChoicePayload :=
              (Pick => Items (I), Maybe => (Is_Present => True, Value => Items (I)));
            Absent : constant ChoicePayload := (Pick => Items (I), Maybe => (Is_Present => False));
            P : constant ChoicePayload := Decode_ChoicePayload (Encode_ChoicePayload (Present));
            A : constant ChoicePayload := Decode_ChoicePayload (Encode_ChoicePayload (Absent));
            Expected : constant JSON_Value := Create_Object;
         begin
            Expected.Set_Field ("Pick", Get (Golden, I));
            Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_ChoicePayload (Absent)) = Expected);
            Expected.Set_Field ("Maybe", Get (Golden, I));
            Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_ChoicePayload (Present)) = Expected);
            Check (P.Maybe.Is_Present and then not A.Maybe.Is_Present);
            Equal (Items (I), P.Pick); Equal (Items (I), P.Maybe.Value); Equal (Items (I), A.Pick);
            -- Independent reverse order, rather than provider serialization order.
            Equal (Items (I), Decode_ChoicePayload
              ("{""Maybe"":" & AMS_GRA_OMS_JSON_Guard.Write (Get (Golden, I))
               & ",""Pick"":" & AMS_GRA_OMS_JSON_Guard.Write (Get (Golden, I)) & "}").Pick);
         end;
      end loop;
   end;
   Equal (Items (1), Decode_Selection ("{""Enabled"":false,""Enabled"":true}"));
   Reject ("{}");
   Reject ("{""Enabled"":true,""Number"":1}");
   Reject ("{""Enabled"":true,""Unknown"":1}");
   Reject ("{""Unknown"":1}");
   Reject ("[]"); Reject ("true"); Reject ("1"); Reject ("null"); Reject ("""text""");
   Reject ("{""Enabled"":null}"); Reject ("{""Enabled"":""true""}");
   Reject ("{""Number"":2147483648}"); Reject ("{""Number"":-2147483649}");
   Reject ("{""Number"":9223372036854775808}");
   Reject ("{""Number"":1.0}"); Reject ("{""Number"":""7""}");
   Reject ("{""Checked"":""""}"); Reject ("{""Checked"":""\n""}");
   Reject ("{""Signal"":""Value_5G""}");
   Reject ("{""begin"":false}"); Reject ("{""begin"":""\uD800""}");
   Reject ("{""begin"":""" & Character'Val (255) & """}");
   Reject ("{""Nested"":{}}"); Reject ("{""Nested"":{""Text"":true}}");
   Reject ("{""Nested"":{""Text"":""ok"",""Unknown"":1}}");
   Reject ("{""Other"":{}}"); Reject ("{""Other"":{""Flag"":null}}");
   Reject ("{""$type"":""Selection""}"); Reject ("{");
   Put_Line ("TASK088_CODEC_COMPLETE cases=" & Passed'Image);
end Choice_Consumer;