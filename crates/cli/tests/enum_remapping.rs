//! Task 044: frontend, selected readiness, backend naming and real compilers.
use ams_gra_oms_codegen_core::{BackendLanguage, generated_enum_variant_name};
use ams_gra_oms_ir::TypeKind;
use ams_gra_oms_schema_docs::generate as generate_docs;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate")
        .join(name)
}

fn output(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("ams-gra-oms-task044-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn cli(command: &str, schema: &str, language: &str, output: Option<&Path>) -> std::process::Output {
    let mut process = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    process.args([
        command,
        "--schema",
        fixture(schema).to_str().unwrap(),
        "--contract",
        fixture("enum-remapping.yaml").to_str().unwrap(),
        "--language",
        language,
        "--world",
        "closed-schema",
    ]);
    if let Some(output) = output {
        process.arg("--output").arg(output);
    }
    process.output().expect("CLI runs")
}

#[test]
fn task044_frontend_and_docs_preserve_wire_values() {
    let schema = load_schema_set(&fixture("enum-remapping.xsd")).unwrap();
    let enumeration = schema
        .types
        .iter()
        .find(|t| t.name.local_name == "SignalCode")
        .unwrap();
    let TypeKind::Enumeration { variants } = &enumeration.kind else {
        panic!("expected enum")
    };
    for wire in ["25X1", "AND"] {
        assert!(variants.iter().any(|v| v.wire_value == wire));
    }
    let docs = generate_docs(&schema).unwrap();
    let page = docs
        .iter()
        .find(|f| {
            f.contents.contains("SignalCode")
                && f.relative_path.to_string_lossy().starts_with("types/")
        })
        .unwrap();
    for wire in ["25X1", "AND"] {
        assert!(page.contents.contains(wire));
    }
    assert!(!page.contents.contains("Value_25X1") && !page.contents.contains("Value_AND"));
    let search = docs
        .iter()
        .find(|f| {
            f.relative_path
                .to_string_lossy()
                .ends_with("search-index.js")
        })
        .unwrap();
    assert!(search.contents.contains("25X1") && search.contents.contains("AND"));
}

#[test]
fn task044_selected_services_are_ready_for_each_authorized_category() {
    for (schema, languages) in [
        ("enum-remapping.xsd", ["ada", "rust", "cpp"]),
        ("enum-reserved.xsd", ["ada", "rust", "cpp"]),
    ] {
        for language in languages {
            let report = cli("service-check", schema, language, None);
            assert!(
                report.status.success()
                    && String::from_utf8_lossy(&report.stdout).contains("status: READY"),
                "{} / {}: {}",
                schema,
                language,
                String::from_utf8_lossy(&report.stderr)
            );
        }
    }
}

#[test]
fn task044_ada_reserved_only_selection_compiles_under_gnat() {
    let root = output("ada-reserved-only");
    let generated = cli("service-generate", "enum-reserved.xsd", "ada", Some(&root));
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let spec = std::fs::read_to_string(root.join("urn-test.ads")).unwrap();
    let name = generated_enum_variant_name(BackendLanguage::Ada, "AND").unwrap();
    assert!(spec.contains(&name) && spec.contains("NORMAL"));
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "GNAT required"
        );
        return;
    }
    std::fs::write(root.join("probe.adb"), format!("with Urn.Test;\nprocedure Probe is\n   Item : Urn.Test.SignalPayload := (Signal => Urn.Test.NORMAL);\nbegin\n   Item.Signal := Urn.Test.{name};\nend Probe;\n")).unwrap();
    let compiled = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "probe.adb"])
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert!(Command::new(root.join("probe")).status().unwrap().success());
}

#[test]
fn task044_generated_selected_models_compile_and_use_every_variant() {
    let values = [
        "NORMAL",
        "SOME_VALUE",
        "70W",
        "5G",
        "25X1",
        "1METER",
        "25_FEET",
        "0_TO_2METERS",
        "1_IN_1",
        "0_TO_1METERS",
        "1_SIGMA",
        "AND",
        "Self",
    ];
    for (language, backend) in [
        ("ada", BackendLanguage::Ada),
        ("rust", BackendLanguage::Rust),
        ("cpp", BackendLanguage::Cpp),
    ] {
        let root = output(language);
        let generated = cli(
            "service-generate",
            "enum-remapping.xsd",
            language,
            Some(&root),
        );
        assert!(
            generated.status.success(),
            "{}",
            String::from_utf8_lossy(&generated.stderr)
        );
        let generated_names: Vec<_> = values
            .iter()
            .map(|v| generated_enum_variant_name(backend, v).unwrap())
            .collect();
        // Task 047: `service-generate` also emits the `service_api.*` wrapper
        // entrypoint. This test is about the UCI MODEL's enumeration
        // spellings, so the wrapper is excluded from the model file set.
        let files: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .map(|f| f.unwrap().path())
            .filter(|p| {
                p.file_stem()
                    .is_none_or(|stem| !stem.eq_ignore_ascii_case("service_api"))
            })
            .collect();
        let source = files
            .iter()
            .map(|p| std::fs::read_to_string(p).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        for name in &generated_names {
            assert!(
                source.contains(name),
                "{language} renderer did not emit helper spelling {name}"
            );
        }
        match language {
            "ada" => {
                if Command::new("gnatmake").arg("--version").output().is_err() {
                    assert!(
                        std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                        "GNAT required"
                    );
                    continue;
                }
                let assignments = generated_names
                    .iter()
                    .map(|n| format!("   Item.Signal := Urn.Test.{n};\n"))
                    .collect::<String>();
                std::fs::write(
                    root.join("probe.adb"),
                    format!("with Urn.Test;\nprocedure Probe is\n   use type Urn.Test.SignalCode;\n   Item : Urn.Test.SignalPayload;\nbegin\n{assignments}   if Item.Signal /= Urn.Test.{} then raise Program_Error; end if;\nend Probe;\n", generated_names.last().unwrap()),
                ).unwrap();
                let compiled = Command::new("gnatmake")
                    .current_dir(&root)
                    .args(["-q", "probe.adb"])
                    .output()
                    .unwrap();
                assert!(
                    compiled.status.success(),
                    "Ada: {}",
                    String::from_utf8_lossy(&compiled.stderr)
                );
                assert!(Command::new(root.join("probe")).status().unwrap().success());
            }
            "rust" => {
                let source_path = files
                    .iter()
                    .find(|p| p.extension().is_some_and(|e| e == "rs"))
                    .unwrap();
                let declarations = std::fs::read_to_string(source_path).unwrap();
                let uses = generated_names.iter().map(|n| format!("    let item = SignalPayload {{ signal: SignalCode::{n} }}; let _ = item.signal;\n")).collect::<String>();
                std::fs::write(
                    root.join("probe.rs"),
                    format!("{declarations}\nfn main() {{\n{uses}}}\n"),
                )
                .unwrap();
                let compiled = Command::new("rustc")
                    .current_dir(&root)
                    .args(["--edition=2024", "probe.rs", "-o", "probe"])
                    .output()
                    .unwrap();
                assert!(
                    compiled.status.success(),
                    "Rust: {}",
                    String::from_utf8_lossy(&compiled.stderr)
                );
                assert!(Command::new(root.join("probe")).status().unwrap().success());
            }
            _ => {
                let header = files
                    .iter()
                    .find(|p| p.extension().is_some_and(|e| e == "hpp"))
                    .unwrap();
                let uses = generated_names
                    .iter()
                    .map(|n| {
                        format!("    {{ auto item = urn::test::SignalPayload{{urn::test::SignalCode::{n}}}; (void)item; }}\n")
                    })
                    .collect::<String>();
                std::fs::write(
                    root.join("probe.cpp"),
                    format!(
                        "#include \"{}\"\nint main() {{\n{uses}}}\n",
                        header.file_name().unwrap().to_string_lossy()
                    ),
                )
                .unwrap();
                let compiled = Command::new("g++")
                    .current_dir(&root)
                    .args([
                        "-std=c++17",
                        "-Wall",
                        "-Wextra",
                        "-Werror",
                        "-pedantic-errors",
                        "probe.cpp",
                        "-o",
                        "probe",
                    ])
                    .output()
                    .unwrap();
                assert!(
                    compiled.status.success(),
                    "C++: {}",
                    String::from_utf8_lossy(&compiled.stderr)
                );
                assert!(Command::new(root.join("probe")).status().unwrap().success());
            }
        }
    }
}
