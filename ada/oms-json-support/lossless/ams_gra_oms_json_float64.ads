with Interfaces;
package AMS_GRA_OMS_JSON_Float64 is
   subtype Float64 is Interfaces.IEEE_Float_64;
   Unsupported_Platform : exception;
   Validation_Error : exception;
   Resource_Error : exception;
   procedure Require_Platform;
   function Number (Token : String) return Float64;
   function Special (Text : String) return Float64;
   --  Valid JSON token, finite NUMBER or exact quoted OMS special. Consumers
   --  must not pass finite tokens through a numeric DOM writer.
   function Token (Value : Float64) return String;
end AMS_GRA_OMS_JSON_Float64;