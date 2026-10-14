with Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Programs.Oam; use Programs.Oam;
with Service_API;
procedure Duration_Model_Consumer is
   Present : constant DurationPayload :=
     (InheritedDuration => Create ("P999999999999999999999999Y"),
      Named => Create ("P12M"), OtherNamed => Create ("P1Y"),
      OptionalNamed => (Is_Present => True, Value => Create ("-P0D")),
      DirectDuration => Create ("PT6.2500S"),
      OptionalDirect => (Is_Present => True, Value => Create ("-PT0.0S")),
      Details => (NestedDuration => Create ("P3DT4H5M6S"),
                  NestedDirect => Create ("PT0S")),
      Label => To_Unbounded_String ("duration control"));
   Absent : constant DurationPayload :=
     (InheritedDuration => Present.InheritedDuration,
      Named => Present.Named, OtherNamed => Present.OtherNamed,
      OptionalNamed => (Is_Present => False),
      DirectDuration => Present.DirectDuration,
      OptionalDirect => (Is_Present => False),
      Details => Present.Details, Label => Present.Label);
begin
   if Value (Present.Named) /= "P12M"
     or else Value (Present.OtherNamed) /= "P1Y"
     or else Value (Present.OptionalNamed.Value) /= "-P0D"
     or else Value (Present.DirectDuration) /= "PT6.2500S"
     or else Value (Present.OptionalDirect.Value) /= "-PT0.0S"
     or else Value (Present.InheritedDuration) /= "P999999999999999999999999Y"
     or else Value (Present.Details.NestedDuration) /= "P3DT4H5M6S"
     or else Value (Present.Details.NestedDirect) /= "PT0S"
     or else Absent.OptionalNamed.Is_Present
     or else Absent.OptionalDirect.Is_Present
   then
      raise Program_Error with "model baseline failed";
   end if;
   Ada.Text_IO.Put_Line ("TASK091_MODEL_COMPLETE cases= 10");
end Duration_Model_Consumer;