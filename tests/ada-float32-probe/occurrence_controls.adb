with Ada.Exceptions;
with Ada.Text_IO;
with GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Occurrences;
with AMS_GRA_OMS_JSON_Float32;
procedure Occurrence_Controls is
   package O renames AMS_GRA_OMS_JSON_Occurrences;
   package F renames AMS_GRA_OMS_JSON_Float32;
   use type F.Float32;
   use type GNATCOLL.JSON.JSON_Value_Type;
   type Text_Array is array (Positive range <>) of String (1 .. 5);
   Texts : constant Text_Array := ["true ", "null ", "false"];
   Checks : Natural := 0;
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "occurrence check failed"; end if;
      Checks := Checks + 1;
   end;
   procedure Reject (Text : String) is
   begin
      declare
         V : constant O.Value := O.Read (Text);
         pragma Unreferenced (V);
      begin null; end;
      raise Program_Error with "accepted invalid occurrence input";
   exception
      when O.Validation_Error => Checks := Checks + 1;
   end;
begin
   declare
      V : constant O.Value := O.Read
        ("{""a"":99,""\u0061"":-0,""nested"":{""a"":1.000000059604644775390626},""array"":[0.1,-0.0,1e-45],""n"":null,""t"":true,""f"":false}");
      A : constant O.Value_Array := O.Get (V.Get ("array"));
      X : constant F.Float32 := O.Floating (V.Get ("a"));
   begin
      Check (V.Kind = GNATCOLL.JSON.JSON_Object_Type);
      Check (X = 0.0 and then F.Float32'Copy_Sign (1.0, X) < 0.0);
      Check (O.Floating (V.Get ("nested").Get ("a")) = 1.0 + 2.0 ** (-23));
      Check (O.Length (A) = 3);
      Check (O.Floating (O.Get (A, 1)) = F.Number ("0.1"));
      Check (F.Float32'Copy_Sign (1.0, O.Floating (O.Get (A, 2))) < 0.0);
      Check (O.Floating (O.Get (A, 3)) = F.Float32'Scaling (1.0, -149));
      Check (V.Has_Field ("n") and then V.Get ("n").Kind = GNATCOLL.JSON.JSON_Null_Type);
      Check (not V.Has_Field ("missing"));
      Check (Boolean'(O.Get (V.Get ("t"))) and then not Boolean'(O.Get (V.Get ("f"))));
      Check (V.Get ("a").First > 1 and then V.Get ("a").Last = V.Get ("a").First + 1);
   end;
   for Text of Texts loop
      declare
         V : constant O.Value := O.Read (Text);
      begin
         Check (V.First = 1);
         Check (V.Last = (if Text (1) = 'f' then 5 else 4));
      end;
   end loop;
   declare
      V : constant O.Value := O.Read ("{""{urn:x}a"":[99],""{urn:x}\u0061"":[-0,1],""a"":{}}");
      Items : constant O.Value_Array := O.Get (V.Get ("{urn:x}a"));
   begin
      Check (O.Length (Items) = 2);
      Check (F.Float32'Copy_Sign (1.0, O.Floating (O.Get (Items, 1))) < 0.0);
      Check (V.Get ("a").Kind = GNATCOLL.JSON.JSON_Object_Type);
   end;
   Reject ("{""a"":1e+}");
   Reject ("{""a"":""\uD800"",""a"":0}");
   Reject ("{""a"":0}{}");
   Reject ("[0,]");
   Reject ("{""a"":" & '"' & Character'Val (255) & '"' & "}");
   Reject ((1 .. 129 => '[') & "0" & (1 .. 129 => ']'));
   Reject ((1 .. 1_048_577 => ' '));
   Ada.Text_IO.Put_Line ("TASK095_OCCURRENCE_CONTROLS_COMPLETE checks=" & Checks'Image);
exception
   when E : others =>
      Ada.Text_IO.Put_Line (Ada.Exceptions.Exception_Information (E));
      raise;
end Occurrence_Controls;