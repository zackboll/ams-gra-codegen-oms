//! CPP-OMS-001: generated C++17 OMS JSON codec.
//!
//! * readiness and generation agree (`service-check` / `service-generate`
//!   `--language cpp --with-codec`), and a NOT READY codec writes nothing;
//! * the codec is a separate artifact: model and service API headers are
//!   byte-identical with and without the flag;
//! * the exact generated header is pinned by a golden file;
//! * generated consumers compile under strict C++17 flags and exercise
//!   round trips, exact wire text and malformed-input rejection.
//!
//! Re-bless the golden after an intentional generator change with
//! `CPP_OMS_001_BLESS=1 cargo test -p ams-gra-codegen-oms --test cpp_codec`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const STRICT: &[&str] = &[
    "-std=c++17",
    "-Wall",
    "-Wextra",
    "-Wpedantic",
    "-Werror",
    "-Wconversion",
    "-Wsign-conversion",
    "-Wshadow",
    "-Wold-style-cast",
    "-Wcast-align",
    "-Wnon-virtual-dtor",
    "-Woverloaded-virtual",
    "-Wnull-dereference",
    "-Wdouble-promotion",
    "-Wformat=2",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> PathBuf {
    root().join("tests/fixtures")
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ams-gra-oms-cpp-oms-001-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

struct Case {
    schema: PathBuf,
    contract: PathBuf,
}

fn codec_case() -> Case {
    Case {
        schema: fixtures().join("cpp-codec/oam.xsd"),
        contract: fixtures().join("cpp-codec/service.yaml"),
    }
}

fn shared_case(name: &str) -> Case {
    Case {
        schema: fixtures().join(format!("service-generate/{name}.xsd")),
        contract: fixtures().join(format!("service-generate/{name}.yaml")),
    }
}

fn run(command: &str, case: &Case, codec: bool, output: Option<&Path>) -> Output {
    let mut process = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    process
        .arg(command)
        .arg("--schema")
        .arg(&case.schema)
        .arg("--contract")
        .arg(&case.contract)
        .args(["--language", "cpp", "--world", "closed-schema"]);
    if codec {
        process.arg("--with-codec");
    }
    if let Some(output) = output {
        process.arg("--output").arg(output);
    }
    process.output().expect("CLI runs")
}

fn generate(case: &Case, codec: bool, label: &str) -> PathBuf {
    let dir = scratch(label);
    let output = run("service-generate", case, codec, Some(&dir));
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    dir
}

fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("output dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn compiler_available(compiler: &str) -> bool {
    Command::new(compiler)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Compile `sources` with `flags` against generated headers in `include`, and
/// run the program. Returns its stdout.
fn compile_and_run(
    compiler: &str,
    flags: &[&str],
    include: &Path,
    sources: &[PathBuf],
    label: &str,
) -> String {
    let dir = scratch(&format!("build-{label}"));
    std::fs::create_dir_all(&dir).unwrap();
    let program = dir.join("program");
    let compiled = Command::new(compiler)
        .args(flags)
        .arg("-I")
        .arg(include)
        .args(sources)
        .arg("-o")
        .arg(&program)
        .output()
        .expect("compiler runs");
    assert!(
        compiled.status.success(),
        "{compiler} failed for {label}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let ran = Command::new(&program).output().expect("program runs");
    assert!(
        ran.status.success(),
        "{label} failed:\n{}{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    String::from_utf8(ran.stdout).unwrap()
}

#[test]
fn readiness_report_names_the_codec_status() {
    let case = codec_case();
    let output = run("service-check", &case, true, None);
    assert_eq!(output.status.code(), Some(0));
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("status: READY\n"), "{report}");
    assert!(
        report.contains("codec renderable emitted declarations: 10/10\n"),
        "{report}"
    );
    assert!(report.ends_with("codec status: READY\n"), "{report}");
    // Without the flag the report has no codec line at all.
    let plain = run("service-check", &case, false, None);
    let plain = String::from_utf8(plain.stdout).unwrap();
    assert!(!plain.contains("codec"), "{plain}");
}

#[test]
fn codec_is_a_separate_artifact_and_leaves_the_model_and_api_unchanged() {
    let case = codec_case();
    let with = generate(&case, true, "with");
    let without = generate(&case, false, "without");
    assert_eq!(listing(&without), ["oam.hpp", "service_api.hpp"]);
    assert_eq!(
        listing(&with),
        ["oam.hpp", "service_api.hpp", "service_codec.hpp"]
    );
    for file in ["oam.hpp", "service_api.hpp"] {
        assert_eq!(
            std::fs::read(with.join(file)).unwrap(),
            std::fs::read(without.join(file)).unwrap(),
            "{file} must not depend on --with-codec"
        );
    }
    // The default summary is unchanged; the flag only appends codec lines.
    let plain = run(
        "service-generate",
        &case,
        false,
        Some(&scratch("summary-plain")),
    );
    let plain = String::from_utf8(plain.stdout).unwrap();
    assert!(!plain.contains("codec"), "{plain}");
    let flagged = run(
        "service-generate",
        &case,
        true,
        Some(&scratch("summary-flag")),
    );
    let flagged = String::from_utf8(flagged.stdout).unwrap();
    assert!(
        flagged.contains("generated service codec files: 1\n"),
        "{flagged}"
    );
    assert!(
        flagged.ends_with("generated 3 file(s) including the codec\n"),
        "{flagged}"
    );
}

#[test]
fn generated_codec_matches_the_golden_and_is_deterministic() {
    let case = codec_case();
    let first = generate(&case, true, "golden-a");
    let second = generate(&case, true, "golden-b");
    let generated = std::fs::read_to_string(first.join("service_codec.hpp")).unwrap();
    assert_eq!(
        generated,
        std::fs::read_to_string(second.join("service_codec.hpp")).unwrap()
    );
    let golden = fixtures().join("cpp-codec/expected/service_codec.hpp");
    if std::env::var_os("CPP_OMS_001_BLESS").is_some() {
        std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
        std::fs::write(&golden, &generated).unwrap();
    }
    assert_eq!(
        generated,
        std::fs::read_to_string(&golden).expect("golden; re-bless with CPP_OMS_001_BLESS=1"),
        "generated service_codec.hpp differs from the golden"
    );
    // One pair per UNIQUE selected payload (two messages share Payload), and
    // nothing for a declaration outside the selected closure.
    assert_eq!(generated.matches("encode_Payload(const").count(), 1);
    assert_eq!(
        generated.matches("decode_Payload(std::string_view").count(),
        1
    );
    assert!(generated.contains("encode_Ack(const"));
    assert!(!generated.contains("Unselected"));
    // Standard library only: no third-party include.
    for line in generated
        .lines()
        .filter(|line| line.starts_with("#include"))
    {
        assert!(
            line.contains('<') || line == "#include \"oam.hpp\"",
            "unexpected include: {line}"
        );
    }
}

#[test]
fn consumer_compiles_under_strict_flags_and_passes() {
    let case = codec_case();
    let dir = generate(&case, true, "consumer");
    let consumer = root().join("tests/cpp-codec/consumer.cpp");
    let mut compilers = vec!["g++"];
    if compiler_available("clang++") {
        compilers.push("clang++");
    }
    for compiler in compilers {
        let out = compile_and_run(
            compiler,
            STRICT,
            &dir,
            std::slice::from_ref(&consumer),
            &format!("consumer-{compiler}"),
        );
        assert!(out.ends_with(" 0 failures\n"), "{compiler}: {out}");
    }
}

#[test]
fn consumer_is_clean_under_address_and_undefined_behavior_sanitizers() {
    let case = codec_case();
    let dir = generate(&case, true, "sanitize");
    let consumer = root().join("tests/cpp-codec/consumer.cpp");
    let probe = scratch("sanitize-probe");
    std::fs::create_dir_all(&probe).unwrap();
    std::fs::write(probe.join("probe.cpp"), "int main() { return 0; }\n").unwrap();
    let flags = [
        "-std=c++17",
        "-O1",
        "-g",
        "-fsanitize=address,undefined",
        "-fno-sanitize-recover=all",
    ];
    let usable = Command::new("g++")
        .args(flags)
        .arg(probe.join("probe.cpp"))
        .arg("-o")
        .arg(probe.join("probe"))
        .output()
        .is_ok_and(|output| output.status.success());
    if !usable {
        eprintln!("skipping: this g++ cannot build with -fsanitize=address,undefined");
        return;
    }
    let out = compile_and_run(
        "g++",
        &flags,
        &dir,
        std::slice::from_ref(&consumer),
        "sanitize",
    );
    assert!(out.ends_with(" 0 failures\n"), "{out}");
}

/// Non-OAM namespace: member keys are Clark-qualified, a Record's own "$type"
/// is Clark-qualified too, inheritance is flattened, and a local element with
/// an explicit `form="qualified"` override is keyed by the schema namespace.
#[test]
fn non_oam_namespaces_use_clark_names_and_flattened_inheritance() {
    let inherit = shared_case("codec-inherit");
    let dir = generate(&inherit, true, "inherit");
    let source = scratch("inherit-source");
    std::fs::create_dir_all(&source).unwrap();
    let main = source.join("main.cpp");
    let header = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .find(|name| name != "service_api.hpp" && name != "service_codec.hpp")
        .expect("model header");
    std::fs::write(
        &main,
        format!(
            r#"#include "{header}"
#include "service_codec.hpp"
#include <iostream>
int main() {{
    namespace sc = ::service_codec;
    const std::string wire =
        R"({{"{{urn:inherit}}BaseValue":-7,"{{urn:inherit}}ExtraValue":"x"}})";
    const auto decoded = sc::decode_Derived(wire);
    if (!decoded.ok() || decoded.value().basevalue.value() != -7 || decoded.value().extravalue != "x") {{
        std::cerr << "decode failed\n";
        return 1;
    }}
    const auto again = sc::encode_Derived(decoded.value());
    if (!again.ok() || again.value() != wire) {{
        std::cerr << "encode mismatch: " << again.value() << "\n";
        return 1;
    }}
    // Bare local names are NOT the wire names outside the OAM namespace.
    if (sc::decode_Derived(R"({{"BaseValue":1,"ExtraValue":"x"}})").ok()) {{
        std::cerr << "bare names accepted\n";
        return 1;
    }}
    // "$type" must be the Clark-qualified name of exactly this type.
    if (!sc::decode_Derived(R"({{"$type":"{{urn:inherit}}Derived","{{urn:inherit}}BaseValue":1,"{{urn:inherit}}ExtraValue":"x"}})").ok()) {{
        std::cerr << "own $type rejected\n";
        return 1;
    }}
    if (sc::decode_Derived(R"({{"$type":"Derived","{{urn:inherit}}BaseValue":1,"{{urn:inherit}}ExtraValue":"x"}})").ok()) {{
        std::cerr << "bare $type accepted\n";
        return 1;
    }}
    // No nested base object.
    if (sc::decode_Derived(R"({{"Base":{{}},"{{urn:inherit}}ExtraValue":"x"}})").ok()) {{
        return 1;
    }}
    std::cout << "inherit ok\n";
    return 0;
}}
"#
        ),
    )
    .unwrap();
    let out = compile_and_run("g++", STRICT, &dir, &[main], "inherit");
    assert_eq!(out, "inherit ok\n");

    let qualified = shared_case("codec-form-qualified");
    let dir = generate(&qualified, true, "form");
    let main = source.join("form.cpp");
    let header = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .find(|name| name != "service_api.hpp" && name != "service_codec.hpp")
        .expect("model header");
    std::fs::write(
        &main,
        format!(
            r#"#include "{header}"
#include "service_codec.hpp"
#include <iostream>
int main() {{
    namespace sc = ::service_codec;
    const auto decoded = sc::decode_Payload(R"({{"{{urn:form}}Value":5}})");
    if (!decoded.ok() || decoded.value().value.value() != 5) {{
        return 1;
    }}
    if (sc::encode_Payload(decoded.value()).value() != R"({{"{{urn:form}}Value":5}})") {{
        return 1;
    }}
    if (sc::decode_Payload(R"({{"Value":5}})").ok()) {{
        return 1;
    }}
    std::cout << "form ok\n";
    return 0;
}}
"#
        ),
    )
    .unwrap();
    let out = compile_and_run("g++", STRICT, &dir, &[main], "form");
    assert_eq!(out, "form ok\n");
}

/// Every construct outside the subset is rejected explicitly, by BOTH
/// commands, while the MODEL stays READY (so the blocker is the codec's) and
/// before the output directory exists.
///
/// CPP-OMS-002 moved DateTime, Duration, hexBinary, Choice and the Log String
/// profiles into the subset (see `newly_supported_fixtures_*`); what remains
/// here is still NOT READY.
#[test]
fn unsupported_constructs_are_rejected_before_any_output() {
    let cases: &[(&str, &str)] = &[
        ("codec-ada-float64", "Float64"),
        ("codec-ada-float32", "Float32"),
        ("codec-time-zulu", "Time"),
        ("codec-bounded-ascii", "bounded ASCII"),
        ("codec-alternating-ascii", "alternating ASCII"),
        ("codec-ipv6", "IPv6 address"),
        ("codec-shape", "abstract structural value"),
        ("codec-form-unqualified", "unqualified local element"),
        ("codec-unqualified", "unqualified local element"),
    ];
    for (name, expected) in cases {
        let case = shared_case(name);
        let check = run("service-check", &case, true, None);
        let report = String::from_utf8(check.stdout).unwrap();
        assert_eq!(check.status.code(), Some(1), "{name}: {report}");
        assert!(
            report.contains("status: READY\n"),
            "{name}: model must stay READY: {report}"
        );
        assert!(
            report.contains("codec status: NOT READY\n"),
            "{name}: {report}"
        );
        assert!(
            report.contains("service codec boundary: "),
            "{name}: {report}"
        );
        assert!(
            report.contains(expected),
            "{name} should mention {expected}: {report}"
        );

        let dir = scratch(&format!("reject-{name}"));
        let generated = run("service-generate", &case, true, Some(&dir));
        assert_eq!(generated.status.code(), Some(1), "{name}");
        assert!(
            !dir.exists(),
            "{name}: NOT READY must not create the output directory"
        );
        assert!(
            String::from_utf8_lossy(&generated.stderr).contains("no files were generated"),
            "{name}"
        );
        // The same selection is still generated normally WITHOUT the flag.
        let plain = run(
            "service-generate",
            &case,
            false,
            Some(&scratch(&format!("plain-{name}"))),
        );
        assert_eq!(plain.status.code(), Some(0), "{name}");
    }
}

/// A service with no OMS exchange needs no codec: no file is written and the
/// rest of the output is byte-identical to default generation.
#[test]
fn zero_oms_service_gets_no_codec_file() {
    let case = shared_case("non-uci");
    let case = Case {
        schema: fixtures().join("service-generate/root.xsd"),
        contract: case.contract,
    };
    let with = generate(&case, true, "zero-with");
    let without = generate(&case, false, "zero-without");
    assert_eq!(listing(&with), listing(&without));
    assert!(!listing(&with).contains(&"service_codec.hpp".to_owned()));
}
