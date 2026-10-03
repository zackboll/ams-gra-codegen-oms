//! Task 059: generated checked C++17 carriers against the shared corpus.
mod common;
use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::{fmt::Write as _, path::Path, process::Command};

#[test]
fn generated_structured_ascii_cpp_corpus() {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-structured-ascii.xsd"),
    )
    .unwrap();
    let model = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let dir = std::env::temp_dir().join("task059-cpp-corpus");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.hpp"), model).unwrap();
    let groups = common::structured_ascii_cases();
    assert_eq!(groups.len(), 12);
    let mut probe = String::from(
        "#include \"generated.hpp\"\n#include <type_traits>\n#include <cstdio>\nusing namespace test::structured;\nint main() {\n",
    );
    for group in groups {
        writeln!(
            probe,
            "static_assert(!std::is_default_constructible_v<{}>);",
            group.carrier
        )
        .unwrap();
        for (index, case) in group.cases.iter().enumerate() {
            let input = common::escape_for_cpp_source(&case.input);
            let expected = case
                .expected
                .as_ref()
                .map(|s| {
                    format!(
                        "v && v->value() == std::string(\"{}\", {})",
                        common::escape_for_cpp_source(s),
                        s.len()
                    )
                })
                .unwrap_or_else(|| "!v".into());
            writeln!(probe,"{{ auto v = {}::create(std::string(\"{input}\", {})); if (!({expected})) {{ std::puts(\"{} {index}\"); return 1; }} }}",group.carrier,case.input.len(),group.carrier).unwrap();
        }
    }
    probe.push_str("return 0;\n}\n");
    std::fs::write(dir.join("probe.cpp"), probe).unwrap();
    let built = Command::new("c++")
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
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new(dir.join("probe")).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}
