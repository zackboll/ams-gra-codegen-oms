with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Streams;
with Ada.Streams.Stream_IO;
with Ada.Text_IO;
with Interfaces.C;
with Allocation_Faults;
with AMS_GRA_OMS_JSON_Guard;
with GNATCOLL.JSON;
procedure C4_Validate is
   package Guard renames AMS_GRA_OMS_JSON_Guard;
   package IO renames Ada.Streams.Stream_IO;
   use type Ada.Streams.Stream_Element_Offset;
   use type Interfaces.C.size_t;
   use type Ada.Exceptions.Exception_Id;
   function Read_File return String is
      F : IO.File_Type;
   begin
      IO.Open (F, IO.In_File, Ada.Command_Line.Argument (2));
      declare
         Size : constant Natural := Natural (IO.Size (F));
         Buffer : Ada.Streams.Stream_Element_Array
           (1 .. Ada.Streams.Stream_Element_Offset (Size));
         Last : Ada.Streams.Stream_Element_Offset;
         Result : String (1 .. Size);
      begin
         IO.Read (F, Buffer, Last); IO.Close (F);
         if Last /= Buffer'Last then raise Program_Error; end if;
         for J in Result'Range loop
            Result (J) := Character'Val (Buffer (Ada.Streams.Stream_Element_Offset (J)));
         end loop;
         return Result;
      end;
   end Read_File;
   Text : constant String := Read_File;
   Mode : constant String := Ada.Command_Line.Argument (1);
   Budget : constant Natural := Natural'Value (Ada.Command_Line.Argument (3));
   First : constant Positive :=
     (if Ada.Command_Line.Argument_Count < 4 then 1
      elsif Ada.Command_Line.Argument (4) = "last" then Integer'Last - Text'Length + 1
      else Positive'Value (Ada.Command_Line.Argument (4)));
   Input : String (First .. First + (Text'Length - 1)) := Text;
   Accepted : Boolean := False;
   Unexpected : Ada.Exceptions.Exception_Occurrence;
   Bad : Boolean := False;
begin
   Allocation_Faults.Arm (Budget);
   begin
      if Mode = "internal" then
         raise Program_Error with "unrelated C4 internal negative control";
      elsif Mode = "old" then
         declare
            Value : constant GNATCOLL.JSON.JSON_Value := Guard.Read (Input);
            pragma Unreferenced (Value);
         begin null; end;
      else
         Guard.Validate_Encoded_Document (Input);
      end if;
      Allocation_Faults.Disarm;
      Accepted := True;
   exception
      when Guard.Validation_Error => Allocation_Faults.Disarm;
      when E : others =>
         Allocation_Faults.Disarm;
         Bad := True;
         Ada.Exceptions.Save_Occurrence (Unexpected, E);
   end;
   if Bad then
      Ada.Text_IO.Put_Line ("C4_EXCEPTION " & Ada.Exceptions.Exception_Name (Unexpected));
   else
      Ada.Text_IO.Put_Line ((if Accepted then "C4_ACCEPT" else "C4_REJECT"));
   end if;
   Ada.Text_IO.Put_Line ("C4_COUNTS attempts=" & Allocation_Faults.Attempts'Image &
      " failures=" & Allocation_Faults.Failures'Image & " size=" & Allocation_Faults.Failed_Size'Image);
   if Mode = "inject" then
      Guard.Validate_Encoded_Document (Input);
      if Allocation_Faults.Failures /= 1 then raise Program_Error with "failure not reached"; end if;
      if not Bad then raise Program_Error with "injected resource failure missing"; end if;
      if Ada.Exceptions.Exception_Identity (Unexpected) /= Storage_Error'Identity then
         raise Program_Error with "wrong exception category";
      end if;
      Ada.Text_IO.Put_Line ("C4_VALIDATION_RECOVERY_COMPLETE");
   end if;
   if Mode = "internal" then
      if not Bad or else Ada.Exceptions.Exception_Identity (Unexpected) /= Program_Error'Identity then
         raise Constraint_Error with "internal exception hidden";
      end if;
      Guard.Validate_Encoded_Document (Input);
      Ada.Text_IO.Put_Line ("C4_INTERNAL_ERROR_RECOVERY_COMPLETE");
   end if;
end C4_Validate;