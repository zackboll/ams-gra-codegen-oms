use ams_gra_oms_codegen_core::{UNICODE31_ND, UnicodeStringProfile, UnicodeStringToken};

pub(super) fn support() -> String {
    let nd = UNICODE31_ND
        .iter()
        .map(|(a, b)| format!("16#{a:X}# .. 16#{b:X}#"))
        .collect::<Vec<_>>()
        .join(" | ");
    let mut source = format!(
        "   function Unicode31_String_Valid (Text : String; Profile : Natural) return Boolean is\n      type Scalars is array (Natural range 0 .. 20) of Natural;\n      C : Scalars := (others => 0);\n      Count, Pos : Natural := 0;\n      Lead, B, Scalar, Extra, Minimum : Natural;\n      function Byte (Offset : Natural) return Natural is\n        (Character'Pos (Text (Text'First + Offset)));\n      function Nd (Value : Natural) return Boolean is\n        (Value in {nd});\n   begin\n      --  Bound scalar storage, not absolute String indices. Never form an\n      --  index at Text'Length; Pos is advanced only as a zero-based offset.\n      while Pos < Text'Length loop\n         if Count = C'Length then return False; end if;\n         Lead := Byte (Pos); Pos := Pos + 1;\n         if Lead <= 16#7F# then Scalar := Lead; Extra := 0; Minimum := 0;\n         elsif Lead in 16#C2# .. 16#DF# then Scalar := Lead mod 32; Extra := 1; Minimum := 16#80#;\n         elsif Lead in 16#E0# .. 16#EF# then Scalar := Lead mod 16; Extra := 2; Minimum := 16#800#;\n         elsif Lead in 16#F0# .. 16#F4# then Scalar := Lead mod 8; Extra := 3; Minimum := 16#10000#;\n         else return False; end if;\n         if Text'Length - Pos < Extra then return False; end if;\n         for I in 1 .. Extra loop\n            B := Byte (Pos); Pos := Pos + 1;\n            if B not in 16#80# .. 16#BF# then return False; end if;\n            Scalar := Scalar * 64 + B mod 64;\n         end loop;\n         if Scalar < Minimum or else Scalar > 16#10FFFF# or else Scalar in 16#D800# .. 16#DFFF# then return False; end if;\n         C (Count) := Scalar; Count := Count + 1;\n      end loop;\n      case Profile is\n"
    );
    for profile in UnicodeStringProfile::ALL {
        let branches = profile
            .branches()
            .iter()
            .map(|branch| {
                branch
                    .iter()
                    .enumerate()
                    .map(|(i, t)| match t {
                        UnicodeStringToken::DecimalDigit => format!("Nd (C ({i}))"),
                        UnicodeStringToken::Ascii(set) => format!(
                            "C ({i}) in {}",
                            set.bytes()
                                .map(|b| u32::from(b).to_string())
                                .collect::<Vec<_>>()
                                .join(" | ")
                        ),
                    })
                    .collect::<Vec<_>>()
                    .join(" and then ")
            })
            .map(|s| format!("({s})"))
            .collect::<Vec<_>>()
            .join(" or else\n              ");
        source.push_str(&format!(
            "         when {} => return Count = {} and then ({branches});\n",
            profile.id(),
            profile.length()
        ));
    }
    source.push_str(
        "         when others => return False;\n      end case;\n   end Unicode31_String_Valid;\n",
    );
    source
}

pub(super) fn render(name: &str, profile: UnicodeStringProfile) -> String {
    format!(
        "   function Create (Value : String) return {name} is\n   begin\n      if not Unicode31_String_Valid (Value, {}) then\n         raise Constraint_Error with \"invalid Unicode 3.1 profile string\";\n      end if;\n      return {name}'(Text => Standard.Ada.Strings.Unbounded.To_Unbounded_String (Value));\n   end Create;\n   function Value (Item : {name}) return String is\n     (Standard.Ada.Strings.Unbounded.To_String (Item.Text));\n",
        profile.id()
    )
}
