//! Create-local scanner and collapse: no generated package-level helper name.
pub(super) fn body() -> String {
    let source = super::ADA_DATE_TIME_ZULU_BODY;
    let start = source
        .find("      function Collapse (Raw : String) return String is")
        .expect("collapse start");
    let end =
        source.find("      end Collapse;").expect("collapse end") + "      end Collapse;".len();
    BODY.replace("{collapse}", &source[start..end])
}

const BODY: &str = r#"
   function Create (Value : String) return {name} is
{collapse}
      function Valid (Text : String) return Boolean is
         function Two (From : Positive; Result : out Natural) return Boolean is
         begin
            Result := 0;
            if Text (From) not in '0' .. '9'
              or else Text (From + 1) not in '0' .. '9'
            then
               return False;
            end if;
            Result := (Character'Pos (Text (From)) - Character'Pos ('0')) * 10
              + Character'Pos (Text (From + 1)) - Character'Pos ('0');
            return True;
         end Two;
         Hour, Minute, Second : Natural;
         Zero : Boolean := True;
      begin
         --  Collapse returns a 1-based string; input slice bounds are irrelevant.
         if Text'Length < 9 or else Text (Text'Last) /= 'Z'
           or else Text (Text'First + 2) /= ':' or else Text (Text'First + 5) /= ':'
           or else not Two (Text'First, Hour) or else not Two (Text'First + 3, Minute)
           or else not Two (Text'First + 6, Second)
         then
            return False;
         end if;
         --  XSD 1.0 Appendix D: seconds 60 and fractional seconds 60 are lexical.
         if Hour > 24 or else Minute > 59 or else Second > 60 then
            return False;
         end if;
         if Text'Length > 9 then
            if Text'Length < 11 or else Text (Text'First + 8) /= '.' then
               return False;
            end if;
            for Index in Text'First + 9 .. Text'Last - 1 loop
               if Text (Index) not in '0' .. '9' then return False; end if;
               if Text (Index) /= '0' then Zero := False; end if;
            end loop;
         end if;
         return Hour /= 24
           or else (Minute = 0 and then Second = 0 and then Zero);
      end Valid;
      Normalized : constant String := Collapse (Value);
   begin
      if not Valid (Normalized) then
         raise Constraint_Error with "not a valid XML Schema Time in Zulu";
      end if;
      return {name}'(Lexical => Standard.Ada.Strings.Unbounded.To_Unbounded_String (Normalized));
   end Create;
   function Value (Item : {name}) return String is
     (Standard.Ada.Strings.Unbounded.To_String (Item.Lexical));
"#;
