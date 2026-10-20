with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Streams;
with Ada.Streams.Stream_IO;
with Ada.Text_IO; use Ada.Text_IO;
with Allocation_Faults;
with Interfaces.C;
with GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Lossless_Ingress;
with AMS_GRA_OMS_JSON_Occurrences;
procedure C5_Ingress is
   package New_Ingress renames AMS_GRA_OMS_JSON_Lossless_Ingress;
   package Old renames AMS_GRA_OMS_JSON_Occurrences;
   use type Interfaces.C.size_t;
   use type Ada.Streams.Stream_Element_Offset;
   use type Ada.Exceptions.Exception_Id;
   function Read_File return String is
      package IO renames Ada.Streams.Stream_IO;
      F : IO.File_Type;
   begin
      IO.Open (F, IO.In_File, Ada.Command_Line.Argument (2));
      declare
         Size : constant Natural := Natural (IO.Size (F));
         Bytes : Ada.Streams.Stream_Element_Array (1 .. Ada.Streams.Stream_Element_Offset (Size));
         Last : Ada.Streams.Stream_Element_Offset;
         Text : String (17 .. Size + 16);
      begin
         IO.Read (F, Bytes, Last); IO.Close (F);
         if Last /= Bytes'Last then raise Program_Error; end if;
         for J in 1 .. Size loop
            Text (J + 16) := Character'Val (Bytes (Ada.Streams.Stream_Element_Offset (J)));
         end loop;
         return Text;
      end;
   end Read_File;
   Text : constant String := Read_File;
   Mode : constant String := Ada.Command_Line.Argument (1);
   Budget : constant Natural := Natural'Value (Ada.Command_Line.Argument (3));
   Result : New_Ingress.Value;
   Failed : Boolean := False;
   Invalid : Boolean := False;
   Error : Ada.Exceptions.Exception_Occurrence;
   function Hex (Text : String) return String is
      Hex_Digits : constant String := "0123456789ABCDEF";
      Encoded : String (1 .. Text'Length * 2);
      J : Natural := 1;
   begin
      for C of Text loop
         Encoded (J) := Hex_Digits (Character'Pos (C) / 16 + 1);
         Encoded (J + 1) := Hex_Digits (Character'Pos (C) mod 16 + 1); J := J + 2;
      end loop;
      return Encoded;
   end Hex;
   generic
      type Value is tagged private;
      type Array_View is private;
      with function Kind (Item : Value) return GNATCOLL.JSON.JSON_Value_Type;
      with function First (Item : Value) return Long_Long_Integer;
      with function Last (Item : Value) return Long_Long_Integer;
      with function Get (Item : Value) return String;
      with function Get (Item : Value) return Boolean;
      with function Get (Item : Value) return Array_View;
      with function Get (Items : Array_View; Index : Positive) return Value;
      with function Length (Items : Array_View) return Natural;
      with function Numeric_Lexeme (Item : Value) return String;
      with procedure Map_JSON_Object
        (Item : Value; Process : not null access procedure (Name : String; Child : Value));
   procedure Dump (Item : Value; Path : String);
   procedure Dump (Item : Value; Path : String) is
      use GNATCOLL.JSON;
      K : constant JSON_Value_Type := Kind (Item);
      procedure Member (Name : String; Child : Value) is
      begin Dump (Child, Path & "/k" & Hex (Name)); end;
   begin
      Put (Path & " " & K'Image & First (Item)'Image & Last (Item)'Image & " ");
      case K is
         when JSON_String_Type => Put (Hex (Get (Item)));
         when JSON_Boolean_Type => Put (Boolean'Image (Get (Item)));
         when JSON_Int_Type | JSON_Float_Type => Put (Hex (Numeric_Lexeme (Item)));
         when others => null;
      end case;
      New_Line;
      if K = JSON_Object_Type then Map_JSON_Object (Item, Member'Access);
      elsif K = JSON_Array_Type then
         declare Items : constant Array_View := Get (Item);
         begin
            for I in 1 .. Length (Items) loop Dump (Get (Items, I), Path & "/i" & I'Image); end loop;
         end;
      end if;
   end Dump;
   procedure Dump_New is new Dump
     (New_Ingress.Value, New_Ingress.Value_Array, New_Ingress.Kind, New_Ingress.First,
      New_Ingress.Last, New_Ingress.Get, New_Ingress.Get, New_Ingress.Get, New_Ingress.Get,
      New_Ingress.Length, New_Ingress.Numeric_Lexeme, New_Ingress.Map_JSON_Object);
   procedure Dump_Old is new Dump
     (Old.Value, Old.Value_Array, Old.Kind, Old.First, Old.Last, Old.Get, Old.Get, Old.Get,
      Old.Get, Old.Length, Old.Numeric_Lexeme, Old.Map_JSON_Object);
begin
   if Mode = "old" then
      begin
         Dump_Old (Old.Read (Text), "$ ");
         Put_Line ("C5_OLD_ACCEPT");
      exception when Old.Validation_Error => Put_Line ("C5_OLD_REJECT"); end;
      return;
   end if;
   Allocation_Faults.Arm (Budget);
   begin
      Result := New_Ingress.Read (Text);
      Allocation_Faults.Disarm;
   exception
      when New_Ingress.Validation_Error => Allocation_Faults.Disarm; Invalid := True;
      when E : others =>
         Allocation_Faults.Disarm; Failed := True;
         Ada.Exceptions.Save_Occurrence (Error, E);
   end;
   if Failed then Put_Line ("C5_EXCEPTION " & Ada.Exceptions.Exception_Name (Error));
   elsif Invalid then Put_Line ("C5_REJECT");
   else Put_Line ("C5_ACCEPT"); end if;
   Put_Line ("C5_COUNTS attempts=" & Allocation_Faults.Attempts'Image &
             " failures=" & Allocation_Faults.Failures'Image & " size=" & Allocation_Faults.Failed_Size'Image);
   if Mode = "inject" then
      if not Failed or Invalid or Allocation_Faults.Failures /= 1 or
        Ada.Exceptions.Exception_Identity (Error) /= Storage_Error'Identity
      then raise Program_Error with "unreached or transformed failure"; end if;
      begin
         Put_Line (New_Ingress.Kind (Result)'Image);
         raise Program_Error with "partial result published";
      exception when New_Ingress.Validation_Error => null; end;
      Result := New_Ingress.Read (Text);
      Put_Line ("C5_INGRESS_RECOVERY_COMPLETE");
   elsif Mode = "dump" and not Invalid and not Failed then
      Dump_New (Result, "$ ");
   elsif Mode = "handles" then
      declare
         Child : New_Ingress.Value := Result.Get ("a");
         Copy : New_Ingress.Value;
         Items : New_Ingress.Value_Array;
      begin
         Result := New_Ingress.Read ("null");
         Allocation_Faults.Arm (0);
         for I in 1 .. 100 loop Copy := Child; Items := New_Ingress.Get (Copy); end loop;
         Copy := New_Ingress.Get (Items, 1);
         Child := Copy;
         Allocation_Faults.Disarm;
         if Allocation_Faults.Attempts /= 0 or else New_Ingress.Get (Copy) /= "retained" then
            raise Program_Error with "handle copied storage or lost lifetime";
         end if;
         Put_Line ("C5_NONALLOCATING_HANDLE_LIFETIME_COMPLETE");
      end;
   elsif Mode = "callback" then
      declare
         procedure Internal (Name : String; Child : New_Ingress.Value) is
            pragma Unreferenced (Name, Child);
         begin raise Program_Error with "unrelated callback invariant"; end;
      begin
         begin
            New_Ingress.Map_JSON_Object (Result, Internal'Access);
            raise Constraint_Error with "callback not executed";
         exception when Program_Error => null; end;
         Result := New_Ingress.Read (Text);
         Put_Line ("C5_CALLBACK_INTERNAL_RECOVERY_COMPLETE");
      end;
   end if;
end C5_Ingress;