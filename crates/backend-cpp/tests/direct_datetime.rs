mod common;

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

#[test]
fn direct_date_time_and_named_zulu_share_compiled_cpp17_parser() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-direct-datetime.xsd"),
    )
    .unwrap();
    let source = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    assert_eq!(source.matches("class XmlSchemaDateTimeParser {").count(), 1);
    assert!(source.contains("std::optional<XmlSchemaDateTime> timestamp;"));
    assert!(source.contains("BoundedVector<XmlSchemaDateTime"));
    assert!(source.contains("UnboundedVector<XmlSchemaDateTime"));
    let mut probe = String::from(
        "#include \"generated.hpp\"\n#include <utility>\n#include <string>\n#include <type_traits>\nusing test::temporal::XmlSchemaDateTime;\nusing test::temporal::Instant;\nstatic_assert(!std::is_default_constructible_v<XmlSchemaDateTime>);\nstatic_assert(std::is_copy_constructible_v<XmlSchemaDateTime>);\nstatic_assert(std::is_copy_assignable_v<XmlSchemaDateTime>);\nint main() {\n",
    );
    for case in common::load_cases()
        .into_iter()
        .chain(common::direct_date_time_cases())
    {
        let input = common::escape_for_cpp_source(&case.input);
        if let Some(expected) = case.expected {
            let expected = common::escape_for_cpp_source(&expected);
            writeln!(probe, "if (!XmlSchemaDateTime::create(\"{input}\") || XmlSchemaDateTime::create(\"{input}\")->value() != \"{expected}\") return 1;").unwrap();
        } else if ![
            "2026-09-20T12:34:56",
            "2026-09-20T12:34:56+00:00",
            "2026-09-20T12:34:56-00:00",
            "2026-09-20T12:34:56+05:00",
            "2026-09-20T12:34:56-13:30",
            "2026-09-20T12:34:56+14:00",
        ]
        .contains(&case.input.as_str())
        {
            writeln!(
                probe,
                "if (XmlSchemaDateTime::create(\"{input}\")) return 2;"
            )
            .unwrap();
        }
    }
    for input in [
        "2026-09-25T12:00:00",
        "2026-09-25T12:00:00+00:00",
        "2026-09-25T12:00:00-05:00",
    ] {
        writeln!(
            probe,
            "if (!XmlSchemaDateTime::create(\"{input}\") || Instant::create(\"{input}\")) return 3;"
        )
        .unwrap();
    }
    probe.push_str("auto made = XmlSchemaDateTime::create(\"2026-09-25T12:34:56-00:00\");\nif (!made) return 4;\nXmlSchemaDateTime copy(*made);\nXmlSchemaDateTime assigned = *made;\nassigned = std::move(copy);\nif (copy.value() != made->value() || assigned.value() != made->value()) return 5;\nXmlSchemaDateTime moved(std::move(assigned));\nif (moved.value() != made->value() || assigned.value() != made->value()) return 6;\nreturn 0;\n}\n");
    let dir = std::env::temp_dir().join("ams-gra-task046-cpp-direct");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.hpp"), source).unwrap();
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
    assert!(Command::new(dir.join("probe")).status().unwrap().success());
    std::fs::remove_dir_all(dir).unwrap();
}
