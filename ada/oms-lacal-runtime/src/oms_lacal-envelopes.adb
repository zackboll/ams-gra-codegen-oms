with AMS_GRA_OMS_JSON_Guard;
with GNATCOLL.JSON;

package body OMS_LACAL.Envelopes is
   package Guard renames AMS_GRA_OMS_JSON_Guard;

   function Wrap (Name, Payload : String) return String is
   begin
      Guard.Validate_Encoded_Document (Payload);
      declare
         Result : constant String := "{" &
           Guard.Write (GNATCOLL.JSON.Create (Name)) & ":" & Payload & "}";
      begin
         if Result'Length > Max_Frame_Bytes then
            raise Envelope_Error with "envelope size limit";
         end if;
         return Result;
      end;
   exception
      when Guard.Validation_Error =>
         raise Envelope_Error with "invalid encoded payload";
   end Wrap;

   function Extract (Name, Document : String) return String is
      --  The full bounded SAX/UTF-8 validator runs first. This scanner only
      --  locates spans in known-valid JSON; it never interprets a number.
      T : constant String (1 .. Document'Length) := Document;
      P : Natural := 1;
      Key_First, Key_Last, Payload_First, Payload_Last : Natural;
      Depth : Natural := 0;
      In_String, Escaped : Boolean := False;
      procedure White is
      begin
         while P <= T'Last and then T (P) in ' ' | ASCII.HT | ASCII.LF | ASCII.CR loop
            P := P + 1;
         end loop;
      end White;
      procedure Require (C : Character) is
      begin
         if P > T'Last or else T (P) /= C then
            raise Envelope_Error with "expected exactly one object member";
         end if;
         P := P + 1;
      end Require;
   begin
      Guard.Validate_Encoded_Document (T);
      White;
      Require ('{');
      White;
      Key_First := P;
      Require ('"');
      while P <= T'Last loop
         if T (P) = '"' and then not Escaped then
            exit;
         elsif T (P) = '\' and then not Escaped then
            Escaped := True;
         else
            Escaped := False;
         end if;
         P := P + 1;
      end loop;
      Require ('"');
      Key_Last := P - 1;
      --  Only the key is decoded through the DOM. Payload bytes never are.
      if String'(GNATCOLL.JSON.Get (Guard.Read (T (Key_First .. Key_Last)))) /= Name then
         raise Envelope_Error with "wrong global-element QName";
      end if;
      White;
      Require (':');
      White;
      Payload_First := P;
      Escaped := False;
      while P <= T'Last loop
         if In_String then
            if Escaped then
               Escaped := False;
            elsif T (P) = '\' then
               Escaped := True;
            elsif T (P) = '"' then
               In_String := False;
            end if;
         else
            case T (P) is
               when '"' => In_String := True;
               when '{' | '[' => Depth := Depth + 1;
               when '}' | ']' =>
                  exit when Depth = 0;
                  Depth := Depth - 1;
               when ',' =>
                  if Depth = 0 then
                     raise Envelope_Error with "multiple original envelope members";
                  end if;
               when others => null;
            end case;
         end if;
         P := P + 1;
      end loop;
      Payload_Last := P - 1;
      Require ('}');
      White;
      if P <= T'Last then
         raise Envelope_Error with "trailing envelope bytes";
      end if;
      return T (Payload_First .. Payload_Last);
   exception
      when Guard.Validation_Error =>
         raise Envelope_Error with "malformed envelope";
   end Extract;
end OMS_LACAL.Envelopes;