with Ada.Text_IO;
with Interfaces;
with Interfaces.C;
with Allocation_Faults;
with Programs.Oam.Codec_Views;
procedure C3_Views is
   package M renames Programs.Oam;
   package V renames Programs.Oam.Codec_Views;
   use type Interfaces.Unsigned_8;
   use type Interfaces.C.size_t;
   Bytes : M.Binary_Vectors.Vector;
   Empty : M.Binary_Vectors.Vector;
   Checks : Natural := 0;
   Internal_Error : exception;
   procedure Check (B : Boolean) is
   begin
      if not B then raise Constraint_Error with "C3 view check"; end if;
      Checks := Checks + 1;
   end Check;
   procedure Read (Octets : in M.Binary_Vectors.Vector) is
   begin
      Check (Natural (Octets.Length) = 2);
      Check (Octets.Element (0) = 16#80# and Octets.Element (1) = 16#FF#);
   end Read;
   procedure Read_Empty (Octets : in M.Binary_Vectors.Vector) is
   begin Check (Octets.Is_Empty); end Read_Empty;
   procedure Resource (Octets : in M.Binary_Vectors.Vector) is
   begin Read (Octets); raise Storage_Error; end Resource;
   procedure Internal (Octets : in M.Binary_Vectors.Vector) is
   begin Read (Octets); raise Program_Error; end Internal;
   type Buffer is array (1 .. 64) of Interfaces.Unsigned_8;
   type Pointer is access Buffer;
   Retained : Pointer := null;
   procedure Allocate (Octets : in M.Binary_Vectors.Vector) is
   begin Read (Octets); Retained := new Buffer'(others => 0); end Allocate;
begin
   Bytes.Append (16#80#); Bytes.Append (16#FF#);
   declare
      D : constant M.Digest := M.Create (Bytes);
      E : constant M.EmptyDigest := M.Create (Empty);
      X : constant M.ExactDigest := M.Create (Bytes);
      Sequence : M.Leaf_Digests_Sequence;
      procedure Nested (Element : in M.Leaf_Digests_Item) is
      begin V.Visit_Binary (Element, Read'Access); end Nested;
   begin
      M.Append (Sequence, D); M.Append (Sequence, D);
      Allocation_Faults.Arm (0);
      V.Visit_Binary (D, Read'Access);
      V.Visit_Binary (D, Read'Access);
      V.Visit_Binary (E, Read_Empty'Access);
      V.Visit_Binary (X, Read'Access);
      V.Visit_Element (Sequence, 1, Nested'Access);
      V.Visit_Element (Sequence, 2, Nested'Access);
      Allocation_Faults.Disarm;
      Check (Allocation_Faults.Attempts = 0);
      begin V.Visit_Binary (D, Resource'Access); raise Internal_Error;
      exception when Storage_Error => null; end;
      V.Visit_Binary (D, Read'Access);
      begin V.Visit_Binary (D, Internal'Access); raise Internal_Error;
      exception when Program_Error => null; end;
      V.Visit_Binary (D, Read'Access);
      Allocation_Faults.Arm (0);
      begin V.Visit_Binary (D, Allocate'Access); raise Internal_Error;
      exception when Storage_Error => Allocation_Faults.Disarm; end;
      Check (Retained = null and Allocation_Faults.Attempts = 1);
      Check (Allocation_Faults.Failures = 1 and Allocation_Faults.Failed_Size = 64);
      V.Visit_Binary (D, Read'Access);
      V.Visit_Element (Sequence, 1, Nested'Access);
      Check (M.Length (Sequence) = 2);
      begin
         declare Invalid : constant M.Digest := M.Create (Empty);
         begin V.Visit_Binary (Invalid, Read'Access); end;
         raise Internal_Error;
      exception when Constraint_Error => null; end;
      Bytes.Append (1);
      begin
         declare Invalid : constant M.Digest := M.Create (Bytes);
         begin V.Visit_Binary (Invalid, Read'Access); end;
         raise Internal_Error;
      exception when Constraint_Error => null; end;
      V.Visit_Binary (D, Read'Access);
   end;
   Ada.Text_IO.Put_Line ("C3_BINARY_VIEWS_COMPLETE checks=" & Checks'Image);
exception when others => Allocation_Faults.Disarm; raise;
end C3_Views;