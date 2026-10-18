with GNATCOLL.JSON;

--  Guarded JSON boundary. No unchecked reader or writer is exported.
--  Duplicate keys retain the provider's last-value-wins policy.
package AMS_GRA_OMS_JSON_Guard is
   Max_Document_Bytes : constant := 1_048_576;
   Max_Depth : constant := 128;
   Max_Nodes : constant := 100_000;
   Validation_Error : exception;
   --  Validate decoded UTF-8 before a bounded structural emitter escapes it.
   procedure Validate_String (Text : String);
   function Read (Text : String) return GNATCOLL.JSON.JSON_Value;
   function Write (Value : GNATCOLL.JSON.JSON_Value) return String;
end AMS_GRA_OMS_JSON_Guard;
