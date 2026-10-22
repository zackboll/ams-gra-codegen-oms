//! PY-004 actual CLI -> generated Python -> independent wire vector probes.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture(name: &str) -> PathBuf {
    root().join("tests/fixtures/python-codec").join(name)
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "py004-{}-{}",
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

#[test]
fn qualified_oam_and_extension_codecs_execute_independent_vectors() {
    let temp = Temp::new();
    for (schema, namespace) in [
        ("oam.xsd", "https://www.vdl.afrl.af.mil/programs/oam"),
        ("non-oam.xsd", "urn:python:codec"),
    ] {
        let mut previous = None;
        for world in ["closed-schema", "open-extensions"] {
            let dest = temp.0.join(format!(
                "{}_{}",
                schema.replace(['.', '-'], "_"),
                world.replace('-', "_")
            ));
            let plain = run(
                &fixture(schema),
                &fixture("service.yaml"),
                world,
                false,
                Some(&dest),
            );
            assert!(plain.status.success(), "{plain:?}");
            let originals =
                ["model.py", "service_api.py"].map(|name| fs::read(dest.join(name)).unwrap());
            let check = run(
                &fixture(schema),
                &fixture("service.yaml"),
                world,
                true,
                None,
            );
            assert!(check.status.success(), "{check:?}");
            let report = String::from_utf8_lossy(&check.stdout);
            assert!(
                report.contains("model status: READY\nservice api status: READY\nstatus: READY")
            );
            assert!(report.contains("codec status: READY"));
            let generation = run(
                &fixture(schema),
                &fixture("service.yaml"),
                world,
                true,
                Some(&dest),
            );
            assert!(generation.status.success(), "{generation:?}");
            assert!(
                String::from_utf8_lossy(&generation.stdout)
                    .contains("generated service codec files: 1")
            );
            assert_eq!(fs::read_dir(&dest).unwrap().count(), 3);
            for (name, original) in ["model.py", "service_api.py"].into_iter().zip(originals) {
                assert_eq!(fs::read(dest.join(name)).unwrap(), original);
            }
            let source = fs::read(dest.join("service_codec.py")).unwrap();
            assert_eq!(
                String::from_utf8_lossy(&source)
                    .matches("def encode_Payload(")
                    .count(),
                1
            );
            if let Some(previous) = &previous {
                assert_eq!(previous, &source);
            }
            previous = Some(source.clone());
            let again = run(
                &fixture(schema),
                &fixture("service.yaml"),
                world,
                true,
                Some(&dest),
            );
            assert!(again.status.success());
            assert_eq!(generation.stdout, again.stdout);
            assert_eq!(source, fs::read(dest.join("service_codec.py")).unwrap());
            for optimized in [false, true] {
                for style in ["standalone", "package"] {
                    let mut cmd = Command::new(
                        std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()),
                    );
                    if optimized {
                        cmd.arg("-O");
                    }
                    let output = cmd
                        .arg(root().join("crates/backend-python/tests/codec_runtime.py"))
                        .arg(&dest)
                        .arg(style)
                        .arg(namespace)
                        .arg(fixture(""))
                        .output()
                        .expect("Python 3.11+ required, not skippable");
                    assert!(
                        output.status.success(),
                        "{schema} {world} optimized={optimized} style={style}: {output:?}"
                    );
                    eprintln!(
                        "{schema} {world} optimized={optimized} style={style}:\n{}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
        }
    }
}

#[test]
fn shared_rendered_readiness_contains_exact_codec_artifacts_or_no_partial_files() {
    use ams_gra_oms_backend_python::{PythonBackend, generate_service_codec};
    use ams_gra_oms_codegen_core::{
        GenerationWorld, analyze_rendered_service_with_codec, resolve_service_plan,
    };
    use ams_gra_oms_service_contract::load_contract;
    use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
    let temp = Temp::new();
    let unqualified = temp.0.join("unqualified.xsd");
    fs::write(
        &unqualified,
        fs::read_to_string(fixture("oam.xsd")).unwrap().replace(
            "elementFormDefault=\"qualified\"",
            "elementFormDefault=\"unqualified\"",
        ),
    )
    .unwrap();
    for (path, expected) in [(fixture("oam.xsd"), true), (unqualified, false)] {
        let schema = load_schema_set_with_overlays(&path, &[]).unwrap();
        let plan = resolve_service_plan(&load_contract(&fixture("service.yaml")).unwrap(), &schema)
            .unwrap();
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let result = analyze_rendered_service_with_codec(
                &plan,
                &schema,
                world,
                &PythonBackend,
                generate_service_codec,
            )
            .unwrap();
            assert_eq!(result.is_ready(), expected);
            assert!(result.codec_measured);
            assert!(result.model_blocker.is_none());
            assert!(result.service_api_blocker.is_none());
            if expected {
                assert_eq!(result.model_file_count, 1);
                assert_eq!(result.codec_file_count, 1);
                assert_eq!(result.files.len(), 3);
                let dest = temp.0.join(format!("exact-{}", world.label()));
                let generated = run(
                    &path,
                    &fixture("service.yaml"),
                    world.label(),
                    true,
                    Some(&dest),
                );
                assert!(generated.status.success(), "{generated:?}");
                for file in &result.files {
                    assert_eq!(
                        fs::read(dest.join(&file.relative_path)).unwrap(),
                        file.contents.as_bytes()
                    );
                }
            } else {
                assert!(result.codec_blocker.is_some());
                assert!(result.files.is_empty());
            }
        }
    }
}

#[test]
fn unqualified_members_are_codec_only_blockers_without_writes() {
    let temp = Temp::new();
    let source = fs::read_to_string(fixture("oam.xsd")).unwrap();
    for (label, source) in [
        (
            "default",
            source.replace(
                "elementFormDefault=\"qualified\"",
                "elementFormDefault=\"unqualified\"",
            ),
        ),
        (
            "nested",
            source.replace("name=\"Label\"", "name=\"Label\" form=\"unqualified\""),
        ),
    ] {
        let schema = temp.0.join(format!("{label}.xsd"));
        fs::write(&schema, source).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            let plain = run(&schema, &fixture("service.yaml"), world, false, None);
            assert!(plain.status.success(), "{plain:?}");
            let check = run(&schema, &fixture("service.yaml"), world, true, None);
            assert_eq!(check.status.code(), Some(1), "{check:?}");
            let report = String::from_utf8_lossy(&check.stdout);
            assert!(
                report
                    .contains("model status: READY\nservice api status: READY\nstatus: NOT READY")
            );
            assert!(report.contains("codec status: NOT READY"));
            assert!(report.contains("unqualified element member"));
            let dest = temp.0.join(format!("{label}-{world}"));
            let generation = run(&schema, &fixture("service.yaml"), world, true, Some(&dest));
            assert_eq!(generation.status.code(), Some(1));
            assert_eq!(check.stdout, generation.stdout);
            assert!(!dest.exists());
            fs::create_dir(&dest).unwrap();
            for name in ["model.py", "service_api.py", "service_codec.py"] {
                fs::write(dest.join(name), name).unwrap();
            }
            let generation = run(&schema, &fixture("service.yaml"), world, true, Some(&dest));
            assert_eq!(generation.status.code(), Some(1));
            assert_eq!(check.stdout, generation.stdout);
            for name in ["model.py", "service_api.py", "service_codec.py"] {
                assert_eq!(fs::read(dest.join(name)).unwrap(), name.as_bytes());
            }
            assert_eq!(fs::read_dir(dest).unwrap().count(), 3);
        }
    }
}

#[test]
fn name_specific_uuid_mapping_is_not_invented() {
    let temp = Temp::new();
    let schema = temp.0.join("uuid.xsd");
    fs::write(
        &schema,
        fs::read_to_string(fixture("oam.xsd"))
            .unwrap()
            .replace("name=\"Text\"", "name=\"UniversallyUniqueIdentifierType\"")
            .replace("t:Text", "t:UniversallyUniqueIdentifierType"),
    )
    .unwrap();
    let plain = run(
        &schema,
        &fixture("service.yaml"),
        "closed-schema",
        false,
        None,
    );
    assert!(plain.status.success(), "{plain:?}");
    let dest = temp.0.join("absent");
    let blocked = run(
        &schema,
        &fixture("service.yaml"),
        "closed-schema",
        true,
        Some(&dest),
    );
    assert_eq!(blocked.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&blocked.stdout).contains("RFC 4122 mapping is unsupported"));
    assert!(!dest.exists());
}

#[test]
fn all_model_boundaries_still_fail_closed_with_codec_requested() {
    let temp = Temp::new();
    let source = fs::read_to_string(fixture("oam.xsd")).unwrap();
    for primitive in [
        "float",
        "double",
        "dateTime",
        "duration",
        "time",
        "decimal",
        "hexBinary",
    ] {
        let schema = temp.0.join(format!("{primitive}.xsd"));
        fs::write(
            &schema,
            source.replace(
                "type=\"xs:string\" minOccurs=\"0\"",
                &format!("type=\"xs:{primitive}\" minOccurs=\"0\""),
            ),
        )
        .unwrap();
        for world in ["closed-schema", "open-extensions"] {
            let dest = temp.0.join(format!("absent-{primitive}-{world}"));
            let blocked = run(&schema, &fixture("service.yaml"), world, true, Some(&dest));
            assert_eq!(blocked.status.code(), Some(1), "{blocked:?}");
            let text = String::from_utf8_lossy(&blocked.stdout);
            if primitive == "decimal" {
                // The established frontend rejects decimal before normalized
                // backend readiness. Do not relax that independent boundary.
                assert!(text.is_empty());
                assert!(String::from_utf8_lossy(&blocked.stderr).contains("xs:decimal"));
            } else {
                assert!(text.contains("model status: NOT READY"), "{blocked:?}");
                assert!(text.contains("codec: not measured"));
            }
            assert!(!dest.exists());
        }
    }
    for (label, source) in [
        (
            "nil",
            source.replace("name=\"Note\"", "name=\"Note\" nillable=\"true\""),
        ),
        (
            "unbounded",
            source.replace("maxOccurs=\"3\"", "maxOccurs=\"unbounded\""),
        ),
        (
            "abstract",
            source.replace("name=\"Payload\"", "name=\"Payload\" abstract=\"true\""),
        ),
        (
            "facet",
            source.replace(
                "<xs:restriction base=\"xs:string\"/>",
                "<xs:restriction base=\"xs:string\"><xs:maxLength value=\"8\"/></xs:restriction>",
            ),
        ),
        (
            "choice",
            source.replace("type=\"t:Payload\"", "type=\"t:UnusedUnsupported\""),
        ),
        (
            "inheritance",
            source.replace("<xs:complexType name=\"Empty\"><xs:sequence/></xs:complexType>",
                "<xs:complexType name=\"Empty\"><xs:complexContent><xs:extension base=\"t:Child\"><xs:sequence/></xs:extension></xs:complexContent></xs:complexType>"),
        ),
    ] {
        let schema = temp.0.join(format!("{label}.xsd"));
        fs::write(&schema, source).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            let dest = temp.0.join(format!("absent-{label}-{world}"));
            let blocked = run(&schema, &fixture("service.yaml"), world, true, Some(&dest));
            assert_eq!(blocked.status.code(), Some(1), "{blocked:?}");
            assert!(
                String::from_utf8_lossy(&blocked.stdout).contains("status: NOT READY"),
                "{blocked:?}"
            );
            assert!(!dest.exists());
        }
    }
}
