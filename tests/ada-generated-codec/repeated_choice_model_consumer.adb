with Ada.Text_IO;
with Programs.Oam;

procedure Repeated_Choice_Model_Consumer is
   package M renames Programs.Oam;
   use type M.Selection_Kind;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then
         raise Program_Error with "Task 092 model check failed";
      end if;
   end Check;
   Enabled : constant M.Selection := (Kind => M.Enabled_Kind, Enabled => True);
   State : constant M.Selection := (Kind => M.State_Kind, State => M.online);
   Bounded : M.RepeatedPayload_Bounded_Sequence :=
     M.To_Sequence (M.RepeatedPayload_Bounded_Values'(1 => Enabled, 2 => State));
   Positive_Bounded : constant M.RepeatedPayload_PositiveBounded_Sequence :=
     M.To_Sequence (M.RepeatedPayload_PositiveBounded_Values'(1 => State));
   Unbounded : M.RepeatedPayload_Unbounded_Sequence;
   Positive_Unbounded : M.RepeatedPayload_PositiveUnbounded_Sequence :=
     (Required => (1 => Enabled, 2 => State), Additional => <>);
begin
   Check (M.Length (Bounded) = 2);
   Check (M.Element (Bounded, 1).Kind = M.Enabled_Kind);
   Check (M.Element (Bounded, 2).Kind = M.State_Kind);
   M.Append (Bounded, Enabled);
   Check (M.Length (Bounded) = 3);
   Check (M.Length (Positive_Bounded) = 1);
   Check (M.Element (Positive_Bounded, 1).Kind = M.State_Kind);
   Check (M.Length (Unbounded) = 0);
   M.Append (Unbounded, State);
   M.Append (Unbounded, Enabled);
   Check (M.Element (Unbounded, 1).Kind = M.State_Kind);
   Check (M.Element (Unbounded, 2).Enabled);
   Check (Positive_Unbounded.Required (1).Enabled);
   Check (Positive_Unbounded.Required (2).Kind = M.State_Kind);
   M.Append (Positive_Unbounded.Additional, State);
   M.Append (Positive_Unbounded.Additional, Enabled);
   Check (M.Length (Positive_Unbounded.Additional) = 2);
   Check (M.Element (Positive_Unbounded.Additional, 1).Kind = M.State_Kind);
   Check (M.Element (Positive_Unbounded.Additional, 2).Enabled);
   Ada.Text_IO.Put_Line ("TASK092_MODEL_PASS checks=14");
end Repeated_Choice_Model_Consumer;