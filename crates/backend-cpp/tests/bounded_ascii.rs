//! Task 058: generated C++17 bounded-ASCII carriers, built with
//! `-std=c++17 -Wall -Wextra -Werror -pedantic-errors` and run against the
//! shared corpus for EVERY admitted alphabet.

mod common;

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn source() -> String {
    let schema = load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-bounded-ascii.xsd"),
    )
    .expect("fixture parses");
    generate(&schema, GenerationWorld::ClosedSchemaSet).expect("fixture generates")
}

#[test]
fn bounded_ascii_carriers_keep_the_validated_carrier_api() {
    let source = source();
    for carrier in ["Blank", "Tail8", "Serial7", "Fixed10", "DerivedTail"] {
        assert!(source.contains(&format!(
            "static std::optional<{carrier}> create(std::string_view value)"
        )));
        assert!(source.contains(&format!("explicit {carrier}(std::string validated)")));
        assert!(source.contains(&format!("{carrier}(const {carrier}&) = default;")));
    }
    assert!(source.contains("const std::string& value() const noexcept"));
    assert!(source.contains("static constexpr std::size_t kLength = 0;"));
    assert!(source.contains("static constexpr std::size_t kMinLength = 4;"));
    assert!(source.contains("(ordinal >= 0x3B && ordinal <= 0x60)"));
    for forbidden in [
        "<regex>",
        "std::regex",
        "<cctype>",
        "isalnum",
        "std::locale",
        "operator==",
    ] {
        assert!(!source.contains(forbidden), "{forbidden}");
    }
    for needle in [
        "Blank marker;",
        "std::optional<Blank> optionalmarker;",
        "BoundedVector<Blank, 0, 3> markers;",
        "UnboundedVector<Code4, 0> codes;",
        "Serial7 serial;",
        "struct ByEmpty { Blank value; };",
        "std::string notes;",
    ] {
        assert!(source.contains(needle), "missing {needle:?}");
    }
}

#[test]
fn generated_bounded_ascii_validators_match_the_shared_corpus() {
    let groups = common::bounded_ascii_cases();
    assert_eq!(groups.len(), 23);
    let mut probe = String::from(PROBE_HEAD);
    for group in &groups {
        let carrier = &group.carrier;
        writeln!(
            probe,
            "static_assert(!std::is_default_constructible_v<{carrier}>);\n\
             static_assert(std::is_copy_constructible_v<{carrier}>);\n\
             static_assert(!std::is_constructible_v<{carrier}, std::string>);"
        )
        .unwrap();
    }
    probe.push_str("int main() {\n");
    let mut total = 0;
    for group in &groups {
        for (index, case) in group.cases.iter().enumerate() {
            probe.push_str(&case_line(&group.carrier, index, case));
            total += 1;
        }
    }
    assert!(total >= 700, "{total}");
    probe.push_str(PROBE_TAIL);
    compile_and_run(&probe);
}

/// One corpus case. Explicit byte lengths so an embedded NUL is really
/// passed through rather than truncating the literal.
fn case_line(carrier: &str, index: usize, case: &common::TemporalCase) -> String {
    let input = format!(
        "std::string(\"{}\", {})",
        common::escape_for_cpp_source(&case.input),
        case.input.len()
    );
    let mut line = String::new();
    match &case.expected {
        Some(stored) => writeln!(
            line,
            "  {{ auto v = {carrier}::create({input}); if (!v || v->value() != std::string(\"{}\", {})) {{ std::puts(\"{carrier} case {index}\"); return 1; }} }}",
            common::escape_for_cpp_source(stored),
            stored.len()
        ),
        None => writeln!(
            line,
            "  if ({carrier}::create({input})) {{ std::puts(\"{carrier} case {index}\"); return 1; }}"
        ),
    }
    .unwrap();
    line
}

fn compile_and_run(probe: &str) {
    let dir = std::env::temp_dir().join("ams-gra-task058-cpp");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("generated.hpp"), source()).unwrap();
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
        .expect("a C++ compiler is available");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Command::new(dir.join("probe")).output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stdout)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

const PROBE_HEAD: &str = "#include \"generated.hpp\"\n#include <cstdio>\n#include <string>\n\
#include <type_traits>\n#include <utility>\n#include <vector>\nusing namespace test::bounded;\n";

/// Zero-length values in every storage position, Task 040 copy-only moves,
/// spare capacity, the derived chain, inheritance and Choice.
const PROBE_TAIL: &str = r#"  auto empty = Blank::create("");
  if (!empty || !empty->value().empty()) return 2;
  Blank source = *empty;
  Blank moved(std::move(source));
  if (!source.value().empty() || !moved.value().empty()) return 3;
  Tail8 tail = *Tail8::create("AB12 XY ");
  Tail8 target = *Tail8::create("        ");
  target = std::move(tail);
  if (tail.value() != "AB12 XY " || target.value() != "AB12 XY ") return 4;
  auto markers = decltype(Payload::markers)::create({*empty, *empty});
  if (!markers) return 5;
  if (decltype(Payload::markers)::create({*empty, *empty, *empty, *empty})) return 6;
  auto codes = decltype(Payload::codes)::create(
      std::vector<Code4>(40, *Code4::create("aB3z")));
  if (!codes) return 7;
  Payload payload{*empty, target, *empty, std::nullopt, *markers, *codes,
                  *DerivedTail::create("        "), " free text "};
  if (!payload.optionalmarker || !payload.optionalmarker->value().empty()) return 8;
  if (payload.optionalword) return 9;
  if (DerivedTail::create("AB12XY")) return 10;
  Concrete concrete{*Letters3::create("ABC"), *Serial7::create("\"'\\[]{}")};
  if (concrete.serial.value() != "\"'\\[]{}") return 11;
  Pick pick{Pick::ByEmpty{*empty}};
  if (!std::get<Pick::ByEmpty>(pick.value).value.value().empty()) return 12;
  if (Word20::create("abc")->value() == Word20::create("abc ")->value()) return 13;
  std::puts("ok");
  return 0;
}
"#;
