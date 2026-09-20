use std::process::Command;

#[test]
fn binary_prints_real_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .arg("--help")
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help should be UTF-8");
    assert!(stdout.contains("validate --schema PATH"));
    assert!(stdout.contains("coverage --schema PATH"));
    assert!(stdout.contains("generate --schema PATH [--overlay PATH]... --language LANGUAGE"));
    // Task 029: the repeatable overlay option is documented in top-level help.
    assert!(stdout.contains("SCHEMA OVERLAYS:"));
    assert!(stdout.contains("--overlay PATH"));
    // Task 030: the new read-only contract planning command and its explicit
    // extension-identifier mapping are documented too.
    assert!(stdout.contains("service-plan --schema PATH --contract PATH"));
    assert!(stdout.contains("SERVICE CONTRACT PLANNING:"));
    assert!(stdout.contains("--extension ID=PATH"));
    assert!(!stdout.contains("BOOTSTRAP STATUS"));
    assert!(output.stderr.is_empty());
}

#[test]
fn binary_reports_usage_errors_on_stderr_with_exit_code_two() {
    let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["generate", "--schema", "root.xsd"])
        .output()
        .expect("CLI should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("missing required option '--language'"));
}

/// Task 030 adds a new consumer path; it must not alter the existing ones.
#[test]
fn existing_commands_remain_unchanged() {
    for command in ["validate", "coverage", "generate"] {
        let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args([command, "--help"])
            .output()
            .expect("CLI should run");
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).expect("help should be UTF-8");
        // The Task 029 overlay surface is untouched on every prior command.
        assert!(stdout.contains("--overlay PATH"), "{command}");
        // None of them gained a contract surface.
        assert!(!stdout.contains("--contract"), "{command}");
        assert!(!stdout.contains("--extension"), "{command}");
    }

    // 'service-plan' is symmetric: no world, language, or output.
    let output = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["service-plan", "--help"])
        .output()
        .expect("CLI should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help should be UTF-8");
    assert!(stdout.contains("--contract PATH"));
    assert!(!stdout.contains("--world WORLD"));
    assert!(!stdout.contains("--language LANGUAGE"));
    assert!(!stdout.contains("--output DIR"));
}
