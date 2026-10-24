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
   --  Validation only: no numeric conversion, decoded strings, DOM or returned
   --  tree. Used by the lossless emitter before publishing its completed text.
   --  The pinned SAX reader owns a bounded copy of Text (at most 1 MiB); parser
   --  state grows only with the capped depth. Resource exceptions propagate.
   --  Keys are not value nodes; duplicate keys are legal JSON and are not
   --  rewritten. Existing Read/Write retain their last-value-wins semantics.
   procedure Validate_Encoded_Document (Text : String);
   function Read (Text : String) return GNATCOLL.JSON.JSON_Value;
   function Write (Value : GNATCOLL.JSON.JSON_Value) return String;
end AMS_GRA_OMS_JSON_Guard;
