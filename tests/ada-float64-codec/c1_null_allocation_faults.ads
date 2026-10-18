with Interfaces.C;
with System;
package Allocation_Faults is
   procedure Arm (After : Natural);
   procedure Disarm;
   function Attempts return Natural;
   function Failures return Interfaces.C.size_t;
   function Failed_Size return Interfaces.C.size_t;
   function Allocate (Size : Interfaces.C.size_t) return System.Address
     with Export, Convention => C, External_Name => "__wrap___gnat_malloc";
end Allocation_Faults;