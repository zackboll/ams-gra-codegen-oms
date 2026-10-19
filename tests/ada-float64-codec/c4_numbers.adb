with Ada.Text_IO;
with Allocation_Faults;
with Ada.Unchecked_Conversion;
with Interfaces;
with AMS_GRA_OMS_JSON_Emitter;
procedure C4_Numbers is
   function Floating is new Ada.Unchecked_Conversion
     (Interfaces.Unsigned_64, Interfaces.IEEE_Float_64);
   type Patterns is array (Positive range <>) of Interfaces.Unsigned_64;
   Values : constant Patterns :=
     (16#8000_0000_0000_0000#, 1, 16#000F_FFFF_FFFF_FFFF#,
      16#0010_0000_0000_0000#, 16#3FEF_FFFF_FFFF_FFFF#,
      16#3FF0_0000_0000_0000#, 16#3FF0_0000_0000_0001#,
      16#43E0_0000_0000_0000#, 16#43F0_0000_0000_0000#,
      16#7FEF_FFFF_FFFF_FFFF#, 16#FFEF_FFFF_FFFF_FFFF#,
      16#7FF0_0000_0000_0000#, 16#FFF0_0000_0000_0000#,
      16#7FF8_0000_0000_0000#);
begin
   for Bits of Values loop
      Ada.Text_IO.Put_Line
        (Bits'Image & " " & AMS_GRA_OMS_JSON_Emitter.Write
          (AMS_GRA_OMS_JSON_Emitter.Floating_64 (Floating (Bits))));
   end loop;
   Ada.Text_IO.Put_Line ("C4_NUMERICAL_OUTPUT_COMPLETE");
end C4_Numbers;