with GNATCOLL.JSON;
with Interfaces;
package AMS_GRA_OMS_JSON_Float32 is
   subtype Float32 is Interfaces.IEEE_Float_32;
   Validation_Error : exception;
   Unsupported_Platform : exception;
   Resource_Error : exception;
   -- Explicit deployment contract, never inferred from the generator host.
   procedure Require_Platform;
   -- Parse only one provider-certified JSON number, not Ada/C numeric syntax.
   function Number (Token : String) return Float32;
   function Special (Text : String) return Float32;
   function Encode (Value : Float32) return GNATCOLL.JSON.JSON_Value;
end AMS_GRA_OMS_JSON_Float32;