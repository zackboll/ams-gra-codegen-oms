with Ada.Text_IO;
with Programs.Oam; use Programs.Oam;

procedure Repeated_Temporal_Model_Consumer is
   Date_A : constant String := "2024-02-29T12:34:56Z";
   Date_B : constant String := "2026-10-15T01:02:03Z";
   Item : TemporalPayload :=
     (DirectDates => <>, MoreDirectDates => <>,
      NamedDates => To_Sequence
        (TemporalPayload_NamedDates_Values'(1 => Create (Date_A))),
      MoreNamedDates =>
        (Required => (Create (Date_A), Create (Date_B)), Additional => <>),
      DirectSpans => To_Sequence
        (TemporalPayload_DirectSpans_Values'(1 => Create ("P1D"))),
      MoreDirectSpans =>
        (Required => (Create ("P1D"), Create ("PT2S")), Additional => <>),
      NamedSpans => <>, MoreNamedSpans => <>);
   Checks : Natural := 0;
   procedure Check (Condition : Boolean; Label : String) is
   begin
      if not Condition then
         raise Program_Error with Label;
      end if;
      Checks := Checks + 1;
   end Check;
begin
   Check (Length (Item.DirectDates) = 0, "DirectDates initial empty");
   Check (Length (Item.MoreDirectDates) = 0, "MoreDirectDates initial empty");
   Append (Item.DirectDates, Create (Date_A));
   Append (Item.DirectDates, Create (Date_B));
   Append (Item.MoreDirectDates, Create (Date_A));
   Append (Item.MoreDirectDates, Create (Date_B));
   Append (Item.NamedDates, Create (Date_B));
   Append (Item.MoreNamedDates.Additional, Create (Date_A));
   Append (Item.DirectSpans, Create ("PT2S"));
   Append (Item.MoreDirectSpans.Additional, Create ("P3M"));
   Append (Item.NamedSpans, Create ("P1D"));
   Append (Item.NamedSpans, Create ("PT2S"));
   Append (Item.MoreNamedSpans, Create ("P1D"));
   Append (Item.MoreNamedSpans, Create ("PT2S"));
   Check (Value (Element (Item.DirectDates, 1)) = Date_A, "DirectDates 1 read");
   Check (Value (Element (Item.DirectDates, 2)) = Date_B, "DirectDates 2 read");
   Check (Value (Element (Item.MoreDirectDates, 1)) = Date_A, "MoreDirectDates 1 read");
   Check (Value (Element (Item.MoreDirectDates, 2)) = Date_B, "MoreDirectDates 2 read");
   Check (Value (Element (Item.NamedDates, 1)) = Date_A, "NamedDates 1 read");
   Check (Value (Element (Item.NamedDates, 2)) = Date_B, "NamedDates 2 read");
   Check (Value (Item.MoreNamedDates.Required (1)) = Date_A, "MoreNamedDates 1 read");
   Check (Value (Item.MoreNamedDates.Required (2)) = Date_B, "MoreNamedDates 2 read");
   Check (Value (Element (Item.MoreNamedDates.Additional, 1)) = Date_A, "MoreNamedDates 3 read");
   Check (Value (Element (Item.DirectSpans, 1)) = "P1D", "DirectSpans 1 read");
   Check (Value (Element (Item.DirectSpans, 2)) = "PT2S", "DirectSpans 2 read");
   Check (Value (Item.MoreDirectSpans.Required (1)) = "P1D", "MoreDirectSpans 1 read");
   Check (Value (Item.MoreDirectSpans.Required (2)) = "PT2S", "MoreDirectSpans 2 read");
   Check (Value (Element (Item.MoreDirectSpans.Additional, 1)) = "P3M", "MoreDirectSpans 3 read");
   Check (Value (Element (Item.NamedSpans, 1)) = "P1D", "NamedSpans 1 read");
   Check (Value (Element (Item.NamedSpans, 2)) = "PT2S", "NamedSpans 2 read");
   Check (Value (Element (Item.MoreNamedSpans, 1)) = "P1D", "MoreNamedSpans 1 read");
   Check (Value (Element (Item.MoreNamedSpans, 2)) = "PT2S", "MoreNamedSpans 2 read");
   for Kind in 1 .. 4 loop
      begin
         case Kind is
            when 1 => Append (Item.DirectDates, Create ("2023-02-29T00:00:00Z"));
            when 2 => Append (Item.NamedDates, Create ("2024-02-29T00:00:00+01:00"));
            when 3 => Append (Item.DirectSpans, Create ("P1S2D"));
            when 4 => Append (Item.NamedSpans, Create ("invalid"));
            when others => raise Program_Error;
         end case;
         raise Program_Error with "invalid Create accepted" & Kind'Image;
      exception
         when Constraint_Error => Checks := Checks + 1;
      end;
   end loop;
   Ada.Text_IO.Put_Line ("TASK094_MODEL_COMPLETE checks=" & Checks'Image);
end Repeated_Temporal_Model_Consumer;