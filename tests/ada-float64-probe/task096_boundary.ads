with Interfaces;
package Task096_Boundary is
   subtype F64 is Interfaces.IEEE_Float_64;
   Invalid_Value, Unsupported_Environment, Resource_Error : exception;
   procedure Require_Environment;
   function Number (Token : String) return F64;
   function Special (Text : String) return F64;
   function Token (Value : F64) return String;
end Task096_Boundary;