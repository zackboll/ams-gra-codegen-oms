with Ada.Strings.Unbounded;
with Interfaces;
package AMS_GRA_OMS_JSON_Emitter is
   --  Owned bounded typed fragments. There is deliberately no public raw-text
   --  constructor. Objects retain insertion order; arrays retain element order.
   type Value is tagged private;
   type Value_Array is private;
   Validation_Error : exception;
   function Create_Object return Value;
   function Create (Text : String) return Value;
   function Create (Item : Boolean) return Value;
   function Create (Item : Long_Long_Integer) return Value;
   function Create (Items : Value_Array) return Value;
   function Floating_32 (Item : Interfaces.IEEE_Float_32) return Value;
   function Floating_64 (Item : Interfaces.IEEE_Float_64) return Value;
   procedure Set_Field (Object : in out Value; Name : String; Child : Value);
   procedure Append (Items : in out Value_Array; Child : Value);
   function Write (Item : Value) return String;
private
   type Value is tagged record
      Text : Ada.Strings.Unbounded.Unbounded_String;
      Nodes : Natural := 1;
      Depth : Natural := 0;
      Object : Boolean := False;
      Count : Natural := 0;
   end record;
   type Value_Array is record
      Contents : Value := (Text => Ada.Strings.Unbounded.To_Unbounded_String ("[]"),
                          Nodes => 1, Depth => 0, Object => False, Count => 0);
   end record;
end AMS_GRA_OMS_JSON_Emitter;