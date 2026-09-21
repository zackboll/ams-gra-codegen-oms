//! Task 038: the generated C++ UUID carrier, compiled and run.
//!
//! Asserting on generated *text* alone would not prove the emitted validator
//! behaves the way the authoritative XSD requires, so every case in the shared
//! corpus is executed against a real compiled program built under strict
//! `-std=c++17 -Wall -Wextra -pedantic-errors`.

mod common;

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn uuid_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-uuid.xsd"),
    )
    .expect("uuid fixture should parse")
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_uuid_api_is_an_opaque_validated_carrier() {
    let source =
        generate(&uuid_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");

    // A validated lexical carrier: a factory returning std::optional, a const
    // observer, and a private constructor plus private storage.
    assert!(source.contains("static std::optional<Uuid> create(std::string_view value)"));
    assert!(source.contains("const std::string& value() const noexcept"));
    assert!(source.contains("explicit Uuid(std::string validated)"));
    assert!(source.contains("std::string value_;"));

    // No regex engine, no UUID library, and no numeric conversion: the carrier
    // is the validated spelling.
    for forbidden in ["<regex>", "std::regex", "uuid_t", "uuid_parse", "__int128"] {
        assert!(
            !source.contains(forbidden),
            "generated C++ must not reference {forbidden}"
        );
    }

    // Task 034 composition: the optional named occurrence uses the ordinary
    // named-type occurrence machinery, with no UUID-specific optional path.
    assert!(source.contains("Uuid identifier;"));
    assert!(source.contains("std::optional<Uuid> correlation;"));
    assert!(source.contains("std::optional<PeerUuid> peer;"));

    // The Task 037 profile still renders, in the same generated header.
    assert!(source.contains("class SchemaVersion {"));
    assert!(source.contains("SchemaVersion version;"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(source.contains("std::string label;"));
}

/// Every shared-corpus case, executed against a compiled probe built under
/// `-std=c++17 -Wall -Wextra -pedantic-errors`.
#[test]
fn generated_uuid_validator_matches_the_shared_corpus() {
    let source =
        generate(&uuid_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");
    let cases = common::load_corpus(&common::uuid_corpus_path());

    let mut probe = String::from(
        "#include \"generated.hpp\"\n#include <cstdio>\n\n\
         using test::identity::Uuid;\n\nint main() {\n",
    );
    for (index, case) in cases.iter().enumerate() {
        let input = common::escape_for_source(&case.input);
        // Cases are reported by corpus index, never by interpolating their own
        // text into a diagnostic, so a braced case cannot disturb the probe.
        match &case.expected {
            Some(stored) => {
                let stored = common::escape_for_source(stored);
                writeln!(
                    probe,
                    "    {{\n\
                     \x20       auto made = Uuid::create(\"{input}\");\n\
                     \x20       if (!made) {{ std::puts(\"must accept case {index}\"); return 1; }}\n\
                     \x20       if (made->value() != \"{stored}\") {{ std::puts(\"bad storage case {index}\"); return 1; }}\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    probe,
                    "    if (Uuid::create(\"{input}\")) {{ std::puts(\"must reject case {index}\"); return 1; }}"
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    // Case is significant: this remains an xs:string carrier, so two otherwise
    // valid UUIDs differing only in letter case are DISTINCT values and the
    // stored spelling is preserved exactly.
    probe.push_str(
        "    {\n\
         \x20       auto lower = Uuid::create(\"123e4567-e89b-12d3-a456-42661417400f\");\n\
         \x20       auto upper = Uuid::create(\"123E4567-E89B-12D3-A456-42661417400F\");\n\
         \x20       if (!lower || !upper) { std::puts(\"case pair must be valid\"); return 1; }\n\
         \x20       if (lower->value() == upper->value()) { std::puts(\"case must be significant\"); return 1; }\n\
         \x20       if (upper->value() != \"123E4567-E89B-12D3-A456-42661417400F\") { std::puts(\"case must be preserved\"); return 1; }\n\
         \x20   }\n",
    );
    probe.push_str("    std::puts(\"ok\");\n    return 0;\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task038-cpp-uuid");
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

/// Task 038: a client cannot bypass validation by touching the storage.
///
/// The only constructor and the storage member are both private, so building
/// one directly or reading `value_` from outside must fail to compile.
#[test]
fn the_private_uuid_storage_cannot_be_reached_from_outside_the_class() {
    let source =
        generate(&uuid_schema(), GenerationWorld::ClosedSchemaSet).expect("fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task038-cpp-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create C++ probe directory");
    std::fs::write(directory.join("generated.hpp"), &source).expect("write generated header");

    for (label, body) in [
        (
            "direct construction",
            "    test::identity::Uuid bad(std::string(\"nope\"));\n    (void) bad;",
        ),
        (
            "member read",
            "    auto made = test::identity::Uuid::create(\"00000000-0000-0000-0000-000000000000\");\n\
             \x20   (void) made->value_;",
        ),
    ] {
        let probe = format!(
            "#include \"generated.hpp\"\n#include <string>\n\nint main() {{\n{body}\n    return 0;\n}}\n"
        );
        std::fs::write(directory.join("probe.cpp"), &probe).expect("write probe");
        let output = Command::new("c++")
            .current_dir(&directory)
            .args(["-std=c++17", "-fsyntax-only", "probe.cpp"])
            .output()
            .expect("a C++ compiler should be available");
        assert!(
            !output.status.success(),
            "the {label} bypass must not compile"
        );
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostics.contains("private"),
            "the {label} bypass must be rejected for privacy, got:\n{diagnostics}"
        );
    }
    std::fs::remove_dir_all(&directory).expect("remove C++ probe directory");
}
