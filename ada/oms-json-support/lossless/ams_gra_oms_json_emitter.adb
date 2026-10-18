with Ada.Strings.Fixed;
with AMS_GRA_OMS_JSON_Guard;
with AMS_GRA_OMS_JSON_Occurrences;
with AMS_GRA_OMS_JSON_Float32;
with AMS_GRA_OMS_JSON_Float64;
with GNATCOLL.JSON;
package body AMS_GRA_OMS_JSON_Emitter is
   use Ada.Strings.Unbounded;
   package Guard renames AMS_GRA_OMS_JSON_Guard;
   procedure Bytes (Count : Natural) is
   begin
      if Count > Guard.Max_Document_Bytes then
         raise Validation_Error with "JSON output byte limit";
      end if;
   end Bytes;
   function Scalar (Text : String) return Value is
   begin
      Bytes (Text'Length);
      return (Text => To_Unbounded_String (Text), others => <>);
   end Scalar;
   function Create_Object return Value is
     (Text => To_Unbounded_String ("{}"), Object => True, others => <>);
   function Create (Text : String) return Value is
      Expanded : Natural := 2;
      Offset : Natural := 0;
      B : Natural;
   begin
      Bytes (Text'Length);
      Guard.Validate_String (Text);
      --  Exact provider expansion, after UTF-8 validation, before allocating
      --  the provider's escaped string. Non-BMP uses two surrogate escapes.
      while Offset < Text'Length loop
         B := Character'Pos (Text (Text'First + Offset));
         if B >= 16#F0# then
            Expanded := Expanded + 12; Offset := Offset + 4;
         elsif B >= 16#E0# then
            Expanded := Expanded + 6; Offset := Offset + 3;
         elsif B >= 16#C0# then
            Expanded := Expanded + 6; Offset := Offset + 2;
         else
            Expanded := Expanded +
              (if B in 8 | 9 | 10 | 12 | 13 | 34 | 92 then 2
               elsif B < 32 then 6 else 1);
            Offset := Offset + 1;
         end if;
         Bytes (Expanded);
      end loop;
      return Scalar (Guard.Write (GNATCOLL.JSON.Create (Text)));
   exception
      when Guard.Validation_Error => raise Validation_Error with "invalid output string";
   end Create;
   function Create (Item : Boolean) return Value is
     (Scalar ((if Item then "true" else "false")));
   function Create (Item : Long_Long_Integer) return Value is
     (Scalar (Ada.Strings.Fixed.Trim (Item'Image, Ada.Strings.Both)));
   function Create (Items : Value_Array) return Value is (Items.Contents);
   function Floating_32 (Item : Interfaces.IEEE_Float_32) return Value is
     (Scalar (Guard.Write (AMS_GRA_OMS_JSON_Float32.Encode (Item))));
   function Floating_64 (Item : Interfaces.IEEE_Float_64) return Value is
     (Scalar (AMS_GRA_OMS_JSON_Float64.Token (Item)));
   procedure Add (Container : in out Value; Prefix : String; Child : Value) is
      Size : constant Natural := Length (Container.Text);
      Child_Depth : constant Natural := Child.Depth + 1;
   begin
      if Child.Nodes > Guard.Max_Nodes - Container.Nodes or else
        Child_Depth > Guard.Max_Depth
      then raise Validation_Error with "JSON output tree limit"; end if;
      --  All component sizes were bounded on construction, so these sums are
      --  bounded well below Natural'Last before any concatenation/allocation.
      Bytes (Size + Prefix'Length + Length (Child.Text) +
             (if Container.Count = 0 then 0 else 1));
      declare
         Temporary : Unbounded_String := Container.Text;
         Closing : constant Character := Element (Temporary, Size);
      begin
         Delete (Temporary, Size, Size);
         if Container.Count /= 0 then Append (Temporary, ','); end if;
         Append (Temporary, Prefix);
         Append (Temporary, Child.Text);
         Append (Temporary, Closing);
         Container.Text := Temporary;
      end;
      Container.Nodes := Container.Nodes + Child.Nodes;
      Container.Depth := Natural'Max (Container.Depth, Child_Depth);
      Container.Count := Container.Count + 1;
   end Add;
   procedure Set_Field (Object : in out Value; Name : String; Child : Value) is
   begin
      if not Object.Object then raise Program_Error with "emitter expected object"; end if;
      declare
         Key : constant Value := Create (Name);
      begin
         Add (Object, To_String (Key.Text) & ":", Child);
      end;
   end Set_Field;
   procedure Append (Items : in out Value_Array; Child : Value) is
   begin
      Add (Items.Contents, "", Child);
   end Append;
   function Write (Item : Value) return String is
      Text : constant String := To_String (Item.Text);
      Checked : constant AMS_GRA_OMS_JSON_Occurrences.Value :=
        AMS_GRA_OMS_JSON_Occurrences.Read (Text);
      pragma Unreferenced (Checked);
   begin
      return Text;
   exception
      when AMS_GRA_OMS_JSON_Occurrences.Validation_Error =>
         raise Validation_Error with "invalid completed JSON output";
   end Write;
end AMS_GRA_OMS_JSON_Emitter;