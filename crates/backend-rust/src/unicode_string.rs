use ams_gra_oms_codegen_core::{UNICODE31_ND, UnicodeStringProfile, UnicodeStringToken};

pub(super) fn support() -> String {
    let nd = UNICODE31_ND
        .iter()
        .map(|(a, b)| format!("0x{a:X}..=0x{b:X}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let mut source = format!(
        "struct Unicode31StringValidator;\nimpl Unicode31StringValidator {{\n    fn accepts(text: &str, profile: u8) -> bool {{\n        let mut c = [0_u32; 21];\n        let mut count = 0;\n        for scalar in text.chars() {{\n            if count == c.len() {{ return false; }}\n            c[count] = scalar as u32; count += 1;\n        }}\n        let nd = |value: u32| matches!(value, {nd});\n        match profile {{\n"
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
                        UnicodeStringToken::DecimalDigit => format!("nd(c[{i}])"),
                        UnicodeStringToken::Ascii(set) => format!(
                            "matches!(c[{i}], {})",
                            set.bytes()
                                .map(|b| u32::from(b).to_string())
                                .collect::<Vec<_>>()
                                .join(" | ")
                        ),
                    })
                    .collect::<Vec<_>>()
                    .join(" && ")
            })
            .map(|s| format!("({s})"))
            .collect::<Vec<_>>()
            .join(" ||\n                ");
        source.push_str(&format!(
            "            {} => count == {} && ({branches}),\n",
            profile.id(),
            profile.length()
        ));
    }
    source.push_str("            _ => false,\n        }\n    }\n}\n");
    source
}

pub(super) fn render(name: &str, profile: UnicodeStringProfile) -> String {
    format!(
        "#[derive(Clone, Debug, PartialEq, Eq)]\npub struct {name} {{ value: String }}\nimpl {name} {{\n    pub fn new(value: &str) -> Option<Self> {{\n        if !Unicode31StringValidator::accepts(value, {}) {{ return None; }}\n        Some(Self {{ value: value.to_owned() }})\n    }}\n    pub fn as_str(&self) -> &str {{ &self.value }}\n}}\n",
        profile.id()
    )
}
