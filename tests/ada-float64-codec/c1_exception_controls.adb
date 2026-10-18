with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Finalization;
with Ada.Text_IO;
with Allocation_Faults;
with Interfaces.C;
procedure C1_Exception_Controls is
   --  These controls must remain visible, not become resource classifications.
   type Control is new Ada.Finalization.Controlled with null record;
   Original : exception;
   overriding procedure Adjust (Item : in out Control) is
      pragma Unreferenced (Item);
   begin
      raise Original with "unrelated controlled exception";
   end Adjust;
   A, B : Control;
   Mode : constant String := Ada.Command_Line.Argument (1);
   function Count return Interfaces.C.size_t
     with Import, Convention => C, External_Name => "task097_c1_malloc_failures";
   use type Interfaces.C.size_t;
   Before : constant Interfaces.C.size_t := Count;
begin
   begin
      if Mode = "program" then
         raise Program_Error with "no allocation failure";
      elsif Mode = "lookalike" then
         raise Program_Error with "gnatcoll-json-utility.adb:152 finalize/adjust raised exception";
      elsif Mode = "controlled" then
         A := B;
      elsif Mode = "unrelated-resource" then
         raise Storage_Error with "unrelated operation";
      else
         raise Constraint_Error with "ordinary validation control";
      end if;
   exception
      when E : others =>
         Allocation_Faults.Disarm;
         if Count /= Before then raise Program_Error with "negative control unexpectedly allocated failure"; end if;
         Ada.Text_IO.Put_Line ("TASK097_C1_CONTROL " & Mode & " " & Ada.Exceptions.Exception_Name (E));
         Ada.Text_IO.Put_Line (Ada.Exceptions.Exception_Message (E));
   end;
end C1_Exception_Controls;