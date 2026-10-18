with AMS_GRA_OMS_JSON_Guard;
with GNATCOLL.Buffer;
with GNATCOLL.JSON;
with Interfaces.C;
package body AMS_GRA_OMS_JSON_Float64 is
   use type Float64;
   use type Interfaces.C.int;
   use type Interfaces.C.size_t;
   function Platform return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float64_platform";
   function Parse (Text : Interfaces.C.char_array; Length, Capacity : Interfaces.C.size_t;
                   Value : out Interfaces.C.double) return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float64_parse";
   function Format (Value : access Interfaces.C.double; Text : out Interfaces.C.char_array;
                    Capacity : Interfaces.C.size_t) return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float64_format";
   function ABI (Input : Interfaces.C.double; Positive, Negative_Zero : out Interfaces.C.double)
     return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float64_abi";
   function Make_Special (Which : Interfaces.C.int; Value : out Interfaces.C.double)
     return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float64_special";
   procedure Check (Status : Interfaces.C.int) is
   begin
      case Status is
         when 0 => null;
         when 1 => raise Unsupported_Platform with "requires Linux x86-64 glibc 2.41 binary64 ABI, x87/MXCSR nearest, DAZ/FTZ off";
         when 2 => raise Validation_Error with "invalid or overflowing Float64";
         when 3 => raise Resource_Error with "Float64 locale or formatter failure";
         when others => raise Program_Error with "invalid Float64 bridge status";
      end case;
   end Check;
   procedure Require_Platform is
      Positive, Negative_Zero : Interfaces.C.double;
   begin
      Check (Platform);
      if Float64'Object_Size /= 64 or else Float64'Machine_Radix /= 2 or else
        Float64'Machine_Mantissa /= 53 or else Float64'Machine_Emin /= -1021 or else
        Float64'Machine_Emax /= 1024 or else not Float64'Signed_Zeros or else
        Interfaces.C.double'Size /= 64 or else Interfaces.C.double'Machine_Mantissa /= 53
      then raise Unsupported_Platform with "unsupported Ada binary64 ABI"; end if;
      Check (ABI (Interfaces.C.double (1.0 + 2.0 ** (-52)), Positive, Negative_Zero));
      if Float64 (Positive) /= 1.0 + 2.0 ** (-52) or else
        Float64 (Negative_Zero) /= 0.0 or else
        Float64'Copy_Sign (1.0, Float64 (Negative_Zero)) /= -1.0
      then raise Unsupported_Platform with "C/Ada binary64 ABI mismatch"; end if;
   end Require_Platform;
   function Number (Token : String) return Float64 is
      use GNATCOLL.JSON;
      Value : Interfaces.C.double;
   begin
      Require_Platform;
      if Token'Length not in 1 .. AMS_GRA_OMS_JSON_Guard.Max_Document_Bytes then
         raise Validation_Error with "Float64 token size";
      end if;
      declare
         Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (Token);
         Parser : JSON_Parser;
         Event : JSON_Parser_Event := Parse_Next (Parser, Data);
      begin
         if Event.Kind not in INTEGER_VALUE | NUMBER_VALUE or else
           Event.First /= 1 or else Event.Last /= Token'Length
         then raise Validation_Error with "expected exact JSON numeric token"; end if;
         Event := Parse_Next (Parser, Data);
         if Event.Kind /= DOC_END then raise Validation_Error with "trailing Float64 token"; end if;
      end;
      Check (Parse (Interfaces.C.To_C (Token), Token'Length, Token'Length + 1, Value));
      return Float64 (Value);
   exception
      when GNATCOLL.JSON.Invalid_JSON_Stream => raise Validation_Error with "invalid JSON numeric token";
   end Number;
   function Special (Text : String) return Float64 is
      Which : Interfaces.C.int;
      Value : Interfaces.C.double;
   begin
      Require_Platform;
      if Text = "NaN" then Which := 0;
      elsif Text = "Infinity" then Which := 1;
      elsif Text = "-Infinity" then Which := -1;
      else raise Validation_Error with "invalid OMS Float64 special spelling"; end if;
      Check (Make_Special (Which, Value));
      return Float64 (Value);
   end Special;
   function Token (Value : Float64) return String is
   begin
      Require_Platform;
      if Value /= Value then return """NaN""";
      elsif Value > Float64'Last then return """Infinity""";
      elsif Value < -Float64'Last then return """-Infinity""";
      end if;
      declare
         C_Value : aliased Interfaces.C.double := Interfaces.C.double (Value);
         Buffer : Interfaces.C.char_array (0 .. 31);
      begin
         Check (Format (C_Value'Access, Buffer, Buffer'Length));
         return Interfaces.C.To_Ada (Buffer);
      end;
   end Token;
end AMS_GRA_OMS_JSON_Float64;