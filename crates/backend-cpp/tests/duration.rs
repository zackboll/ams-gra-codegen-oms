//! Task 057: XML Schema `duration` checked lexical carriers in strict C++17.
//!
//! The shared `tests/fixtures/temporal/duration.txt` corpus is run against
//! BOTH the named zero-facet carrier (`Span`) and the direct support carrier
//! (`XmlSchemaDuration`) under `-std=c++17 -Wall -Wextra -Werror
//! -pedantic-errors`, together with the Task 040 lifecycle traits.

mod common;

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

#[test]
fn named_and_direct_duration_share_one_compiled_cpp17_parser() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-duration.xsd"),
    )
    .unwrap();
    let source = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    assert_eq!(source.matches("class XmlSchemaDurationParser {").count(), 1);
    assert_eq!(source.matches("class XmlSchemaDuration {").count(), 1);
    assert_eq!(source.matches("class Span {").count(), 1);
    assert!(!source.contains("XmlSchemaDateTimeParser"));
    assert!(!source.contains("operator==") && !source.contains("operator<"));
    assert!(!source.contains("std::chrono"));
    assert!(source.contains("#include <string_view>"));
    for needle in [
        "Span namedduration;",
        "XmlSchemaDuration directduration;",
        "std::optional<XmlSchemaDuration> optionalduration;",
        "BoundedVector<XmlSchemaDuration",
        "UnboundedVector<XmlSchemaDuration",
        "XmlSchemaDuration step;",
        "struct ByDuration { XmlSchemaDuration value; };",
    ] {
        assert!(source.contains(needle), "missing {needle:?}");
    }
    let mut probe = String::from(concat!(
        "#include \"generated.hpp\"\n#include <string>\n#include <type_traits>\n#include <utility>\n",
        "using namespace test::duration;\n",
        "static_assert(!std::is_default_constructible_v<XmlSchemaDuration>);\n",
        "static_assert(!std::is_default_constructible_v<Span>);\n",
        "static_assert(std::is_copy_constructible_v<XmlSchemaDuration>);\n",
        "static_assert(std::is_copy_assignable_v<Span>);\n",
        "static_assert(!std::is_constructible_v<XmlSchemaDuration, std::string>);\n",
        "static_assert(!std::is_constructible_v<Span, const char*>);\n",
        "int main() {\n",
    ));
    let (mut valid, mut invalid) = (0, 0);
    for case in common::duration_cases() {
        // Explicit byte length: a literal alone would stop at an embedded NUL
        // and silently test a shorter string (the `P1D\u{0}` case).
        let input = format!(
            "std::string(\"{}\", {})",
            common::escape_for_cpp_source(&case.input),
            case.input.len()
        );
        if let Some(expected) = case.expected {
            let expected = common::escape_for_cpp_source(&expected);
            writeln!(
                probe,
                "{{ auto d = XmlSchemaDuration::create({input}); auto n = Span::create({input});\n  \
                 if (!d || d->value() != \"{expected}\" || !n || n->value() != \"{expected}\") return 1; }}"
            )
            .unwrap();
            valid += 1;
        } else {
            writeln!(
                probe,
                "if (XmlSchemaDuration::create({input}) || Span::create({input})) return 2;"
            )
            .unwrap();
            invalid += 1;
        }
    }
    assert!(
        valid >= 30 && invalid >= 60,
        "{valid} valid / {invalid} invalid"
    );
    probe.push_str(concat!(
        // No canonicalization.
        "if (XmlSchemaDuration::create(\"P12M\")->value() != \"P12M\") return 3;\n",
        // Task 040: an rvalue copies, so a moved-from carrier stays valid.
        "auto made = XmlSchemaDuration::create(\"-P1D\");\n",
        "XmlSchemaDuration source = *made;\n",
        "XmlSchemaDuration moved(std::move(source));\n",
        "if (source.value() != \"-P1D\" || moved.value() != \"-P1D\") return 4;\n",
        "auto named = Span::create(\"PT1H\");\n",
        "Span named_source = *named;\n",
        "Span named_target = *Span::create(\"P1Y\");\n",
        "named_target = std::move(named_source);\n",
        "if (named_source.value() != \"PT1H\" || named_target.value() != \"PT1H\") return 5;\n",
        // Composition in every storage position, inherited and Choice.
        "auto repeated = decltype(Payload::repeatedduration)::create({*made, *made});\n",
        "auto unbounded = decltype(Payload::unboundedduration)::create({*made, *made, *made});\n",
        "if (!repeated || !unbounded) return 6;\n",
        "if (decltype(Payload::repeatedduration)::create({*made, *made, *made, *made})) return 7;\n",
        "Payload payload{*named, *made, *made, *repeated, *unbounded};\n",
        "if (!payload.optionalduration || payload.optionalduration->value() != \"-P1D\") return 8;\n",
        "Concrete concrete{*XmlSchemaDuration::create(\"PT0.5S\"), \"x\"};\n",
        "if (concrete.step.value() != \"PT0.5S\") return 9;\n",
        "Pick pick{Pick::ByDuration{*made}};\n",
        "if (std::get<Pick::ByDuration>(pick.value).value.value() != \"-P1D\") return 10;\n",
        "return 0;\n}\n",
    ));
    let dir = std::env::temp_dir().join("ams-gra-task057-cpp");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.hpp"), &source).unwrap();
    std::fs::write(dir.join("probe.cpp"), probe).unwrap();
    let output = Command::new("c++")
        .current_dir(&dir)
        .args([
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic-errors",
            "-o",
            "probe",
            "probe.cpp",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Command::new(dir.join("probe")).status().unwrap();
    assert_eq!(run.code(), Some(0), "C++ duration probe failed");
    std::fs::remove_dir_all(dir).unwrap();
}
