//! Task 050: `--with-codec` readiness and generation controls.
//!
//! Codec readiness is opt-in and separate from model readiness: the MODEL may
//! be READY while its codec is not, and every codec failure is a distinct
//! `service codec boundary:` line that stops generation before any file is
//! written. Without `--with-codec` nothing changes. Task 051 replaces the
//! blanket non-OAM rejection with the element-QName semantics.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate")
        .join(name)
}

fn output_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ams-gra-oms-task050-cli-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn run(args: &[&str], schema: &str, contract: &str, output: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    command
        .arg(args[0])
        .arg("--schema")
        .arg(fixture(schema))
        .arg("--contract")
        .arg(fixture(contract))
        .args(&args[1..]);
    if let Some(output) = output {
        command.arg("--output").arg(output);
    }
    command.output().expect("CLI runs")
}

fn check(schema: &str, contract: &str, language: &str, codec: bool) -> (Option<i32>, String) {
    let mut args = vec![
        "service-check",
        "--language",
        language,
        "--world",
        "closed-schema",
    ];
    if codec {
        args.push("--with-codec");
    }
    let output = run(&args, schema, contract, None);
    (
        output.status.code(),
        String::from_utf8(output.stdout).expect("UTF-8"),
    )
}

fn generate(
    schema: &str,
    contract: &str,
    language: &str,
    codec: bool,
    label: &str,
) -> (Output, PathBuf) {
    let dir = output_dir(label);
    let mut args = vec![
        "service-generate",
        "--language",
        language,
        "--world",
        "closed-schema",
    ];
    if codec {
        args.push("--with-codec");
    }
    (run(&args, schema, contract, Some(&dir)), dir)
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

/// A. The OAM codec-safe service is READY with the codec and generates it.
#[test]
fn task050_oam_codec_service_is_ready_and_generates_the_codec() {
    let (code, report) = check("codec-oam.xsd", "codec-oam.yaml", "rust", true);
    assert_eq!(code, Some(0), "{report}");
    assert!(report.contains("status: READY"));
    // 16 projected declarations; ShapeBase is emitted as the closed-sum
    // wrapper; IdentityBase is a concrete Record; all 16 are emitted.
    assert!(
        report.contains("codec renderable emitted declarations: 16/16\n"),
        "{report}"
    );
    assert!(report.contains("codec status: READY\n"));
    assert!(!report.contains("service codec boundary"));

    let (output, dir) = generate("codec-oam.xsd", "codec-oam.yaml", "rust", true, "oam");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        listing(&dir),
        ["oam.rs", "service_api.rs", "service_codec.rs"]
    );
    let wrapper = std::fs::read_to_string(dir.join("service_api.rs")).expect("wrapper");
    assert!(wrapper.contains("#[path = \"oam.rs\"]\npub mod model;\n\n#[path = \"service_codec.rs\"]\npub mod service_codec;\n"));
    let codec = std::fs::read_to_string(dir.join("service_codec.rs")).expect("codec");
    // Two messages share one payload: exactly one impl.
    assert_eq!(codec.matches("impl OmsJsonCodec<").count(), 1);
    assert!(codec.contains("impl OmsJsonCodec<super::model::CodecPayload> for ServiceCodec"));
    // The only public item is ServiceCodec; helpers are private emission IDs.
    let public: Vec<&str> = codec
        .lines()
        .filter(|line| line.starts_with("pub "))
        .collect();
    assert_eq!(public, ["pub struct ServiceCodec;"]);
    assert!(codec.contains("fn encode_t000(") && codec.contains("fn decode_t000("));
    // No serde derive anywhere, and the model is untouched by the flag.
    assert!(!codec.contains("derive(Serialize") && !codec.contains("Deserialize"));
    let (_, plain) = generate(
        "codec-oam.xsd",
        "codec-oam.yaml",
        "rust",
        false,
        "oam-plain",
    );
    assert_eq!(
        std::fs::read(dir.join("oam.rs")).expect("model"),
        std::fs::read(plain.join("oam.rs")).expect("plain model")
    );
    assert_eq!(listing(&plain), ["oam.rs", "service_api.rs"]);
}

/// B. A selected Binary.
///
/// Historical (Task 050): model READY but codec NOT READY and nothing
/// written, because Schema IR did not retain whether the XSD primitive was
/// hexBinary or base64Binary ("service codec boundary: BlobPayload.Data is
/// Binary; Schema IR does not retain whether the XSD primitive was hexBinary
/// or base64Binary, ...").
///
/// Task 052: the frontend records xs:hexBinary lexical provenance, so the
/// SAME fixture is model READY and codec READY, and `Data` is spelled as
/// canonical hexBinary. Default output is unchanged. The unknown-provenance
/// negative control is codegen-core `task052_unknown_binary_provenance_fails_closed`
/// (manual IR), because the production frontend no longer produces it.
#[test]
fn task052_selected_hex_binary_is_model_and_codec_ready() {
    let (code, plain) = check("codec-binary.xsd", "codec-binary.yaml", "rust", false);
    assert_eq!(code, Some(0), "{plain}");
    assert!(!plain.contains("codec status") && !plain.contains("service codec boundary"));
    let (code, report) = check("codec-binary.xsd", "codec-binary.yaml", "rust", true);
    assert_eq!(code, Some(0), "{report}");
    assert!(
        report.starts_with(&plain),
        "the codec section is only appended"
    );
    assert!(
        report.ends_with("\ncodec renderable emitted declarations: 1/1\ncodec status: READY\n"),
        "{report}"
    );
    assert!(!report.contains("service codec boundary"));

    let (output, dir) = generate(
        "codec-binary.xsd",
        "codec-binary.yaml",
        "rust",
        true,
        "binary",
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        listing(&dir),
        ["oam.rs", "service_api.rs", "service_codec.rs"]
    );
    let codec = std::fs::read_to_string(dir.join("service_codec.rs")).expect("codec");
    assert!(codec.contains(
        "object.insert(\"Data\".to_owned(), { let x = &value.data; encode_hex_binary(x) });"
    ));
    assert!(codec.contains(
        "data: decode_hex_binary(required(object, \"Data\", path)?, &format!(\"{path}.Data\"))?,"
    ));
    // Exactly ONE private helper pair, no public helper, no serde bytes.
    assert_eq!(codec.matches("fn encode_hex_binary(").count(), 1);
    assert_eq!(codec.matches("fn decode_hex_binary(").count(), 1);
    let public: Vec<&str> = codec
        .lines()
        .filter(|line| line.starts_with("pub "))
        .collect();
    assert_eq!(public, ["pub struct ServiceCodec;"]);
    assert!(!codec.contains("serde_bytes") && !codec.contains("hex::"));

    // The model is byte-identical with and without the flag, and still bytes.
    let (_, plain_dir) = generate(
        "codec-binary.xsd",
        "codec-binary.yaml",
        "rust",
        false,
        "binary-plain",
    );
    assert_eq!(listing(&plain_dir), ["oam.rs", "service_api.rs"]);
    let model = std::fs::read(dir.join("oam.rs")).expect("model");
    assert_eq!(
        model,
        std::fs::read(plain_dir.join("oam.rs")).expect("plain")
    );
    assert!(
        String::from_utf8(model)
            .expect("UTF-8")
            .contains("    pub data: Vec<u8>,\n")
    );
}

/// Task 052: a codec surface WITHOUT Binary emits no hex helper at all.
#[test]
fn task052_hex_helpers_are_emitted_only_for_hex_binary_surfaces() {
    for (schema, contract, label) in [
        ("codec-oam.xsd", "codec-oam.yaml", "nohex-oam"),
        ("runtime-test.xsd", "runtime-test.yaml", "nohex-test"),
        ("codec-choice.xsd", "codec-choice.yaml", "nohex-choice"),
    ] {
        let (output, dir) = generate(schema, contract, "rust", true, label);
        assert_eq!(output.status.code(), Some(0), "{schema}");
        let codec = std::fs::read_to_string(dir.join("service_codec.rs")).expect("codec");
        assert!(!codec.contains("hex_binary"), "{schema}");
    }
    let (output, dir) = generate(
        "codec-hexbinary.xsd",
        "codec-hexbinary.yaml",
        "rust",
        true,
        "hex-matrix",
    );
    assert_eq!(output.status.code(), Some(0));
    let codec = std::fs::read_to_string(dir.join("service_codec.rs")).expect("codec");
    assert_eq!(codec.matches("fn encode_hex_binary(").count(), 1);
    assert_eq!(codec.matches("fn decode_hex_binary(").count(), 1);
    // Named Binary goes through the generated wrapper's public API only.
    assert!(codec.contains("encode_hex_binary(value.as_slice())"));
    assert!(codec.contains("decode_hex_binary(value, path).map(super::model::BlobBytes::new)"));
    assert!(codec.contains("decode_hex_binary(value, path).map(super::model::BlobAlias::new)"));
}

/// Task 052 established that provenance and value-space support are
/// independent. Task 053 flips the value-space half: a LENGTH-constrained
/// named hexBinary is now a checked carrier and model + codec READY, while a
/// PATTERN-constrained one is still a MODEL blocker, reported before codec
/// readiness is measured. `--with-codec` never widens the model.
#[test]
fn task052_constrained_hex_binary_remains_a_model_blocker() {
    let run = |label: &str, facet: &str| {
        let dir = output_dir(label);
        std::fs::create_dir_all(&dir).expect("dir");
        let schema = dir.join("constrained.xsd");
        std::fs::write(
            &schema,
            std::fs::read_to_string(fixture("codec-binary.xsd"))
                .expect("fixture")
                .replace(
                    "<xs:complexType name=\"BlobPayload\">",
                    &format!(
                        "<xs:simpleType name=\"CodeType\"><xs:restriction base=\"xs:hexBinary\">\
                         {facet}</xs:restriction></xs:simpleType>\n  \
                         <xs:complexType name=\"BlobPayload\">"
                    ),
                )
                .replace("type=\"xs:hexBinary\"", "type=\"uci:CodeType\""),
        )
        .expect("write");
        let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(["service-check", "--schema"])
            .arg(&schema)
            .arg("--contract")
            .arg(fixture("codec-binary.yaml"))
            .args([
                "--language",
                "rust",
                "--world",
                "closed-schema",
                "--with-codec",
            ])
            .output()
            .expect("CLI runs");
        let _ = std::fs::remove_dir_all(&dir);
        (
            output.status.code(),
            String::from_utf8(output.stdout).expect("UTF-8"),
            String::from_utf8(output.stderr).expect("UTF-8"),
        )
    };

    // Task 053: length-only is supported by the model and the codec.
    let (code, report, _) = run("constrained-hex-length", "<xs:length value=\"6\"/>");
    assert_eq!(code, Some(0), "{report}");
    assert!(report.contains("status: READY\n"), "{report}");
    assert!(report.contains("codec status: READY\n"), "{report}");

    // Every non-length Binary facet still fails closed. Through XSD the
    // frontend is the first gate and rejects it as an unsupported construct
    // before any readiness is computed (the shared classifier rejects the
    // same shapes in hand-built IR; see codegen-core `binary` tests).
    for facet in [
        "<xs:pattern value=\"[0-9A-F]*\"/>",
        "<xs:whiteSpace value=\"collapse\"/>",
        "<xs:minInclusive value=\"1\"/>",
    ] {
        let (code, report, stderr) = run(
            "constrained-hex-unsupported",
            &format!("<xs:length value=\"6\"/>{facet}"),
        );
        assert_ne!(code, Some(0), "{facet}: {report}");
        assert!(!report.contains("status: READY"), "{facet}: {report}");
        assert!(
            stderr.contains("unsupported XSD construct"),
            "{facet}: {stderr}"
        );
    }
}

/// C (Task 051). A qualified NON-OAM payload (`runtime-test.xsd`,
/// `elementFormDefault="qualified"`) was codec NOT READY in Task 050 only
/// because Schema IR lost the element namespace. It is now codec READY and
/// its member is the Clark key `{urn:test}Count`. The default report is
/// unchanged.
#[test]
fn task051_qualified_non_oam_payload_is_codec_ready() {
    let (code, plain) = check("runtime-test.xsd", "runtime-test.yaml", "rust", false);
    assert_eq!(code, Some(0));
    assert!(!plain.contains("codec status"));
    let (code, report) = check("runtime-test.xsd", "runtime-test.yaml", "rust", true);
    assert_eq!(code, Some(0), "{report}");
    assert!(
        report.starts_with(&plain),
        "the codec section is only appended"
    );
    assert!(
        report.ends_with("\ncodec renderable emitted declarations: 2/2\ncodec status: READY\n")
    );
    assert!(!report.contains("service codec boundary"));
    let (output, dir) = generate(
        "runtime-test.xsd",
        "runtime-test.yaml",
        "rust",
        true,
        "non-oam",
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        listing(&dir),
        ["service_api.rs", "service_codec.rs", "test.rs"]
    );
    let codec = std::fs::read_to_string(dir.join("service_codec.rs")).expect("codec");
    assert!(codec.contains("object.insert(\"{urn:test}Count\".to_owned(),"));
    assert!(codec.contains("required(object, \"{urn:test}Count\", path)"));
    assert!(!codec.contains("\"Count\""), "no bare member key");
    // Model and wrapper are exactly the default generation's.
    let (_, plain_dir) = generate(
        "runtime-test.xsd",
        "runtime-test.yaml",
        "rust",
        false,
        "non-oam-plain",
    );
    assert_eq!(listing(&plain_dir), ["service_api.rs", "test.rs"]);
    assert_eq!(
        std::fs::read(dir.join("test.rs")).expect("model"),
        std::fs::read(plain_dir.join("test.rs")).expect("plain model")
    );
}

/// Task 051 fail-closed control: XSD default form (no elementFormDefault)
/// makes the local element UNQUALIFIED. Model READY; default generation
/// unchanged; `--with-codec` NOT READY at the codec boundary; nothing
/// written.
#[test]
fn task051_unqualified_member_is_model_ready_but_codec_not_ready() {
    let (code, plain) = check(
        "codec-unqualified.xsd",
        "codec-unqualified.yaml",
        "rust",
        false,
    );
    assert_eq!(code, Some(0), "{plain}");
    assert!(plain.contains("status: READY\n"));
    let (output, plain_dir) = generate(
        "codec-unqualified.xsd",
        "codec-unqualified.yaml",
        "rust",
        false,
        "unqualified-plain",
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(listing(&plain_dir), ["service_api.rs", "unqualified.rs"]);

    let (code, report) = check(
        "codec-unqualified.xsd",
        "codec-unqualified.yaml",
        "rust",
        true,
    );
    assert_eq!(code, Some(1));
    assert!(report.starts_with(&plain));
    assert!(
        report.ends_with(
            "\ncodec renderable emitted declarations: 0/1\ncodec status: NOT READY\n\n\
             service codec boundary: Payload.Field has an unqualified local element with \
             no evidenced OMS JSON member-name mapping\n"
        ),
        "{report}"
    );
    let (output, dir) = generate(
        "codec-unqualified.xsd",
        "codec-unqualified.yaml",
        "rust",
        true,
        "unqualified",
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(!dir.exists(), "no output directory may be created");
}

/// Task 051 local-form controls: readiness follows the EFFECTIVE element
/// declaration, not the schema default alone.
#[test]
fn task051_local_form_overrides_decide_codec_readiness() {
    // A. default unqualified + form="qualified" -> READY, Clark key.
    let (code, report) = check(
        "codec-form-qualified.xsd",
        "codec-form-qualified.yaml",
        "rust",
        true,
    );
    assert_eq!(code, Some(0), "{report}");
    assert!(report.ends_with("\ncodec status: READY\n"), "{report}");
    let (output, dir) = generate(
        "codec-form-qualified.xsd",
        "codec-form-qualified.yaml",
        "rust",
        true,
        "form-qualified",
    );
    assert_eq!(output.status.code(), Some(0));
    let codec = std::fs::read_to_string(dir.join("service_codec.rs")).expect("codec");
    assert!(codec.contains("object.insert(\"{urn:form}Value\".to_owned(),"));

    // B. default qualified + form="unqualified" -> model READY, codec NOT.
    let (code, report) = check(
        "codec-form-unqualified.xsd",
        "codec-form-unqualified.yaml",
        "rust",
        true,
    );
    assert_eq!(code, Some(1));
    assert!(report.contains("status: READY\n"), "{report}");
    assert!(report.contains(
        "service codec boundary: Payload.Value has an unqualified local element with no \
         evidenced OMS JSON member-name mapping\n"
    ));
}

/// Task 051 does NOT widen the single-namespace backend boundary: a selected
/// closure spanning two namespaces is still model NOT READY (so the codec is
/// not even measured), with or without `--with-codec`, in every language.
#[test]
fn task051_multi_namespace_backend_boundary_is_unchanged() {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-check/multi-namespace.xsd");
    let contract = schema.with_file_name("multi-namespace-spanning.yaml");
    for language in ["rust", "ada", "cpp"] {
        let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(["service-check", "--schema"])
            .arg(&schema)
            .arg("--contract")
            .arg(&contract)
            .args([
                "--language",
                language,
                "--world",
                "closed-schema",
                "--with-codec",
            ])
            .output()
            .expect("CLI runs");
        let report = String::from_utf8(output.stdout).expect("UTF-8");
        assert_eq!(output.status.code(), Some(1), "{language}: {report}");
        assert!(report.contains("status: NOT READY\n"), "{report}");
        assert!(
            report.contains(
                "generation requires exactly one namespace, but the selected schema uses 2: \
                 urn:test:a, urn:test:b"
            ),
            "{report}"
        );
        assert!(report.ends_with("codec: not measured (selected model is NOT READY)\n"));
    }
}

/// D. Zero OMS exchanges: the codec is vacuous; no codec file is written and
/// the wrapper is byte-identical to default generation.
#[test]
fn task050_zero_oms_service_needs_no_codec() {
    for language in ["rust", "ada", "cpp"] {
        let (code, report) = check("root.xsd", "non-uci.yaml", language, true);
        assert_eq!(code, Some(0), "{language}: {report}");
        assert!(report.ends_with("\ncodec: not required (no OMS message exchange)\n"));
        let (output, dir) = generate(
            "root.xsd",
            "non-uci.yaml",
            language,
            true,
            &format!("zero-{language}"),
        );
        assert_eq!(output.status.code(), Some(0));
        let (_, plain) = generate(
            "root.xsd",
            "non-uci.yaml",
            language,
            false,
            &format!("zero-plain-{language}"),
        );
        assert_eq!(listing(&dir), listing(&plain));
        for name in listing(&dir) {
            assert_eq!(
                std::fs::read(dir.join(&name)).ok(),
                std::fs::read(plain.join(&name)).ok()
            );
        }
        let summary = String::from_utf8(output.stdout).expect("UTF-8");
        assert!(summary.contains("generated service codec files: 0\n"));
    }
}

/// Ada and C++ have no generated codec: an OMS service is NOT READY with
/// --with-codec at the codec boundary, and nothing is generated.
#[test]
fn task050_ada_and_cpp_have_no_generated_codec() {
    for (language, name) in [("ada", "Ada"), ("cpp", "C++")] {
        let (code, report) = check("codec-oam.xsd", "codec-oam.yaml", language, true);
        assert_eq!(code, Some(1));
        assert!(
            report.contains("status: READY\n"),
            "model stays READY: {report}"
        );
        if language == "cpp" {
            assert!(report.contains(&format!(
                "service codec boundary: generated OMS JSON codec is not implemented for {name}\n"
            )));
        } else {
            assert!(report.contains("service codec boundary:"), "{report}");
            assert!(
                !report.contains("codec is not implemented for Ada"),
                "{report}"
            );
            assert!(
                report.contains("unsupported Ada codec primitive")
                    || report.contains("repeated member")
                    || report.contains("Choice/alias/list")
                    || report.contains("polymorphic"),
                "{report}"
            );
        }
        let (output, dir) = generate(
            "codec-oam.xsd",
            "codec-oam.yaml",
            language,
            true,
            &format!("lang-{language}"),
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(!dir.exists());
        // Default generation is unaffected.
        let (output, _) = generate(
            "codec-oam.xsd",
            "codec-oam.yaml",
            language,
            false,
            &format!("lang-plain-{language}"),
        );
        assert_eq!(output.status.code(), Some(0));
    }
}

/// The flag is a value-less boolean; repeating it is a usage error.
#[test]
fn task050_with_codec_is_a_singular_flag() {
    let output = run(
        &[
            "service-check",
            "--language",
            "rust",
            "--world",
            "closed-schema",
            "--with-codec",
            "--with-codec",
        ],
        "codec-oam.xsd",
        "codec-oam.yaml",
        None,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate option '--with-codec'"));
    for command in ["service-check", "service-generate"] {
        let help = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args([command, "--help"])
            .output()
            .expect("help");
        let text = String::from_utf8(help.stdout).expect("UTF-8");
        assert!(
            text.contains("[--with-codec]") && text.contains("WITH CODEC"),
            "{command}"
        );
    }
}

#[test]
fn task090_ada_datetime_selected_closures_and_model_bytes() {
    for (fixture, count) in [("temporal", "3/3"), ("direct-datetime", "1/1")] {
        let schema = format!("{fixture}.xsd");
        let contract = format!("{fixture}.yaml");
        let (code, report) = check(&schema, &contract, "ada", true);
        assert_eq!(code, Some(0), "{report}");
        assert!(report.contains(&format!("codec renderable emitted declarations: {count}\n")));
        assert!(report.contains("codec status: READY\n"));
        let (output, coded) = generate(
            &schema,
            &contract,
            "ada",
            true,
            &format!("090-{fixture}-coded"),
        );
        assert!(output.status.success(), "{output:?}");
        let (output, plain) = generate(
            &schema,
            &contract,
            "ada",
            false,
            &format!("090-{fixture}-plain"),
        );
        assert!(output.status.success(), "{output:?}");
        for name in listing(&plain) {
            assert_eq!(
                std::fs::read(plain.join(&name)).unwrap(),
                std::fs::read(coded.join(&name)).unwrap()
            );
        }
        let model = std::fs::read_to_string(coded.join("urn-test.ads")).unwrap();
        assert!(!model.contains("UnselectedTemporalPayload"));
        assert!(!model.contains("WallClock"));
        let body = std::fs::read_to_string(coded.join("service_codec.adb")).unwrap();
        assert!(body.contains("Model.Value"));
        assert!(body.contains("Model.Create (Text)"));
        assert!(!body.contains("Is_Date_Time"));
        assert!(!body.contains("Is_Zulu_Date_Time"));
    }
}

#[test]
fn task087b_ada_codec_ready_deduplicated_and_preserves_model_api() {
    let (code, report) = check(
        "codec-ada-minimal.xsd",
        "codec-ada-minimal.yaml",
        "ada",
        true,
    );
    assert_eq!(code, Some(0), "{report}");
    assert!(report.contains("codec renderable emitted declarations: 5/5\n"));
    assert!(report.contains("codec status: READY\n"));
    let (output, coded) = generate(
        "codec-ada-minimal.xsd",
        "codec-ada-minimal.yaml",
        "ada",
        true,
        "087b-coded",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("generated service codec files: 2\n"));
    let (output, plain) = generate(
        "codec-ada-minimal.xsd",
        "codec-ada-minimal.yaml",
        "ada",
        false,
        "087b-plain",
    );
    assert!(output.status.success());
    assert_eq!(listing(&coded).len(), listing(&plain).len() + 2);
    for name in listing(&plain) {
        assert_eq!(
            std::fs::read(plain.join(&name)).unwrap(),
            std::fs::read(coded.join(&name)).unwrap(),
            "{name}"
        );
    }
    let spec = std::fs::read_to_string(coded.join("service_codec.ads")).unwrap();
    assert_eq!(spec.matches("function Encode_AdaPayload").count(), 1);
    assert_eq!(spec.matches("function Decode_AdaPayload").count(), 1);
    let body = std::fs::read_to_string(coded.join("service_codec.adb")).unwrap();
    assert!(body.contains("AMS_GRA_OMS_JSON_Guard.Read"));
    assert!(body.contains("AMS_GRA_OMS_JSON_Guard.Write"));
    assert!(!body.contains("GNATCOLL.JSON.Read"));
    assert!(!body.contains("GNATCOLL.JSON.Write"));
}

#[test]
fn task087b_ada_codec_unsupported_and_collision_preflight_is_atomic() {
    let source = std::fs::read_to_string(fixture("codec-ada-minimal.xsd")).unwrap();
    for (label, schema, expected) in [
        (
            "float",
            source.replace(
                "name=\"Enabled\" type=\"xs:boolean\"",
                "name=\"Enabled\" type=\"xs:double\"",
            ),
            "unsupported Ada codec primitive",
        ),
        (
            "unsigned64",
            source.replace(
                "name=\"Enabled\" type=\"xs:boolean\"",
                "name=\"Enabled\" type=\"xs:unsignedLong\"",
            ),
            "integer domain exceeds signed-64 JSON boundary",
        ),
        (
            "repeated-temporal",
            source.replace(
                "name=\"Note\" type=\"xs:string\" minOccurs=\"0\"",
                "name=\"Note\" type=\"xs:dateTime\" minOccurs=\"0\" maxOccurs=\"3\"",
            ),
            "direct DateTime repeated element is unsupported",
        ),
        (
            "unqualified",
            source.replace(
                "name=\"Note\" type=\"xs:string\"",
                "name=\"Note\" type=\"xs:string\" form=\"unqualified\"",
            ),
            "unqualified local element",
        ),
        (
            "package-collision",
            source
                .replace("xmlns:uci=\"https://www.vdl.afrl.af.mil/programs/oam\"", "xmlns:version=\"https://www.vdl.afrl.af.mil/programs/oam\" xmlns:uci=\"urn:Interfaces:model\"")
                .replace(
                    "https://www.vdl.afrl.af.mil/programs/oam",
                    "urn:Interfaces:model",
                )
                .replace("xmlns:version=\"urn:Interfaces:model\"", "xmlns:version=\"https://www.vdl.afrl.af.mil/programs/oam\"")
                .replace(" uci:version=", " version:version="),
            "model package collision",
        ),
    ] {
        let scratch = output_dir(&format!("087b-{label}-input"));
        std::fs::create_dir(&scratch).unwrap();
        let schema_path = scratch.join("schema.xsd");
        std::fs::write(&schema_path, schema).unwrap();
        let output_path = scratch.join("output");
        for action in ["service-check", "service-generate"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
            command
                .arg(action)
                .arg("--schema")
                .arg(&schema_path)
                .arg("--contract")
                .arg(fixture("codec-ada-minimal.yaml"))
                .args([
                    "--language",
                    "ada",
                    "--world",
                    "closed-schema",
                    "--with-codec",
                ]);
            if action == "service-generate" {
                command.arg("--output").arg(&output_path);
            }
            let output = command.output().unwrap();
            assert_eq!(
                output.status.code(),
                Some(1),
                "{label}: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                combined.contains(if label == "unsigned64" {
                    "unsupported XSD construct: xs:unsignedLong"
                } else {
                    expected
                }),
                "{label}: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            assert!(!output_path.exists());
        }
    }
}

#[test]
fn task089r_combined_choice_repetition_ready_and_rejections_atomic() {
    let (code, report) = check(
        "codec-ada-combined.xsd",
        "codec-ada-combined.yaml",
        "ada",
        true,
    );
    assert_eq!(code, Some(0), "{report}");
    assert!(report.contains("codec renderable emitted declarations: 4/4\n"));
    let source = std::fs::read_to_string(fixture("codec-ada-combined.xsd")).unwrap();
    for (label, schema, expected) in [
        (
            "choice-member",
            source.replace(
                "name=\"Pick\" type=\"uci:Selection\"",
                "name=\"Pick\" type=\"uci:Selection\" maxOccurs=\"2\"",
            ),
            "Choice/alias/list repeated element",
        ),
        (
            "choice-alternative",
            source.replace(
                "name=\"Nested\" type=\"uci:Details\"",
                "name=\"Nested\" type=\"uci:Details\" maxOccurs=\"2\"",
            ),
            "Choice alternative must be required-single",
        ),
        (
            "nested",
            source.replace("type=\"xs:byte\"", "type=\"xs:double\""),
            "unsupported Ada codec primitive",
        ),
    ] {
        let scratch = output_dir(&format!("089r-{label}"));
        std::fs::create_dir(&scratch).unwrap();
        let path = scratch.join("schema.xsd");
        std::fs::write(&path, schema).unwrap();
        for action in ["service-check", "service-generate"] {
            let output_path = scratch.join("output");
            let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
            command
                .arg(action)
                .arg("--schema")
                .arg(&path)
                .arg("--contract")
                .arg(fixture("codec-ada-combined.yaml"))
                .args([
                    "--language",
                    "ada",
                    "--world",
                    "closed-schema",
                    "--with-codec",
                ]);
            if action == "service-generate" {
                command.arg("--output").arg(&output_path);
            }
            let output = command.output().unwrap();
            assert_eq!(output.status.code(), Some(1), "{label}");
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(diagnostic.contains(expected), "{label}: {diagnostic}");
            assert!(!output_path.exists());
        }
    }
}

#[test]
fn task089_repeated_ada_codec_ready_and_model_api_unchanged() {
    let (code, report) = check(
        "codec-ada-repeated.xsd",
        "codec-ada-repeated.yaml",
        "ada",
        true,
    );
    assert_eq!(code, Some(0), "{report}");
    assert!(report.contains("codec renderable emitted declarations: 5/5\n"));
    let (output, coded) = generate(
        "codec-ada-repeated.xsd",
        "codec-ada-repeated.yaml",
        "ada",
        true,
        "089-coded",
    );
    assert!(output.status.success(), "{output:?}");
    let (output, plain) = generate(
        "codec-ada-repeated.xsd",
        "codec-ada-repeated.yaml",
        "ada",
        false,
        "089-plain",
    );
    assert!(output.status.success());
    for name in listing(&plain) {
        assert_eq!(
            std::fs::read(plain.join(&name)).unwrap(),
            std::fs::read(coded.join(&name)).unwrap()
        );
    }
    let spec = std::fs::read_to_string(coded.join("service_codec.ads")).unwrap();
    assert_eq!(spec.matches("function Encode_RepeatedPayload").count(), 1);
    let body = std::fs::read_to_string(coded.join("service_codec.adb")).unwrap();
    assert!(body.contains("Model.To_Sequence (Values)"));
    assert!(body.contains("Required => Required, Additional => Additional"));
    assert!(!body.contains(".Count"));
    assert!(!body.contains(".Items"));
}

#[test]
fn task089_repeated_unsupported_elements_fail_atomically() {
    let source = std::fs::read_to_string(fixture("codec-ada-repeated.xsd")).unwrap();
    for (label, schema) in [
        (
            "float",
            source.replace(
                "name=\"Words\" type=\"xs:string\"",
                "name=\"Words\" type=\"xs:double\"",
            ),
        ),
        (
            "binary",
            source.replace(
                "name=\"Words\" type=\"xs:string\"",
                "name=\"Words\" type=\"xs:hexBinary\"",
            ),
        ),
        (
            "unqualified",
            source.replace(
                "name=\"Words\" type=\"xs:string\"",
                "name=\"Words\" type=\"xs:string\" form=\"unqualified\"",
            ),
        ),
        (
            "nillable",
            source.replace(
                "name=\"Words\" type=\"xs:string\"",
                "name=\"Words\" type=\"xs:string\" nillable=\"true\"",
            ),
        ),
        (
            "choice",
            source
                .replace(
                    "<xs:complexType name=\"Details\"><xs:sequence>",
                    "<xs:complexType name=\"Details\"><xs:choice>",
                )
                .replace(
                    "</xs:sequence></xs:complexType>\n  <xs:complexType name=\"Base\">",
                    "</xs:choice></xs:complexType>\n  <xs:complexType name=\"Base\">",
                ),
        ),
        (
            "nested-unsupported-primitive",
            source.replace(
                "<xs:element name=\"Label\" type=\"uci:Label\"/>",
                "<xs:element name=\"Label\" type=\"xs:double\"/>",
            ),
        ),
    ] {
        let scratch = output_dir(&format!("089-{label}"));
        std::fs::create_dir(&scratch).unwrap();
        let schema_path = scratch.join("schema.xsd");
        std::fs::write(&schema_path, schema).unwrap();
        let output_path = scratch.join("output");
        let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .arg("service-generate")
            .arg("--schema")
            .arg(schema_path)
            .arg("--contract")
            .arg(fixture("codec-ada-repeated.yaml"))
            .args([
                "--language",
                "ada",
                "--world",
                "closed-schema",
                "--with-codec",
            ])
            .arg("--output")
            .arg(&output_path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{label}: {output:?}");
        assert!(!output_path.exists(), "{label} left partial output");
    }
}
