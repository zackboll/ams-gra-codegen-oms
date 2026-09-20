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
