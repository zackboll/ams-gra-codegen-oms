//! Task 056: readiness / generation parity over generated-support types.
//!
//! Every fixture here is synthetic and local (Fast CI). Each case makes one
//! readiness boundary observable on its own, and pins that `service-check`
//! and `service-generate` agree on it:
//!
//! | Case | Boundary                                   | Verdict   |
//! | ---- | ------------------------------------------ | --------- |
//! | A    | selected unsupported type                  | NOT READY |
//! | B    | selected OK, generated support unsupported | NOT READY |
//! | C    | selected + generated support all supported | READY     |
//! | D    | only an unrelated declaration unsupported  | READY     |
//! | E    | global backend preflight (multi-namespace) | NOT READY |
//! | F    | service API wrapper name collision         | NOT READY |
//!
//! B is the Task 056 defect: before it, readiness said READY and the backend
//! then failed on the support-only declaration.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LANGUAGES: [&str; 3] = ["ada", "rust", "cpp"];

fn fixtures(directory: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(directory)
}

fn generate_fixture(name: &str) -> PathBuf {
    fixtures("service-generate").join(name)
}

fn output_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ams-gra-oms-task056-{label}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn run(
    command: &str,
    schema: &Path,
    contract: &Path,
    language: &str,
    extra: &[&str],
    output: Option<&Path>,
) -> Output {
    let mut cli = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    cli.arg(command)
        .arg("--schema")
        .arg(schema)
        .arg("--contract")
        .arg(contract)
        .args(["--language", language, "--world", "closed-schema"])
        .args(extra);
    if let Some(output) = output {
        cli.arg("--output").arg(output);
    }
    cli.output().expect("CLI should run")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 report")
}

/// The parity contract every case asserts: `service-check` and
/// `service-generate` agree; a NOT READY `service-generate` prints the exact
/// `service-check` report, never reaches the backend, and writes nothing (not
/// even the directory); a READY one generates.
///
/// Returns the report and, when READY, the populated output root.
fn assert_parity(
    label: &str,
    schema: &Path,
    contract: &Path,
    language: &str,
    ready: bool,
) -> (String, PathBuf) {
    let check = run("service-check", schema, contract, language, &[], None);
    let report = stdout_of(&check);
    let root = output_dir(&format!("{label}-{language}"));
    let generated = run(
        "service-generate",
        schema,
        contract,
        language,
        &[],
        Some(&root),
    );
    let stderr = String::from_utf8_lossy(&generated.stderr).into_owned();
    if ready {
        assert_eq!(check.status.code(), Some(0), "{label} {language}: {report}");
        assert!(report.contains("status: READY\n"), "{label} {language}");
        assert_eq!(
            generated.status.code(),
            Some(0),
            "{label} {language}: READY must generate: {stderr}"
        );
        assert!(root.exists(), "{label} {language}");
    } else {
        assert_eq!(check.status.code(), Some(1), "{label} {language}: {report}");
        assert!(report.contains("status: NOT READY\n"), "{label} {language}");
        assert_eq!(generated.status.code(), Some(1), "{label} {language}");
        assert_eq!(stdout_of(&generated), report, "{label} {language}");
        assert!(stderr.contains("no files were generated"), "{stderr}");
        // Readiness stopped it: no backend "unsupported ... IR construct".
        assert!(
            !stderr.contains("IR construct"),
            "{label} {language}: {stderr}"
        );
        assert!(!root.exists(), "{label} {language}: nothing may be written");
    }
    (report, root)
}

fn support_schema() -> PathBuf {
    generate_fixture("support-readiness.xsd")
}

/// Case B (the Task 056 defect). The contract selects only `Holder` + `Base`,
/// both renderable; the generated-support descendant `ConcreteBad` reaches
/// `BadDuration` (xs:duration), which no backend renders.
#[test]
fn task056_b_support_only_unsupported_type_is_not_ready_and_writes_nothing() {
    for language in LANGUAGES {
        let (report, _) = assert_parity(
            "b-support",
            &support_schema(),
            &generate_fixture("support-blocked.yaml"),
            language,
            false,
        );
        let expected = format!(
            "service contract valid\n\
             service: Support Readiness HolderReport\n\
             language: {language}\n\
             generation world: closed-schema\n\
             \n\
             selected oms messages: 1\n\
             renderable selected oms messages: 1\n\
             selected type closure: 2\n\
             renderable selected types: 2\n\
             generated support types: 3\n\
             renderable generated support types: 2\n\
             status: NOT READY\n\
             \n\
             unsupported generated support types:\n  \
             {{urn:test}}BadDuration\n"
        );
        assert_eq!(report, expected, "{language}");
        // Never mislabelled as selected; no duplicated wrapper diagnostic.
        assert!(!report.contains("unsupported selected types:"));
        assert!(!report.contains("blocked selected messages:"));
        assert!(!report.contains("service api boundary:"));
    }
}

/// Case A: the unsupported declaration is SELECTED. Ordinary pre-Task-056
/// NOT READY; no support section is invented (the selection has none).
#[test]
fn task056_a_selected_unsupported_type_is_not_ready() {
    for language in LANGUAGES {
        let (report, _) = assert_parity(
            "a-selected",
            &support_schema(),
            &generate_fixture("support-selected-unsupported.yaml"),
            language,
            false,
        );
        assert!(report.contains("unsupported selected types:\n  {urn:test}BadDuration\n"));
        assert!(
            report.contains("    blocker: {urn:test}BadDuration\n"),
            "{report}"
        );
        assert!(!report.contains("generated support types"), "{report}");
    }
}

/// Case C: an abstract value whose every support descendant renders, beside
/// unselected unsupported declarations (`BadDuration`, `UnrelatedDuration`,
/// `ConcreteBad`). READY, generates, compiles, and the output omits them.
#[test]
fn task056_c_supported_support_closure_is_ready_and_generates() {
    for language in LANGUAGES {
        let (report, root) = assert_parity(
            "c-ready",
            &support_schema(),
            &generate_fixture("support-ready.yaml"),
            language,
            true,
        );
        assert!(report.contains("selected type closure: 2\n"), "{report}");
        assert!(report.contains("generated support types: 2\n"), "{report}");
        assert!(report.contains("renderable generated support types: 2\n"));
        let combined = collect(&root);
        for present in ["GoodConcreteA", "GoodConcreteB"] {
            assert!(combined.contains(present), "{language} {present}");
        }
        for absent in [
            "BadDuration",
            "UnrelatedDuration",
            "ConcreteBad",
            "SelectedBad",
        ] {
            assert!(!combined.contains(absent), "{language} leaked {absent}");
        }
        compile(language, &root);
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Case D: a concrete-only selection whose schema set contains unrelated
/// unsupported declarations. READY; the support lines are omitted entirely
/// (zero support), keeping the pre-Task-056 report byte-for-byte.
#[test]
fn task056_d_unrelated_unsupported_declaration_stays_ready() {
    for language in LANGUAGES {
        let (report, root) = assert_parity(
            "d-unrelated",
            &generate_fixture("root.xsd"),
            &generate_fixture("only-a.yaml"),
            language,
            true,
        );
        assert!(!report.contains("generated support"), "{report}");
        assert!(!report.contains("UnrelatedDuration"), "{report}");
        assert!(!collect(&root).contains("UnrelatedDuration"), "{language}");
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Case E: every declaration renders, but the projected selection spans two
/// namespaces -- a backend-GLOBAL precondition, reported separately from any
/// per-declaration or support blocker.
#[test]
fn task056_e_global_backend_preflight_stays_a_separate_boundary() {
    for language in LANGUAGES {
        let (report, _) = assert_parity(
            "e-global",
            &fixtures("service-check").join("multi-namespace.xsd"),
            &fixtures("service-check").join("multi-namespace-spanning.yaml"),
            language,
            false,
        );
        assert!(report.contains("backend boundary:"), "{report}");
        assert!(!report.contains("unsupported generated support types"));
    }
}

/// Case F: a service API wrapper name collision, reported on its own line.
#[test]
fn task056_f_service_api_wrapper_boundary_stays_separate() {
    for language in LANGUAGES {
        let (report, _) = assert_parity(
            "f-api",
            &generate_fixture("root.xsd"),
            &generate_fixture("api-function-collision.yaml"),
            language,
            false,
        );
        assert!(report.contains("service api boundary:"), "{report}");
        assert!(!report.contains("unsupported generated support types"));
    }
}

/// Section 14: a model blocked solely by generated support is NOT READY, so
/// `--with-codec` does not measure the codec at all.
#[test]
fn task056_support_blocked_model_leaves_codec_unmeasured() {
    for language in LANGUAGES {
        let output = run(
            "service-check",
            &support_schema(),
            &generate_fixture("support-blocked.yaml"),
            language,
            &["--with-codec"],
            None,
        );
        let report = stdout_of(&output);
        assert_eq!(output.status.code(), Some(1), "{report}");
        assert!(report.contains("unsupported generated support types:\n"));
        assert!(
            report.ends_with("codec: not measured (selected model is NOT READY)\n"),
            "{language}: {report}"
        );
    }
}

/// Sections 10/18/37: the Task 032 abstract fixture stays READY with its
/// support counted separately, and still generates compilable output.
/// `AbstractMiddle` is generated support retained only for inheritance and is
/// not condemned for being abstract.
#[test]
fn task056_abstract_support_fixture_stays_ready_and_compiles() {
    let schema = generate_fixture("abstract.xsd");
    let contract = generate_fixture("abstract.yaml");
    for language in LANGUAGES {
        let (report, root) = assert_parity("abstract", &schema, &contract, language, true);
        for line in [
            "selected oms messages: 1\n",
            "renderable selected oms messages: 1\n",
            "selected type closure: 2\n",
            "renderable selected types: 2\n",
            "generated support types: 8\n",
            "renderable generated support types: 8\n",
        ] {
            assert!(report.contains(line), "{language}: {line:?} in\n{report}");
        }
        compile(language, &root);
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Section 37: the projection membership of the Task 032 abstract fixture is
/// exactly what it was; Task 056 changes readiness accounting only.
#[test]
fn task056_abstract_fixture_projection_membership_is_unchanged() {
    use ams_gra_oms_codegen_core::{
        GenerationWorld, project_service_generation_schema, resolve_service_plan,
    };
    let schema = ams_gra_oms_xsd_frontend::load_schema_set_with_overlays(
        &generate_fixture("abstract.xsd"),
        &[],
    )
    .expect("schema");
    let contract = ams_gra_oms_service_contract::load_contract(&generate_fixture("abstract.yaml"))
        .expect("contract");
    let plan = resolve_service_plan(&contract, &schema).expect("plan");
    let projection =
        project_service_generation_schema(&plan, &schema, GenerationWorld::ClosedSchemaSet)
            .expect("projection");
    let names = |list: &[ams_gra_oms_ir::QualifiedName]| {
        list.iter()
            .map(|name| name.local_name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(projection.selected_type_names()), ["Base", "Holder"]);
    assert_eq!(
        names(projection.generated_support_type_names()),
        [
            "NestedNote",
            "BaseB",
            "ConcreteB",
            "ConcreteA",
            "ConcreteParent",
            "ConcreteLeaf",
            "AbstractMiddle",
            "MiddleLeaf",
        ]
    );
}

/// Every generated file's contents, in sorted path order.
fn collect(root: &Path) -> String {
    let mut paths = std::fs::read_dir(root)
        .expect("generated output")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("UTF-8 source"))
        .collect()
}

/// Host-language compile of the generated model and wrapper.
fn compile(language: &str, root: &Path) {
    let mut command = match language {
        "rust" => {
            let mut command = Command::new("rustc");
            command.args([
                "--edition",
                "2021",
                "--crate-type",
                "lib",
                "--deny",
                "warnings",
                "test.rs",
            ]);
            command
        }
        "cpp" => {
            if Command::new("c++").arg("--version").output().is_err() {
                return;
            }
            std::fs::write(
                root.join("probe.cpp"),
                "#include \"test.hpp\"\n#include \"service_api.hpp\"\nint main() { return 0; }\n",
            )
            .expect("write C++ probe");
            let mut command = Command::new("c++");
            command.args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic-errors",
                "-fsyntax-only",
                "probe.cpp",
            ]);
            command
        }
        _ => {
            if Command::new("gnatmake").arg("--version").output().is_err() {
                assert!(
                    std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                    "GNAT required"
                );
                return;
            }
            let mut command = Command::new("gnatmake");
            command.args([
                "-q",
                "-gnat2012",
                "-gnatwa",
                "-gnatc",
                "urn-test.ads",
                "service_api.ads",
            ]);
            command
        }
    };
    let output = command
        .current_dir(root)
        .output()
        .expect("host compiler runs");
    assert!(
        output.status.success(),
        "{language} generated output must compile: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
