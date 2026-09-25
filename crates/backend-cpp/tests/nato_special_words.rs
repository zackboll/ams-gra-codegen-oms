//! Task 042: the generated C++17 NATO special-words carrier, compiled and run.
//!
//! Every probe is built under `-std=c++17 -Wall -Wextra -pedantic-errors`, and
//! every corpus case is passed with an EXPLICIT length so an embedded NUL case
//! cannot be silently truncated into a shorter, accidentally valid input. The
//! alphabet sweep's expected answers come from an alphabet stated in this
//! file, never from the production validator.

mod common;

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn nato_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-nato-special-words.xsd"),
    )
    .expect("NATO special-words fixture should parse")
}

/// The suffix alphabet `[a-zA-Z\-_]` over single bytes, stated independently.
fn expected_suffix_byte(byte: u8) -> bool {
    matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_')
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_nato_special_words_api_is_a_checked_factory_carrier() {
    let source =
        generate(&nato_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");
    for name in ["SpecialWord", "DerivedSpecialWord"] {
        assert!(source.contains(&format!(
            "static std::optional<{name}> create(std::string_view value)"
        )));
        // Task 040: copy declared (suppressing implicit moves), not noexcept.
        assert!(source.contains(&format!("{name}(const {name}&) = default;")));
        assert!(source.contains(&format!("{name}& operator=(const {name}&) = default;")));
        assert!(!source.contains(&format!("{name}({name}&&)")));
        assert!(!source.contains(&format!("{name}(const {name}&) noexcept")));
    }
    assert_eq!(
        source
            .matches("static constexpr std::size_t kMinLength = 6;")
            .count(),
        2
    );
    assert_eq!(
        source
            .matches("static constexpr std::size_t kMaxLength = 261;")
            .count(),
        2
    );
    assert!(source.contains("static constexpr std::string_view kPrefix = \"NATO:\";"));
    assert!(source.contains("static_cast<unsigned char>(character)"));
    for forbidden in [
        "<regex>",
        "std::regex",
        "#include <cctype>",
        "isalpha(",
        "isalnum(",
        "std::locale",
        ".substr(",
    ] {
        assert!(
            !source.contains(forbidden),
            "generated C++ must not reference {forbidden}"
        );
    }
    // Composition through the existing containers only.
    assert!(source.contains("std::optional<SpecialWord> alternate;"));
    assert!(source.contains("BoundedVector<SpecialWord, 0, 4> history;"));
    assert!(source.contains("BoundedVector<SpecialWord, 2, 3> required;"));
    assert!(source.contains("UnboundedVector<SpecialWord, 0> log;"));
    assert!(source.contains("std::string notes;"));
}

/// The corpus, boundaries, alphabet sweep, and lifecycle, under strict C++17.
#[test]
fn generated_nato_special_words_validator_matches_the_shared_corpus() {
    let source =
        generate(&nato_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");
    let mut probe = String::from(
        "#include \"generated.hpp\"\n#include <cstdio>\n#include <string>\n\
         #include <utility>\n#include <vector>\n\n\
         using namespace test::markings;\n\nint main() {\n",
    );
    let cases = common::load_corpus(&common::nato_special_words_corpus_path());
    assert!(cases.len() > 100, "the corpus must be substantial");
    for (index, case) in cases.iter().enumerate() {
        let input = common::escape_for_cpp_source(&case.input);
        let bytes = case.input.len();
        match &case.expected {
            Some(stored) => {
                let want = common::escape_for_cpp_source(stored);
                let want_bytes = stored.len();
                writeln!(
                    probe,
                    "    {{\n\
                     \x20       const std::string input(\"{input}\", {bytes});\n\
                     \x20       const std::string want(\"{want}\", {want_bytes});\n\
                     \x20       auto made = SpecialWord::create(input);\n\
                     \x20       if (!made) {{ std::puts(\"must accept case {index}\"); return 1; }}\n\
                     \x20       if (made->value() != want) {{ std::puts(\"storage case {index}\"); return 1; }}\n\
                     \x20       auto again = SpecialWord::create(made->value());\n\
                     \x20       if (!again || again->value() != want) {{ std::puts(\"idempotent case {index}\"); return 1; }}\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => writeln!(
                probe,
                "    if (SpecialWord::create(std::string(\"{input}\", {bytes}))) {{ std::puts(\"must reject case {index}\"); return 1; }}"
            )
            .expect("writing to String cannot fail"),
        }
    }
    probe.push_str(&alphabet_probe());
    probe.push_str(CPP_SEMANTICS_PROBE);
    probe.push_str("    std::puts(\"ok\");\n    return 0;\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task042-cpp-nato");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create C++ probe directory");
    std::fs::write(directory.join("generated.hpp"), &source).expect("write generated header");
    std::fs::write(directory.join("probe.cpp"), &probe).expect("write probe");
    let output = Command::new("c++")
        .current_dir(&directory)
        .args([
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-pedantic-errors",
            "-o",
            "probe",
            "probe.cpp",
        ])
        .output()
        .expect("a C++ compiler should be available");
    assert!(
        output.status.success(),
        "generated header must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Command::new(directory.join("probe"))
        .output()
        .expect("compiled probe must run");
    assert!(
        run.status.success() && String::from_utf8_lossy(&run.stdout).trim() == "ok",
        "generated validator must agree with the shared corpus:\n{}",
        String::from_utf8_lossy(&run.stdout)
    );
    std::fs::remove_dir_all(&directory).expect("remove C++ probe directory");
}

/// All 256 single-byte suffix candidates, each built with an explicit length.
/// Expected acceptance is computed HERE from the independent alphabet.
fn alphabet_probe() -> String {
    let mut probe = String::new();
    let accepted: Vec<String> = (0_u16..=255)
        .map(|byte| u8::try_from(byte).expect("byte range"))
        .filter(|&byte| expected_suffix_byte(byte))
        .map(|byte| byte.to_string())
        .collect();
    assert_eq!(
        accepted.len(),
        54,
        "the independent alphabet has 54 members"
    );
    writeln!(
        probe,
        "    {{\n\
         \x20       const std::vector<int> accepted = {{{}}};\n\
         \x20       for (int byte = 0; byte <= 255; ++byte) {{\n\
         \x20           bool expected = false;\n\
         \x20           for (const int member : accepted) {{ if (member == byte) {{ expected = true; }} }}\n\
         \x20           std::string text(\"NATO:\");\n\
         \x20           text.push_back(static_cast<char>(byte));\n\
         \x20           if (text.size() != 6) {{ std::puts(\"sweep length\"); return 1; }}\n\
         \x20           if (SpecialWord::create(text).has_value() != expected) {{\n\
         \x20               std::printf(\"alphabet byte %d\\n\", byte); return 1;\n\
         \x20           }}\n\
         \x20       }}\n\
         \x20   }}",
        accepted.join(", ")
    )
    .expect("writing to String cannot fail");
    probe
}

/// Boundaries, long inputs, lifecycle, and composition.
const CPP_SEMANTICS_PROBE: &str = r#"    // Suffix 0/1/255/256/257 == total 5/6/260/261/262.
    {
        const std::size_t suffixes[] = {0, 1, 255, 256, 257};
        const bool valid[] = {false, true, true, true, false};
        for (std::size_t i = 0; i < 5; ++i) {
            const std::string text = std::string("NATO:") + std::string(suffixes[i], 'a');
            if (text.size() != 5 + suffixes[i]) { std::puts("boundary length"); return 1; }
            const auto made = SpecialWord::create(text);
            if (made.has_value() != valid[i]) { std::printf("boundary suffix %zu\n", suffixes[i]); return 1; }
            if (made && made->value() != text) { std::puts("boundary storage"); return 1; }
        }
    }
    // Long syntactically invalid inputs, and a digit at the last position.
    if (SpecialWord::create(std::string(10000, 'x'))) { std::puts("long x"); return 1; }
    if (SpecialWord::create(std::string("NATO:") + std::string(10000, 'a'))) { std::puts("long suffix"); return 1; }
    if (SpecialWord::create(std::string("NATO:") + std::string(255, 'a') + "1")) { std::puts("late digit"); return 1; }
    // Every truncated prefix through the checked factory, never an exception.
    for (std::size_t n = 0; n <= 5; ++n) {
        if (SpecialWord::create(std::string_view("NATO:", n))) { std::puts("short prefix"); return 1; }
    }
    // Embedded NUL with an explicit length is rejected, not truncated to valid.
    if (SpecialWord::create(std::string("NATO:A\0B", 8))) { std::puts("embedded NUL"); return 1; }
    // High-bit bytes, negative as a signed char.
    if (SpecialWord::create(std::string("NATO:\303\251", 7))) { std::puts("high-bit"); return 1; }
    // Task 040 lifecycle: source AND destination keep valid values after copies
    // and rvalue operations (which select copy).
    {
        auto source = *SpecialWord::create("NATO:keep-me");
        auto dest = source;
        if (source.value() != "NATO:keep-me" || dest.value() != "NATO:keep-me") { std::puts("copy"); return 1; }
        auto moved = std::move(source);
        if (source.value() != "NATO:keep-me" || moved.value() != "NATO:keep-me") { std::puts("rvalue ctor"); return 1; }
        dest = std::move(moved);
        if (moved.value() != "NATO:keep-me" || dest.value() != "NATO:keep-me") { std::puts("rvalue assign"); return 1; }
    }
    // Composition through the existing containers.
    {
        const auto word = *SpecialWord::create("NATO:Word");
        Marking marking{
            word,
            SpecialWord::create("NATO:alt_one"),
            *DerivedSpecialWord::create("NATO:Derived-X"),
            *Callsign::create("ALPHA 1"),
            *SchemaVersion::create("002.5.0"),
            *PreservedRemarks::create("free\ntext"),
            "ordinary",
            *BoundedVector<SpecialWord, 0, 4>::create({word}),
            *BoundedVector<SpecialWord, 2, 3>::create({word, *SpecialWord::create("NATO:r_b")}),
            *UnboundedVector<SpecialWord, 0>::create(std::vector<SpecialWord>(300, word)),
        };
        if (marking.word.value() != "NATO:Word") { std::puts("required"); return 1; }
        if (!marking.alternate || marking.alternate->value() != "NATO:alt_one") { std::puts("optional"); return 1; }
        if (marking.derived.value() != "NATO:Derived-X") { std::puts("derived"); return 1; }
        if (DerivedSpecialWord::create("NATO:1")) { std::puts("derived profile"); return 1; }
        if (marking.required.values().at(1).value() != "NATO:r_b") { std::puts("bounded"); return 1; }
        if (marking.log.values().size() != 300) { std::puts("unbounded"); return 1; }
        const auto too_few = BoundedVector<SpecialWord, 2, 3>::create({word});
        if (too_few.has_value()) { std::puts("positive minimum"); return 1; }
        marking.alternate = std::nullopt;
        if (marking.alternate.has_value()) { std::puts("absent optional"); return 1; }
    }
"#;
