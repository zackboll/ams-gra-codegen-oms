with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Unchecked_Conversion;
with Interfaces;
with Interfaces.C;
with Programs.Oam;
with Task096_Adapter;
with Task096_Boundary;
procedure Document_Probe is
   package Model renames Programs.Oam;
   Item : Model.FloatPayload :=
     (Required => 42.0, Optional => (Is_Present => False), Named => 0.0,
      Closed => Model.Create (Interfaces.IEEE_Float_64'(0.0)),
      Open => Model.Create (Interfaces.IEEE_Float_64'(1.0)),
      Bounded => <>, Unbounded => (Required => (0.0, 0.0), Additional => <>),
      NamedRepeated => Model.To_Sequence ((1 => Model.Create (Interfaces.IEEE_Float_64'(0.0)))),
      Nested => (Value => 0.0), Choice => (Model.Alternative_Record_Kind, (Value => 0.0)),
      Small => 0.0, Count => 0, Enabled => False, Text => To_Unbounded_String ("sentinel"));
   Input : File_Type;
   Text : Unbounded_String;
   Before, After : Unbounded_String;
   Want_Reject : constant Boolean := Ada.Command_Line.Argument (1) = "reject";
   function From_Bits is new Ada.Unchecked_Conversion (Interfaces.Unsigned_64, Interfaces.IEEE_Float_64);
   function Locale_Enter return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_locale_enter";
   function Locale_Leave return Interfaces.C.int
     with Import, Convention => C, External_Name => "task096_locale_leave";
   use type Interfaces.C.int;
   Locale_Active : Boolean := False;
begin
   if Ada.Command_Line.Argument (1) = "locale" then
      if Locale_Enter /= 0 then raise Program_Error with "locale setup"; end if;
      Locale_Active := True;
   end if;
   if Ada.Command_Line.Argument (1) = "encode-corpus" then
      Open (Input, In_File, Ada.Command_Line.Argument (2));
      while not End_Of_File (Input) loop
         declare
            Pattern : constant String := Get_Line (Input);
         begin
            Item.Required := From_Bits (Interfaces.Unsigned_64'Value ("16#" & Pattern & "#"));
            Item.Optional := (True, Item.Required);
            Item.Named := Model.Unconstrained (Item.Required);
            Item.Nested.Value := Item.Required;
            Item.Choice := (Model.Alternative_Record_Kind, (Value => Item.Required));
            Item.Bounded := Model.To_Sequence ((1 => Item.Required));
            Item.Unbounded.Required := (others => Item.Required);
            Task096_Adapter.Encode (Item, After);
            Put_Line (Pattern & ASCII.HT & To_String (After));
         end;
      end loop;
      Close (Input);
      Put_Line ("TASK096_DOCUMENT_ENCODING_COMPLETE");
      return;
   elsif Ada.Command_Line.Argument (1) = "encode-invalid" then
      Item.Text := To_Unbounded_String ("invalid" & Character'Val (16#FF#));
      After := To_Unbounded_String ("unchanged");
      begin
         Task096_Adapter.Encode (Item, After);
         raise Program_Error with "invalid output accepted";
      exception
         when Task096_Adapter.Codec_Error =>
            if After /= "unchanged" then raise Program_Error with "encode not atomic"; end if;
      end;
      Put_Line ("TASK096_ENCODE_REJECT_ATOMIC");
      return;
   elsif Ada.Command_Line.Argument (1) = "specials" then
      for Index in 1 .. 3 loop
         declare
            Text : constant String := (case Index is when 1 => "NaN", when 2 => "Infinity", when 3 => "-Infinity", when others => "");
         begin
            Item.Required := Task096_Boundary.Special (Text);
            Task096_Adapter.Encode (Item, After);
            Put_Line (To_String (After));
         end;
      end loop;
      Put_Line ("TASK096_SPECIAL_DOCUMENTS_COMPLETE");
      return;
   end if;
   Open (Input, In_File, Ada.Command_Line.Argument (2));
   while not End_Of_File (Input) loop
      Append (Text, Get_Line (Input));
      Append (Text, ASCII.LF);
   end loop;
   Close (Input);
   Task096_Adapter.Encode (Item, Before);
   begin
      Task096_Adapter.Decode (To_String (Text), Item);
      if Want_Reject then raise Program_Error with "accepted rejected fixture"; end if;
   exception
      when E : Task096_Adapter.Codec_Error =>
         if not Want_Reject then raise; end if;
         Task096_Adapter.Encode (Item, After);
         if After /= Before then raise Program_Error with "decode publication not atomic"; end if;
         Put_Line ("TASK096_REJECT_ATOMIC " & Ada.Exceptions.Exception_Message (E));
         return;
   end;
   Task096_Adapter.Encode (Item, After);
   if Locale_Active then
      if Locale_Leave /= 0 then raise Program_Error with "locale not restored"; end if;
      Locale_Active := False;
   end if;
   Put_Line (To_String (After));
   Put_Line ("TASK096_DOCUMENT_COMPLETE");
exception
   when E : others =>
      if Locale_Active then
         declare Status : constant Interfaces.C.int := Locale_Leave;
         begin if Status /= 0 then Put_Line ("LOCALE_RESTORATION_FAILED"); end if; end;
      end if;
      Put_Line ("INCOMPLETE " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Document_Probe;