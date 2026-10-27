with AMS_GRA_OMS_JSON_Guard;

package body OMS_LACAL is
   function Valid_Identifier (Value : String) return Boolean is
   begin
      return Value'Length in 1 .. 256 and then
        (for all C of Value => C in 'a' .. 'z' | 'A' .. 'Z' | '0' .. '9'
          | '_' | '-' | '.');
   end Valid_Identifier;

   function QName (Namespace, Local_Name : String) return String is
   begin
      if not Valid_Identifier (Local_Name)
        or else Local_Name (Local_Name'First) not in 'A' .. 'Z' | 'a' .. 'z' | '_'
        or else Namespace'Length not in 1 .. 2048
        or else (for some C of Namespace => Character'Pos (C) <= 32
                   or else C in '{' | '}' | Character'Val (127))
      then
         raise Runtime_Error with "invalid global-element identity";
      end if;
      AMS_GRA_OMS_JSON_Guard.Validate_String (Namespace);
      if Namespace = "https://www.vdl.afrl.af.mil/programs/oam" then
         return Local_Name;
      end if;
      return "{" & Namespace & "}" & Local_Name;
   exception
      when AMS_GRA_OMS_JSON_Guard.Validation_Error =>
         raise Runtime_Error with "invalid namespace UTF-8";
   end QName;
end OMS_LACAL;