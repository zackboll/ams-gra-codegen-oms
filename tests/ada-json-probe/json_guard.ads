with GNATCOLL.JSON;

--  Research boundary only. No unchecked writer is exported by this package.
--  Duplicate keys retain the provider's last-value-wins policy.
package JSON_Guard is
   Validation_Error : exception;
   function Read (Text : String) return GNATCOLL.JSON.JSON_Value;
   function Write (Value : GNATCOLL.JSON.JSON_Value) return String;
end JSON_Guard;