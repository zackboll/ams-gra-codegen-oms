//! Nested scanner functions use offsets, never one-past-end absolute indexes.
pub(super) fn render(name: &str) -> String {
    TEMPLATE.replace("{name}", name)
}

const TEMPLATE: &str = r#"   function Create (Value : String) return {name} is
      function Valid (Text : String) return Boolean is
         function At_Offset (Offset : Natural) return Character is
           (Text (Text'First + Offset));
         function Hex_Byte (B : Character) return Boolean is
           (B in '0' .. '9' | 'a' .. 'f' | 'A' .. 'F');
         function Hex_Group (Start, Finish : Natural) return Boolean is
         begin
            if Finish - Start > 4 then return False; end if;
            for Offset in Start .. Finish - 1 loop
               if not Hex_Byte (At_Offset (Offset)) then return False; end if;
            end loop;
            return True;
         end Hex_Group;
         function Hex_Colon (Start : Natural; Finish : out Natural) return Boolean is
            Pos : Natural := Start;
         begin
            while Pos < Text'Length and then Hex_Byte (At_Offset (Pos)) loop
               Pos := Pos + 1;
            end loop;
            Finish := Pos;
            if Pos - Start <= 4 and then Pos < Text'Length
              and then At_Offset (Pos) = ':'
            then
               Finish := Pos + 1;
               return True;
            end if;
            return False;
         end Hex_Colon;
         function Colon (Start, Finish : Natural) return Boolean is
           (Finish - Start = 1 and then At_Offset (Start) = ':');
         function IPv4_Octet (Start, Finish : Natural) return Boolean is
            Number : Natural := 0;
            B : Character;
         begin
            if Finish = Start or else Finish - Start > 3 then return False; end if;
            for Offset in Start .. Finish - 1 loop
               B := At_Offset (Offset);
               if B not in '0' .. '9' then return False; end if;
               Number := Number * 10 + Character'Pos (B) - Character'Pos ('0');
            end loop;
            return Number <= 255;
         end IPv4_Octet;
         function Embedded_IPv4 (Start : Natural) return Boolean is
            Part : Natural := Start;
            Count : Natural := 0;
         begin
            for Pos in Start .. Text'Length loop
               if Pos = Text'Length or else At_Offset (Pos) = '.' then
                  if not IPv4_Octet (Part, Pos) then return False; end if;
                  Count := Count + 1;
                  if Pos = Text'Length then exit; end if;
                  Part := Pos + 1;
               end if;
            end loop;
            return Count = 4;
         end Embedded_IPv4;
         function Suffix (Start : Natural) return Boolean is
            Finish : Natural;
         begin
            if Hex_Group (Start, Text'Length) or else Colon (Start, Text'Length) then
               return True;
            end if;
            if Hex_Colon (Start, Finish) and then
              (Hex_Group (Finish, Text'Length) or else Colon (Finish, Text'Length))
            then
               return True;
            end if;
            return Embedded_IPv4 (Start);
         end Suffix;
         function After_Prefix (Start : Natural) return Boolean is
            Pos : Natural := Start;
            Finish : Natural;
         begin
            for Count in 0 .. 5 loop
               if Suffix (Pos) then return True; end if;
               if Count = 5 or else not Hex_Colon (Pos, Finish) then exit; end if;
               Pos := Finish;
            end loop;
            return False;
         end After_Prefix;
         Finish : Natural;
      begin
         if Text'Length not in 2 .. 45 then return False; end if;
         if Hex_Colon (0, Finish) and then After_Prefix (Finish) then return True; end if;
         return At_Offset (0) = ':' and then At_Offset (1) = ':' and then After_Prefix (2);
      end Valid;
   begin
      if not Valid (Value) then
         raise Constraint_Error with "invalid IPv6 profile string";
      end if;
      return {name}'(Text => Standard.Ada.Strings.Unbounded.To_Unbounded_String (Value));
   end Create;
   function Value (Item : {name}) return String is
     (Standard.Ada.Strings.Unbounded.To_String (Item.Text));
"#;

#[cfg(test)]
#[path = "../../../tests/task062_probes.rs"]
mod probes;

#[cfg(test)]
mod tests {
    #[test]
    fn standalone_ipv6_compiler_corpus_and_lifecycle() {
        super::probes::run("ada", &super::render("Address"));
    }
}
