with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Programs.Oam; use Programs.Oam;
with Service_API;

procedure Model_Consumer is
   Present : constant AdaPayload :=
     (Enabled => True, Level => -7,
      Name => To_Unbounded_String ("Unicode " & Character'Val (195) & Character'Val (169)),
      Signal => Value_5G, Label => Create ("checked label"),
      Details => (Text => To_Unbounded_String ("nested record")),
      Field_begin => To_Unbounded_String ("wire name"),
      Note => (Is_Present => True, Value => To_Unbounded_String ("present note")),
      OptionalLabel => (Is_Present => True, Value => Create ("present label")));
   Absent : constant AdaPayload :=
     (Enabled => Present.Enabled, Level => Present.Level,
      Name => Present.Name, Signal => Present.Signal, Label => Present.Label,
      Details => Present.Details, Field_begin => Present.Field_begin,
      Note => (Is_Present => False), OptionalLabel => (Is_Present => False));
begin
   if Value (Present.Label) /= "checked label"
     or else Value (Present.OptionalLabel.Value) /= "present label"
     or else Absent.Note.Is_Present or else Absent.OptionalLabel.Is_Present
     or else Service_API.Function_Codec.Exchange_Echo.Message_Name /= "AdaEcho"
   then
      raise Program_Error with "model baseline failed";
   end if;
   Ada.Text_IO.Put_Line ("TASK087B_MODEL_BASELINE_PASS");
end Model_Consumer;