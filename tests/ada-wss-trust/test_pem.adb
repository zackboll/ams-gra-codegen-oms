with Ada.Streams.Stream_IO;

package body Test_PEM is
   function Read (Path : String) return String is
      use Ada.Streams;
      package IO renames Ada.Streams.Stream_IO;
      File : IO.File_Type;
   begin
      IO.Open (File, IO.In_File, Path);
      declare
         Bytes : Stream_Element_Array (1 .. Stream_Element_Offset (IO.Size (File)));
         Last : Stream_Element_Offset;
         Result : String (1 .. Bytes'Length);
      begin
         IO.Read (File, Bytes, Last);
         IO.Close (File);
         if Last /= Bytes'Last then raise Program_Error with "short PEM read"; end if;
         for I in Result'Range loop
            Result (I) := Character'Val (Bytes (Stream_Element_Offset (I)));
         end loop;
         return Result;
      end;
   exception
      when others =>
         if IO.Is_Open (File) then IO.Close (File); end if;
         raise;
   end Read;
end Test_PEM;