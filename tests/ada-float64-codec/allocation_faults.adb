package body Allocation_Faults is
   Active : Boolean := False;
   Remaining : Natural := 0;
   procedure Arm (After : Natural) is
   begin
      Remaining := After; Active := True;
   end Arm;
   procedure Disarm is
   begin
      Active := False;
   end Disarm;
   function Real_Allocate (Size : Interfaces.C.size_t) return System.Address
     with Import, Convention => C, External_Name => "__real___gnat_malloc";
   function Allocate (Size : Interfaces.C.size_t) return System.Address is
   begin
      if Active then
         if Remaining = 0 then
            --  Single-shot so exception propagation/cleanup can allocate.
            Active := False;
            raise Storage_Error with "Task097 injected output/input allocation failure";
         end if;
         Remaining := Remaining - 1;
      end if;
      return Real_Allocate (Size);
   end Allocate;
end Allocation_Faults;