use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-plan")
        .join(name)
}

fn run(contract: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["service-routes", "--schema"])
        .arg(fixture("root.xsd"))
        .arg("--contract")
        .arg(fixture(contract))
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn task071_cli_argument_controls() {
    for extra in [
        vec!["--schema", "duplicate"],
        vec!["--contract", "duplicate"],
        vec!["--format", "json"],
        vec!["--unknown", "value"],
        vec!["--extension", "malformed"],
        vec!["--extension", "unknown=file"],
        vec!["--extension", "id=file", "--extension", "id=file"],
        vec!["--world", "closed-schema"],
        vec!["--language", "rust"],
        vec!["--with-codec"],
        vec!["--output", "/tmp/task071/forbidden-output"],
        vec!["--service-id", "svc"],
        vec!["--service-uuid", "uuid"],
        vec!["--format", "sleet-toml"],
        vec!["--format", "sleet-toml", "--service-id", "svc"],
        vec![
            "--format",
            "sleet-toml",
            "--service-uuid",
            "550e8400-e29b-41d4-a716-446655440071",
        ],
        vec![
            "--format",
            "sleet-toml",
            "--service-id",
            "bad id",
            "--service-uuid",
            "550e8400-e29b-41d4-a716-446655440071",
        ],
        vec![
            "--format",
            "sleet-toml",
            "--service-id",
            "svc",
            "--service-uuid",
            "invalid",
        ],
    ] {
        let output = run("service.yaml", &extra);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{extra:?}: {:?}",
            output.stderr
        );
        assert!(output.stdout.is_empty());
    }
    for args in [
        vec!["service-routes"],
        vec!["service-routes", "--schema", "missing"],
        vec!["service-routes", "--contract", "missing"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    assert_eq!(run("service-extension.yaml", &[]).status.code(), Some(2));
    assert!(!Path::new("/tmp/task071/forbidden-output").exists());
}

#[test]
fn task071_cli_determinism_namespace_and_failures() {
    let first = run("service.yaml", &[]);
    assert_eq!(first.status.code(), Some(0));
    assert_eq!(
        first.stdout,
        run("service.yaml", &["--format", "tsv"]).stdout
    );
    let text = String::from_utf8(first.stdout).unwrap();
    assert_eq!(text.lines().count(), 3);
    assert!(text.contains("mission-data\tposition-input\tinput\tsubscribe\tmandatory\tmission.position-report\turn:test\tPositionReport\tfalse\t"));
    assert!(text.contains("observation-output\toutput\tpublish\toptional"));
    let output = run(
        "service.yaml",
        &[
            "--format",
            "sleet-toml",
            "--service-id",
            "explicit-id",
            "--service-uuid",
            "550e8400-e29b-41d4-a716-446655440071",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unsupported Sleet message namespace")
    );
    let output = run("missing.yaml", &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn task071_private_overlay_uses_existing_resolution() {
    let a = format!("ext-a-1.0={}", fixture("private-overlay.xsd").display());
    let b = format!("ext-b-2.0={}", fixture("private-overlay-b.xsd").display());
    let first = run(
        "service-extension.yaml",
        &["--extension", &a, "--extension", &b],
    );
    assert_eq!(first.status.code(), Some(0), "{:?}", first.stderr);
    assert_eq!(
        first.stdout,
        run(
            "service-extension.yaml",
            &["--extension", &b, "--extension", &a]
        )
        .stdout
    );
    assert!(
        String::from_utf8(first.stdout)
            .unwrap()
            .contains("PrivateReport")
    );
}

#[test]
fn task071_oam_and_empty_production_cli() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    for (contract, rows) in [("policy.yaml", 2), ("empty.yaml", 1)] {
        let mut cli = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
        cli.arg("service-routes")
            .arg("--schema")
            .arg(root.join("service-generate/runtime-oam.xsd"))
            .arg("--contract")
            .arg(root.join("service-routes").join(contract));
        let output = cli.output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().lines().count(),
            rows
        );
        let output = cli
            .args([
                "--format",
                "sleet-toml",
                "--service-id",
                "explicit-id",
                "--service-uuid",
                "550e8400-e29b-41d4-a716-446655440071",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
        let expected = if contract == "empty.yaml" {
            "allowed_topics = []\n"
        } else {
            "allowed_topics = [\"loop-topic\"]\n\n[[topic_bindings]]\ntopic = \"loop-topic\"\nallowed_messages = [\"MessageA\"]\n"
        };
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!(
                "service_id = \"explicit-id\"\nservice_uuid = \"550e8400-e29b-41d4-a716-446655440071\"\n{expected}"
            )
        );
    }
}
