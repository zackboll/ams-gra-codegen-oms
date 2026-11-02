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
        sanitizers_unavailable("g++");
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

/// Sanitizer coverage that cannot run is NOT a pass. It fails the test unless
/// the environment explicitly opts out with `CPP_OMS_ALLOW_SANITIZER_SKIP=1`,
/// in which case a loud SKIPPED line is printed (the harness cannot mark a
/// Rust test as skipped, so the opt-out is the only way to proceed).
fn sanitizers_unavailable(compiler: &str) {
    assert!(
        std::env::var_os("CPP_OMS_ALLOW_SANITIZER_SKIP").is_some(),
        "{compiler} cannot build with -fsanitize=address,undefined; install the sanitizer runtime \
         or set CPP_OMS_ALLOW_SANITIZER_SKIP=1 to record an explicit SKIP (not a pass)"
    );
    eprintln!("SANITIZER SKIPPED: {compiler} cannot build with -fsanitize=address,undefined");
}

fn sanitizer_flags_usable(compiler: &str, label: &str) -> bool {
    let probe = scratch(&format!("sanitize-probe-{label}"));
    std::fs::create_dir_all(&probe).unwrap();
    std::fs::write(probe.join("probe.cpp"), "int main() { return 0; }\n").unwrap();
    Command::new(compiler)
        .args(SANITIZE)
        .arg(probe.join("probe.cpp"))
        .arg("-o")
        .arg(probe.join("probe"))
        .output()
        .is_ok_and(|output| output.status.success())
}

const SANITIZE: &[&str] = &[
    "-std=c++17",
    "-O1",
    "-g",
    "-fsanitize=address,undefined",
    "-fno-sanitize-recover=all",
];

/// CPP-OMS-002: the synthetic shapes the authentic Log closure does not use
/// (repeated hexBinary / dateTime / duration, a named hexBinary chain, a Choice
/// with a reserved-word member and a nested Choice) compile under strict flags
/// with every available compiler and pass.
#[test]
fn synthetic_shapes_for_the_new_value_kinds_pass_under_strict_flags() {
    let consumer = root().join("tests/cpp-codec/shapes_consumer.cpp");
    let mut compilers = vec!["g++"];
    if compiler_available("clang++") {
        compilers.push("clang++");
    }
    for (fixture, shape) in SHAPE_FIXTURES {
        let dir = generate(&shared_case(fixture), true, &format!("shape-{fixture}"));
        for compiler in &compilers {
            let define = format!("-DSHAPE_{shape}");
            let mut flags = STRICT.to_vec();
            flags.push(&define);
            let out = compile_and_run(
                compiler,
                &flags,
                &dir,
                std::slice::from_ref(&consumer),
                &format!("shape-{shape}-{compiler}"),
            );
            assert!(
                out.starts_with(&format!(
                    "CPP_OMS_002_SHAPES_COMPLETE shape={shape} checks="
                )),
                "{fixture}/{compiler}: {out}"
            );
        }
    }
}

#[test]
fn synthetic_shapes_for_the_new_value_kinds_are_clean_under_sanitizers() {
    if !sanitizer_flags_usable("g++", "shapes") {
        sanitizers_unavailable("g++");
        return;
    }
    let consumer = root().join("tests/cpp-codec/shapes_consumer.cpp");
    for (fixture, shape) in SHAPE_FIXTURES {
        let dir = generate(&shared_case(fixture), true, &format!("shape-san-{fixture}"));
        let define = format!("-DSHAPE_{shape}");
        let mut flags = SANITIZE.to_vec();
        flags.push(&define);
        let out = compile_and_run(
            "g++",
            &flags,
            &dir,
            std::slice::from_ref(&consumer),
            &format!("shape-san-{shape}"),
        );
        assert!(
            out.starts_with(&format!(
                "CPP_OMS_002_SHAPES_COMPLETE shape={shape} checks="
            )),
            "{fixture}: {out}"
        );
    }
}

const SHAPE_FIXTURES: &[(&str, &str)] = &[
    ("codec-hexbinary", "HEX"),
    ("codec-ada-repeated-temporal", "TEMPORAL"),
    ("codec-ada-choice", "CHOICE"),
];

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
/// profiles into the subset (see `newly_supported_fixtures_*`), CPP-OMS-004
/// moved Float64 and CPP-OMS-005 moved bounded ASCII; what remains here is
/// still NOT READY.
#[test]
fn unsupported_constructs_are_rejected_before_any_output() {
    let cases: &[(&str, &str)] = &[
        // CPP-OMS-004: Float64 is now in the subset; this fixture's blocker
        // is its direct xs:float member, not its Float64 members.
        ("codec-ada-float64", "PathLength.Single is Float32"),
        (
            "codec-ada-float32",
            "named Float32 has no C++ codec mapping",
        ),
        ("constrained-float", "ThrustRatio: named Float32"),
        ("codec-time-zulu", "Time"),
        (
            "codec-ada-hexbinary-single",
            "length-constrained named hexBinary",
        ),
        (
            "codec-ada-hexbinary-combined",
            "length-constrained named hexBinary",
        ),
        // CPP-OMS-005 moved bounded ASCII into the subset (`codec-bounded-ascii`
        // is READY, see `bounded_ascii_*`); every other String profile with a
        // model carrier stays NOT READY, whatever its name.
        ("codec-structured-ascii", "structured ASCII"),
        ("codec-alternating-ascii", "alternating ASCII"),
        ("codec-ipv6", "IPv6 address"),
        ("codec-unicode31", "Unicode 3.1"),
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

// ---- CPP-OMS-004: Float64 ------------------------------------------------

fn float64_case() -> Case {
    Case {
        schema: fixtures().join("cpp-codec/float64/float64.xsd"),
        contract: fixtures().join("cpp-codec/float64/service.yaml"),
    }
}

/// The exact edge-suite summary of `tests/cpp-codec/float64_consumer.cpp`.
const FLOAT64_EDGE_SUMMARY: &str = "CPP_OMS_004_FLOAT64 checks=305 rejections=103\n";

fn compile(compiler: &str, flags: &[&str], include: &Path, source: &Path, label: &str) -> PathBuf {
    let dir = scratch(&format!("build-{label}"));
    std::fs::create_dir_all(&dir).unwrap();
    let program = dir.join("program");
    let compiled = Command::new(compiler)
        .args(flags)
        .arg("-I")
        .arg(include)
        .arg(source)
        .arg("-o")
        .arg(&program)
        .output()
        .expect("compiler runs");
    assert!(
        compiled.status.success(),
        "{compiler} failed for {label}:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    program
}

fn run_ok(program: &Path, args: &[&Path], label: &str) -> String {
    let ran = Command::new(program).args(args).output().expect("runs");
    assert!(
        ran.status.success(),
        "{label} failed:\n{}{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    String::from_utf8(ran.stdout).unwrap()
}

/// Regenerate the frozen Task 096 corpus with the unchanged exact oracle (its
/// hashes are checked against the committed manifest by the driver).
fn float64_corpus(label: &str) -> PathBuf {
    let dir = scratch(&format!("corpus-{label}"));
    std::fs::create_dir_all(&dir).unwrap();
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join("tests/cpp-codec/float64_oracle.py"))
        .arg("corpus")
        .arg(&dir)
        .output()
        .expect("python3 runs");
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).starts_with(
            "CPP_OMS_004_CORPUS conversions=27167 encodings=20474 \
             sha256-conversions=14c86c5fa674d279d198947dad03f71f517a40b583729df1184bbf6596a6451b"
        ),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    dir.join("full")
}

/// The Float64 runtime section and its headers appear only for a Float64
/// selection; the model and service API stay byte-identical with and without
/// `--with-codec`; generation is deterministic.
#[test]
fn float64_section_is_emitted_only_for_float64_selections() {
    let case = float64_case();
    let with = generate(&case, true, "f64-with");
    let again = generate(&case, true, "f64-again");
    let without = generate(&case, false, "f64-without");
    for file in ["oam.hpp", "service_api.hpp"] {
        assert_eq!(
            std::fs::read(with.join(file)).unwrap(),
            std::fs::read(without.join(file)).unwrap(),
            "{file}"
        );
    }
    let codec = std::fs::read_to_string(with.join("service_codec.hpp")).unwrap();
    assert_eq!(
        codec,
        std::fs::read_to_string(again.join("service_codec.hpp")).unwrap()
    );
    for needle in [
        "#include <charconv>\n#include <cmath>\n#include <system_error>\n\n#include \"oam.hpp\"",
        "inline std::optional<double> get_f64(",
        "std::from_chars(first, last, result, std::chars_format::general)",
        "std::chars_format::scientific",
        "return put_checked_f64<M::AngleDegreesType>(out, value, error, path);",
        "return get_checked_f64<M::TinyType>(value, error, path);",
        "return M::UnboundedDoubleType(*number);",
    ] {
        assert!(codec.contains(needle), "missing {needle}");
    }
    // No bound is restated by the codec: the model owns every limit.
    for bound in [
        "6378237",
        "180.0",
        "3.5",
        "5.0e-324",
        "min_value",
        "max_value",
    ] {
        assert!(!codec.contains(bound), "codec restates the bound {bound}");
    }
    let plain = generate(&codec_case(), true, "f64-free");
    let plain = std::fs::read_to_string(plain.join("service_codec.hpp")).unwrap();
    for needle in ["charconv", "get_f64", "put_f64", "Float64"] {
        assert!(
            !plain.contains(needle),
            "Float64-free codec contains {needle}"
        );
    }
}

/// CPP-OMS-004 numerical qualification: the generated decoder and encoder
/// against the exact-rational oracle, with strict g++ and clang++.
#[test]
fn float64_numerics_match_the_independent_oracle() {
    let dir = generate(&float64_case(), true, "f64-numerics");
    let consumer = root().join("tests/cpp-codec/float64_consumer.cpp");
    let corpus = float64_corpus("numerics");
    let mut compilers = vec!["g++"];
    if compiler_available("clang++") {
        compilers.push("clang++");
    }
    let selftest = Command::new("python3")
        .arg("-I")
        .arg(root().join("tests/cpp-codec/float64_oracle.py"))
        .arg("selftest")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&selftest.stdout),
        "CPP_OMS_004_ORACLE_SELFTEST cases=18 failures=0\n"
    );
    for compiler in compilers {
        let mut flags = STRICT.to_vec();
        flags.push("-O2");
        let program = compile(
            compiler,
            &flags,
            &dir,
            &consumer,
            &format!("f64-{compiler}"),
        );
        assert_eq!(run_ok(&program, &[], compiler), FLOAT64_EDGE_SUMMARY);
        let decode = run_ok(
            &program,
            &[Path::new("decode"), &corpus.join("conversions.tsv")],
            compiler,
        );
        assert_eq!(
            decode,
            "CPP_OMS_004_DECODE rows=27167 finite=27162 overflow=5 mismatches=0\n"
        );
        let tokens = program.with_file_name("tokens.tsv");
        let encode = run_ok(
            &program,
            &[Path::new("encode"), &corpus.join("encoding.tsv"), &tokens],
            compiler,
        );
        assert_eq!(
            encode,
            "CPP_OMS_004_ENCODE rows=20474 roundtrip_mismatches=0\n"
        );
        let checked = Command::new("python3")
            .arg("-I")
            .arg(root().join("tests/cpp-codec/float64_oracle.py"))
            .arg("check-encoding")
            .arg(&tokens)
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&checked.stdout),
            "CPP_OMS_004_ORACLE_ENCODING rows=20474 mismatches=0\n",
            "{compiler}: {}",
            String::from_utf8_lossy(&checked.stderr)
        );
    }
}

#[test]
fn float64_numerics_are_clean_under_sanitizers() {
    if !sanitizer_flags_usable("g++", "f64") {
        sanitizers_unavailable("g++");
        return;
    }
    let dir = generate(&float64_case(), true, "f64-san");
    let consumer = root().join("tests/cpp-codec/float64_consumer.cpp");
    let corpus = float64_corpus("san");
    let program = compile("g++", SANITIZE, &dir, &consumer, "f64-san");
    assert_eq!(run_ok(&program, &[], "f64-san"), FLOAT64_EDGE_SUMMARY);
    assert_eq!(
        run_ok(
            &program,
            &[Path::new("decode"), &corpus.join("conversions.tsv")],
            "f64-san"
        ),
        "CPP_OMS_004_DECODE rows=27167 finite=27162 overflow=5 mismatches=0\n"
    );
}

// ---- CPP-OMS-005: bounded ASCII ---------------------------------------------

fn bounded_ascii_case() -> Case {
    Case {
        schema: fixtures().join("cpp-codec/bounded-ascii/bounded-ascii.xsd"),
        contract: fixtures().join("cpp-codec/bounded-ascii/service.yaml"),
    }
}

/// The exact summary of `tests/cpp-codec/bounded_ascii_consumer.cpp`.
const BOUNDED_ASCII_SUMMARY: &str = "CPP_OMS_005_BOUNDED_ASCII checks=289 rejections=148\n";

/// Readiness and generation agree; the codec is a separate artifact; it
/// delegates every bounded-ASCII value to the model's checked `create` and
/// restates no character class, length or pattern.
#[test]
fn bounded_ascii_codec_is_ready_delegating_and_separate() {
    let case = bounded_ascii_case();
    let check = run("service-check", &case, true, None);
    let report = String::from_utf8(check.stdout).unwrap();
    assert_eq!(check.status.code(), Some(0), "{report}");
    assert!(
        report.contains("codec renderable emitted declarations: 15/15\n"),
        "{report}"
    );
    assert!(report.ends_with("codec status: READY\n"), "{report}");
    let with = generate(&case, true, "ba-with");
    let again = generate(&case, true, "ba-again");
    let without = generate(&case, false, "ba-without");
    for file in ["oam.hpp", "service_api.hpp"] {
        assert_eq!(
            std::fs::read(with.join(file)).unwrap(),
            std::fs::read(without.join(file)).unwrap(),
            "{file} must not depend on --with-codec"
        );
    }
    let codec = std::fs::read_to_string(with.join("service_codec.hpp")).unwrap();
    assert_eq!(
        codec,
        std::fs::read_to_string(again.join("service_codec.hpp")).unwrap()
    );
    for carrier in [
        "EmptyMarkerType",
        "TailType",
        "CodeType",
        "NameType",
        "LabelType",
        "MissionType",
        "UnitNameType",
        "SerialType",
        "PinType",
        "OctalType",
        "VisibleType",
        "LaunchType",
    ] {
        assert!(
            codec.contains(&format!(
                "return get_checked_text<M::{carrier}>(value, error, path, \"{carrier}\");"
            )),
            "{carrier} must decode through the model's create"
        );
    }
    // The model owns the profile: nothing of it is restated by the codec, and
    // no regex machinery or extra dependency appears.
    for restated in [
        "kLength",
        "kMinLength",
        "kMaxLength",
        "is_member",
        "ordinal",
        "0x41",
        "<regex>",
        "std::regex",
        "Float64",
    ] {
        assert!(!codec.contains(restated), "codec restates {restated}");
    }
    for line in codec.lines().filter(|line| line.starts_with("#include")) {
        assert!(
            line.contains('<') || line == "#include \"oam.hpp\"",
            "unexpected include: {line}"
        );
    }
}

/// Positive, negative and encode cases against hand-written JSON, with every
/// available compiler under the strict flags.
#[test]
fn bounded_ascii_consumer_passes_under_strict_flags() {
    let dir = generate(&bounded_ascii_case(), true, "ba-consumer");
    let consumer = root().join("tests/cpp-codec/bounded_ascii_consumer.cpp");
    let mut compilers = vec!["g++"];
    if compiler_available("clang++") {
        compilers.push("clang++");
    }
    for compiler in compilers {
        let mut flags = STRICT.to_vec();
        flags.push("-O1");
        let out = compile_and_run(
            compiler,
            &flags,
            &dir,
            std::slice::from_ref(&consumer),
            &format!("ba-{compiler}"),
        );
        assert_eq!(out, BOUNDED_ASCII_SUMMARY, "{compiler}");
    }
}

#[test]
fn bounded_ascii_consumer_is_clean_under_sanitizers() {
    if !sanitizer_flags_usable("g++", "ba") {
        sanitizers_unavailable("g++");
        return;
    }
    let dir = generate(&bounded_ascii_case(), true, "ba-san");
    let consumer = root().join("tests/cpp-codec/bounded_ascii_consumer.cpp");
    let out = compile_and_run(
        "g++",
        SANITIZE,
        &dir,
        std::slice::from_ref(&consumer),
        "ba-san",
    );
    assert_eq!(out, BOUNDED_ASCII_SUMMARY);
}

/// Negative controls: deliberately weakened copies of the generated model and
/// codec (length, class, case, non-ASCII, control characters, trimming,
/// present-empty as absent, JSON kind, error path, escaping) must each make
/// the consumer fail.
#[test]
fn bounded_ascii_mutation_controls_are_detected() {
    let dir = generate(&bounded_ascii_case(), true, "ba-mutants-src");
    let work = scratch("ba-mutants");
    let out = Command::new("python3")
        .arg("-I")
        .arg(root().join("tests/cpp-codec/bounded_ascii_mutations.py"))
        .arg(&dir)
        .arg(root().join("tests/cpp-codec/bounded_ascii_consumer.cpp"))
        .arg(&work)
        .output()
        .expect("python3 runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.ends_with("CPP_OMS_005_MUTATIONS detected=13/13\n"),
        "{stdout}"
    );
}

/// The shared Task 058 fixture (also used by the Rust codec) is C++ codec
/// READY too, and round-trips the same documents the Rust mock-OWP test uses.
#[test]
fn task058_shared_bounded_ascii_fixture_round_trips_in_cpp() {
    let case = shared_case("codec-bounded-ascii");
    let check = run("service-check", &case, true, None);
    let report = String::from_utf8(check.stdout).unwrap();
    assert_eq!(check.status.code(), Some(0), "{report}");
    assert!(report.ends_with("codec status: READY\n"), "{report}");
    let dir = generate(&case, true, "ba-shared");
    let source = scratch("ba-shared-source");
    std::fs::create_dir_all(&source).unwrap();
    let main = source.join("main.cpp");
    std::fs::write(
        &main,
        r#"#include "oam.hpp"
#include "service_codec.hpp"
#include <iostream>
#include <string>
int main() {
    namespace sc = ::service_codec;
    const std::string wire = R"({"Marker":"","Tail":"N123AB  ","Markers":[""],"Pick":{"Empty":""}})";
    const auto decoded = sc::decode_BoundedPayload(wire);
    if (!decoded.ok() || decoded.value().tail.value() != "N123AB  " || !decoded.value().marker.value().empty()) {
        return 1;
    }
    if (sc::encode_BoundedPayload(decoded.value()).value() != wire) {
        return 2;
    }
    const std::string spaced = R"({"Marker":"","Tail":" 7 7 7 7","Label":"a b","Pick":{"Launch":"Z"}})";
    const auto label = sc::decode_BoundedPayload(spaced);
    if (!label.ok() || label.value().tail.value() != " 7 7 7 7" || !label.value().label ||
        label.value().label->value() != "a b" || sc::encode_BoundedPayload(label.value()).value() != spaced) {
        return 3;
    }
    const auto wrong = sc::decode_BoundedPayload(R"({"Marker":"","Tail":"n123ab  ","Pick":{"Launch":"Z"}})");
    if (wrong.ok() || wrong.error().path != "BoundedPayload.Tail" ||
        wrong.error().message != "value rejected by the generated model's TailType check") {
        return 4;
    }
    std::cout << "shared bounded ascii ok\n";
    return 0;
}
"#,
    )
    .unwrap();
    let out = compile_and_run("g++", STRICT, &dir, &[main], "ba-shared");
    assert_eq!(out, "shared bounded ascii ok\n");
}
