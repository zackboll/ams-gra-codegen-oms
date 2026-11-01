with Ada.Text_IO;

procedure Owned_Smoke is
   function Add (Left, Right : Integer) return Integer is (Left + Right);
   Raised : Boolean := False;
begin
   if Add (20, 22) /= 42 then
      raise Program_Error with "incorrect arithmetic";
   end if;
   Ada.Text_IO.Put_Line ("CI004_PASS checked-arithmetic");
   begin
      raise Constraint_Error with "mandatory negative case";
   exception
      when Constraint_Error => Raised := True;
   end;
   if not Raised then
      raise Program_Error with "failure was masked";
   end if;
   Ada.Text_IO.Put_Line ("CI004_PASS exception-propagation");
end Owned_Smoke;