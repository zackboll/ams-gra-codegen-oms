//! Task 039: the generated C++ visible-ASCII carrier, compiled and run.
//!
//! Asserting on generated *text* alone would not prove the emitted validator
//! behaves the way the authoritative XSD requires, so every case in the shared
//! corpus is executed against a real program built under strict C++17.

mod common;

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn visible_ascii_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-visible-ascii.xsd"),
    )
    .expect("visible-ASCII fixture should parse")
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_visible_ascii_api_is_an_opaque_validated_carrier() {
    let source = generate(&visible_ascii_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("fixture must generate");

    // A validated lexical carrier with a private constructor and storage.
    assert!(source.contains("static std::optional<Callsign> create(std::string_view value)"));
    assert!(source.contains("const std::string& value() const noexcept"));
    assert!(source.contains("explicit Callsign(std::string validated)"));
    assert!(source.contains("std::string value_;"));

    // The profile is PARAMETERIZED: each carrier must carry its OWN bounds.
    assert!(source.contains("static constexpr std::size_t kMinLength = 1;"));
    assert!(source.contains("static constexpr std::size_t kMaxLength = 256;"));
    assert!(source.contains("static constexpr std::size_t kMaxLength = 32;"));
    assert!(source.contains("static constexpr std::size_t kMinLength = 2;"));
    assert!(source.contains("static constexpr std::size_t kMaxLength = 4;"));

    // The class is the exact ordinal interval, compared as unsigned char.
    assert!(source.contains("static constexpr unsigned char kMinCodePoint = 0x20;"));
    assert!(source.contains("static constexpr unsigned char kMaxCodePoint = 0x7E;"));
    assert!(source.contains("static_cast<unsigned char>(character)"));

    // No regex engine, and nothing locale-sensitive: <cctype> classification
    // varies by locale and would admit characters outside [ -~].
    for forbidden in [
        "<regex>",
        "std::regex",
        "<cctype>",
        "isprint",
        "isalnum",
        "isgraph",
        "std::locale",
    ] {
        assert!(
            !source.contains(forbidden),
            "generated C++ must not reference {forbidden}"
        );
    }

    // Task 034 composition: the optional named occurrence is the ordinary
    // std::optional, with no visible-string-specific optional path.
    assert!(source.contains("Callsign primary;"));
    assert!(source.contains("std::optional<Callsign> alternate;"));
    assert!(source.contains("std::optional<ShortLabel> label;"));

    // A transitive restriction that adds no facets is equally renderable.
    assert!(source.contains("class DerivedLabel {"));

    // The Task 037 and Task 038 profiles still render, in the same unit.
    assert!(source.contains("class SchemaVersion {"));
    assert!(source.contains("class Uuid {"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(source.contains("std::string notes;"));
}

/// Every shared-corpus case, executed against a compiled probe built under
/// `-std=c++17 -Wall -Wextra -pedantic-errors`.
#[test]
fn generated_visible_ascii_validator_matches_the_shared_corpus() {
    let source = generate(&visible_ascii_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("fixture must generate");
    let cases = common::load_corpus(&common::visible_ascii_corpus_path());

    let mut probe = String::from(
        "#include \"generated.hpp\"\n#include <cstdio>\n#include <string>\n\n\
         using test::callsign::Callsign;\n\
         using test::callsign::CountryCode;\n\
         using test::callsign::DerivedLabel;\n\
         using test::callsign::ShortLabel;\n\nint main() {\n",
    );
    for (index, case) in cases.iter().enumerate() {
        // Cases are reported by corpus index, never by interpolating their own
        // text into a diagnostic, so a braced case cannot disturb the probe.
        let input = common::escape_for_cpp_source(&case.input);
        match &case.expected {
            Some(stored) => {
                let stored = common::escape_for_cpp_source(stored);
                // The length is passed explicitly: a corpus case may contain an
                // embedded NUL, which a bare string literal would truncate.
                let bytes = case.input.len();
                let stored_bytes = case
                    .expected
                    .as_deref()
                    .expect("a VALID case carries its stored form")
                    .len();
                writeln!(
                    probe,
                    "    {{\n\
                     \x20       const std::string input(\"{input}\", {bytes});\n\
                     \x20       auto made = Callsign::create(input);\n\
                     \x20       if (!made) {{ std::puts(\"must accept case {index}\"); return 1; }}\n\
                     \x20       if (made->value() != std::string(\"{stored}\", {stored_bytes})) {{ std::puts(\"bad storage case {index}\"); return 1; }}\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                let bytes = case.input.len();
                writeln!(
                    probe,
                    "    if (Callsign::create(std::string(\"{input}\", {bytes}))) {{ std::puts(\"must reject case {index}\"); return 1; }}"
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    probe.push_str(&boundary_probe());
    probe.push_str("    std::puts(\"ok\");\n    return 0;\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task039-cpp-visible-ascii");
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
        run.status.success(),
        "generated validator must agree with the shared corpus:\n{}",
        String::from_utf8_lossy(&run.stdout)
    );
    std::fs::remove_dir_all(&directory).expect("remove C++ probe directory");
}

/// The assertions the shared corpus cannot express, appended to the probe.
///
/// Three things are pinned here: the four ordinal boundaries, the fact that
/// the bounds really are per-declaration, and preservation of spacing under
/// `whiteSpace = preserve`.
fn boundary_probe() -> String {
    let mut probe = String::new();
    // The four ordinal boundaries, pinned directly rather than only via the
    // corpus, so this interval can never silently become "printable ASCII".
    probe.push_str(
        "    if (Callsign::create(std::string(\"\\037\", 1))) { std::puts(\"U+001F must be rejected\"); return 1; }\n\
         \x20   if (!Callsign::create(std::string(\"\\040\", 1))) { std::puts(\"U+0020 must be accepted\"); return 1; }\n\
         \x20   if (!Callsign::create(std::string(\"\\176\", 1))) { std::puts(\"U+007E must be accepted\"); return 1; }\n\
         \x20   if (Callsign::create(std::string(\"\\177\", 1))) { std::puts(\"U+007F must be rejected\"); return 1; }\n",
    );
    // The bounds really are per-declaration. `ShortLabel` is 1..32 and
    // `CountryCode` is 2..4.
    probe.push_str(
        "    {\n\
         \x20       const std::string long_value(33, 'x');\n\
         \x20       if (!Callsign::create(long_value)) { std::puts(\"33 fits 1..256\"); return 1; }\n\
         \x20       if (ShortLabel::create(long_value)) { std::puts(\"33 exceeds 1..32\"); return 1; }\n\
         \x20       if (!ShortLabel::create(std::string(32, 'x'))) { std::puts(\"32 fits 1..32\"); return 1; }\n\
         \x20       if (DerivedLabel::create(long_value)) { std::puts(\"the chain keeps 1..32\"); return 1; }\n\
         \x20       if (CountryCode::create(\"x\")) { std::puts(\"1 is below minLength 2\"); return 1; }\n\
         \x20       if (!CountryCode::create(\"xx\")) { std::puts(\"2 meets minLength 2\"); return 1; }\n\
         \x20       if (!CountryCode::create(\"xxxx\")) { std::puts(\"4 meets maxLength 4\"); return 1; }\n\
         \x20       if (CountryCode::create(\"xxxxx\")) { std::puts(\"5 exceeds maxLength 4\"); return 1; }\n\
         \x20   }\n",
    );
    // whiteSpace = preserve: spaces are members of the class and are stored
    // exactly as supplied, so these three values stay DISTINCT.
    probe.push_str(
        "    {\n\
         \x20       auto plain = Callsign::create(\"abc\");\n\
         \x20       auto trailing = Callsign::create(\"abc \");\n\
         \x20       auto leading = Callsign::create(\" abc\");\n\
         \x20       if (!plain || !trailing || !leading) { std::puts(\"spaced values must be valid\"); return 1; }\n\
         \x20       if (trailing->value() != \"abc \") { std::puts(\"a trailing space must not be trimmed\"); return 1; }\n\
         \x20       if (leading->value() != \" abc\") { std::puts(\"a leading space must not be trimmed\"); return 1; }\n\
         \x20       if (plain->value() == trailing->value()) { std::puts(\"spacing must be significant\"); return 1; }\n\
         \x20   }\n",
    );
    probe
}

/// Task 039: a client cannot bypass validation by reaching the storage.
#[test]
fn the_private_visible_ascii_storage_cannot_be_reached_from_outside_the_class() {
    let source = generate(&visible_ascii_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task039-cpp-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create C++ probe directory");
    std::fs::write(directory.join("generated.hpp"), &source).expect("write generated header");

    for (label, body) in [
        (
            "private constructor",
            "    test::callsign::Callsign value(std::string(\"nope\"));\n    (void) value;",
        ),
        (
            "private storage",
            "    auto value = test::callsign::Callsign::create(\"ok\");\n    (void) value->value_;",
        ),
    ] {
        let probe = format!(
            "#include \"generated.hpp\"\n#include <string>\n\nint main() {{\n{body}\n    return 0;\n}}\n"
        );
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
            !output.status.success(),
            "the {label} bypass must not compile"
        );
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostics.contains("private"),
            "the {label} bypass must be rejected for access control, got:\n{diagnostics}"
        );
    }
    std::fs::remove_dir_all(&directory).expect("remove C++ probe directory");
}
