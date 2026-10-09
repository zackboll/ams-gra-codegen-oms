use ams_gra_oms_codegen_core::{UNICODE31_ND, UnicodeStringProfile, UnicodeStringToken};

pub(super) fn support(schema: &ams_gra_oms_ir::SchemaIr) -> String {
    let friends = schema
        .types
        .iter()
        .filter(|d| {
            matches!(
                ams_gra_oms_codegen_core::string_profile(
                    ams_gra_oms_ir::PrimitiveKind::String,
                    &d.constraints
                ),
                Ok(Some(ams_gra_oms_codegen_core::StringProfile::Unicode(_)))
            ) && matches!(
                d.kind,
                ams_gra_oms_ir::TypeKind::Primitive(ams_gra_oms_ir::PrimitiveKind::String)
            )
        })
        .map(|d| {
            format!(
                "    friend class {};\n",
                super::upper_camel(&d.name.local_name).expect("preflight validated name")
            )
        })
        .collect::<String>();
    let nd = UNICODE31_ND
        .iter()
        .map(|(a, b)| format!("(value >= 0x{a:X} && value <= 0x{b:X})"))
        .collect::<Vec<_>>()
        .join(" || ");
    let mut source = format!(
        "class Unicode31StringValidator {{\npublic:\n    static bool accepts(std::string_view text, unsigned profile) {{\n        std::uint32_t c[21] = {{}};\n        std::size_t count = 0, pos = 0;\n        while (pos < text.size()) {{\n            if (count == 21) return false;\n            const unsigned lead = static_cast<unsigned char>(text[pos++]);\n            std::uint32_t scalar; unsigned extra; std::uint32_t minimum;\n            if (lead <= 0x7F) {{ scalar = lead; extra = 0; minimum = 0; }}\n            else if (lead >= 0xC2 && lead <= 0xDF) {{ scalar = lead & 0x1F; extra = 1; minimum = 0x80; }}\n            else if (lead >= 0xE0 && lead <= 0xEF) {{ scalar = lead & 0x0F; extra = 2; minimum = 0x800; }}\n            else if (lead >= 0xF0 && lead <= 0xF4) {{ scalar = lead & 0x07; extra = 3; minimum = 0x10000; }}\n            else return false;\n            if (text.size() - pos < extra) return false;\n            for (unsigned i = 0; i < extra; ++i) {{\n                const unsigned byte = static_cast<unsigned char>(text[pos++]);\n                if (byte < 0x80 || byte > 0xBF) return false;\n                scalar = scalar * 64 + (byte & 0x3F);\n            }}\n            if (scalar < minimum || scalar > 0x10FFFF || (scalar >= 0xD800 && scalar <= 0xDFFF)) return false;\n            c[count++] = scalar;\n        }}\n        const auto nd = [](std::uint32_t value) {{ return {nd}; }};\n        switch (profile) {{\n"
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
                            "({})",
                            set.bytes()
                                .map(|b| format!("c[{i}] == {}", u32::from(b)))
                                .collect::<Vec<_>>()
                                .join(" || ")
                        ),
                    })
                    .collect::<Vec<_>>()
                    .join(" && ")
            })
            .map(|s| format!("({s})"))
            .collect::<Vec<_>>()
            .join(" ||\n                ");
        source.push_str(&format!(
            "        case {}: return count == {} && ({branches});\n",
            profile.id(),
            profile.length()
        ));
    }
    source.push_str("        default: return false;\n        }\n    }\n};\n");
    source = source.replacen("public:\n", &format!("{friends}private:\n"), 1);
    source
}

pub(super) fn render(name: &str, profile: UnicodeStringProfile) -> String {
    format!(
        "class {name} {{\npublic:\n    static std::optional<{name}> create(std::string_view text) {{\n        if (!Unicode31StringValidator::accepts(text, {})) return std::nullopt;\n        return {name}(std::string(text));\n    }}\n    {name}(const {name}&) = default;\n    {name}& operator=(const {name}&) = default;\n    const std::string& value() const noexcept {{ return value_; }}\nprivate:\n    explicit {name}(std::string text) : value_(std::move(text)) {{}}\n    std::string value_;\n}};\n",
        profile.id()
    )
}
