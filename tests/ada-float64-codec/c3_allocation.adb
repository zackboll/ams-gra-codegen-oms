with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Text_IO; use Ada.Text_IO;
with Allocation_Faults;
with Interfaces.C;
with Programs.Oam;
with Service_Codec;
procedure C3_Allocation is
   use type Interfaces.C.size_t;
   function Event_Size (Index : Interfaces.C.size_t) return Interfaces.C.size_t
     with Import, Convention => C, External_Name => "task097_c3_event_size";
   function Event_Caller (Index : Interfaces.C.size_t) return Interfaces.C.size_t
     with Import, Convention => C, External_Name => "task097_c3_event_caller";
   function Event_Realloc (Index : Interfaces.C.size_t) return Interfaces.C.int
     with Import, Convention => C, External_Name => "task097_c3_event_realloc";
   function Read_File return String is
      F : File_Type;
      S : Unbounded_String;
   begin
      Open (F, In_File, Ada.Command_Line.Argument (2));
      while not End_Of_File (F) loop Append (S, Get_Line (F)); end loop;
      Close (F); return To_String (S);
   end Read_File;
   Text : constant String := Read_File;
   Item : Programs.Oam.LosslessEnvelope := Service_Codec.Decode_LosslessEnvelope (Text);
   Before : constant String := Service_Codec.Encode_LosslessEnvelope (Item);
   Output : Unbounded_String := To_Unbounded_String ("sentinel");
   Mode : constant String := Ada.Command_Line.Argument (1);
   Budget : constant Natural := Natural'Value (Ada.Command_Line.Argument (3));
   Success : Boolean := False;
   Failure : Ada.Exceptions.Exception_Occurrence;
begin
   Allocation_Faults.Arm (Budget);
   begin
      if Mode = "encode" or Mode = "inventory" then
         Output := To_Unbounded_String (Service_Codec.Encode_LosslessEnvelope (Item));
      elsif Mode = "decode" then
         Item := Service_Codec.Decode_LosslessEnvelope (Text);
      else raise Program_Error with "bad mode"; end if;
      Allocation_Faults.Disarm;
      Success := True;
   exception
      when E : others =>
         Allocation_Faults.Disarm;
         Ada.Exceptions.Save_Occurrence (Failure, E);
   end;
   if Success then
      if Mode /= "decode" and To_String (Output) /= Before then raise Program_Error; end if;
      Put_Line ("C3_ALLOCATION_SUCCESS");
   else
      if Output /= "sentinel" then raise Program_Error with "partial output"; end if;
      Put_Line ("C3_ALLOCATION_EXCEPTION " & Ada.Exceptions.Exception_Name (Failure));
   end if;
   if Service_Codec.Encode_LosslessEnvelope (Item) /= Before then
      raise Program_Error with "retained model corruption";
   end if;
   Put_Line ("C3_ALLOCATION_RECOVERY_COMPLETE attempts=" & Allocation_Faults.Attempts'Image &
     " failures=" & Allocation_Faults.Failures'Image & " size=" & Allocation_Faults.Failed_Size'Image);
   if Mode = "inventory" then
      if Allocation_Faults.Attempts > 65_536 then raise Program_Error with "trace capacity"; end if;
      for Index in 1 .. Allocation_Faults.Attempts loop
         declare I : constant Interfaces.C.size_t := Interfaces.C.size_t (Index - 1);
         begin
            Put_Line ("EVENT " & I'Image & Event_Size (I)'Image & Event_Caller (I)'Image & Event_Realloc (I)'Image);
         end;
      end loop;
   end if;
exception when others => Allocation_Faults.Disarm; raise;
end C3_Allocation;