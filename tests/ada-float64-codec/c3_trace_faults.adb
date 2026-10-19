package body Allocation_Faults is
   procedure Start (After : Interfaces.C.size_t) with Import, Convention => C,
     External_Name => "task097_c3_start";
   procedure Stop with Import, Convention => C, External_Name => "task097_c3_stop";
   function Count return Interfaces.C.size_t with Import, Convention => C,
     External_Name => "task097_c3_count";
   function C_Failures return Interfaces.C.size_t with Import, Convention => C,
     External_Name => "task097_c3_failures";
   function C_Failed_Size return Interfaces.C.size_t with Import, Convention => C,
     External_Name => "task097_c3_size";
   function Attempts return Natural is (Natural (Count));
   function Failures return Interfaces.C.size_t is (C_Failures);
   function Failed_Size return Interfaces.C.size_t is (C_Failed_Size);
   procedure Arm (After : Natural) is
   begin Start (Interfaces.C.size_t (After)); end Arm;
   procedure Disarm is
   begin Stop; end Disarm;
   function Real_Allocate (Size : Interfaces.C.size_t) return System.Address
     with Import, Convention => C, External_Name => "__real___gnat_malloc";
   function Allocate (Size : Interfaces.C.size_t) return System.Address is
   begin return Real_Allocate (Size); end Allocate;
end Allocation_Faults;