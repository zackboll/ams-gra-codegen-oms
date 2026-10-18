-- Test-only control mutation; production-style calls never set the environment.
with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Unchecked_Conversion;
with Interfaces;
with Interfaces.C;
with Programs.Oam;
with Task096_Adapter;
with Task096_Boundary;
procedure Environment_Probe is
   package Model renames Programs.Oam;
   subtype F64 is Interfaces.IEEE_Float_64;
   use type Interfaces.Unsigned_64;
   use type Interfaces.C.int;
   use type Interfaces.C.unsigned;
   function Bits is new Ada.Unchecked_Conversion (F64, Interfaces.Unsigned_64);
   function Save return Interfaces.C.int
     with Import, Convention => C, External_Name => "task095_rounding_save";
   function Restore return Interfaces.C.int
     with Import, Convention => C, External_Name => "task095_rounding_restore";
   function Pair (X87, SSE : Interfaces.C.unsigned) return Interfaces.C.int
     with Import, Convention => C, External_Name => "task095_rounding_pair";
   procedure Mode (Value : Interfaces.C.unsigned)
     with Import, Convention => C, External_Name => "task095_mode";
   function X87_Control return Interfaces.C.unsigned
     with Import, Convention => C, External_Name => "task095_x87";
   function SSE_Control return Interfaces.C.unsigned
     with Import, Convention => C, External_Name => "task095_mxcsr";
   X87 : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (1));
   SSE : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (2));
   Flushing : constant Interfaces.C.unsigned := Interfaces.C.unsigned'Value (Ada.Command_Line.Argument (3));
   Supported : constant Boolean := X87 = 0 and SSE = 0 and Flushing = 0;
   Saved : Boolean := False;
   Original_X87, Original_SSE, Before_X87, Before_SSE : Interfaces.C.unsigned;
   Checks, Rejections : Natural := 0;
   Item : Model.FloatPayload :=
     (Required => 42.0, Optional => (Is_Present => False), Named => 0.0,
      Closed => Model.Create (F64'(0.0)), Open => Model.Create (F64'(1.0)),
      Bounded => <>, Unbounded => (Required => (0.0, 0.0), Additional => <>),
      NamedRepeated => Model.To_Sequence ((1 => Model.Create (F64'(0.0)))),
      Nested => (Value => 0.0), Choice => (Model.Alternative_Record_Kind, (Value => 0.0)),
      Small => 0.0, Count => 0, Enabled => False, Text => To_Unbounded_String ("sentinel"));
   Before : Unbounded_String;
   Output : Unbounded_String := To_Unbounded_String ("sentinel");
   Value : F64 := 42.0;
   Special_Text : Unbounded_String;
   procedure Check (Condition : Boolean) is
   begin
      Checks := Checks + 1;
      if not Condition then raise Program_Error with "environment check" & Checks'Image; end if;
   end Check;
   procedure Rejected is
   begin
      Check (not Supported);
      Rejections := Rejections + 1;
   end Rejected;
begin
   Original_X87 := X87_Control; Original_SSE := SSE_Control;
   Check (Save = 0); Saved := True;
   Task096_Adapter.Encode (Item, Before);
   Check (Pair (X87, SSE) = 0); Mode (Flushing);
   Before_X87 := X87_Control; Before_SSE := SSE_Control;
   begin
      Value := Task096_Boundary.Number ("1.0000000000000002");
      Check (Supported);
   exception when Task096_Boundary.Unsupported_Environment => Rejected; end;
   begin
      Special_Text := To_Unbounded_String (Task096_Boundary.Token (Item.Required));
      Check (Supported);
   exception when Task096_Boundary.Unsupported_Environment => Rejected; end;
   begin
      Value := Task096_Boundary.Special ("Infinity");
      Check (Supported);
   exception when Task096_Boundary.Unsupported_Environment => Rejected; end;
   begin
      Task096_Adapter.Encode (Item, Output);
      Check (Supported);
   exception when Task096_Boundary.Unsupported_Environment => Rejected; end;
   begin
      -- Unsupported environments must be rejected before even parsing JSON.
      Task096_Adapter.Decode ("{}", Item);
      raise Program_Error with "missing fields accepted";
   exception
      when Task096_Boundary.Unsupported_Environment => Rejected;
      when Task096_Adapter.Codec_Error => Check (Supported);
   end;
   Check (X87_Control = Before_X87);
   Check ((SSE_Control and not 63) = (Before_SSE and not 63));
   if not Supported then
      Check (Bits (Value) = 16#4045_0000_0000_0000#);
      Check (Output = "sentinel" and Length (Special_Text) = 0);
   end if;
   Check (Restore = 0); Saved := False;
   Check (X87_Control = Original_X87 and SSE_Control = Original_SSE);
   Check (Save = 0); Saved := True;
   Task096_Adapter.Encode (Item, Output);
   Check (Output = Before);
   Value := Task096_Boundary.Number ("-0");
   Check (Bits (Value) = 16#8000_0000_0000_0000#);
   Task096_Adapter.Decode (To_String (Before), Item);
   Task096_Adapter.Encode (Item, Output);
   Check (Output = Before);
   Check (Restore = 0); Saved := False;
   Check (X87_Control = Original_X87 and SSE_Control = Original_SSE);
   Put_Line ("TASK096_ADA_ENVIRONMENT_COMPLETE x87=" & X87'Image & " mxcsr=" & SSE'Image &
             " flushing=" & Flushing'Image & " checks=" & Checks'Image & " rejections=" & Rejections'Image);
exception
   when E : others =>
      if Saved then
         declare Status : constant Interfaces.C.int := Restore;
         begin if Status /= 0 then Put_Line ("RESTORATION_FAILED"); end if; end;
      end if;
      Put_Line ("INCOMPLETE " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Environment_Probe;