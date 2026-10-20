with Ada.Unchecked_Deallocation;
with GNATCOLL.Buffer;
with AMS_GRA_OMS_JSON_Guard;

package body AMS_GRA_OMS_JSON_Lossless_Ingress is
   use GNATCOLL.JSON;
   package Guard renames AMS_GRA_OMS_JSON_Guard;
   type Node is record
      K : JSON_Value_Type := JSON_Null_Type;
      Start, Finish : Natural := 0;
      Key_Start : Positive := 1;
      Key_End : Natural := 0;
      Text_Start : Positive := 1;
      Text_End : Natural := 0;
      Child, Tail, Next, Count, Index_Start : Natural := 0;
      --  Object-member AVL index. Canonical member slots retain insertion
      --  order; Selected replaces the ENTIRE old subtree on duplicate keys.
      Root, Left, Right : Natural := 0;
      Height : Natural := 1;
      Selected : Natural := 0;
   end record;
   type Nodes is array (Positive range <>) of Node;
   type Nodes_Access is access Nodes;
   type Indices is array (Positive range <>) of Natural;
   type Indices_Access is access Indices;
   type String_Access is access String;
   type Document is limited record
      References : System.Atomic_Counters.Atomic_Counter;
      Input, Bytes : String_Access := null;
      Items : Nodes_Access := null;
      Elements : Indices_Access := null;
      Used, Bytes_Used, Elements_Used : Natural := 0;
   end record;
   procedure Free is new Ada.Unchecked_Deallocation (String, String_Access);
   procedure Free is new Ada.Unchecked_Deallocation (Nodes, Nodes_Access);
   procedure Free is new Ada.Unchecked_Deallocation (Indices, Indices_Access);
   procedure Free is new Ada.Unchecked_Deallocation (Document, Document_Access);

   overriding procedure Adjust (Item : in out Value) is
   begin
      --  Atomic integer operation only, supported by both qualified x86_64
      --  runtimes. No allocation, node/string copy, or user callback.
      if Item.Owner /= null then
         System.Atomic_Counters.Increment (Item.Owner.References);
      end if;
   end Adjust;

   overriding procedure Finalize (Item : in out Value) is
      Owned : Document_Access := Item.Owner;
   begin
      Item.Owner := null;
      Item.Id := 0;
      if Owned /= null and then
        System.Atomic_Counters.Decrement (Owned.References)
      then
         --  All allocations are ordinary arrays of noncontrolled components.
         --  Published pointers are either null or fully initialized allocations.
         Free (Owned.Input);
         Free (Owned.Bytes);
         Free (Owned.Items);
         Free (Owned.Elements);
         Free (Owned);
      end if;
   end Finalize;

   function Handle (Owner : Document_Access; Id : Natural) return Value is
   begin
      return Result : Value do
         if Owner /= null then
            System.Atomic_Counters.Increment (Owner.References);
         end if;
         Result.Owner := Owner;
         Result.Id := Id;
      end return;
   end Handle;

   procedure Check (Item : Value) is
   begin
      if Item.Owner = null or else Item.Id = 0 or else
        Item.Id > Item.Owner.Used
      then
         raise Validation_Error with "missing JSON occurrence";
      end if;
   end Check;
   function Kind (Item : Value) return JSON_Value_Type is
   begin Check (Item); return Item.Owner.Items (Item.Id).K; end Kind;
   function First (Item : Value) return Long_Long_Integer is
   begin Check (Item); return Long_Long_Integer (Item.Owner.Items (Item.Id).Start); end First;
   function Last (Item : Value) return Long_Long_Integer is
   begin Check (Item); return Long_Long_Integer (Item.Owner.Items (Item.Id).Finish); end Last;
   procedure Require_Kind (Item : Value; Expected : JSON_Value_Type) is
   begin
      if Kind (Item) /= Expected then
         raise Validation_Error with "wrong JSON kind at byte" & First (Item)'Image;
      end if;
   end Require_Kind;

   function Find (Item : Value; Name : String) return Natural is
      D : Document renames Item.Owner.all;
      Id : Natural := D.Items (Item.Id).Root;
   begin
      while Id /= 0 loop
         declare
            N : Node renames D.Items (Id);
            Key : String renames D.Bytes (N.Key_Start .. N.Key_End);
         begin
            if Name = Key then return N.Selected;
            elsif Name < Key then Id := N.Left;
            else Id := N.Right;
            end if;
         end;
      end loop;
      return 0;
   end Find;
   function Has_Field (Item : Value; Name : String) return Boolean is
   begin Require_Kind (Item, JSON_Object_Type); return Find (Item, Name) /= 0; end;
   function Get (Item : Value; Name : String) return Value is
      Id : Natural;
   begin
      Require_Kind (Item, JSON_Object_Type);
      Id := Find (Item, Name);
      if Id = 0 then raise Validation_Error with "missing member: " & Name; end if;
      return Handle (Item.Owner, Id);
   end Get;
   function Get (Item : Value) return Boolean is
   begin
      Require_Kind (Item, JSON_Boolean_Type);
      return Item.Owner.Input (Item.Owner.Items (Item.Id).Start) = 't';
   end Get;
   function Get (Item : Value) return String is
   begin
      Require_Kind (Item, JSON_String_Type);
      declare N : Node renames Item.Owner.Items (Item.Id);
      begin return Item.Owner.Bytes (N.Text_Start .. N.Text_End); end;
   end Get;
   function Numeric_Lexeme (Item : Value) return String is
   begin
      if Kind (Item) not in JSON_Int_Type | JSON_Float_Type then
         raise Validation_Error with "expected numeric occurrence at byte" & First (Item)'Image;
      end if;
      declare N : Node renames Item.Owner.Items (Item.Id);
      begin return Item.Owner.Input (N.Start .. N.Finish); end;
   end Numeric_Lexeme;
   function Get (Item : Value) return Long_Long_Integer is
   begin
      Require_Kind (Item, JSON_Int_Type);
      begin return Long_Long_Integer'Value (Numeric_Lexeme (Item));
      exception when Constraint_Error =>
         raise Validation_Error with "integer outside signed-64 domain at byte" & First (Item)'Image;
      end;
   end Get;
   function Get (Item : Value) return Value_Array is
   begin Require_Kind (Item, JSON_Array_Type); return (Root => Item); end;
   function Length (Items : Value_Array) return Natural is
   begin
      if Items.Root.Owner = null then return 0; end if;
      Require_Kind (Items.Root, JSON_Array_Type);
      return Items.Root.Owner.Items (Items.Root.Id).Count;
   end Length;
   function Get (Items : Value_Array; Index : Positive) return Value is
   begin
      if Index > Length (Items) then
         raise Validation_Error with "JSON array index out of range";
      end if;
      declare
         D : Document renames Items.Root.Owner.all;
         N : Node renames D.Items (Items.Root.Id);
      begin
         return Handle (Items.Root.Owner, D.Elements (N.Index_Start + Index - 1));
      end;
   end Get;
   function Floating (Item : Value) return AMS_GRA_OMS_JSON_Float32.Float32 is
   begin
      if Kind (Item) = JSON_String_Type then
         return AMS_GRA_OMS_JSON_Float32.Special (Get (Item));
      end if;
      return AMS_GRA_OMS_JSON_Float32.Number (Numeric_Lexeme (Item));
   end Floating;
   procedure Map_JSON_Object
     (Item : Value; Process : not null access procedure (Name : String; Child : Value))
   is
      Id : Natural;
   begin
      Require_Kind (Item, JSON_Object_Type);
      Id := Item.Owner.Items (Item.Id).Child;
      while Id /= 0 loop
         declare N : Node renames Item.Owner.Items (Id);
         begin
            Process (Item.Owner.Bytes (N.Key_Start .. N.Key_End),
                     Handle (Item.Owner, N.Selected));
            Id := N.Next;
         end;
      end loop;
   end Map_JSON_Object;

   function Read (Text : String) return Value is
   begin
      AMS_GRA_OMS_JSON_Float32.Require_Platform;
      Guard.Validate_Encoded_Document (Text);
      return Result : Value do
         --  Install the fully initialized owner before any fallible substorage
         --  allocation. Extended return finalization owns all failure cleanup.
         Result.Owner := new Document;
         declare
            D : Document renames Result.Owner.all;
            Capacity : constant Positive := Positive'Min (Guard.Max_Nodes, Text'Length);
         begin
            --  Normalize arbitrary Ada String bounds for provider byte spans.
            D.Input := new String (1 .. Text'Length);
            D.Input.all := Text;
            D.Bytes := new String (1 .. Text'Length);
            D.Items := new Nodes (1 .. Capacity);
            D.Elements := new Indices (1 .. Capacity);
            declare
               Data : GNATCOLL.Buffer.Reader := GNATCOLL.Buffer.Open_String (D.Input.all);
               Parser : JSON_Parser;
               Event : JSON_Parser_Event;
               type Frame is record
                  Id : Natural := 0;
                  Key_Start : Positive := 1;
                  Key_End : Natural := 0;
                  Expect_Key : Boolean := True;
               end record;
               Stack : array (Positive range 1 .. Guard.Max_Depth) of Frame;
               Level : Natural := 0;

               procedure Decode_String (Start, Finish : Natural;
                                        From : out Positive; Through : out Natural)
               is
                  P : Natural := Start + 1;
                  Last_Content : constant Natural := Finish - 1;
                  function Unit_16 return Natural is
                     Unit : Natural := 0;
                     C : Character;
                  begin
                     for I in 1 .. 4 loop
                        C := D.Input (P); P := P + 1;
                        Unit := Unit * 16 +
                          (if C in '0' .. '9' then Character'Pos (C) - 48
                           elsif C in 'A' .. 'F' then Character'Pos (C) - 55
                           else Character'Pos (C) - 87);
                     end loop;
                     return Unit;
                  end Unit_16;
                  procedure Put (Byte : Natural) is
                  begin
                     if D.Bytes_Used = D.Bytes'Length then
                        raise Program_Error with "validated string expansion invariant";
                     end if;
                     D.Bytes (D.Bytes_Used + 1) := Character'Val (Byte);
                     D.Bytes_Used := D.Bytes_Used + 1;
                  end Put;
                  Code, Low : Natural;
               begin
                  --  C4 has already validated this exact original document.
                  --  Decoded UTF-8 never exceeds the source token content size.
                  From := D.Bytes_Used + 1;
                  while P <= Last_Content loop
                     if D.Input (P) /= '\' then
                        Put (Character'Pos (D.Input (P))); P := P + 1;
                     else
                        P := P + 1;
                        case D.Input (P) is
                           when '"' | '\' | '/' => Put (Character'Pos (D.Input (P))); P := P + 1;
                           when 'b' => Put (8); P := P + 1;
                           when 'f' => Put (12); P := P + 1;
                           when 'n' => Put (10); P := P + 1;
                           when 'r' => Put (13); P := P + 1;
                           when 't' => Put (9); P := P + 1;
                           when 'u' =>
                              P := P + 1; Code := Unit_16;
                              if Code in 16#D800# .. 16#DBFF# then
                                 P := P + 2; Low := Unit_16;
                                 Code := 16#10000# + (Code - 16#D800#) * 1024 + Low - 16#DC00#;
                              end if;
                              if Code <= 16#7F# then Put (Code);
                              elsif Code <= 16#7FF# then
                                 Put (16#C0# + Code / 64); Put (16#80# + Code mod 64);
                              elsif Code <= 16#FFFF# then
                                 Put (16#E0# + Code / 4096);
                                 Put (16#80# + Code / 64 mod 64); Put (16#80# + Code mod 64);
                              else
                                 Put (16#F0# + Code / 262144);
                                 Put (16#80# + Code / 4096 mod 64);
                                 Put (16#80# + Code / 64 mod 64); Put (16#80# + Code mod 64);
                              end if;
                           when others => raise Program_Error with "validated escape invariant";
                        end case;
                     end if;
                  end loop;
                  Through := D.Bytes_Used;
               end Decode_String;

               function Height (Id : Natural) return Natural is
                 (if Id = 0 then 0 else D.Items (Id).Height);
               procedure Update (Id : Positive) is
               begin D.Items (Id).Height := 1 + Natural'Max
                 (Height (D.Items (Id).Left), Height (D.Items (Id).Right)); end;
               function Rotate_Left (Id : Positive) return Positive is
                  Top : constant Positive := D.Items (Id).Right;
               begin
                  D.Items (Id).Right := D.Items (Top).Left;
                  D.Items (Top).Left := Id; Update (Id); Update (Top); return Top;
               end;
               function Rotate_Right (Id : Positive) return Positive is
                  Top : constant Positive := D.Items (Id).Left;
               begin
                  D.Items (Id).Left := D.Items (Top).Right;
                  D.Items (Top).Right := Id; Update (Id); Update (Top); return Top;
               end;
               function Insert (Root : Natural; Id : Positive; Added : out Boolean)
                 return Positive
               is
                  New_Node : Node renames D.Items (Id);
               begin
                  if Root = 0 then Added := True; return Id; end if;
                  declare
                     N : Node renames D.Items (Root);
                     Key : String renames D.Bytes (New_Node.Key_Start .. New_Node.Key_End);
                     Existing : String renames D.Bytes (N.Key_Start .. N.Key_End);
                  begin
                     if Key = Existing then
                        N.Selected := Id; Added := False; return Root;
                     elsif Key < Existing then N.Left := Insert (N.Left, Id, Added);
                     else N.Right := Insert (N.Right, Id, Added);
                     end if;
                     Update (Root);
                     if Height (N.Left) > Height (N.Right) + 1 then
                        if Height (D.Items (N.Left).Right) > Height (D.Items (N.Left).Left) then
                           N.Left := Rotate_Left (N.Left);
                        end if;
                        return Rotate_Right (Root);
                     elsif Height (N.Right) > Height (N.Left) + 1 then
                        if Height (D.Items (N.Right).Left) > Height (D.Items (N.Right).Right) then
                           N.Right := Rotate_Right (N.Right);
                        end if;
                        return Rotate_Left (Root);
                     end if;
                     return Root;
                  end;
               end Insert;

               procedure Attach (Id : Positive) is
                  Parent : Node renames D.Items (Stack (Level).Id);
                  Added : Boolean := True;
               begin
                  if Parent.K = JSON_Object_Type then
                     D.Items (Id).Key_Start := Stack (Level).Key_Start;
                     D.Items (Id).Key_End := Stack (Level).Key_End;
                     Parent.Root := Insert (Parent.Root, Id, Added);
                     Stack (Level).Expect_Key := True;
                  end if;
                  if Added then
                     if Parent.Tail = 0 then Parent.Child := Id;
                     else D.Items (Parent.Tail).Next := Id;
                     end if;
                     Parent.Tail := Id; Parent.Count := Parent.Count + 1;
                  end if;
               end Attach;
            begin
               loop
                  Event := Parse_Next (Parser, Data);
                  exit when Event.Kind = DOC_END;
                  if Event.Kind in ARRAY_END | OBJECT_END then
                     declare
                        N : Node renames D.Items (Stack (Level).Id);
                        Child : Natural := N.Child;
                     begin
                        N.Finish := Natural (GNATCOLL.Buffer.Current_Position (Data));
                        if N.K = JSON_Array_Type then
                           N.Index_Start := D.Elements_Used + 1;
                           while Child /= 0 loop
                              D.Elements_Used := D.Elements_Used + 1;
                              D.Elements (D.Elements_Used) := Child;
                              Child := D.Items (Child).Next;
                           end loop;
                        end if;
                     end;
                     Level := Level - 1;
                  elsif Level > 0 and then
                    D.Items (Stack (Level).Id).K = JSON_Object_Type and then Stack (Level).Expect_Key
                  then
                     Decode_String (Natural (Event.First), Natural (Event.Last),
                                    Stack (Level).Key_Start, Stack (Level).Key_End);
                     Stack (Level).Expect_Key := False;
                  else
                     D.Used := D.Used + 1;
                     declare
                        Id : constant Positive := D.Used;
                        N : Node renames D.Items (Id);
                        Finish : constant Natural := Natural (GNATCOLL.Buffer.Current_Position (Data));
                     begin
                        N.Selected := Id;
                        N.Start := (case Event.Kind is
                          when INTEGER_VALUE | NUMBER_VALUE | STRING_VALUE => Natural (Event.First),
                          when TRUE_VALUE | NULL_VALUE => Finish - 3,
                          when FALSE_VALUE => Finish - 4,
                          when others => Finish);
                        N.Finish := Finish;
                        N.K := (case Event.Kind is
                          when INTEGER_VALUE => JSON_Int_Type, when NUMBER_VALUE => JSON_Float_Type,
                          when STRING_VALUE => JSON_String_Type,
                          when TRUE_VALUE | FALSE_VALUE => JSON_Boolean_Type,
                          when ARRAY_START => JSON_Array_Type, when OBJECT_START => JSON_Object_Type,
                          when NULL_VALUE => JSON_Null_Type,
                          when others => raise Program_Error with "validated event invariant");
                        if Event.Kind in INTEGER_VALUE | NUMBER_VALUE | STRING_VALUE then
                           N.Finish := Natural (Event.Last);
                        end if;
                        if N.K = JSON_String_Type then
                           Decode_String (N.Start, N.Finish, N.Text_Start, N.Text_End);
                        end if;
                        if Level > 0 then Attach (Id); end if;
                        if Event.Kind in ARRAY_START | OBJECT_START then
                           Level := Level + 1;
                           Stack (Level) := (Id => Id, others => <>);
                        end if;
                     end;
                  end if;
               end loop;
               Result.Id := 1;
            end;
         end;
      end return;
   exception
      when Guard.Validation_Error | Invalid_JSON_Stream =>
         raise Validation_Error with "invalid JSON document";
   end Read;
end AMS_GRA_OMS_JSON_Lossless_Ingress;