//! Task 057: synthetic selected-service evidence for XML Schema `duration`.
//!
//! Local fixtures only (Fast CI). `service-check` and `service-generate`
//! must agree in every backend: a selection holding named, direct,
//! optional, repeated and inherited Duration is READY and its generated
//! model compiles; selecting `xs:time` stays NOT READY and writes nothing;
//! a constrained Duration never loads.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LANGUAGES: [&str; 3] = ["ada", "rust", "cpp"];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate")
        .join(name)
}

fn output_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ams-gra-oms-task057-{label}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn cli(command: &str, schema: &str, contract: &str, language: &str, out: Option<&Path>) -> Output {
    let mut cli = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    cli.arg(command)
        .arg("--schema")
        .arg(fixture(schema))
        .arg("--contract")
        .arg(fixture(contract))
        .args(["--language", language, "--world", "closed-schema"]);
    if let Some(out) = out {
        cli.arg("--output").arg(out);
    }
    cli.output().expect("CLI runs")
}

fn files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn assert_success(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `xs:time` is NOT a Task 057 gain: selecting it stays NOT READY in every
/// backend and `service-generate` writes nothing.
#[test]
fn task057_selected_time_stays_not_ready_and_writes_nothing() {
    for language in LANGUAGES {
        let check = cli(
            "service-check",
            "duration.xsd",
            "duration-clock.yaml",
            language,
            None,
        );
        assert_eq!(check.status.code(), Some(1), "{language}");
        let report = String::from_utf8(check.stdout).expect("UTF-8");
        assert!(report.contains("status: NOT READY"), "{language}: {report}");
        assert!(report.contains("{urn:test}ClockPayload"), "{report}");
        let root = output_dir(&format!("time-{language}"));
        let generated = cli(
            "service-generate",
            "duration.xsd",
            "duration-clock.yaml",
            language,
            Some(&root),
        );
        assert_eq!(generated.status.code(), Some(1), "{language}");
        assert!(files(&root).is_empty(), "{language}: partial output");
    }
}

/// A constrained Duration is never accepted: the frontend rejects the facet
/// before any readiness verdict or output exists.
#[test]
fn task057_constrained_duration_fails_closed_before_output() {
    for language in LANGUAGES {
        let check = cli(
            "service-check",
            "duration-constrained.xsd",
            "duration-constrained.yaml",
            language,
            None,
        );
        assert_ne!(check.status.code(), Some(0), "{language}");
        let stderr = String::from_utf8_lossy(&check.stderr);
        assert!(stderr.contains("xs:maxInclusive"), "{language}: {stderr}");
        let root = output_dir(&format!("constrained-{language}"));
        let generated = cli(
            "service-generate",
            "duration-constrained.xsd",
            "duration-constrained.yaml",
            language,
            Some(&root),
        );
        assert_ne!(generated.status.code(), Some(0), "{language}");
        assert!(files(&root).is_empty(), "{language}: partial output");
    }
}

fn compile(language: &str, root: &Path, all: &[PathBuf]) {
    match language {
        "rust" => {
            let model = all
                .iter()
                .find(|path| {
                    path.extension().is_some_and(|ext| ext == "rs")
                        && !path.ends_with("service_api.rs")
                })
                .expect("model");
            let compiled = Command::new("rustc")
                .args(["--edition", "2021", "--crate-type", "lib", "-D", "warnings"])
                .args(["-A", "dead_code"])
                .arg(model)
                .arg("-o")
                .arg(root.join("model.rlib"))
                .output()
                .expect("rustc");
            assert_success(&compiled, "rustc");
        }
        "cpp" => {
            let header = all
                .iter()
                .find(|path| path.extension().is_some_and(|ext| ext == "hpp"))
                .expect("header");
            std::fs::write(
                root.join("client.cpp"),
                format!(
                    "#include \"{}\"\nint main() {{ return 0; }}\n",
                    header.display()
                ),
            )
            .expect("probe");
            let compiled = Command::new("c++")
                .current_dir(root)
                .args([
                    "-std=c++17",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-pedantic-errors",
                ])
                .args(["-c", "client.cpp", "-o", "client.o"])
                .output()
                .expect("c++");
            assert_success(&compiled, "c++");
        }
        "ada" => {
            let body = all
                .iter()
                .find(|path| {
                    path.extension().is_some_and(|ext| ext == "adb")
                        && path.to_string_lossy().contains('-')
                })
                .expect("model body");
            let compiled = Command::new("gnatmake")
                .current_dir(body.parent().expect("dir"))
                .args(["-q", "-c", "-gnatc"])
                .arg(body.file_name().expect("name"))
                .output()
                .expect("gnatmake");
            assert_success(&compiled, "gnatmake");
        }
        _ => unreachable!(),
    }
}

/// The selected Duration service is READY, generates, and its model
/// compiles in all three backends (Rust warnings denied; strict C++17; GNAT).
#[test]
fn task057_selected_duration_service_is_ready_generates_and_compiles() {
    for language in LANGUAGES {
        let check = cli(
            "service-check",
            "duration.xsd",
            "duration-duration.yaml",
            language,
            None,
        );
        assert_eq!(check.status.code(), Some(0), "{language}");
        let report = String::from_utf8(check.stdout).expect("UTF-8");
        assert!(report.contains("status: READY"), "{language}: {report}");
        let root = output_dir(&format!("service-{language}"));
        let generated = cli(
            "service-generate",
            "duration.xsd",
            "duration-duration.yaml",
            language,
            Some(&root),
        );
        assert_success(&generated, language);
        let all = files(&root);
        let text: String = all
            .iter()
            .map(|path| std::fs::read_to_string(path).expect("text"))
            .collect();
        let carrier = if language == "ada" {
            "XML_Schema_Duration"
        } else {
            "XmlSchemaDuration"
        };
        assert!(text.contains(carrier), "{language}");
        assert!(!text.contains("ClockPayload"), "{language}: leaked Time");
        compile(language, &root, &all);
        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
