//! Task 037: the generated C++17 schema-version carrier, compiled and run.
//!
//! Generated text alone would not prove the validator behaves as XML Schema
//! requires, so every case in the shared corpus is executed against a real
//! compiled program under strict C++17.

mod common;

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn string_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-schema-version.xsd"),
    )
    .expect("string fixture should parse")
}

/// The generated API shape, asserted before anything is compiled.
#[test]
fn generated_schema_version_api_is_an_opaque_validated_carrier() {
    let source = generate(&string_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("string fixture must generate");

    assert!(source.contains("static std::optional<SchemaVersion> create(std::string_view value)"));
    assert!(source.contains("const std::string& value() const noexcept"));
    // The only constructor is private, so no unchecked construction path is
    // reachable from a client.
    assert!(source.contains("private:\n    explicit SchemaVersion(std::string validated)"));

    // No comparison operator is invented. Current backend convention is that
    // generated value types expose construction and observation only.
    let carrier = source
        .split("class SchemaVersion {")
        .nth(1)
        .and_then(|rest| rest.split("\n};").next())
        .expect("SchemaVersion must be generated");
    for forbidden in [
        "operator==",
        "operator!=",
        "operator<",
        "operator<=>",
        "compare",
    ] {
        assert!(
            !carrier.contains(forbidden),
            "generated carrier must not declare {forbidden}"
        );
    }

    // No regex library, and no locale-dependent character classification.
    for forbidden in ["<regex>", "std::regex", "<cctype>", "isdigit", "isalnum"] {
        assert!(
            !source.contains(forbidden),
            "generated C++ must not reference {forbidden}"
        );
    }

    // Task 034 composition: the optional named occurrence uses the ordinary
    // named-type occurrence machinery, with no String-profile special path.
    assert!(source.contains("SchemaVersion declared;"));
    assert!(source.contains("std::optional<SchemaVersion> negotiated;"));
    assert!(source.contains("std::optional<PeerVersion> peer;"));

    // Ordinary unconstrained String keeps its existing plain representation.
    assert!(source.contains("std::string label;"));
}

/// Every shared-corpus case, executed against a compiled probe built under
/// `-std=c++17 -Wall -Wextra -pedantic-errors`.
#[test]
fn generated_schema_version_validator_matches_the_shared_corpus() {
    let source = generate(&string_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("string fixture must generate");
    let cases = common::load_corpus(&common::string_corpus_path());

    let mut probe = String::from(
        "#include \"generated.hpp\"\n#include <cstdio>\n\n\
         using test::versioning::SchemaVersion;\n\nint main() {\n",
    );
    for case in &cases {
        let input = common::escape_for_source(&case.input);
        match &case.expected {
            Some(stored) => {
                let stored = common::escape_for_source(stored);
                writeln!(
                    probe,
                    "    {{\n\
                     \x20       auto made = SchemaVersion::create(\"{input}\");\n\
                     \x20       if (!made) {{ std::puts(\"must accept: {input}\"); return 1; }}\n\
                     \x20       if (made->value() != \"{stored}\") {{ std::puts(\"bad storage: {input}\"); return 1; }}\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    probe,
                    "    if (SchemaVersion::create(\"{input}\")) {{ std::puts(\"must reject: {input}\"); return 1; }}"
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    probe.push_str("    std::puts(\"ok\");\n    return 0;\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task037-cpp-string");
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

/// Task 037: a client cannot bypass validation by touching the storage.
///
/// The only constructor and the storage member are both private, so building
/// one directly or reading `value_` from outside must fail to compile. A
/// runtime assertion would not prove this; only the compiler can.
#[test]
fn the_private_storage_cannot_be_reached_from_outside_the_class() {
    let source = generate(&string_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("string fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task037-cpp-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create C++ probe directory");
    std::fs::write(directory.join("generated.hpp"), &source).expect("write generated header");

    for (label, body) in [
        (
            "direct construction",
            "    test::versioning::SchemaVersion bad(std::string(\"nope\"));\n    (void) bad;",
        ),
        (
            "member read",
            "    auto made = test::versioning::SchemaVersion::create(\"002.5.0\");\n\
             \x20   (void) made->value_;",
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
                "-fsyntax-only",
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
