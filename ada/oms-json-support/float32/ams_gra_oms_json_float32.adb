with GNATCOLL.Buffer;
with Interfaces.C;
package body AMS_GRA_OMS_JSON_Float32 is
   use GNATCOLL.JSON;
   use type Float32;
   use type Interfaces.C.int;
   function Platform return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float32_platform";
   function Parse (Text : Interfaces.C.char_array; Length, Capacity : Interfaces.C.size_t;
                   Value : out Interfaces.C.C_float) return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float32_parse";
   function ABI (Input : Interfaces.C.C_float; Positive, Negative_Zero : out Interfaces.C.C_float)
     return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float32_abi";
   function Make_Special (Which : Interfaces.C.int; Value : out Interfaces.C.C_float)
     return Interfaces.C.int
     with Import, Convention => C, External_Name => "ams_gra_float32_special";
   procedure Check (Status : Interfaces.C.int) is
   begin
      case Status is
         when 0 => null;
         when 1 => raise Unsupported_Platform with "requires Linux x86-64 glibc 2.41 binary32 ABI, x87 and MXCSR nearest rounding, DAZ off and FTZ off";
         when 2 => raise Validation_Error with "invalid or overflowing Float32";
         when 3 => raise Resource_Error with "Float32 locale resource failure";
         when others => raise Program_Error with "invalid Float32 bridge status";
      end case;
   end Check;
   procedure Require_Platform is
      Positive, Negative_Zero : Interfaces.C.C_float;
   begin
      if Float32'Size /= 32 or else Float32'Machine_Mantissa /= 24 or else
        Interfaces.C.C_float'Size /= 32 or else Long_Float'Size /= 64 or else
        Long_Float'Machine_Mantissa /= 53 then
         raise Unsupported_Platform with "unsupported Ada Float32 interoperability";
      end if;
      Check (Platform);
      Check (ABI (Interfaces.C.C_float (1.0 + 2.0 ** (-23)), Positive, Negative_Zero));
      if Float32 (Positive) /= 1.0 + 2.0 ** (-23) or else
        Float32 (Negative_Zero) /= 0.0 or else Float32'Copy_Sign (1.0, Float32 (Negative_Zero)) /= -1.0 then
         raise Unsupported_Platform with "C/Ada binary32 ABI mismatch";
      end if;
   end Require_Platform;
   function Number (Token : String) return Float32 is
      Value : Interfaces.C.C_float;
   begin
      Require_Platform;
      if Token'Length = 0 or else Token'Length > 1_048_576 then
         raise Validation_Error with "Float32 token size";
      end if;
      declare
         Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (Token);
         Parser : JSON_Parser;
         Event : JSON_Parser_Event := Parse_Next (Parser, Data);
      begin
         if Event.Kind not in INTEGER_VALUE | NUMBER_VALUE or else
           Event.First /= 1 or else Event.Last /= Token'Length then
            raise Validation_Error with "expected exact JSON numeric token";
         end if;
         Event := Parse_Next (Parser, Data);
         if Event.Kind /= DOC_END then raise Validation_Error with "trailing Float32 token"; end if;
      end;
      Check (Parse (Interfaces.C.To_C (Token), Interfaces.C.size_t (Token'Length), Interfaces.C.size_t (Token'Length + 1), Value));
      return Float32 (Value);
   exception
      when Invalid_JSON_Stream => raise Validation_Error with "invalid JSON numeric token";
   end Number;
   function Special (Text : String) return Float32 is
      Which : Interfaces.C.int;
      Value : Interfaces.C.C_float;
   begin
      Require_Platform;
      if Text = "NaN" then Which := 0;
      elsif Text = "Infinity" then Which := 1;
      elsif Text = "-Infinity" then Which := -1;
      else raise Validation_Error with "invalid OMS Float32 special spelling";
      end if;
      Check (Make_Special (Which, Value));
      return Float32 (Value);
   end Special;
   function Encode (Value : Float32) return JSON_Value is
   begin
      Require_Platform;
      if Value /= Value then return Create ("NaN");
      elsif Value > Float32'Last then return Create ("Infinity");
      elsif Value < -Float32'Last then return Create ("-Infinity");
      else return Create (Long_Float (Value)); end if;
   end Encode;
end AMS_GRA_OMS_JSON_Float32;