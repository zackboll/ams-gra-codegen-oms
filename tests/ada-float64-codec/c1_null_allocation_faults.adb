package body Allocation_Faults is
   --  Test process owns one thread of Ada operations. The C interposer is
   --  thread-local; Arm scopes exactly one real malloc, never a C exception.
   Active : Boolean := False;
   Remaining, Calls : Natural := 0;
   procedure Fail_Next with Import, Convention => C,
     External_Name => "task097_c1_fail_next_malloc";
   function C_Failures return Interfaces.C.size_t
     with Import, Convention => C, External_Name => "task097_c1_malloc_failures";
   function C_Failed_Size return Interfaces.C.size_t
     with Import, Convention => C, External_Name => "task097_c1_malloc_size";
   function Attempts return Natural is (Calls);
   function Failures return Interfaces.C.size_t is (C_Failures);
   function Failed_Size return Interfaces.C.size_t is (C_Failed_Size);
   procedure Arm (After : Natural) is
   begin
      Remaining := After; Calls := 0; Active := True;
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
         Calls := Calls + 1;
         if Remaining = 0 then
            Active := False;
            Fail_Next;
            return Real_Allocate (Size);
         end if;
         Remaining := Remaining - 1;
      end if;
      return Real_Allocate (Size);
   end Allocate;
end Allocation_Faults;