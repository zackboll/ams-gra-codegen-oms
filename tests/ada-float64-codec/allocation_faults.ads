with Interfaces.C;
with System;
package Allocation_Faults is
   procedure Arm (After : Natural);
   procedure Disarm;
   function Allocate (Size : Interfaces.C.size_t) return System.Address
     with Export, Convention => C, External_Name => "__wrap___gnat_malloc";
end Allocation_Faults;