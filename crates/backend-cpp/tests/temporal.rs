//! Task 036: the generated C++17 DateTime Zulu carrier, compiled and run.
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

fn temporal_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-temporal-datetime.xsd"),
    )
    .expect("temporal fixture should parse")
}

/// The generated API shape, asserted before anything is compiled.
#[test]
fn generated_date_time_api_is_an_opaque_validated_carrier() {
    let source = generate(&temporal_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("temporal fixture must generate");

    assert!(source.contains("static std::optional<Instant> create(std::string_view value)"));
    assert!(source.contains("const std::string& value() const noexcept"));
    // The only constructor is private, so no unchecked construction path is
    // reachable from a client.
    assert!(source.contains("private:\n    explicit Instant(std::string normalized)"));

    // No comparison operator or ordering helper is declared: XML Schema
    // dateTime equality is value-space equality over spellings, and its order
    // relation is only partial. Task 036 implements neither.
    let carrier = source
        .split("class Instant {")
        .nth(1)
        .and_then(|rest| rest.split("\n};").next())
        .expect("Instant must be generated");
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

    // No external temporal or regex library, and no calendar/epoch type.
    for forbidden in ["<chrono>", "<regex>", "<ctime>", "time_t", "std::chrono"] {
        assert!(
            !source.contains(forbidden),
            "generated C++ must not reference {forbidden}"
        );
    }

    // Task 034 composition: the optional named occurrence is the ordinary
    // `std::optional`, with no DateTime-specific optional path.
    assert!(source.contains("std::optional<Instant> timestamp;"));
    assert!(source.contains("Instant observed;"));
    assert!(source.contains("std::optional<Deadline> expires;"));
}

/// A schema with no Task 036 carrier must not gain the new include.
#[test]
fn schemas_without_a_temporal_carrier_do_not_gain_the_string_view_include() {
    let track = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../xsd-frontend/tests/fixtures/track.xsd"),
    )
    .expect("track fixture should parse");
    let source =
        generate(&track, GenerationWorld::ClosedSchemaSet).expect("track must still generate");
    assert!(!source.contains("#include <string_view>"));
}

/// Every shared-corpus case, executed against the compiled generated header
/// under `-std=c++17 -Wall -Wextra -pedantic-errors`.
#[test]
fn generated_date_time_validator_matches_the_shared_corpus() {
    let source = generate(&temporal_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("temporal fixture must generate");
    let cases = common::load_cases();

    let mut probe = String::from(
        "#include \"generated.hpp\"\n#include <cassert>\n#include <cstdio>\n\n\
         using test::temporal::Instant;\n\nint main() {\n",
    );
    for case in &cases {
        let input = common::escape_for_source(&case.input);
        match &case.expected {
            Some(normalized) => {
                let normalized = common::escape_for_source(normalized);
                writeln!(
                    probe,
                    "    {{\n\
                     \x20       auto made = Instant::create(\"{input}\");\n\
                     \x20       if (!made) {{ std::puts(\"must accept: {input}\"); return 1; }}\n\
                     \x20       if (made->value() != \"{normalized}\") {{ std::puts(\"bad normalization: {input}\"); return 1; }}\n\
                     \x20   }}"
                )
                .expect("writing to String cannot fail");
            }
            None => {
                writeln!(
                    probe,
                    "    if (Instant::create(\"{input}\")) {{ std::puts(\"must reject: {input}\"); return 1; }}"
                )
                .expect("writing to String cannot fail");
            }
        }
    }
    probe.push_str("    std::puts(\"ok\");\n    return 0;\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task036-cpp-temporal");
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

/// Task 036: a client cannot bypass validation by touching the storage.
///
/// The only constructor and the storage member are both private, so building
/// one directly or reading `value_` from outside must fail to compile. A
/// runtime assertion would not prove this; only the compiler can.
#[test]
fn the_private_storage_cannot_be_reached_from_outside_the_class() {
    let source = generate(&temporal_schema(), GenerationWorld::ClosedSchemaSet)
        .expect("temporal fixture must generate");

    let directory = std::env::temp_dir().join("ams-gra-oms-task036-cpp-bypass");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create C++ probe directory");
    std::fs::write(directory.join("generated.hpp"), &source).expect("write generated header");

    for (label, body) in [
        (
            "direct construction",
            "    test::temporal::Instant bad(std::string(\"garbageZ\"));\n    (void) bad;",
        ),
        (
            "member read",
            "    auto made = test::temporal::Instant::create(\"2026-09-20T12:34:56Z\");\n\
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
