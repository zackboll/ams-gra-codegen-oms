with Ada.Assertions;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with AMS_GRA_OMS_JSON_Guard;
with AMS_GRA_OMS_JSON_Float32;
with AMS_GRA_OMS_JSON_Occurrences;
with AMS_GRA_OMS_JSON_Occurrences.Research;
with GNATCOLL.JSON;
with Interfaces;
with Task096_Boundary;
package body Task096_Adapter is
   package Ingress renames AMS_GRA_OMS_JSON_Occurrences;
   package Boundary renames Task096_Boundary;
   use GNATCOLL.JSON;
   use type Interfaces.IEEE_Float_64;
   use type Model.Selection_Kind;
   subtype F64 is Interfaces.IEEE_Float_64;
   function Key (Local : String) return String is (Local);
   function Scalar (Node : Ingress.Value) return F64 is
   begin
      if Ingress.Kind (Node) = JSON_String_Type then
         return Boundary.Special (Ingress.Get (Node));
      end if;
      return Boundary.Number (Ingress.Research.Numeric_Token (Node));
   end Scalar;
   function Named_Closed (Node : Ingress.Value) return Model.ClosedRange is
   begin
      return Model.Create (Scalar (Node));
   exception when Ada.Assertions.Assertion_Error => raise Codec_Error with "ClosedRange";
   end Named_Closed;
   function Named_Open (Node : Ingress.Value) return Model.OpenRange is
   begin
      return Model.Create (Scalar (Node));
   exception when Ada.Assertions.Assertion_Error => raise Codec_Error with "OpenRange";
   end Named_Open;
   function Nested (Node : Ingress.Value) return Model.Nested is
   begin
      if Ingress.Kind (Node) /= JSON_Object_Type then raise Codec_Error; end if;
      return (Value => Scalar (Ingress.Get (Node, Key ("Value"))));
   end Nested;
   function Choice (Node : Ingress.Value) return Model.Selection is
   begin
      if Ingress.Kind (Node) /= JSON_Object_Type then raise Codec_Error; end if;
      if Ingress.Has_Field (Node, Key ("Record")) and not Ingress.Has_Field (Node, Key ("Integer")) then
         return (Kind => Model.Alternative_Record_Kind,
                 Alternative_Record => Nested (Ingress.Get (Node, Key ("Record"))));
      elsif Ingress.Has_Field (Node, Key ("Integer")) and not Ingress.Has_Field (Node, Key ("Record")) then
         declare
            Number : constant Long_Long_Integer := Ingress.Get (Ingress.Get (Node, Key ("Integer")));
         begin
            if Number not in -2_147_483_648 .. 2_147_483_647 then raise Codec_Error; end if;
            return (Model.Integer_Kind, Number);
         end;
      else raise Codec_Error; end if;
   end Choice;
   procedure Decode (Text : String; Output : in out Model.FloatPayload) is
   begin
      Boundary.Require_Environment;
      declare
         Root : constant Ingress.Value := Ingress.Read (Text);
         function Member (Local : String) return Ingress.Value is (Ingress.Get (Root, Key (Local)));
         function Optional return Model.FloatPayload_Optional_Optional is
         begin
            if Ingress.Has_Field (Root, Key ("Optional")) then return (True, Scalar (Member ("Optional")));
            else return (Is_Present => False); end if;
         end Optional;
         function Bounded return Model.FloatPayload_Bounded_Sequence is
            Result : Model.FloatPayload_Bounded_Sequence;
         begin
            if not Ingress.Has_Field (Root, Key ("Bounded")) then return Result; end if;
            declare
               Items : constant Ingress.Value_Array := Ingress.Get (Member ("Bounded"));
            begin
               if Ingress.Length (Items) > 3 then raise Codec_Error with "Bounded cardinality"; end if;
               for Index in 1 .. Ingress.Length (Items) loop
                  begin Model.Append (Result, Scalar (Ingress.Get (Items, Index)));
                  exception when Boundary.Invalid_Value | Ingress.Validation_Error =>
                     raise Codec_Error with "Bounded item" & Index'Image;
                  end;
               end loop;
            end;
            return Result;
         end Bounded;
         function Unbounded return Model.FloatPayload_Unbounded_Sequence is
            Items : constant Ingress.Value_Array := Ingress.Get (Member ("Unbounded"));
            function Element (Index : Positive) return F64 is
            begin return Scalar (Ingress.Get (Items, Index));
            exception when Boundary.Invalid_Value | Ingress.Validation_Error =>
               raise Codec_Error with "Unbounded item" & Index'Image;
            end Element;
         begin
            if Ingress.Length (Items) < 2 then raise Codec_Error with "Unbounded cardinality"; end if;
            return Result : Model.FloatPayload_Unbounded_Sequence :=
              (Required => (Element (1), Element (2)), Additional => <>) do
               for Index in 3 .. Ingress.Length (Items) loop Model.Append (Result.Additional, Element (Index)); end loop;
            end return;
         end Unbounded;
         function NamedRepeated return Model.FloatPayload_NamedRepeated_Sequence is
            Items : constant Ingress.Value_Array := Ingress.Get (Member ("NamedRepeated"));
            function Element (Index : Positive) return Model.ClosedRange is
            begin return Named_Closed (Ingress.Get (Items, Index));
            exception when Codec_Error | Boundary.Invalid_Value | Ingress.Validation_Error =>
               raise Codec_Error with "NamedRepeated item" & Index'Image;
            end Element;
         begin
            if Ingress.Length (Items) not in 1 .. 3 then raise Codec_Error with "NamedRepeated cardinality"; end if;
            return Model.To_Sequence (Model.FloatPayload_NamedRepeated_Values'
              (for Index in 1 .. Ingress.Length (Items) => Element (Index)));
         end NamedRepeated;
         function Count return Long_Long_Integer is
            Value : constant Long_Long_Integer := Ingress.Get (Member ("Count"));
         begin
            if Value not in -2_147_483_648 .. 2_147_483_647 then raise Codec_Error; end if;
            return Value;
         end Count;
         Result : constant Model.FloatPayload :=
           (Required => Scalar (Member ("Required")), Optional => Optional,
            Named => Model.Unconstrained (Scalar (Member ("Named"))),
            Closed => Named_Closed (Member ("Closed")), Open => Named_Open (Member ("Open")),
            Bounded => Bounded, Unbounded => Unbounded, NamedRepeated => NamedRepeated,
            Nested => Nested (Member ("Nested")), Choice => Choice (Member ("Choice")),
            Small => Ingress.Floating (Member ("Small")), Count => Count,
            Enabled => Ingress.Get (Member ("Enabled")), Text => To_Unbounded_String (Ingress.Get (Member ("Text"))));
      begin
         Output := Result;
      end;
   exception
      when E : Ingress.Validation_Error | Boundary.Invalid_Value | AMS_GRA_OMS_JSON_Float32.Validation_Error =>
         raise Codec_Error with Ada.Exceptions.Exception_Message (E);
   end Decode;
   procedure Encode (Item : Model.FloatPayload; Output : in out Unbounded_String) is
      Buffer : Unbounded_String;
      First_Member : Boolean := True;
      function Quoted (Text : String) return String is
        (AMS_GRA_OMS_JSON_Guard.Write (Create (Text)));
      procedure Put (Text : String) is
      begin
         if Text'Length > AMS_GRA_OMS_JSON_Guard.Max_Document_Bytes or else
           Length (Buffer) > AMS_GRA_OMS_JSON_Guard.Max_Document_Bytes - Text'Length then
            raise Codec_Error with "output document size";
         end if;
         Append (Buffer, Text);
      end Put;
      procedure Member (Name, Value : String) is
      begin
         if not First_Member then Put (","); end if;
         First_Member := False;
         Put (Quoted (Key (Name))); Put (":"); Put (Value);
      end Member;
      function Nested (Value : Model.Nested) return String is
        ("{" & Quoted (Key ("Value")) & ":" & Boundary.Token (Value.Value) & "}");
   begin
      Boundary.Require_Environment;
      Put ("{");
      Member ("Required", Boundary.Token (Item.Required));
      if Item.Optional.Is_Present then Member ("Optional", Boundary.Token (Item.Optional.Value)); end if;
      Member ("Named", Boundary.Token (F64 (Item.Named)));
      Member ("Closed", Boundary.Token (Model.Value (Item.Closed)));
      Member ("Open", Boundary.Token (Model.Value (Item.Open)));
      if Model.Length (Item.Bounded) > 0 then
         Member ("Bounded", "[");
         for Index in 1 .. Model.Length (Item.Bounded) loop
            if Index > 1 then Put (","); end if;
            Put (Boundary.Token (Model.Element (Item.Bounded, Index)));
         end loop;
         Put ("]");
      end if;
      Member ("Unbounded", "[");
      for Index in Item.Unbounded.Required'Range loop
         if Index > 1 then Put (","); end if;
         Put (Boundary.Token (Item.Unbounded.Required (Index)));
      end loop;
      for Index in 1 .. Model.Length (Item.Unbounded.Additional) loop
         Put (","); Put (Boundary.Token (Model.Element (Item.Unbounded.Additional, Index)));
      end loop;
      Put ("]");
      Member ("NamedRepeated", "[");
      for Index in 1 .. Model.Length (Item.NamedRepeated) loop
         if Index > 1 then Put (","); end if;
         Put (Boundary.Token (Model.Value (Model.Element (Item.NamedRepeated, Index))));
      end loop;
      Put ("]");
      Member ("Nested", Nested (Item.Nested));
      if Item.Choice.Kind = Model.Alternative_Record_Kind then
         Member ("Choice", "{" & Quoted (Key ("Record")) & ":" & Nested (Item.Choice.Alternative_Record) & "}");
      else
         Member ("Choice", "{" & Quoted (Key ("Integer")) & ":" &
           AMS_GRA_OMS_JSON_Guard.Write (Create (Item.Choice.Integer)) & "}");
      end if;
      Member ("Small", AMS_GRA_OMS_JSON_Guard.Write (AMS_GRA_OMS_JSON_Float32.Encode (Item.Small)));
      Member ("Count", AMS_GRA_OMS_JSON_Guard.Write (Create (Item.Count)));
      Member ("Enabled", AMS_GRA_OMS_JSON_Guard.Write (Create (Item.Enabled)));
      Member ("Text", Quoted (To_String (Item.Text)));
      Put ("}");
      -- Validate structure/Unicode/limits using the ORIGINAL occurrence reader.
      -- This does not reserialize a floating DOM and cannot reduce precision.
      declare
         Checked : constant Ingress.Value := Ingress.Read (To_String (Buffer));
         pragma Unreferenced (Checked);
      begin null; end;
      Output := Buffer;
   exception
      when E : AMS_GRA_OMS_JSON_Guard.Validation_Error | Ingress.Validation_Error =>
         raise Codec_Error with Ada.Exceptions.Exception_Message (E);
   end Encode;
end Task096_Adapter;