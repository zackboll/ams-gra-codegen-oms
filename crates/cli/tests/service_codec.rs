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
        assert!(report.contains(&format!(
            "service codec boundary: generated OMS JSON codec is not implemented for {name}\n"
        )));
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
