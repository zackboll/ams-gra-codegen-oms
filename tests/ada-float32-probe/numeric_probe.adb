with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Fixed;
with Ada.Text_IO;
with GNATCOLL.JSON;
with GNATCOLL.Buffer;
with Interfaces;
with Interfaces.C;
with AMS_GRA_OMS_JSON_Guard;
with AMS_GRA_OMS_JSON_Float32;

procedure Numeric_Probe is
   use Ada.Text_IO;
   use GNATCOLL.JSON;
   subtype F32 is Interfaces.IEEE_Float_32;
   use type F32;
   use type Interfaces.Unsigned_32;
   use type Interfaces.C.int;
   function Direct (S : Interfaces.C.char_array; Valid : out Interfaces.C.int)
     return Interfaces.C.C_float is
   begin
      declare
         Value : constant F32 := AMS_GRA_OMS_JSON_Float32.Number (Interfaces.C.To_Ada (S));
      begin
         Valid := 1;
         return Interfaces.C.C_float (Value);
      end;
   exception
      when AMS_GRA_OMS_JSON_Float32.Validation_Error => Valid := 0; return 0.0;
   end;
   function From_Bits (B : Interfaces.Unsigned_32) return F32 is
      E : constant Integer := Integer (Interfaces.Shift_Right (B, 23) and 255);
      M : constant Interfaces.Unsigned_32 := B and 16#7F_FFFF#;
      X : F32;
   begin
      X := (if E = 0 then F32' Scaling (F32 (M), -149)
            else F32' Scaling (F32 (M + 16#80_0000#), E - 150));
      return (if (B and 16#8000_0000#) /= 0 then -X else X);
   end From_Bits;
   function Bits (X : F32) return String is
      Lo : Interfaces.Unsigned_32 := 0;
      Hi : Interfaces.Unsigned_32 := 16#7F7F_FFFF#;
      Mid, B : Interfaces.Unsigned_32;
      Hex : constant String := "0123456789ABCDEF";
      R : String (1 .. 8);
   begin
      if not (abs X <= F32'Last) then return "NONFINITE"; end if;
      while Lo < Hi loop
         Mid := Lo + (Hi-Lo+1)/2;
         if From_Bits (Mid) <= abs X then Lo := Mid; else Hi := Mid-1; end if;
      end loop;
      B := Lo;
      if F32'Copy_Sign (1.0,X) < 0.0 then B := B or 16#8000_0000#; end if;
      for I in reverse R'Range loop
         R(I) := Hex(Integer(B and 15)+1); B := Interfaces.Shift_Right(B,4);
      end loop;
      return R;
   end Bits;
   Input : File_Type;
   Mode : constant String := Ada.Command_Line.Argument(1);
   Count, Fail_IEEE, Fail_Float, Fail_C, Fail_Write : Natural := 0;
begin
   if F32'Size /= 32 or else Float'Size /= 32 or else Float'Machine_Mantissa /= 24 then
      raise Program_Error with "unsupported width";
   end if;
   if Mode = "tokens" then
      declare
         Source : constant String := Ada.Command_Line.Argument(2);
         Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open(Source);
         Parser : JSON_Parser;
         Event : JSON_Parser_Event;
      begin
         loop
            Event := Parse_Next(Parser,Data);
            exit when Event.Kind = DOC_END;
            if Event.Kind in INTEGER_VALUE | NUMBER_VALUE then
               Count := Count+1;
               Put_Line(Event.Kind'Image & ASCII.HT & Event.First'Image & ASCII.HT & Event.Last'Image &
                        ASCII.HT & GNATCOLL.Buffer.Token(Data,Event.First,Event.Last));
            end if;
         end loop;
      end;
   elsif Mode = "convert" then
      Open(Input,In_File,Ada.Command_Line.Argument(2));
      while not End_Of_File(Input) loop
         declare
            Line : constant String := Get_Line(Input);
            T1 : constant Natural := Ada.Strings.Fixed.Index(Line,ASCII.HT & "");
            T2 : constant Natural := Ada.Strings.Fixed.Index(Line,ASCII.HT & "",T1+1);
            Want : constant String := Line(T1+1 .. T2-1);
            S : constant String := Line(T2+1 .. Line'Last);
            Valid : Interfaces.C.int;
            X : F32;
            procedure Result (Method, Got : String; Failures : in out Natural) is
            begin
               if Got /= Want then
                  Failures := Failures+1;
                  Put_Line("MISMATCH" & ASCII.HT & Method & ASCII.HT & Want & ASCII.HT & Got & ASCII.HT & S);
               end if;
            end;
         begin
            Count := Count+1;
            begin
               X := F32'Value(S);
               Result("IEEE_Value",(if abs X <= F32'Last then Bits(X) else "OVERFLOW"),Fail_IEEE);
            exception when Constraint_Error => Result("IEEE_Value","EXCEPTION",Fail_IEEE); end;
            begin
               X := F32(Float'Value(S));
               Result("Float_Value",(if abs X <= F32'Last then Bits(X) else "OVERFLOW"),Fail_Float);
            exception when Constraint_Error => Result("Float_Value","EXCEPTION",Fail_Float); end;
            X := F32(Direct(Interfaces.C.To_C(S),Valid));
            Result("strtof_l",(if Valid=1 then Bits(X) else "OVERFLOW"),Fail_C);
         end;
      end loop;
      Close(Input);
   elsif Mode = "encode" then
      Open(Input,In_File,Ada.Command_Line.Argument(2));
      while not End_Of_File(Input) loop
         declare
            Want : constant String := Get_Line(Input);
            X : constant F32 := From_Bits(Interfaces.Unsigned_32'Value("16#" & Want & "#"));
            Text : constant String := AMS_GRA_OMS_JSON_Guard.Write(AMS_GRA_OMS_JSON_Float32.Encode(X));
            Valid : Interfaces.C.int;
            Back : constant F32 := F32(Direct(Interfaces.C.To_C(Text),Valid));
         begin
            Count := Count+1;
            Put_Line(Want & ASCII.HT & Bits(Back) & ASCII.HT & Text);
            if Valid/=1 or else Bits(Back)/=Want then Fail_Write := Fail_Write+1; end if;
         end;
      end loop;
      Close(Input);
      Put_Line("SPECIAL" & ASCII.HT & AMS_GRA_OMS_JSON_Guard.Write(Create("NaN")));
      Put_Line("SPECIAL" & ASCII.HT & AMS_GRA_OMS_JSON_Guard.Write(Create("Infinity")));
      Put_Line("SPECIAL" & ASCII.HT & AMS_GRA_OMS_JSON_Guard.Write(Create("-Infinity")));
   else
      raise Program_Error with "bad mode";
   end if;
   Put_Line("R1_NUMERIC_COMPLETE mode=" & Mode & " count=" & Count'Image &
            " ieee_fail=" & Fail_IEEE'Image & " float_fail=" & Fail_Float'Image &
            " c_fail=" & Fail_C'Image & " write_fail=" & Fail_Write'Image);
   -- Candidate failures are measured evidence; the runner decides qualification.
exception when E : others =>
   Put_Line("INCOMPLETE " & Ada.Exceptions.Exception_Information(E));
   Ada.Command_Line.Set_Exit_Status(Ada.Command_Line.Failure);
end Numeric_Probe;