with Ada.Text_IO; use Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Programs.Oam; use Programs.Oam;
with Service_API;
procedure Choice_Model_Consumer is
   Items : constant array (Positive range <>) of Selection :=
     ((Kind => Enabled_Kind, Enabled => True),
      (Kind => Number_Kind, Number => -7),
      (Kind => Alternative_begin_Kind, Alternative_begin => To_Unbounded_String ("text")),
      (Kind => Signal_Kind, Signal => Value_5G),
      (Kind => Checked_Kind, Checked => Create ("checked")),
      (Kind => Nested_Kind, Nested => (Text => To_Unbounded_String ("nested"))),
      (Kind => Other_Kind, Other => (Kind => Flag_Kind, Flag => False)));
   Present : constant ChoicePayload :=
     (Pick => Items (1), Maybe => (Is_Present => True, Value => Items (3)));
   Absent : constant ChoicePayload :=
     (Pick => Items (7), Maybe => (Is_Present => False));
begin
   if Present.Pick.Kind /= Enabled_Kind
     or else Present.Maybe.Value.Kind /= Alternative_begin_Kind
     or else Absent.Maybe.Is_Present
     or else Absent.Pick.Other.Flag
   then
      raise Program_Error;
   end if;
   Put_Line ("TASK088_MODEL_COMPLETE cases= 7");
end Choice_Model_Consumer;