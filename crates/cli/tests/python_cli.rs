//! Public-binary qualification: model support must never imply service readiness.
use ams_gra_oms_backend_python::PythonBackend;
use ams_gra_oms_codegen_core::{Backend, BackendLanguage, GenerationWorld};
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const WORLDS: [(&str, GenerationWorld); 2] = [
    ("closed-schema", GenerationWorld::ClosedSchemaSet),
    ("open-extensions", GenerationWorld::OpenExtensions),
];

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
}

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "py002-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn generate(schema: &Path, language: &OsStr, world: &OsStr, output: &Path) -> Output {
    cli()
        .arg("generate")
        .arg("--schema")
        .arg(schema)
        .arg("--language")
        .arg(language)
        .arg("--world")
        .arg(world)
        .arg("--output")
        .arg(output)
        .output()
        .expect("public CLI must execute")
}

fn failed(output: &Output, code: i32, diagnostic: &str) {
    assert_eq!(output.status.code(), Some(code), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(diagnostic),
        "{output:?}"
    );
}

#[test]
fn track_generates_deterministically_and_executes_in_both_worlds() {
    let temp = Temp::new();
    let schema_path = fixture("../xsd-frontend/tests/fixtures/track.xsd");
    let schema = load_schema_set_with_overlays(&schema_path, &[]).unwrap();
    let mut previous_world = None;
    for (world, policy) in WORLDS {
        let dest = temp.0.join(world);
        let run = || generate(&schema_path, OsStr::new("python"), OsStr::new(world), &dest);
        let first = run();
        assert!(first.status.success(), "{first:?}");
        assert!(first.stderr.is_empty());
        assert!(String::from_utf8_lossy(&first.stdout).contains("generated 1 file(s)"));
        assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
        let contents = fs::read(dest.join("model.py")).unwrap();
        // The CLI must forward the exact world to the existing backend, not
        // duplicate rendering or substitute another world's readiness.
        let expected = PythonBackend.generate(&schema, policy).unwrap();
        assert_eq!(contents, expected[0].contents.as_bytes());
        let second = run();
        assert!(second.status.success(), "{second:?}");
        assert_eq!(first.stdout, second.stdout);
        assert_eq!(first.stderr, second.stderr);
        assert_eq!(contents, fs::read(dest.join("model.py")).unwrap());
        let other_dest = temp.0.join(format!("{world}-repeat"));
        let repeated = generate(
            &schema_path,
            OsStr::new("python"),
            OsStr::new(world),
            &other_dest,
        );
        assert!(repeated.status.success(), "{repeated:?}");
        assert_eq!(contents, fs::read(other_dest.join("model.py")).unwrap());
        if let Some(previous) = previous_world.replace(contents.clone()) {
            // track has no abstract values: the worlds must not invent a
            // representational difference where their policies agree.
            assert_eq!(previous, contents);
        }

        // Reuse the backend's full track runtime corpus, but import the file
        // produced by the PUBLIC binary. No pip/runtime package is involved.
        fs::write(dest.join("probe.py"), format!(
            "import sys\nassert sys.version_info >= (3, 11)\nimport py_compile\npy_compile.compile('model.py', doraise=True)\nimport model\nfrom model import *\nimport typing\nfor value in vars(model).values():\n    if isinstance(value, type) and value.__module__ == 'model':\n        typing.get_type_hints(value)\n{}",
            include_str!("../../backend-python/tests/track_runtime.py")
        )).unwrap();
        for optimized in [false, true] {
            let mut python =
                Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
            python.arg("-I");
            if optimized {
                python.arg("-O");
            }
            let output = python.args(["-c", "import runpy, sys; sys.path.insert(0, '.'); runpy.run_path('probe.py', run_name='__main__')"])
                .current_dir(&dest).output().expect("Python 3.11+ is required, never skipped");
            println!(
                "{world}, optimized={optimized}:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.status.success(), "{output:?}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("Ran 4 tests"));
        }
    }
}

#[test]
fn unsupported_schemas_fail_before_creating_or_changing_any_files() {
    let temp = Temp::new();
    for (world, _) in WORLDS {
        for (path, diagnostic) in [
            (
                "../backend-python/tests/fixtures/unsupported-choice.xsd",
                "Choice / List",
            ),
            (
                "../backend-python/tests/fixtures/unsupported-nillable.xsd",
                "nillable field",
            ),
            (
                "../backend-python/tests/fixtures/unsupported-pattern.xsd",
                "unsupported constraints",
            ),
            (
                "../xsd-frontend/tests/fixtures/backend-abstract-value-reference.xsd",
                if world == "closed-schema" {
                    "abstract"
                } else {
                    "open"
                },
            ),
        ] {
            let schema = fixture(path);
            // Prove this is valid frontend input, rejected by generation, not
            // a malformed fixture that accidentally makes a negative pass.
            let valid = cli()
                .arg("validate")
                .arg("--schema")
                .arg(&schema)
                .output()
                .unwrap();
            assert!(valid.status.success(), "{valid:?}");
            let dest = temp.0.join("absent");
            failed(
                &generate(&schema, OsStr::new("python"), OsStr::new(world), &dest),
                1,
                diagnostic,
            );
            assert!(!dest.exists());

            fs::create_dir(&dest).unwrap();
            fs::write(dest.join("sentinel"), b"do not change").unwrap();
            fs::write(dest.join("model.py"), b"existing model").unwrap();
            failed(
                &generate(&schema, OsStr::new("python"), OsStr::new(world), &dest),
                1,
                diagnostic,
            );
            assert_eq!(fs::read_dir(&dest).unwrap().count(), 2);
            assert_eq!(fs::read(dest.join("sentinel")).unwrap(), b"do not change");
            assert_eq!(fs::read(dest.join("model.py")).unwrap(), b"existing model");
            fs::remove_dir_all(dest).unwrap();
        }
    }
}

#[test]
fn invalid_language_and_world_are_usage_errors_without_output() {
    let temp = Temp::new();
    let dest = temp.0.join("absent");
    let schema = fixture("../xsd-frontend/tests/fixtures/track.xsd");
    for language in ["java", "Python", "py", ""] {
        failed(
            &generate(
                &schema,
                OsStr::new(language),
                OsStr::new("closed-schema"),
                &dest,
            ),
            2,
            "unsupported language",
        );
    }
    for world in ["closed", "open", "CLOSED-SCHEMA", ""] {
        failed(
            &generate(&schema, OsStr::new("python"), OsStr::new(world), &dest),
            2,
            "unsupported generation world",
        );
    }
    let missing = cli()
        .arg("generate")
        .arg("--schema")
        .arg(&schema)
        .args(["--language", "python", "--output"])
        .arg(&dest)
        .output()
        .unwrap();
    failed(&missing, 2, "missing required option '--world'");
    assert!(!dest.exists());
}

#[test]
fn unsupported_python_selected_models_fail_without_writing() {
    let temp = Temp::new();
    for (world, _) in WORLDS {
        for command in ["service-check", "service-generate"] {
            for codec in [false, true] {
                for contract in [
                    "../../tests/fixtures/service-check/ready.yaml",
                    "missing-contract.yaml",
                ] {
                    let dest = temp.0.join("absent");
                    let invoke = || {
                        let mut cmd = cli();
                        cmd.arg(command)
                            .arg("--schema")
                            .arg(fixture("../../tests/fixtures/service-check/root.xsd"))
                            .arg("--contract")
                            .arg(fixture(contract))
                            .args(["--language", "python", "--world", world]);
                        if command == "service-generate" {
                            cmd.arg("--output").arg(&dest);
                        }
                        if codec {
                            cmd.arg("--with-codec");
                        }
                        cmd.output().unwrap()
                    };
                    let output = invoke();
                    assert_eq!(output.status.code(), Some(1), "{output:?}");
                    if !contract.starts_with("missing") {
                        assert!(
                            String::from_utf8_lossy(&output.stdout).contains("status: NOT READY")
                        );
                        assert!(String::from_utf8_lossy(&output.stdout).contains("Float64"));
                    }
                    assert!(!dest.exists());
                    fs::create_dir(&dest).unwrap();
                    fs::write(dest.join("model.py"), b"existing model").unwrap();
                    assert_eq!(invoke().status.code(), Some(1));
                    assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
                    assert_eq!(fs::read(dest.join("model.py")).unwrap(), b"existing model");
                    fs::remove_dir_all(dest).unwrap();
                }
            }
        }
    }
}

#[test]
fn help_separates_python_models_from_service_support_and_coverage() {
    for args in [vec!["--help"], vec!["generate", "--help"]] {
        let output = cli().args(args).output().unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("python"));
    }
    for command in ["service-check", "service-generate"] {
        let output = cli().args([command, "--help"]).output().unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Python 3.11+"));
        assert!(String::from_utf8_lossy(&output.stdout).contains("no OMS codec"));
    }
    assert_eq!(
        BackendLanguage::ALL,
        [
            BackendLanguage::Ada,
            BackendLanguage::Rust,
            BackendLanguage::Cpp
        ]
    );
    for (world, _) in WORLDS {
        let output = cli()
            .arg("coverage")
            .arg("--schema")
            .arg(fixture("../xsd-frontend/tests/fixtures/track.xsd"))
            .args(["--world", world])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(
            !String::from_utf8_lossy(&output.stdout)
                .to_lowercase()
                .contains("python")
        );
    }
}
