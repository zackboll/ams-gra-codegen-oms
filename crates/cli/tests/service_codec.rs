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

#[test]
fn task093_c4_helper_demand_uses_operations_not_schema_names() {
    for floating in [false, true] {
        for (index, name) in [
            "ArraySample",
            "Array_Encode_Hex_Binary",
            "Encode_Hex_Binary",
            "Array_encode_hex_binary",
            "Array_Decode_Hex_Binary",
        ]
        .iter()
        .enumerate()
        {
            let scratch = output_dir(&format!("093-c4-{floating}-{index}"));
            std::fs::create_dir_all(&scratch).unwrap();
            let original = fixture(if floating {
                "codec-ada-helper-demand-float32.xsd"
            } else {
                "codec-ada-helper-demand.xsd"
            });
            let schema = scratch.join("schema.xsd");
            std::fs::write(
                &schema,
                std::fs::read_to_string(original)
                    .unwrap()
                    .replace("ArraySample", name),
            )
            .unwrap();
            let contract = fixture("codec-ada-helper-demand.yaml");
            let run_case = |action: &str, codec: bool, output: Option<&Path>| {
                let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
                command.args([action, "--schema"]).arg(&schema);
                command.arg("--contract").arg(&contract);
                command.args(["--language", "ada", "--world", "closed-schema"]);
                if codec {
                    command.arg("--with-codec");
                    if floating {
                        command.args(["--ada-codec-runtime", "linux-x86_64-glibc-2.41"]);
                    }
                }
                if let Some(output) = output {
                    command.arg("--output").arg(output);
                }
                let result = command.output().unwrap();
                assert!(
                    result.status.success(),
                    "{name}: {}{}",
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                );
            };
            run_case("service-check", false, None);
            run_case("service-check", true, None);
            let plain = scratch.join("plain");
            let codec = scratch.join("codec");
            run_case("service-generate", false, Some(&plain));
            run_case("service-generate", true, Some(&codec));
            for path in std::fs::read_dir(&plain).unwrap() {
                let path = path.unwrap().path();
                assert_eq!(
                    std::fs::read(&path).unwrap(),
                    std::fs::read(codec.join(path.file_name().unwrap())).unwrap()
                );
            }
            let body = std::fs::read_to_string(codec.join("service_codec.adb")).unwrap();
            for helper in ["Encode_Hex_Binary", "Decode_Hex_Binary"] {
                assert!(
                    !body.lines().any(|line| line
                        .trim_start()
                        .starts_with(&format!("function {helper} ("))),
                    "actual helper declaration must be absent"
                );
            }
            assert!(!body.contains("Model.Binary_Vectors"));
            assert!(!body.contains("with Ada.Containers;"));
            assert!(!body.contains("UnselectedBinary"));
            for prefix in ["To_JSON_", "From_JSON_", "Encode_", "Decode_"] {
                assert!(body.contains(&format!("function {prefix}{name} (")));
                assert!(body.contains(&format!("end {prefix}{name};")));
            }
            assert_eq!(body.contains("Require_Platform"), floating);
            std::fs::remove_dir_all(scratch).unwrap();
        }
    }
}

#[test]
fn task093_c4_each_binary_operation_path_demands_one_helper_pair() {
    for (label, reference, extra) in [
        ("direct", "xs:hexBinary", ""),
        ("optional", "xs:hexBinary", " minOccurs=\"0\""),
        ("named", "uci:UnselectedBinary", ""),
    ] {
        let scratch = output_dir(&format!("093-c4-binary-{label}"));
        std::fs::create_dir_all(&scratch).unwrap();
        let schema = scratch.join("schema.xsd");
        let source = std::fs::read_to_string(fixture("codec-ada-helper-demand.xsd"))
            .unwrap()
            .replace(
                "name=\"Number\" type=\"xs:int\"",
                &format!("name=\"Number\" type=\"{reference}\"{extra}"),
            );
        std::fs::write(&schema, source).unwrap();
        let destination = scratch.join("generated");
        let result = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(["service-generate", "--schema"])
            .arg(&schema)
            .arg("--contract")
            .arg(fixture("codec-ada-helper-demand.yaml"))
            .args([
                "--language",
                "ada",
                "--world",
                "closed-schema",
                "--with-codec",
                "--output",
            ])
            .arg(&destination)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let body = std::fs::read_to_string(destination.join("service_codec.adb")).unwrap();
        for helper in ["Encode_Hex_Binary", "Decode_Hex_Binary"] {
            assert_eq!(
                body.lines()
                    .filter(|line| line
                        .trim_start()
                        .starts_with(&format!("function {helper} (")))
                    .count(),
                1
            );
        }
        assert!(body.contains("Decode_Hex_Binary (Value : JSON_Value)"));
        assert!(
            body.contains(
                "Encode_Hex_Binary (Item : Model.Binary_Vectors.Vector) return JSON_Value"
            )
        );
        assert!(
            body.find("end Scalar_String;").unwrap()
                < body.find("   function Encode_Hex_Binary (").unwrap()
        );
        assert!(
            body.find("end Decode_Hex_Binary;").unwrap()
                < body.find("   function To_JSON_").unwrap()
        );
        std::fs::remove_dir_all(scratch).unwrap();
    }
}

#[test]
fn task093_c3_runtime_matrix_and_decoder_helpers() {
    for (schema, contract, mixed) in [
        (
            "codec-ada-hexbinary-combined.xsd",
            "codec-ada-hexbinary-combined.yaml",
            false,
        ),
        (
            "codec-ada-hexbinary-float32.xsd",
            "codec-ada-hexbinary-float32.yaml",
            true,
        ),
    ] {
        let mut portable = None;
        for qualified in [false, true] {
            let mut args = vec![
                "service-check",
                "--language",
                "ada",
                "--world",
                "closed-schema",
                "--with-codec",
            ];
            if qualified {
                args.extend(["--ada-codec-runtime", "linux-x86_64-glibc-2.41"]);
            }
            let ready = run(&args, schema, contract, None);
            let expected = !mixed || qualified;
            assert_eq!(
                ready.status.success(),
                expected,
                "{}{}",
                String::from_utf8_lossy(&ready.stdout),
                String::from_utf8_lossy(&ready.stderr)
            );
            args[0] = "service-generate";
            let dir = output_dir(&format!("093-c3-{mixed}-{qualified}"));
            let generated = run(&args, schema, contract, Some(&dir));
            assert_eq!(
                generated.status.success(),
                expected,
                "{}",
                String::from_utf8_lossy(&generated.stderr)
            );
            if !expected {
                assert!(!dir.exists(), "atomic runtime rejection");
                continue;
            }
            let body = std::fs::read_to_string(dir.join("service_codec.adb")).unwrap();
            assert!(body.contains(
                "Encode_Hex_Binary (Item : Model.Binary_Vectors.Vector) return JSON_Value"
            ));
            assert!(body.contains(&format!(
                "Decode_Hex_Binary (Value : {})",
                if mixed { "Ingress.Value" } else { "JSON_Value" }
            )));
            assert_eq!(body.contains("Require_Platform"), mixed);
            assert_eq!(body.contains("with AMS_GRA_OMS_JSON_Occurrences;"), mixed);
            if !mixed {
                if let Some(ref bytes) = portable {
                    assert_eq!(
                        &body, bytes,
                        "runtime option must not restrict portable Binary"
                    );
                } else {
                    portable = Some(body);
                }
            }
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
}

#[test]
fn task097_c3_lossless_views_are_additive_and_preflight_is_atomic() {
    let scratch = output_dir("097-c3");
    std::fs::create_dir_all(&scratch).unwrap();
    let original = std::fs::read_to_string(fixture("codec-ada-copy-views.xsd")).unwrap();
    for (index, name) in [
        "Digest",
        "Visit_Binary",
        "Visit_Element",
        "Codec_Views",
        "CODEC_VIEWS",
    ]
    .iter()
    .enumerate()
    {
        let schema = scratch.join(format!("{index}.xsd"));
        std::fs::write(
            &schema,
            original
                .replace("\"Digest\"", &format!("\"{name}\""))
                .replace("uci:Digest", &format!("uci:{name}")),
        )
        .unwrap();
        for codec in [false, true] {
            let dir = scratch.join(format!("out-{index}-{codec}"));
            let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
            command
                .arg("service-generate")
                .arg("--schema")
                .arg(&schema)
                .arg("--contract")
                .arg(fixture("codec-ada-float64.yaml"))
                .args(["--language", "ada", "--world", "closed-schema", "--output"])
                .arg(&dir);
            if codec {
                command.args([
                    "--with-codec",
                    "--ada-codec-runtime",
                    "linux-x86_64-glibc-2.41-lossless-v1",
                ]);
            }
            let output = command.output().unwrap();
            let collision = codec && name.eq_ignore_ascii_case("Codec_Views");
            assert_eq!(
                output.status.success(),
                !collision,
                "{name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if collision {
                assert!(!dir.exists());
                continue;
            }
            let child = dir.join("programs-oam-codec_views.ads");
            assert_eq!(child.exists(), codec);
            if codec {
                let body = std::fs::read_to_string(dir.join("service_codec.adb")).unwrap();
                assert!(body.contains("Codec_Views.Visit_Element"));
                assert!(body.contains("Codec_Views.Visit_Binary (Item, Encode_Octets'Access)"));
                assert!(!body.contains("Model.Element"));
                assert!(!body.contains("Encode_Hex_Binary (Model.Value (Item))"));
                let child_body =
                    std::fs::read_to_string(dir.join("programs-oam-codec_views.adb")).unwrap();
                assert!(child_body.contains("Process (Item.Data)"));
                assert!(child_body.contains("Query_Element"));
                for parent in ["programs-oam.ads", "programs-oam.adb"] {
                    assert_eq!(
                        std::fs::read(dir.join(parent)).unwrap(),
                        std::fs::read(scratch.join(format!("out-{index}-false")).join(parent))
                            .unwrap()
                    );
                }
            }
        }
    }
    std::fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn task097_c3_views_are_absent_without_demand_and_binary_admission_is_narrow() {
    let scratch = output_dir("097-c3-demand");
    std::fs::create_dir_all(&scratch).unwrap();
    let original = std::fs::read_to_string(fixture("codec-ada-copy-views.xsd")).unwrap();
    for (index, text) in [
        original.clone(),
        original.replace(
            "type=\"uci:Digest\" minOccurs=\"0\" maxOccurs=\"unbounded\"",
            "type=\"xs:hexBinary\" minOccurs=\"0\" maxOccurs=\"unbounded\"",
        ),
        original.replace("<xs:minLength value=\"1\"/><xs:maxLength value=\"2\"/>", ""),
        original.replace(
            "<xs:minLength value=\"1\"/><xs:maxLength value=\"2\"/>",
            "<xs:pattern value=\"[0-9A-F]+\"/>",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let schema = scratch.join(format!("{index}.xsd"));
        std::fs::write(&schema, text).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(["service-check", "--schema"])
            .arg(&schema)
            .arg("--contract")
            .arg(fixture("codec-ada-float64.yaml"))
            .args([
                "--language",
                "ada",
                "--world",
                "closed-schema",
                "--with-codec",
                "--ada-codec-runtime",
                "linux-x86_64-glibc-2.41-lossless-v1",
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            index == 0,
            "{index}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    let schema = scratch.join("no-demand.xsd");
    std::fs::write(&schema, r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam" targetNamespace="https://www.vdl.afrl.af.mil/programs/oam" elementFormDefault="qualified"><xs:complexType name="Simple"><xs:sequence><xs:element name="Number" type="xs:double"/></xs:sequence></xs:complexType><xs:element name="Float64Notice" type="uci:Simple" uci:version="001.000.000.000"/></xs:schema>"#).unwrap();
    let dir = scratch.join("no-demand");
    let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["service-generate", "--schema"])
        .arg(schema)
        .arg("--contract")
        .arg(fixture("codec-ada-float64.yaml"))
        .args([
            "--language",
            "ada",
            "--world",
            "closed-schema",
            "--with-codec",
            "--ada-codec-runtime",
            "linux-x86_64-glibc-2.41-lossless-v1",
            "--output",
        ])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!listing(&dir).iter().any(|f| f.contains("codec_views")));
    assert!(
        !std::fs::read_to_string(dir.join("service_codec.adb"))
            .unwrap()
            .contains("Codec_Views")
    );
    std::fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn task097_lossless_profile_matrix_and_legacy_bytes() {
    let lossless = "linux-x86_64-glibc-2.41-lossless-v1";
    let old = "linux-x86_64-glibc-2.41";
    for (index, profile) in [None, Some(old), Some(lossless)].into_iter().enumerate() {
        let mut args = vec![
            "service-check",
            "--language",
            "ada",
            "--world",
            "closed-schema",
            "--with-codec",
        ];
        if let Some(profile) = profile {
            args.extend(["--ada-codec-runtime", profile]);
        }
        let result = run(
            &args,
            "codec-ada-float64.xsd",
            "codec-ada-float64.yaml",
            None,
        );
        assert_eq!(
            result.status.success(),
            profile == Some(lossless),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        args[0] = "service-generate";
        let dir = output_dir(&format!("097-profile-{index}"));
        let result = run(
            &args,
            "codec-ada-float64.xsd",
            "codec-ada-float64.yaml",
            Some(&dir),
        );
        assert_eq!(result.status.success(), profile == Some(lossless));
        if profile == Some(lossless) {
            let body = std::fs::read_to_string(dir.join("service_codec.adb")).unwrap();
            assert!(body.contains("Output.Floating_64"));
            assert!(body.contains("Ingress.Numeric_Lexeme"));
            assert!(body.contains("package Ingress renames AMS_GRA_OMS_JSON_Lossless_Ingress;"));
            assert!(!body.contains("AMS_GRA_OMS_JSON_Occurrences"));
            assert!(body.contains("Output.Write (To_JSON_"));
            assert!(!body.contains("return JSON_Value"));
            std::fs::remove_dir_all(dir).unwrap();
        } else {
            assert!(!dir.exists());
        }
    }
    for (index, (schema, contract)) in [
        (
            "codec-ada-hexbinary-combined.xsd",
            "codec-ada-hexbinary-combined.yaml",
        ),
        ("codec-ada-float32.xsd", "codec-ada-float32.yaml"),
        (
            "codec-ada-hexbinary-float32.xsd",
            "codec-ada-hexbinary-float32.yaml",
        ),
        (
            "codec-ada-helper-demand.xsd",
            "codec-ada-helper-demand.yaml",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let dirs = [
            output_dir(&format!("097-old-{index}")),
            output_dir(&format!("097-new-{index}")),
        ];
        for (dir, profile) in dirs.iter().zip([old, lossless]) {
            let args = [
                "service-generate",
                "--language",
                "ada",
                "--world",
                "closed-schema",
                "--with-codec",
                "--ada-codec-runtime",
                profile,
            ];
            let result = run(&args, schema, contract, Some(dir));
            assert!(
                result.status.success(),
                "{schema}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        assert_eq!(listing(&dirs[0]), listing(&dirs[1]));
        for file in listing(&dirs[0]) {
            assert_eq!(
                std::fs::read(dirs[0].join(&file)).unwrap(),
                std::fs::read(dirs[1].join(&file)).unwrap(),
                "{schema}: {file}"
            );
        }
        for dir in dirs {
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
    for language in ["rust", "cpp"] {
        let dir = output_dir(&format!("097-wrong-language-{language}"));
        let result = run(
            &[
                "service-generate",
                "--language",
                language,
                "--world",
                "closed-schema",
                "--with-codec",
                "--ada-codec-runtime",
                lossless,
            ],
            "codec-ada-float64.xsd",
            "codec-ada-float64.yaml",
            Some(&dir),
        );
        assert!(!result.status.success());
        assert!(!dir.exists());
    }
}

#[test]
fn task095_target_opt_in_agrees_between_check_generate_and_atomic_rejection() {
    let schema = "codec-ada-float32.xsd";
    let contract = "codec-ada-float32.yaml";
    let (code, report) = check(schema, contract, "ada", true);
    assert_ne!(code, Some(0));
    assert!(report.contains("Float32"), "{report}");
    let (blocked, blocked_dir) = generate(schema, contract, "ada", true, "095-portable");
    assert!(!blocked.status.success());
    assert!(!blocked_dir.exists());
    let args = [
        "service-check",
        "--language",
        "ada",
        "--world",
        "closed-schema",
        "--with-codec",
        "--ada-codec-runtime",
        "linux-x86_64-glibc-2.41",
    ];
    let ready = run(&args, schema, contract, None);
    assert!(
        ready.status.success(),
        "{}",
        String::from_utf8_lossy(&ready.stderr)
    );
    assert!(String::from_utf8_lossy(&ready.stdout).contains("codec status: READY"));
    let mut args = args;
    args[0] = "service-generate";
    let dir = output_dir("095-qualified");
    let generated = run(&args, schema, contract, Some(&dir));
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let body = std::fs::read_to_string(dir.join("service_codec.adb")).unwrap();
    assert!(body.contains("Ingress.Read (Text)"));
    assert!(body.contains("Model.Create (Number)"));
    assert!(!body.contains("Get_Long_Float"));
    for wrong in ["windows", "linux-x86_64-glibc-2.39", "musl", ""] {
        args[7] = wrong;
        let invalid = run(&args, schema, contract, None);
        assert!(!invalid.status.success());
        assert!(String::from_utf8_lossy(&invalid.stderr).contains("--ada-codec-runtime"));
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn task095_c2_occurrence_decoder_preserves_adversarial_identifiers() {
    let schema = "codec-ada-float32-identifiers.xsd";
    let contract = "codec-ada-float32-identifiers.yaml";
    let args = [
        "service-generate",
        "--language",
        "ada",
        "--world",
        "closed-schema",
        "--with-codec",
        "--ada-codec-runtime",
        "linux-x86_64-glibc-2.41",
    ];
    let dir = output_dir("095-c2-identifiers");
    let result = run(&args, schema, contract, Some(&dir));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let body = std::fs::read_to_string(dir.join("service_codec.adb")).unwrap();
    for name in ["PathLength", "TargetGet", "ArraySample"] {
        assert!(body.contains(&format!(
            "function From_JSON_{name} (Value : Ingress.Value)"
        )));
        assert!(body.contains(&format!("end From_JSON_{name};")));
        assert!(body.contains(&format!(
            "function To_JSON_{name} (Item : Model.{name}) return JSON_Value"
        )));
        assert!(!body.contains(&format!("From_JSON_{name}Ingress")));
    }
    assert!(body.contains("From_JSON_PathLength (Child)"));
    assert!(body.contains("Ingress.Length (JSON_Items)"));
    assert!(!body.contains("From_JSON_Ingress."));
    let portable = run(
        &[
            "service-check",
            "--language",
            "ada",
            "--world",
            "closed-schema",
            "--with-codec",
        ],
        schema,
        contract,
        None,
    );
    assert!(!portable.status.success());
    std::fs::remove_dir_all(dir).unwrap();
}

/// A. The OAM codec-safe service is READY with the codec and generates it.
#[test]
fn task091_ada_duration_readiness_and_model_preservation() {
    let schema = "codec-ada-duration.xsd";
    let contract = "codec-ada-duration.yaml";
    let (code, report) = check(schema, contract, "ada", true);
    assert_eq!(code, Some(0), "{report}");
    assert!(report.contains("codec renderable emitted declarations: 5/5"));
    assert!(report.contains("codec status: READY"));
    let (plain, plain_dir) = generate(schema, contract, "ada", false, "091-model");
    let (coded, coded_dir) = generate(schema, contract, "ada", true, "091-codec");
    assert!(plain.status.success() && coded.status.success());
    for name in listing(&plain_dir) {
        assert_eq!(
            std::fs::read(plain_dir.join(&name)).unwrap(),
            std::fs::read(coded_dir.join(&name)).unwrap(),
            "{name}"
        );
    }
    let body = std::fs::read_to_string(coded_dir.join("service_codec.adb")).unwrap();
    assert_eq!(body.matches("function Scalar_Duration").count(), 1);
    assert!(body.contains("return Model.Create (Text);"));
    assert!(!body.contains("Is_Duration"));
    let spec = std::fs::read_to_string(coded_dir.join("service_codec.ads")).unwrap();
    assert_eq!(spec.matches("function Encode_DurationPayload").count(), 1);
    let (code, report) = check("codec-duration.xsd", "codec-duration.yaml", "ada", true);
    assert_eq!(code, Some(1));
    assert!(
        report.contains("service codec boundary: IntervalChoice.Every: direct Duration Choice alternative is not qualified"),
        "{report}"
    );
}

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

/// Task 098: named TimeZulu is codec READY under every Ada runtime profile;
/// the codec is additive (model/API bytes unchanged) and uses the checked
/// `Model.Create`/`Model.Value` carrier operations without a new helper.
#[test]
fn task098_ada_named_time_zulu_codec_ready_and_model_bytes_unchanged() {
    let lossless = Some("linux-x86_64-glibc-2.41-lossless-v1");
    for (fixture, runtime, count) in [
        ("codec-ada-time-zulu", None, "4/4"),
        ("codec-ada-time-zulu", lossless, "4/4"),
        (
            "codec-ada-time-zulu-float32",
            Some("linux-x86_64-glibc-2.41"),
            "4/4",
        ),
        ("codec-ada-time-zulu-mixed", lossless, "7/7"),
    ] {
        let schema = format!("{fixture}.xsd");
        let contract = format!("{fixture}.yaml");
        let mut args = vec![
            "service-check",
            "--language",
            "ada",
            "--world",
            "closed-schema",
            "--with-codec",
        ];
        if let Some(runtime) = runtime {
            args.extend(["--ada-codec-runtime", runtime]);
        }
        let output = run(&args, &schema, &contract, None);
        let report = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(0), "{fixture}: {report}");
        assert!(
            report.contains(&format!("codec renderable emitted declarations: {count}\n")),
            "{fixture}: {report}"
        );
        args[0] = "service-generate";
        let coded = output_dir(&format!("098-{fixture}-{}-coded", runtime.is_some()));
        let output = run(&args, &schema, &contract, Some(&coded));
        assert!(output.status.success(), "{output:?}");
        let label = format!("098-{fixture}-{}-plain", runtime.is_some());
        let (output, plain) = generate(&schema, &contract, "ada", false, &label);
        assert!(output.status.success(), "{output:?}");
        for name in listing(&plain) {
            assert_eq!(
                std::fs::read(plain.join(&name)).unwrap(),
                std::fs::read(coded.join(&name)).unwrap(),
                "{fixture}: {name}"
            );
        }
        let body = std::fs::read_to_string(coded.join("service_codec.adb")).unwrap();
        // One named conversion pair (declaration + body), reused everywhere.
        assert_eq!(body.matches("function From_JSON_Clock (").count(), 2);
        assert_eq!(body.matches("\"invalid named Time\"").count(), 1);
        assert!(body.contains("(Model.Value (Item))"));
        assert!(!body.contains("Scalar_Time"));
        // The constructor's Constraint_Error is the only translated exception.
        let decoder = &body[body.rfind("function From_JSON_Clock (").unwrap()..];
        let decoder = &decoder[..decoder.find("end From_JSON_Clock;").unwrap()];
        assert_eq!(decoder.matches("when ").count(), 1, "{decoder}");
        assert!(decoder.contains("when Constraint_Error =>"), "{decoder}");
    }
}

/// Task 098: Time admission is profile-exact. Direct xs:time, unconstrained
/// named Time, a different pattern and an extra facet remain model or codec
/// blockers, atomically, under both the portable and lossless runtimes.
#[test]
fn task098_unsupported_time_neighbours_stay_atomic_rejections() {
    let source = std::fs::read_to_string(fixture("codec-ada-time-zulu.xsd")).unwrap();
    let zulu =
        "<xs:restriction base=\"xs:time\">\n  <xs:pattern value=\".+Z\"/>\n </xs:restriction>";
    assert_eq!(source.matches(zulu).count(), 1);
    for (label, schema) in [
        (
            "unconstrained-named",
            source.replace(zulu, "<xs:restriction base=\"xs:time\"/>"),
        ),
        ("other-pattern", source.replace(".+Z", ".*Z")),
        (
            "extra-facet",
            source.replace(
                "<xs:pattern value=\".+Z\"/>",
                "<xs:pattern value=\".+Z\"/><xs:maxLength value=\"20\"/>",
            ),
        ),
        (
            "direct",
            source.replace(
                "name=\"Required\" type=\"uci:Clock\"",
                "name=\"Required\" type=\"xs:time\"",
            ),
        ),
    ] {
        assert_ne!(schema, source, "{label}: mutation did not apply");
        let scratch = output_dir(&format!("098-{label}"));
        std::fs::create_dir(&scratch).unwrap();
        let schema_path = scratch.join("schema.xsd");
        std::fs::write(&schema_path, schema).unwrap();
        for runtime in [None, Some("linux-x86_64-glibc-2.41-lossless-v1")] {
            let output_path = scratch.join(format!("output-{}", runtime.is_some()));
            for action in ["service-check", "service-generate"] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
                command
                    .arg(action)
                    .arg("--schema")
                    .arg(&schema_path)
                    .arg("--contract")
                    .arg(fixture("codec-ada-time-zulu.yaml"))
                    .args([
                        "--language",
                        "ada",
                        "--world",
                        "closed-schema",
                        "--with-codec",
                    ]);
                if let Some(runtime) = runtime {
                    command.args(["--ada-codec-runtime", runtime]);
                }
                if action == "service-generate" {
                    command.arg("--output").arg(&output_path);
                }
                let output = command.output().unwrap();
                let combined = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{label} {action}: {combined}"
                );
                assert!(!output_path.exists(), "{label}: partial output");
                if label == "extra-facet" {
                    // A length facet on xs:time never reaches the IR at all.
                    assert!(
                        combined.contains("unsupported XSD construct: xs:maxLength"),
                        "{label} {action}: {combined}"
                    );
                    continue;
                }
                // Rejected by the unchanged Task 061 model boundary before any
                // codec work: the codec never widens what the model admits.
                assert!(
                    combined.contains("status: NOT READY\n"),
                    "{label} {action}: {combined}"
                );
                assert!(
                    combined.contains("codec: not measured (selected model is NOT READY)"),
                    "{label} {action}: {combined}"
                );
                let blocker = if label == "direct" {
                    "ClockPayload"
                } else {
                    "}Clock\n"
                };
                assert!(combined.contains(blocker), "{label} {action}: {combined}");
                assert!(!output_path.exists(), "{label}: partial output");
            }
        }
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
            "repeated-time-model-preflight",
            source.replace(
                "name=\"Note\" type=\"xs:string\" minOccurs=\"0\"",
                "name=\"Note\" type=\"xs:time\" minOccurs=\"0\" maxOccurs=\"3\"",
            ),
            "codec: not measured (selected model is NOT READY)",
        ),
        (
            "repeated-float64-codec-readiness",
            source.replace(
                "name=\"Note\" type=\"xs:string\" minOccurs=\"0\"",
                "name=\"Note\" type=\"xs:double\" minOccurs=\"0\" maxOccurs=\"3\"",
            ),
            "AdaPayload.Note: unsupported Ada codec primitive or facets: Float64",
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
        if label == "repeated-float64-codec-readiness" {
            let model_path = scratch.join("model");
            for action in ["service-check", "service-generate"] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
                command.arg(action).arg("--schema").arg(&schema_path)
                    .arg("--contract").arg(fixture("codec-ada-minimal.yaml"))
                    .args(["--language", "ada", "--world", "closed-schema"]);
                if action == "service-generate" {
                    command.arg("--output").arg(&model_path);
                }
                let output = command.output().unwrap();
                assert!(output.status.success(), "model-ready Float64: {output:?}");
            }
            assert!(model_path.join("programs-oam.ads").is_file());
        }
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
            if label == "repeated-time-model-preflight" {
                assert!(combined.contains("status: NOT READY"));
                assert!(combined.contains("unsupported selected types:"));
                assert!(!combined.contains("codec status:"));
            }
            if label == "repeated-float64-codec-readiness" {
                assert!(combined.contains("\nstatus: READY\n"));
                assert!(combined.contains("codec status: NOT READY"));
            }
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
            "nillable-choice-member",
            source.replace(
                "name=\"Pick\" type=\"uci:Selection\"",
                "name=\"Pick\" type=\"uci:Selection\" maxOccurs=\"2\" nillable=\"true\"",
            ),
            "unsupported selected types:",
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
            "optional-choice-alternative",
            source
                .replace(
                    "<xs:complexType name=\"Details\"><xs:sequence>",
                    "<xs:complexType name=\"Details\"><xs:choice>",
                )
                .replace(
                    "</xs:sequence></xs:complexType>\n  <xs:complexType name=\"Base\">",
                    "</xs:choice></xs:complexType>\n  <xs:complexType name=\"Base\">",
                )
                .replace(
                    "name=\"Label\" type=\"uci:Label\"",
                    "name=\"Label\" type=\"uci:Label\" minOccurs=\"0\"",
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
