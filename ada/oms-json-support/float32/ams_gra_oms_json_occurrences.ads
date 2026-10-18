with GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Float32;
package AMS_GRA_OMS_JSON_Occurrences is
   type Value is tagged private;
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
   --  Width-neutral access to the original SAX-certified numeric occurrence.
   --  Strings (including quoted finite decimals) are never numeric lexemes.
   function Numeric_Lexeme (Item : Value) return String;
   procedure Map_JSON_Object
     (Item : Value; Process : not null access procedure (Name : String; Child : Value));
private
   -- Reference-counted provider storage, not a numeric DOM or a token side
   -- table. Every node is tagged metadata owning its payload and source span.
   -- Original number text is stored as a STRING in this representation.
   type Value is tagged record
      Node : GNATCOLL.JSON.JSON_Value;
   end record;
   type Value_Array is record
      Items : GNATCOLL.JSON.JSON_Array;
   end record;
end AMS_GRA_OMS_JSON_Occurrences;