//! CLI `service-plan` integration tests.
//!
//! Paths are resolved against the workspace root so the fixtures under
//! `tests/fixtures/service-plan` are shared with the library-level tests.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/cli has a workspace root")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures/service-plan")
        .join(name)
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
}

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn run_service_plan(extra: &[OsString]) -> std::process::Output {
    let mut command = cli();
    command
        .arg("service-plan")
        .arg("--schema")
        .arg(fixture("root.xsd"))
        .arg("--contract")
        .arg(fixture("service.yaml"));
    command.args(extra);
    command.output().expect("CLI should run")
}

#[test]
fn resolves_the_synthetic_contract_and_reports_a_deterministic_summary() {
    let output = run_service_plan(&[]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("summary should be UTF-8");

    assert!(stdout.contains("service contract valid"));
    assert!(stdout.contains("service: Contract Codegen Test"));
    assert!(stdout.contains("kind: service"));
    assert!(stdout.contains("functions: 1"));
    assert!(stdout.contains("exchange occurrences: 2"));
    assert!(stdout.contains("oms message exchanges: 2"));
    assert!(stdout.contains("unique uci messages: 2"));
    assert!(stdout.contains("mission-data / position-input"));
    assert!(stdout.contains("input PositionReport"));
    assert!(stdout.contains("topic: mission.position-report"));
    assert!(stdout.contains("resolved: {urn:test}PositionReport"));
    assert!(stdout.contains("mission-data / observation-output"));
    assert!(stdout.contains("output ObservationMeasurementReport"));
    assert!(stdout.contains("resolved: {urn:test}ObservationMeasurementReport"));
    // Capability omission is reported as omission, not as "0".
    assert!(stdout.contains("capabilities: (omitted by contract)"));

    // Repeating the run must produce byte-identical output.
    let repeat = run_service_plan(&[]);
    assert_eq!(repeat.stdout, stdout.as_bytes());
}

#[test]
fn writes_no_files() {
    // Snapshot the fixture directory before and after; Task 030 is read-only.
    let directory = fixture("");
    let before = std::fs::read_dir(&directory)
        .expect("fixture directory should exist")
        .map(|entry| entry.expect("entry").path())
        .collect::<std::collections::BTreeSet<_>>();
    let output = run_service_plan(&[]);
    assert!(output.status.success());
    let after = std::fs::read_dir(&directory)
        .expect("fixture directory should exist")
        .map(|entry| entry.expect("entry").path())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(before, after);
}

#[test]
fn rejects_world_language_and_output_options() {
    // Task 030 deliberately has no generation policy or output surface.
    for option in ["--world", "--language", "--output"] {
        let output = run_service_plan(&args(&[option, "closed-schema"]));
        assert_eq!(output.status.code(), Some(2), "{option} should be rejected");
        let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
        assert!(stderr.contains("unknown option"));
    }
}

#[test]
fn contract_with_no_extensions_and_no_mappings_succeeds() {
    assert!(run_service_plan(&[]).status.success());
}

#[test]
fn undeclared_extension_supplied_fails() {
    let overlay = fixture("private-overlay.xsd");
    let output = run_service_plan(&args(&[
        "--extension",
        &format!("ext-unknown={}", overlay.display()),
    ]));
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("does not declare extension 'ext-unknown'"));
}

fn run_extension_plan(extra: &[OsString]) -> std::process::Output {
    let mut command = cli();
    command
        .arg("service-plan")
        .arg("--schema")
        .arg(fixture("root.xsd"))
        .arg("--contract")
        .arg(fixture("service-extension.yaml"));
    command.args(extra);
    command.output().expect("CLI should run")
}

fn extension(id: &str, file: &str) -> OsString {
    let mut value = OsString::from(format!("{id}="));
    value.push(fixture(file));
    value
}

#[test]
fn declared_extension_missing_fails() {
    // Only one of the two declared extensions is mapped.
    let output = run_extension_plan(&[
        OsString::from("--extension"),
        extension("ext-a-1.0", "private-overlay.xsd"),
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("declares extension 'ext-b-2.0'"));
    assert!(stderr.contains("--extension ext-b-2.0=PATH"));
}

#[test]
fn no_mappings_for_a_contract_that_declares_extensions_fails() {
    let output = run_extension_plan(&[]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("declares extension 'ext-a-1.0'"));
}

#[test]
fn duplicate_extension_mapping_fails() {
    let output = run_extension_plan(&[
        OsString::from("--extension"),
        extension("ext-a-1.0", "private-overlay.xsd"),
        OsString::from("--extension"),
        extension("ext-a-1.0", "private-overlay-b.xsd"),
        OsString::from("--extension"),
        extension("ext-b-2.0", "private-overlay-b.xsd"),
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("duplicate '--extension' identifier 'ext-a-1.0'"));
}

#[test]
fn rejects_extension_values_that_are_not_id_equals_path_pairs() {
    // A bare path is NOT accepted: contract extension entries are logical
    // identifiers and must never be guessed from a filename.
    let mut value = OsString::new();
    value.push(fixture("private-overlay.xsd"));
    let output = run_extension_plan(&[OsString::from("--extension"), value]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("expected ID=PATH"));
}

#[test]
fn task_029_overlay_enables_resolution_of_a_private_message() {
    // PrivateReport exists only in the overlay, so success here proves the
    // mapping really reaches load_schema_set_with_overlays.
    let output = run_extension_plan(&[
        OsString::from("--extension"),
        extension("ext-a-1.0", "private-overlay.xsd"),
        OsString::from("--extension"),
        extension("ext-b-2.0", "private-overlay-b.xsd"),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("summary should be UTF-8");
    assert!(stdout.contains("resolved: {urn:test}PrivateReport"));
    assert!(stdout.contains("unique uci messages: 1"));
}

#[test]
fn without_the_overlay_the_private_message_does_not_resolve() {
    // The same contract, with the private overlay replaced by a document that
    // does not declare PrivateReport, must fail: nothing is invented.
    let output = run_extension_plan(&[
        OsString::from("--extension"),
        extension("ext-a-1.0", "private-overlay-b.xsd"),
        OsString::from("--extension"),
        extension("ext-b-2.0", "private-overlay-b.xsd"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("function 'private-data' exchange 'private-input'"));
    assert!(stderr.contains("OMS message 'PrivateReport' was not found"));
}

#[test]
fn cli_option_order_does_not_change_the_result_because_contract_order_wins() {
    // The contract declares ext-a-1.0 then ext-b-2.0. Supplying them in the
    // opposite command-line order must produce identical output, because the
    // loader receives contract order, not CLI order.
    let contract_order = run_extension_plan(&[
        OsString::from("--extension"),
        extension("ext-a-1.0", "private-overlay.xsd"),
        OsString::from("--extension"),
        extension("ext-b-2.0", "private-overlay-b.xsd"),
    ]);
    let reversed_cli_order = run_extension_plan(&[
        OsString::from("--extension"),
        extension("ext-b-2.0", "private-overlay-b.xsd"),
        OsString::from("--extension"),
        extension("ext-a-1.0", "private-overlay.xsd"),
    ]);
    assert!(contract_order.status.success());
    assert!(reversed_cli_order.status.success());
    assert_eq!(contract_order.stdout, reversed_cli_order.stdout);
}

#[test]
fn duplicate_overlay_declaration_still_fails_through_task_029() {
    // Mapping both declared extensions to two documents that declare the same
    // qualified type keeps failing in the frontend: overlay order never
    // implies duplicate-declaration precedence.
    let output = run_extension_plan(&[
        OsString::from("--extension"),
        extension("ext-a-1.0", "private-overlay-b.xsd"),
        OsString::from("--extension"),
        extension("ext-b-2.0", "duplicate-private-aux.xsd"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.to_lowercase().contains("duplicate"));
}
