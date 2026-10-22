//! Actual production standalone/batch comparisons, including failed selections.
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "service-batch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-check")
        .join(name)
        .canonicalize()
        .unwrap()
}

fn manifest(requests: Value) -> Value {
    json!({"format":"service-check-batch-v1", "schema_version":"000.1.0", "requests":requests})
}

fn request(id: &str, contract: &str, language: &str, world: &str, codec: bool) -> Value {
    json!({"id":id, "contract":fixture(contract), "language":language,
           "world":world, "with_codec":codec})
}

fn batch(value: &Value) -> Output {
    let scratch = Scratch::new();
    let path = scratch.0.join("manifest.json");
    std::fs::write(&path, value.to_string()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["service-check-batch", "--schema"])
        .arg(fixture("root.xsd"))
        .arg("--manifest")
        .arg(path)
        .output()
        .unwrap()
}

#[test]
fn exact_production_equivalence_including_failure_and_codec() {
    let mut requests = Vec::new();
    for language in ["ada", "rust", "cpp"] {
        for world in ["closed-schema", "open-extensions"] {
            for contract in ["ready.yaml", "unready.yaml"] {
                for codec in [false, true] {
                    requests.push(request(
                        &format!("r{}", requests.len()),
                        contract,
                        language,
                        world,
                        codec,
                    ));
                }
            }
        }
    }
    let missing = std::env::temp_dir().join("nonexistent-perf001-contract.yaml");
    assert!(!missing.exists());
    requests.push(
        json!({"id":"missing", "contract":missing, "language":"rust", "world":"closed-schema"}),
    );
    let extensions = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-plan/service-extension.yaml")
        .canonicalize()
        .unwrap();
    requests.push(json!({"id":"unmapped-extensions", "contract":extensions,
                        "language":"rust", "world":"closed-schema"}));
    // A successful final selection must still run after failed and missing inputs.
    requests.push(request(
        "last",
        "ready.yaml",
        "rust",
        "closed-schema",
        false,
    ));
    let output = batch(&manifest(json!(requests)));
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["completed"], requests.len());
    assert_eq!(report["schema_loads"], 1);
    for (expected, item) in requests.iter().zip(report["results"].as_array().unwrap()) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
        command
            .args(["service-check", "--schema"])
            .arg(fixture("root.xsd"))
            .arg("--contract")
            .arg(expected["contract"].as_str().unwrap())
            .args([
                "--language",
                expected["language"].as_str().unwrap(),
                "--world",
                expected["world"].as_str().unwrap(),
            ]);
        if expected["with_codec"].as_bool() == Some(true) {
            command.arg("--with-codec");
        }
        let standalone = command.output().unwrap();
        assert_eq!(item["request"]["id"], expected["id"]);
        assert_eq!(item["exit_code"], standalone.status.code().unwrap());
        assert_eq!(
            item["stdout"],
            String::from_utf8(standalone.stdout).unwrap()
        );
        assert_eq!(
            item["stderr"],
            String::from_utf8(standalone.stderr).unwrap()
        );
    }
    assert_eq!(
        report["results"].as_array().unwrap().last().unwrap()["exit_code"],
        0
    );
}

#[test]
fn malformed_duplicate_and_wrong_identity_fail_before_execution() {
    let good = request("id", "ready.yaml", "rust", "closed-schema", false);
    let mut wrong = manifest(json!([good.clone()]));
    wrong["schema_version"] = json!("002.6.0");
    let mut unknown = manifest(json!([good.clone()]));
    unknown["unexpected"] = json!(true);
    let mut bad_language = good.clone();
    bad_language["language"] = json!("python");
    let mut relative = good.clone();
    relative["contract"] = json!("relative.yaml");
    let mut empty_id = good.clone();
    empty_id["id"] = json!("");
    let mut invalid_runtime = good.clone();
    invalid_runtime["ada_codec_runtime"] = json!("linux-x86_64-glibc-2.41");
    for value in [
        wrong,
        unknown,
        manifest(json!([])),
        manifest(json!([good.clone(), good])),
        manifest(json!([bad_language])),
        manifest(json!([relative])),
        manifest(json!([empty_id])),
        manifest(json!([invalid_runtime])),
        json!({}),
        json!(null),
    ] {
        let output = batch(&value);
        assert!(!output.status.success(), "accepted {value}");
        assert!(
            output.stdout.is_empty(),
            "published completion for invalid input {value}"
        );
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn malformed_json_missing_manifest_and_schema_fail() {
    let scratch = Scratch::new();
    let path = scratch.0.join("manifest.json");
    for contents in [None, Some("{"), Some("{\"format\":\"x\",\"format\":\"y\"}")] {
        if let Some(text) = contents {
            std::fs::write(&path, text).unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(["service-check-batch", "--schema"])
            .arg(fixture("root.xsd"))
            .arg("--manifest")
            .arg(&path)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    std::fs::write(
        &path,
        manifest(json!([request(
            "r",
            "ready.yaml",
            "rust",
            "closed-schema",
            false
        )]))
        .to_string(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["service-check-batch", "--schema"])
        .arg(scratch.0.join("missing.xsd"))
        .arg("--manifest")
        .arg(path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn batch_command_help_and_required_options() {
    for options in [
        vec![],
        vec!["--schema", "root.xsd"],
        vec!["--manifest", "requests.json"],
        vec![
            "--schema",
            "root.xsd",
            "--manifest",
            "requests.json",
            "--world",
            "closed-schema",
        ],
        vec![
            "--schema",
            "a.xsd",
            "--schema",
            "b.xsd",
            "--manifest",
            "requests.json",
        ],
        vec![
            "--schema",
            "a.xsd",
            "--manifest",
            "a.json",
            "--manifest",
            "b.json",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .arg("service-check-batch")
            .args(options)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["service-check-batch", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("002.5.0")
    );
}
