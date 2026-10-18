with AMS_GRA_OMS_JSON_Float32;
with GNATCOLL.Buffer;
with GNATCOLL.JSON;
with Interfaces.C;
package body Task096_Boundary is
   use type F64;
   use type Interfaces.C.int;
   use type Interfaces.C.size_t;
   function Platform return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_platform";
   function Parse (Text : Interfaces.C.char_array; Length, Capacity : Interfaces.C.size_t;
                   Value : out Interfaces.C.double; Consumed : out Interfaces.C.size_t;
                   Error : out Interfaces.C.int) return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_parse";
   function Format (Value : access Interfaces.C.double; Text : out Interfaces.C.char_array;
                    Capacity : Interfaces.C.size_t) return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_format";
   procedure Check (Status : Interfaces.C.int) is
   begin
      case Status is
         when 0 => null;
         when 1 => raise Unsupported_Environment;
         when 2 => raise Invalid_Value;
         when 3 => raise Resource_Error;
         when others => raise Program_Error;
      end case;
   end Check;
   procedure Require_Environment is
   begin
      Check (Platform);
      if F64'Object_Size /= 64 or else F64' Machine_Radix /= 2 or else
        F64' Machine_Mantissa /= 53 or else F64' Machine_Emin /= -1021 or else
        F64' Machine_Emax /= 1024 or else not F64'Signed_Zeros or else
        Interfaces.C.double'Size /= 64 or else Interfaces.C.double'Machine_Mantissa /= 53
      then raise Unsupported_Environment; end if;
      AMS_GRA_OMS_JSON_Float32.Require_Platform;
   end Require_Environment;
   function Number (Token : String) return F64 is
      use GNATCOLL.JSON;
      Value : Interfaces.C.double;
      Consumed : Interfaces.C.size_t;
      Error : Interfaces.C.int;
   begin
      Require_Environment;
      if Token'Length not in 1 .. 1_048_576 then raise Invalid_Value; end if;
      declare
         Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (Token);
         Parser : JSON_Parser;
         Event : JSON_Parser_Event := Parse_Next (Parser, Data);
      begin
         if Event.Kind not in INTEGER_VALUE | NUMBER_VALUE or else
           Event.First /= 1 or else Event.Last /= Token'Length then raise Invalid_Value; end if;
         Event := Parse_Next (Parser, Data);
         if Event.Kind /= DOC_END then raise Invalid_Value; end if;
      end;
      Check (Parse (Interfaces.C.To_C (Token), Token'Length, Token'Length + 1, Value, Consumed, Error));
      if Consumed /= Token'Length then raise Program_Error; end if;
      return F64 (Value);
   exception when GNATCOLL.JSON.Invalid_JSON_Stream => raise Invalid_Value;
   end Number;
   function Special (Text : String) return F64 is
      Zero : F64 := 0.0;
      pragma Volatile (Zero);
      Large : F64 := F64'Last;
      pragma Volatile (Large);
   begin
      Require_Environment;
      if Text = "NaN" then return Zero / Zero;
      elsif Text = "Infinity" then return Large * 2.0;
      elsif Text = "-Infinity" then return -Large * 2.0;
      else raise Invalid_Value; end if;
   end Special;
   function Token (Value : F64) return String is
   begin
      Require_Environment;
      if Value /= Value then return """NaN""";
      elsif Value > F64'Last then return """Infinity""";
      elsif Value < -F64'Last then return """-Infinity""";
      end if;
      declare
         C_Value : aliased Interfaces.C.double := Interfaces.C.double (Value);
         Buffer : Interfaces.C.char_array (0 .. 31);
      begin
         Check (Format (C_Value'Access, Buffer, Buffer'Length));
         return Interfaces.C.To_Ada (Buffer);
      end;
   end Token;
end Task096_Boundary;