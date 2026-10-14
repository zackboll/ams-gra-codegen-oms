with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO;
with Interfaces;
with Programs.Oam; use Programs.Oam;
with Service_API;

procedure Repeated_Model_Consumer is
   use type Interfaces.Unsigned_64;
   Item : RepeatedPayload :=
     (Inherited => To_Sequence
        (RepeatedPayload_Inherited_Values'(1 => Create ("base"))),
      Field_begin => To_Sequence
        (RepeatedPayload_Field_begin_Values'
           (for Index in 1 .. 0 => Create ("never evaluated"))),
      Span => To_Sequence
        (RepeatedPayload_Span_Values'
           (for Index in 1 .. 2 => Create ("span" & Index'Image))),
      Pair => To_Sequence
        (RepeatedPayload_Pair_Values'
           (for Index in 1 .. 2 => (Label => Create ("pair" & Index'Image)))),
      RequiredLabels =>
        (Required =>
           (for Index in 1 .. 2 => Create ("required" & Index'Image)),
         Additional => <>),
      Signals => To_Sequence
        (RepeatedPayload_Signals_Values'(Value_5G, NORMAL)),
      Signed => To_Sequence
        (RepeatedPayload_Signed_Values'(-128, 127)),
      Unsigned => To_Sequence
        (RepeatedPayload_Unsigned_Values'(0, 255)),
      Words => <>, Flags => <>, Details => <>);
begin
   if Length (Item.Field_begin) /= 0 or else Length (Item.Words) /= 0
     or else Length (Item.RequiredLabels.Additional) /= 0
   then
      raise Program_Error with "empty storage contains phantom elements";
   end if;
   Append (Item.Field_begin, Create ("one"));
   Append (Item.Field_begin, Create ("two"));
   Append (Item.Field_begin, Create ("three"));
   Append (Item.Span, Create ("span third"));
   for Index in 1 .. 8 loop
      Append (Item.Words, To_Unbounded_String ("word" & Index'Image));
      Append (Item.RequiredLabels.Additional, Create ("additional" & Index'Image));
      Append (Item.Details, (Label => Create ("nested" & Index'Image)));
   end loop;
   Append (Item.Flags, True);
   Append (Item.Flags, False);
   if Length (Item.Field_begin) /= 3
     or else Value (Element (Item.Field_begin, 3)) /= "three"
     or else Length (Item.Span) /= 3
     or else Value (Element (Item.Span, 1)) /= "span 1"
     or else Value (Element (Item.Span, 3)) /= "span third"
     or else Length (Item.Pair) /= 2
     or else Value (Element (Item.Pair, 2).Label) /= "pair 2"
     or else Length (Item.Words) /= 8
     or else To_String (Element (Item.Words, 8)) /= "word 8"
     or else Value (Item.RequiredLabels.Required (1)) /= "required 1"
     or else Value (Item.RequiredLabels.Required (2)) /= "required 2"
     or else Length (Item.RequiredLabels.Additional) /= 8
     or else Value (Element (Item.RequiredLabels.Additional, 8)) /= "additional 8"
     or else Value (Element (Item.Details, 8).Label) /= "nested 8"
     or else Element (Item.Signals, 1) /= Value_5G
     or else Element (Item.Signed, 1) /= -128
     or else Element (Item.Unsigned, 2) /= 255
     or else not Element (Item.Flags, 1) or else Element (Item.Flags, 2)
     or else Value (Element (Item.Inherited, 1)) /= "base"
     or else Service_API.Function_Codec.Exchange_Echo.Message_Name /= "RepeatedEcho"
   then
      raise Program_Error with "repeated model baseline failed";
   end if;
   Ada.Text_IO.Put_Line ("TASK089_MODEL_BASELINE_PASS");
end Repeated_Model_Consumer;