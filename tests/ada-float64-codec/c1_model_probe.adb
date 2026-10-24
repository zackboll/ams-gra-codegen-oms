with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Text_IO;
with Allocation_Faults;
with Interfaces;
with Programs.Oam;
procedure C1_Model_Probe is
   --  No GNATCOLL, parser, emitter, C bridge, or JSON operation is called.
   --  Construct checked model values and expected state before injection.
   package Model renames Programs.Oam;
   R : Model.PathLength :=
     (Inherited => 0.0, Direct => 0.0, Optional => (Is_Present => False), Named => 0.0,
      Checked => Model.Create (Interfaces.IEEE_Float_64'(0.0)),
      Data => Model.Binary_Vectors.Empty_Vector, Single => 0.0, Samples => <>,
      NamedSamples => (Required => (1 => Model.Create (Interfaces.IEEE_Float_64'(0.0))),
                       Additional => <>),
      Unbounded => <>, NamedBounded => <>, Positive => (Is_Present => False),
      Text => <>, Digest => (Is_Present => False), Mode => (Is_Present => False), Date => (Is_Present => False),
      Span => (Is_Present => False), Count => (Is_Present => False));
   Picks : Model.LosslessEnvelope_Picks_Sequence :=
     Model.To_Sequence ((1 => (Kind => Model.Nested_Kind, Nested => R)));
   Output : Model.ArraySample := (Kind => Model.Enabled_Kind, Enabled => False);
   Budget : constant Natural := Natural'Value (Ada.Command_Line.Argument (1));
   use type Interfaces.Unsigned_8;
   use type Model.ArraySample_Kind;
begin
   Model.Binary_Vectors.Append (R.Data, 16#80#);
   Model.Binary_Vectors.Append (R.Data, 16#FF#);
   Picks := Model.To_Sequence ((1 => (Kind => Model.Nested_Kind, Nested => R)));
   Allocation_Faults.Arm (Budget);
   begin
      Output := Model.Element (Picks, 1);
      Allocation_Faults.Disarm;
      Ada.Text_IO.Put_Line ("TASK097_C1_MODEL_SUCCESS");
   exception
      when E : others =>
         Allocation_Faults.Disarm;
         Ada.Text_IO.Put_Line ("TASK097_C1_MODEL_EXCEPTION " & Ada.Exceptions.Exception_Name (E));
         Ada.Text_IO.Put_Line (Ada.Exceptions.Exception_Message (E));
         if Output.Kind /= Model.Enabled_Kind or else Output.Enabled then
            raise Program_Error with "model partial publication";
         end if;
   end;
   Output := Model.Element (Picks, 1);
   if Output.Nested.Data.Element (0) /= 16#80# or else
     Output.Nested.Data.Element (1) /= 16#FF#
   then raise Program_Error with "model recovery mismatch"; end if;
   Ada.Text_IO.Put_Line ("TASK097_C1_MODEL_RECOVERY_COMPLETE");
end C1_Model_Probe;