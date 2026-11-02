//! PY-003 public CLI and actual Python execution; probes must never skip.
use ams_gra_oms_backend_python::PythonBackend;
use ams_gra_oms_codegen_core::{
    GenerationWorld, analyze_rendered_service_readiness, resolve_service_plan,
};
use ams_gra_oms_service_contract::load_contract;
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture(name: &str) -> PathBuf {
    root().join("tests/fixtures/python-service").join(name)
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "py003-{}-{}",
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
fn run(schema: &Path, contract: &Path, world: &str, codec: bool, dest: Option<&Path>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    cmd.arg(if dest.is_some() {
        "service-generate"
    } else {
        "service-check"
    })
    .arg("--schema")
    .arg(schema)
    .arg("--contract")
    .arg(contract)
    .args(["--language", "python", "--world", world]);
    if codec {
        cmd.arg("--with-codec");
    }
    if let Some(dest) = dest {
        cmd.arg("--output").arg(dest);
    }
    cmd.output().unwrap()
}
fn python(args: &[&str], paths: &[&Path]) {
    let output = Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()))
        .args(args)
        .args(paths)
        .output()
        .expect("Python 3.11+ is required");
    assert!(output.status.success(), "{output:?}");
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn supported_selection_is_ready_and_runs_typed_facade_normally_and_optimized() {
    let temp = Temp::new();
    let schema_path = fixture("root.xsd");
    let contract_path = fixture("service.yaml");
    let schema = load_schema_set_with_overlays(&schema_path, &[]).unwrap();
    let plan = resolve_service_plan(&load_contract(&contract_path).unwrap(), &schema).unwrap();
    let mut previous = None;
    for (world, policy) in [
        ("closed-schema", GenerationWorld::ClosedSchemaSet),
        ("open-extensions", GenerationWorld::OpenExtensions),
    ] {
        let check = run(&schema_path, &contract_path, world, false, None);
        assert!(check.status.success(), "{check:?}");
        let report = String::from_utf8_lossy(&check.stdout);
        assert!(report.contains("model status: READY\nservice api status: READY\nstatus: READY"));
        assert!(report.contains("selected oms messages: 2"));
        assert!(report.contains("selected type closure: 3"));
        assert_eq!(
            check.stdout,
            run(&schema_path, &contract_path, world, false, None).stdout
        );
        let dest = temp.0.join(world.replace('-', "_"));
        let generation = run(&schema_path, &contract_path, world, false, Some(&dest));
        assert!(generation.status.success(), "{generation:?}");
        assert_eq!(fs::read_dir(&dest).unwrap().count(), 2);
        let files =
            analyze_rendered_service_readiness(&plan, &schema, policy, &PythonBackend).unwrap();
        assert!(files.is_ready());
        assert_eq!(files.files.len(), 2);
        for file in &files.files {
            assert_eq!(
                fs::read(dest.join(&file.relative_path)).unwrap(),
                file.contents.as_bytes()
            );
        }
        let bytes = (
            fs::read(dest.join("model.py")).unwrap(),
            fs::read(dest.join("service_api.py")).unwrap(),
        );
        let repeat = run(&schema_path, &contract_path, world, false, Some(&dest));
        assert!(repeat.status.success());
        assert_eq!(generation.stdout, repeat.stdout);
        assert_eq!(bytes.0, fs::read(dest.join("model.py")).unwrap());
        assert_eq!(bytes.1, fs::read(dest.join("service_api.py")).unwrap());
        if let Some(previous) = &previous {
            assert_eq!(previous, &bytes);
        }
        previous = Some(bytes);
        let probe = root().join("crates/backend-python/tests/service_runtime.py");
        for optimized in [false, true] {
            for style in ["standalone", "package"] {
                let mut args = Vec::new();
                if optimized {
                    args.push("-O");
                }
                args.push(probe.to_str().unwrap());
                args.push(dest.to_str().unwrap());
                args.push(style);
                python(&args, &[]);
            }
        }
    }
}

fn blocked(schema: &Path, contract: &Path, world: &str, codec: bool, diagnostic: &str) {
    let temp = Temp::new();
    let dest = temp.0.join("absent");
    let check = run(schema, contract, world, codec, None);
    assert_eq!(check.status.code(), Some(1), "{check:?}");
    assert!(
        String::from_utf8_lossy(&check.stdout).contains("status: NOT READY"),
        "{check:?}"
    );
    assert!(
        String::from_utf8_lossy(&check.stdout).contains(diagnostic),
        "{check:?}"
    );
    let generate = run(schema, contract, world, codec, Some(&dest));
    assert_eq!(generate.status.code(), Some(1), "{generate:?}");
    assert_eq!(check.stdout, generate.stdout);
    assert!(!dest.exists());
    fs::create_dir(&dest).unwrap();
    for name in ["model.py", "service_api.py", "service_codec.py"] {
        fs::write(dest.join(name), name).unwrap();
    }
    let again = run(schema, contract, world, codec, Some(&dest));
    assert_eq!(again.status.code(), Some(1));
    assert_eq!(check.stdout, again.stdout);
    assert_eq!(fs::read_dir(&dest).unwrap().count(), 3);
    for name in ["model.py", "service_api.py", "service_codec.py"] {
        assert_eq!(fs::read(dest.join(name)).unwrap(), name.as_bytes());
    }
}

#[test]
fn unsupported_selected_types_and_oms_codecs_fail_atomically() {
    let temp = Temp::new();
    let unqualified = temp.0.join("unqualified.xsd");
    fs::write(
        &unqualified,
        fs::read_to_string(fixture("root.xsd")).unwrap().replace(
            "elementFormDefault=\"qualified\"",
            "elementFormDefault=\"unqualified\"",
        ),
    )
    .unwrap();
    let unsupported = temp.0.join("unsupported.yaml");
    fs::write(
        &unsupported,
        fs::read_to_string(fixture("service.yaml"))
            .unwrap()
            .replace("SelectedReport", "UnsupportedReport"),
    )
    .unwrap();
    let unsupported_schema = temp.0.join("unsupported-choice.xsd");
    fs::write(
        &unsupported_schema,
        fs::read_to_string(fixture("root.xsd")).unwrap().replace(
            "name=\"Text\" type=\"xs:string\"",
            "name=\"Text\" type=\"xs:string\" minOccurs=\"0\"",
        ),
    )
    .unwrap();
    for world in ["closed-schema", "open-extensions"] {
        blocked(&unsupported_schema, &unsupported, world, false, "Choice");
        blocked(
            &unqualified,
            &fixture("service.yaml"),
            world,
            true,
            "unqualified element member",
        );
    }
}

#[test]
fn normalized_function_exchange_and_model_collisions_fail_before_output() {
    let temp = Temp::new();
    let original = fs::read_to_string(fixture("service.yaml")).unwrap();
    for (label, contract) in [
        (
            "function",
            original
                .replace("id: class", "id: track-name")
                .replace("id: final", "id: track_name"),
        ),
        (
            "exchange",
            original
                .replace("id: repeated", "id: track-name")
                .replace("id: same-payload", "id: track_name"),
        ),
    ] {
        let path = temp.0.join(format!("{label}.yaml"));
        fs::write(&path, contract).unwrap();
        blocked(
            &fixture("root.xsd"),
            &path,
            "closed-schema",
            false,
            "Python service API name collision",
        );
    }
    let schema = temp.0.join("collision.xsd");
    fs::write(
        &schema,
        fs::read_to_string(fixture("root.xsd"))
            .unwrap()
            .replace("Identifier", "a-b")
            .replace("State", "a_b"),
    )
    .unwrap();
    blocked(
        &schema,
        &fixture("service.yaml"),
        "closed-schema",
        false,
        "Python name collision",
    );
}

#[test]
fn zero_oms_needs_neither_model_payload_facade_nor_codec() {
    let temp = Temp::new();
    let contract = root().join("tests/fixtures/service-generate/non-uci.yaml");
    for world in ["closed-schema", "open-extensions"] {
        let mut previous = None;
        for codec in [false, true] {
            let dest = temp.0.join(format!("{world}-{codec}"));
            let check = run(&fixture("root.xsd"), &contract, world, codec, None);
            assert!(check.status.success(), "{check:?}");
            let generated = run(&fixture("root.xsd"), &contract, world, codec, Some(&dest));
            assert!(generated.status.success(), "{generated:?}");
            assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
            let source = fs::read_to_string(dest.join("service_api.py")).unwrap();
            for absent in [
                "Payload",
                "import model",
                "PublishAdapter",
                "SubscribeAdapter",
            ] {
                assert!(!source.contains(absent));
            }
            if let Some(previous) = &previous {
                assert_eq!(previous, &source);
            }
            previous = Some(source);
            for optimization in [vec![], vec!["-O"]] {
                let code = "import sys,py_compile; from pathlib import Path; p=Path(sys.argv[1]); py_compile.compile(str(p/'service_api.py'),doraise=True); sys.path.insert(0,str(p)); import service_api as a; from unittest import TestCase; t=TestCase(); t.assertEqual(a.FUNCTIONS[0].EXCHANGES[0].KIND,'data_transfer'); t.assertEqual(a.FUNCTIONS[0].EXCHANGES[1].KIND,'special_signal')";
                let mut args = optimization;
                args.extend(["-c", code]);
                python(&args, &[&dest]);
            }
        }
    }
}

#[test]
fn abstract_values_remain_blocked_in_both_worlds() {
    let temp = Temp::new();
    let schema = temp.0.join("abstract.xsd");
    fs::write(
        &schema,
        fs::read_to_string(fixture("root.xsd")).unwrap().replace(
            "<xs:complexType name=\"Payload\">",
            "<xs:complexType name=\"Payload\" abstract=\"true\">",
        ),
    )
    .unwrap();
    for world in ["closed-schema", "open-extensions"] {
        blocked(&schema, &fixture("service.yaml"), world, false, "abstract");
    }
    // A genuine abstract member value, not only an abstract global payload.
    let source = fs::read_to_string(fixture("root.xsd")).unwrap()
        .replace("<xs:complexType name=\"Payload\">", "<xs:complexType name=\"Base\" abstract=\"true\"><xs:sequence><xs:element name=\"Text\" type=\"xs:string\"/></xs:sequence></xs:complexType><xs:complexType name=\"Payload\">")
        .replace("<xs:element name=\"Label\" type=\"xs:string\" minOccurs=\"0\"/>", "<xs:element name=\"Value\" type=\"t:Base\" minOccurs=\"0\"/>");
    fs::write(&schema, source).unwrap();
    blocked(
        &schema,
        &fixture("service.yaml"),
        "open-extensions",
        false,
        "open-extensions",
    );
    blocked(
        &schema,
        &fixture("service.yaml"),
        "closed-schema",
        false,
        "abstract",
    );
}

#[test]
fn selected_subset_boundaries_are_truthful() {
    let temp = Temp::new();
    let original = fs::read_to_string(fixture("root.xsd")).unwrap();
    for (label, source, diagnostic) in [
        (
            "float",
            original.replace(
                "type=\"xs:string\" minOccurs=\"0\"",
                "type=\"xs:float\" minOccurs=\"0\"",
            ),
            "Float32",
        ),
        (
            "nil",
            original.replace("name=\"Label\"", "name=\"Label\" nillable=\"true\""),
            "nillable",
        ),
        (
            "pattern",
            original.replace(
                "<xs:minInclusive value=\"1\"/>",
                "<xs:pattern value=\"[0-9]+\"/><xs:minInclusive value=\"1\"/>",
            ),
            "lexical constraints",
        ),
    ] {
        let schema = temp.0.join(format!("{label}.xsd"));
        fs::write(&schema, source).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            blocked(&schema, &fixture("service.yaml"), world, false, diagnostic);
        }
    }
}

#[test]
fn all_exchange_kinds_preserve_metadata_without_fabricated_payloads() {
    let temp = Temp::new();
    let contract = temp.0.join("all.yaml");
    fs::write(
        &contract,
        fs::read_to_string(root().join("tests/fixtures/service-generate/api-all-kinds.yaml"))
            .unwrap()
            .replace("MessageA", "SelectedReport")
            .replace("MessageB", "OtherReport"),
    )
    .unwrap();
    let dest = temp.0.join("generated");
    let generated = run(
        &fixture("root.xsd"),
        &contract,
        "closed-schema",
        false,
        Some(&dest),
    );
    assert!(generated.status.success(), "{generated:?}");
    let code = "import sys; sys.path.insert(0,sys.argv[1]); import service_api as a; from unittest import TestCase; t=TestCase(); t.assertEqual(a.SERVICE_KIND,'subsystem'); f=a.function_mission_data; t.assertEqual([e.KIND for e in f.EXCHANGES],['oms_message','data_transfer','special_signal','security_exchange','non_oms_message','oms_message']); [t.assertFalse(hasattr(e,'Payload')) for e in f.EXCHANGES[1:5]]; t.assertEqual(a.function_mission_backup.exchange_status_output.KIND,'non_oms_message')";
    for args in [vec!["-c", code], vec!["-O", "-c", code]] {
        python(&args, &[&dest]);
    }
}

#[test]
fn plan_schema_binding_defects_are_errors_not_ready_results() {
    let schema = load_schema_set_with_overlays(&fixture("root.xsd"), &[]).unwrap();
    let contract = load_contract(&fixture("service.yaml")).unwrap();
    let plan = resolve_service_plan(&contract, &schema).unwrap();
    let mut changed = schema.clone();
    changed.types[0].constraints.max_inclusive = Some(ams_gra_oms_ir::NumericValue::Integer(99));
    assert!(
        analyze_rendered_service_readiness(
            &plan,
            &changed,
            GenerationWorld::ClosedSchemaSet,
            &PythonBackend
        )
        .is_err()
    );
}
