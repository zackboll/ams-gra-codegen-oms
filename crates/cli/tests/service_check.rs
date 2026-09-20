//! CLI `service-check` integration tests.
//!
//! Fixtures live under `tests/fixtures/service-check` at the workspace root.
//! The schema there deliberately contains unrenderable declarations that the
//! ready contract does NOT select, so these tests also pin the Task 031 value
//! proposition: a service can be READY against a schema set whose full-schema
//! `generate` fails.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/cli has a workspace root")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures/service-check")
        .join(name)
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
}

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

/// Run `service-check` against one schema/contract pair with extra options.
fn run(schema: &str, contract: &str, extra: &[OsString]) -> Output {
    let mut command = cli();
    command
        .arg("service-check")
        .arg("--schema")
        .arg(fixture(schema))
        .arg("--contract")
        .arg(fixture(contract));
    command.args(extra);
    command.output().expect("CLI should run")
}

fn run_ready(extra: &[OsString]) -> Output {
    run("root.xsd", "ready.yaml", extra)
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("report should be UTF-8")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("diagnostic should be UTF-8")
}

// ---------------------------------------------------------------------
// Sections 44/45/48 -- report content and exit status
// ---------------------------------------------------------------------

/// Sections 44/45/48: a ready selection exits 0, writes a complete report with
/// stable language/world spellings, and produces no diagnostic.
#[test]
fn ready_selection_reports_and_exits_zero() {
    let output = run_ready(&args(&["--language", "rust", "--world", "closed-schema"]));
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = stdout_of(&output);

    assert!(stdout.contains("service contract valid"));
    assert!(stdout.contains("service: Readiness Ready"));
    // Public spellings, never Rust `Debug` text such as `Rust`/`ClosedSchemaSet`.
    assert!(stdout.contains("language: rust"));
    assert!(stdout.contains("generation world: closed-schema"));
    assert!(stdout.contains("selected oms messages: 1"));
    assert!(stdout.contains("renderable selected oms messages: 1"));
    assert!(stdout.contains("selected type closure: 1"));
    assert!(stdout.contains("renderable selected types: 1"));
    assert!(stdout.contains("status: READY"));
    // A ready report has nothing to blame.
    assert!(!stdout.contains("blocked selected messages"));
    assert!(!stdout.contains("unsupported selected types"));
}

/// Sections 55/57: the ready verdict holds for every backend and both worlds
/// on this fixture, because the selected closure uses only supported
/// constructs -- even though the schema set as a whole does not.
#[test]
fn ready_selection_holds_for_every_backend_and_world() {
    for language in ["ada", "rust", "cpp"] {
        for world in ["closed-schema", "open-extensions"] {
            let output = run_ready(&args(&["--language", language, "--world", world]));
            assert_eq!(
                output.status.code(),
                Some(0),
                "{language}/{world} should be ready"
            );
            assert!(stdout_of(&output).contains("status: READY"));
        }
    }
}

/// Sections 46/47/48: a not-ready selection still writes the FULL report to
/// stdout, emits one concise diagnostic to stderr, and exits 1.
#[test]
fn unready_selection_reports_blockers_and_exits_one() {
    let output = run(
        "root.xsd",
        "unready.yaml",
        &args(&["--language", "rust", "--world", "closed-schema"]),
    );
    assert_eq!(output.status.code(), Some(1));
    let stdout = stdout_of(&output);
    assert!(stdout.contains("status: NOT READY"));

    // Section 46: qualified identities, in schema declaration order.
    let types = stdout
        .find("unsupported selected types:")
        .expect("unsupported types should be reported");
    let listed = &stdout[types..];
    let a = listed
        .find("{urn:test}DurationA")
        .expect("DurationA should be listed");
    let b = listed
        .find("{urn:test}DurationB")
        .expect("DurationB should be listed");
    assert!(a < b, "types must follow schema declaration order");

    // Section 47: contract spelling, resolved identity, and first blocker.
    assert!(stdout.contains("blocked selected messages:"));
    assert!(stdout.contains("resolved: {urn:test}BlockedReportB"));
    assert!(stdout.contains("blocker: {urn:test}DurationB"));
    assert!(stdout.contains("resolved: {urn:test}BlockedReportA"));
    assert!(stdout.contains("blocker: {urn:test}DurationA"));

    // The supported message is not blamed.
    assert!(!stdout.contains("resolved: {urn:test}SelectedReport"));

    // Section 48: one concise stderr diagnostic naming language and world.
    let stderr = stderr_of(&output);
    assert!(stderr.contains("not renderable"));
    assert!(stderr.contains("rust"));
    assert!(stderr.contains("closed-schema"));
}

/// Sections 29/30: blocked messages follow CONTRACT first-occurrence order
/// (B before A) even though unsupported types follow SCHEMA order (A before
/// B), and a message selected by two exchanges is reported once.
#[test]
fn blocked_messages_follow_contract_order_and_deduplicate() {
    let output = run(
        "root.xsd",
        "unready.yaml",
        &args(&["--language", "cpp", "--world", "closed-schema"]),
    );
    let stdout = stdout_of(&output);
    let blocked = stdout
        .find("blocked selected messages:")
        .expect("blocked messages should be reported");
    let listed = &stdout[blocked..];
    let first_b = listed.find("BlockedReportB").expect("B should be reported");
    let first_a = listed.find("BlockedReportA").expect("A should be reported");
    assert!(first_b < first_a, "contract order places B before A");

    // `BlockedReportA` is selected by two exchanges but reported once.
    assert_eq!(
        listed.matches("resolved: {urn:test}BlockedReportA").count(),
        1
    );
    // Three unique messages selected, one of them renderable.
    assert!(stdout.contains("selected oms messages: 3"));
    assert!(stdout.contains("renderable selected oms messages: 1"));
}

/// Section 49: repeated runs on identical inputs are byte-identical on both
/// streams.
#[test]
fn output_is_deterministic() {
    let options = args(&["--language", "ada", "--world", "open-extensions"]);
    let first = run("root.xsd", "unready.yaml", &options);
    for _ in 0..3 {
        let repeat = run("root.xsd", "unready.yaml", &options);
        assert_eq!(repeat.stdout, first.stdout);
        assert_eq!(repeat.stderr, first.stderr);
        assert_eq!(repeat.status.code(), first.status.code());
    }
}

// ---------------------------------------------------------------------
// Sections 39/40/41/42 -- required options and rejected surfaces
// ---------------------------------------------------------------------

/// Sections 39/40: both `--language` and `--world` are required, with no
/// default. A readiness verdict that did not state what it measured would be
/// unusable evidence.
#[test]
fn missing_language_or_world_is_a_usage_error() {
    for (options, missing) in [
        (args(&["--world", "closed-schema"]), "--language"),
        (args(&["--language", "rust"]), "--world"),
        (Vec::new(), "--language"),
    ] {
        let output = run_ready(&options);
        assert_eq!(output.status.code(), Some(2));
        let stderr = stderr_of(&output);
        assert!(stderr.contains("missing required option"));
        assert!(stderr.contains(missing), "expected {missing} in: {stderr}");
    }
}

/// Sections 39/40: invalid values are rejected rather than guessed. Note that
/// abbreviations such as `closed` are rejected too: an ambiguous world
/// assertion is exactly what the option exists to prevent.
#[test]
fn invalid_language_or_world_is_a_usage_error() {
    for (options, expected) in [
        (
            args(&["--language", "golang", "--world", "closed-schema"]),
            "unsupported language",
        ),
        (
            args(&["--language", "rust", "--world", "closed"]),
            "unsupported generation world",
        ),
        (
            args(&["--language", "Rust", "--world", "closed-schema"]),
            "unsupported language",
        ),
    ] {
        let output = run_ready(&options);
        assert_eq!(output.status.code(), Some(2));
        assert!(stderr_of(&output).contains(expected));
    }
}

/// Sections 39/40: repeating a singular option is an error rather than
/// last-wins, which would silently discard one of two contradictory
/// assertions.
#[test]
fn duplicate_options_are_usage_errors() {
    for (options, duplicated) in [
        (
            args(&[
                "--language",
                "rust",
                "--language",
                "ada",
                "--world",
                "closed-schema",
            ]),
            "--language",
        ),
        (
            args(&[
                "--language",
                "rust",
                "--world",
                "closed-schema",
                "--world",
                "open-extensions",
            ]),
            "--world",
        ),
    ] {
        let output = run_ready(&options);
        assert_eq!(output.status.code(), Some(2));
        let stderr = stderr_of(&output);
        assert!(stderr.contains("duplicate option"));
        assert!(stderr.contains(duplicated));
    }
}

/// Sections 41/42: `service-check` is analysis only and uses exact contract
/// extension mapping, so neither `--output` nor a raw `--overlay` exists.
#[test]
fn output_and_overlay_options_are_rejected() {
    for (option, value) in [("--output", "out"), ("--overlay", "overlay.xsd")] {
        let output = run_ready(&args(&[
            "--language",
            "rust",
            "--world",
            "closed-schema",
            option,
            value,
        ]));
        assert_eq!(output.status.code(), Some(2), "{option} should be rejected");
        let stderr = stderr_of(&output);
        assert!(stderr.contains("unknown option"));
        assert!(stderr.contains(option));
    }
}

/// Section 68/69: the dedicated help documents the supported options and does
/// not advertise a raw `--overlay`.
#[test]
fn help_documents_options_without_advertising_overlay() {
    let output = cli()
        .args(["service-check", "--help"])
        .output()
        .expect("CLI should run");
    assert!(output.status.success());
    let stdout = stdout_of(&output);
    for expected in [
        "--schema",
        "--contract",
        "--extension ID=PATH",
        "--language",
        "--world",
        "writes no generated source",
    ] {
        assert!(stdout.contains(expected), "help should mention {expected}");
    }
    assert!(!stdout.contains("--overlay"));
}

// ---------------------------------------------------------------------
// Sections 55/56/57 -- backend parity and the value proposition
// ---------------------------------------------------------------------

/// Run ordinary full-schema `generate` into a scratch directory.
fn run_generate(schema: &str, language: &str, world: &str, output: &Path) -> Output {
    cli()
        .arg("generate")
        .arg("--schema")
        .arg(fixture(schema))
        .arg("--language")
        .arg(language)
        .arg("--output")
        .arg(output)
        .arg("--world")
        .arg(world)
        .output()
        .expect("CLI should run")
}

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ams-gra-031-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// Section 55: when the selected closure IS the whole schema, a READY verdict
/// must be backed by ordinary generation actually succeeding. Otherwise the
/// analyzer would be claiming capability a backend lacks.
#[test]
fn ready_verdict_agrees_with_successful_backend_generation() {
    for language in ["ada", "rust", "cpp"] {
        for world in ["closed-schema", "open-extensions"] {
            let check = run(
                "parity-ready.xsd",
                "parity.yaml",
                &args(&["--language", language, "--world", world]),
            );
            assert_eq!(check.status.code(), Some(0), "{language}/{world}");

            let directory = scratch(&format!("ready-{language}-{world}"));
            let generated = run_generate("parity-ready.xsd", language, world, &directory);
            assert!(
                generated.status.success(),
                "{language}/{world} generation must succeed when readiness says READY: {}",
                stderr_of(&generated)
            );
            let _ = std::fs::remove_dir_all(&directory);
        }
    }
}

/// Section 56: the converse. A NOT READY verdict on a minimal selected schema
/// must be backed by ordinary generation failing. Only the support STATUS must
/// agree; the diagnostic wording deliberately need not.
#[test]
fn unready_verdict_agrees_with_failing_backend_generation() {
    for language in ["ada", "rust", "cpp"] {
        for world in ["closed-schema", "open-extensions"] {
            let check = run(
                "parity-unready.xsd",
                "parity.yaml",
                &args(&["--language", language, "--world", world]),
            );
            assert_eq!(check.status.code(), Some(1), "{language}/{world}");
            assert!(stdout_of(&check).contains("status: NOT READY"));

            let directory = scratch(&format!("unready-{language}-{world}"));
            let generated = run_generate("parity-unready.xsd", language, world, &directory);
            assert!(
                !generated.status.success(),
                "{language}/{world} generation must fail when readiness says NOT READY"
            );
            let _ = std::fs::remove_dir_all(&directory);
        }
    }
}

/// Section 57: the documented, intentional asymmetry. `service-check` reports
/// READY for the selected contract while full-schema `generate` on the SAME
/// document fails on an unselected declaration. This is not an inconsistency:
/// it is the reason contract-selected generation will be useful.
#[test]
fn ready_selection_coexists_with_failing_full_schema_generation() {
    let check = run_ready(&args(&["--language", "rust", "--world", "closed-schema"]));
    assert_eq!(check.status.code(), Some(0));
    assert!(stdout_of(&check).contains("status: READY"));

    let directory = scratch("unselected-demo");
    let generated = run_generate("root.xsd", "rust", "closed-schema", &directory);
    assert!(
        !generated.status.success(),
        "full-schema generation should fail on the UNSELECTED Duration declaration"
    );
    assert!(stderr_of(&generated).contains("Duration"));
    let _ = std::fs::remove_dir_all(&directory);
}

/// Section 41: `service-check` performs no filesystem writes at all.
#[test]
fn writes_no_files() {
    let directory = fixture("");
    let snapshot = || {
        std::fs::read_dir(&directory)
            .expect("fixture directory should exist")
            .map(|entry| entry.expect("entry").path())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let before = snapshot();
    assert!(
        run_ready(&args(&["--language", "rust", "--world", "closed-schema"]))
            .status
            .success()
    );
    assert_eq!(before, snapshot());
}

// ---------------------------------------------------------------------
// Section 36 -- shared exact extension-set matching
// ---------------------------------------------------------------------

/// Section 36: `service-check` reuses the same exact extension mapping helper
/// as `service-plan`, so an undeclared mapping fails identically here. The
/// fixture contract declares no extensions at all.
#[test]
fn undeclared_extension_mapping_fails_exactly_as_in_service_plan() {
    let output = run_ready(&args(&[
        "--language",
        "rust",
        "--world",
        "closed-schema",
        "--extension",
        "urn:not-declared=some-overlay.xsd",
    ]));
    assert_eq!(output.status.code(), Some(2));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("does not declare extension"));
    assert!(stderr.contains("urn:not-declared"));
}

/// Section 36: a malformed mapping is rejected rather than guessed at; there
/// is no path-only spelling from which an identifier could be inferred.
#[test]
fn malformed_extension_mapping_is_rejected() {
    let output = run_ready(&args(&[
        "--language",
        "rust",
        "--world",
        "closed-schema",
        "--extension",
        "overlay.xsd",
    ]));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr_of(&output).contains("expected ID=PATH"));
}

/// Section 36: a duplicate extension identifier fails rather than letting the
/// second path silently win.
#[test]
fn duplicate_extension_identifier_is_rejected() {
    let output = run_ready(&args(&[
        "--language",
        "rust",
        "--world",
        "closed-schema",
        "--extension",
        "urn:dup=a.xsd",
        "--extension",
        "urn:dup=b.xsd",
    ]));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr_of(&output).contains("duplicate '--extension' identifier"));
}

// ---------------------------------------------------------------------
// Section 35 -- private overlay readiness
// ---------------------------------------------------------------------

fn service_plan_fixture(name: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures/service-plan")
        .join(name)
}

/// Section 35: a message that exists ONLY in a private overlay resolves
/// through Task 029 composition and then participates in readiness completely
/// normally. Nothing treats it specially for being private; it is measured
/// under the same explicit world semantics as any other selected message.
#[test]
fn private_overlay_message_participates_in_readiness_normally() {
    for world in ["closed-schema", "open-extensions"] {
        let output = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(service_plan_fixture("root.xsd"))
            .arg("--contract")
            .arg(service_plan_fixture("service-extension.yaml"))
            .arg("--extension")
            .arg(format!(
                "ext-a-1.0={}",
                service_plan_fixture("private-overlay.xsd").display()
            ))
            .arg("--extension")
            .arg(format!(
                "ext-b-2.0={}",
                service_plan_fixture("private-overlay-b.xsd").display()
            ))
            .args(["--language", "rust", "--world", world])
            .output()
            .expect("CLI should run");

        let stdout = stdout_of(&output);
        // The overlay-only message really was selected and measured.
        assert!(stdout.contains("selected oms messages: 1"), "{stdout}");
        // Its closure reaches PositionType from the root document, so overlay
        // composition genuinely happened rather than the overlay being read
        // in isolation.
        assert!(stdout.contains("selected type closure: 2"), "{stdout}");
        assert!(stdout.contains("status: READY"), "{world}: {stdout}");
        assert_eq!(output.status.code(), Some(0));
    }
}

/// Section 35: omitting a declared extension still fails here, so readiness
/// can never be measured against a silently smaller type universe.
#[test]
fn missing_declared_extension_mapping_fails_before_readiness() {
    let output = cli()
        .arg("service-check")
        .arg("--schema")
        .arg(service_plan_fixture("root.xsd"))
        .arg("--contract")
        .arg(service_plan_fixture("service-extension.yaml"))
        .arg("--extension")
        .arg(format!(
            "ext-a-1.0={}",
            service_plan_fixture("private-overlay.xsd").display()
        ))
        .args(["--language", "rust", "--world", "closed-schema"])
        .output()
        .expect("CLI should run");
    assert_eq!(output.status.code(), Some(2));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("ext-b-2.0"));
    assert!(stderr.contains("no '--extension"));
}
