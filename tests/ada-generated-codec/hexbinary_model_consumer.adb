with Ada.Text_IO; use Ada.Text_IO;
with Interfaces; use type Interfaces.Unsigned_8;
with Programs.Oam; use Programs.Oam;
procedure Hexbinary_Model_Consumer is
   Passed : Natural := 0;
   Octets : Binary_Vectors.Vector;
   procedure Check (Condition : Boolean; Label_Text : String) is
   begin
      if not Condition then raise Program_Error with Label_Text; end if;
      Passed := Passed + 1;
   end Check;
   procedure Reject_Exact is
   begin
      declare
         Item : constant Exact := Create (Octets);
      begin
         raise Program_Error with "unchecked exact length" & Value (Item).Length'Image;
      end;
   exception
      when Constraint_Error => Check (True, "checked exact length");
   end Reject_Exact;
begin
   declare
      Item : constant Bytes := (Value => Octets);
      Empty : constant Zero := Create (Octets);
   begin
      Check (Natural (Item.Value.Length) = 0, "unconstrained record API");
      Check (Natural (Value (Empty).Length) = 0, "zero constructor");
   end;
   Reject_Exact;
   Octets.Append (0); Octets.Append (16#80#); Octets.Append (16#FF#);
   declare
      Item : constant Exact := Create (Octets);
      Child : constant Leaf := Create (Octets);
   begin
      Check (Value (Item).Element (1) = 16#80#, "exact octets");
      Check (Natural (Value (Child).Length) = 3, "ancestry constructor");
   end;
   Octets.Append (0); Reject_Exact;
   Put_Line ("TASK093_MODEL_COMPLETE cases=" & Natural'Image (Passed));
end Hexbinary_Model_Consumer;