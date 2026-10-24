with Ada.Finalization;
with GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Float32;
with System.Atomic_Counters;

--  Immutable, document-owned SAX occurrences for the lossless profile only.
--  A Value/array view retains its arena; copying handles never copies nodes.
--  Independent handles may be read concurrently, but assignment to the same
--  handle concurrently requires caller synchronization, as for other Ada values.
package AMS_GRA_OMS_JSON_Lossless_Ingress is
   type Value is new Ada.Finalization.Controlled with private;
   type Value_Array is private;
   Validation_Error : exception;
   function Read (Text : String) return Value;
   function Kind (Item : Value) return GNATCOLL.JSON.JSON_Value_Type;
   function First (Item : Value) return Long_Long_Integer;
   function Last (Item : Value) return Long_Long_Integer;
   function Has_Field (Item : Value; Name : String) return Boolean;
   function Get (Item : Value; Name : String) return Value;
   function Get (Item : Value) return Boolean;
   function Get (Item : Value) return String;
   function Get (Item : Value) return Long_Long_Integer;
   function Get (Item : Value) return Value_Array;
   function Get (Items : Value_Array; Index : Positive) return Value;
   function Length (Items : Value_Array) return Natural;
   function Floating (Item : Value) return AMS_GRA_OMS_JSON_Float32.Float32;
   function Numeric_Lexeme (Item : Value) return String;
   procedure Map_JSON_Object
     (Item : Value; Process : not null access procedure (Name : String; Child : Value));
private
   type Document;
   type Document_Access is access Document;
   type Value is new Ada.Finalization.Controlled with record
      Owner : Document_Access := null;
      Id : Natural := 0;
   end record;
   overriding procedure Adjust (Item : in out Value);
   overriding procedure Finalize (Item : in out Value);
   type Value_Array is record
      Root : Value;
   end record;
end AMS_GRA_OMS_JSON_Lossless_Ingress;