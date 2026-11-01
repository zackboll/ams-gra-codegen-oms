//! PY-013 production CLI, atomic rejection and independent executable oracles.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-hexbinary")
}

fn run(schema: &Path, world: &str, codec: bool, dest: Option<&Path>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    cmd.arg(if dest.is_some() {
        "service-generate"
    } else {
        "service-check"
    })
    .arg("--schema")
    .arg(schema)
    .arg("--contract")
    .arg(fixture().join("service.yaml"))
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
fn checked_binary_models_codecs_and_qualified_paths_execute() {
    let temp = std::env::temp_dir().join(format!("py013-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    for namespace in [None, Some("urn:py013")] {
        let schema = temp.join("schema.xsd");
        let text = namespace.map_or_else(
            || original.clone(),
            |ns| {
                original
                    .replace("https://www.vdl.afrl.af.mil/programs/oam", ns)
                    .replace(
                        "xmlns:t=",
                        "xmlns:oam=\"https://www.vdl.afrl.af.mil/programs/oam\" xmlns:t=",
                    )
                    .replace("t:version=", "oam:version=")
            },
        );
        fs::write(&schema, text).unwrap();
        let mut previous = None;
        for world in ["closed-schema", "open-extensions"] {
            let mut models = None;
            for codec in [false, true] {
                let dest = temp.join(
                    format!("generated_{}_{world}_{codec}", namespace.is_some()).replace('-', "_"),
                );
                let checked = run(&schema, world, codec, None);
                assert!(checked.status.success(), "{checked:?}");
                let generated = run(&schema, world, codec, Some(&dest));
                assert!(generated.status.success(), "{generated:?}");
                let mut names = vec!["model.py", "service_api.py"];
                if codec {
                    names.push("service_codec.py");
                }
                let sources: Vec<_> = names
                    .iter()
                    .map(|name| fs::read(dest.join(name)).unwrap())
                    .collect();
                if codec {
                    assert_eq!(models.as_ref().unwrap(), &sources[..2]);
                    if let Some(previous) = &previous {
                        assert_eq!(previous, &sources);
                    }
                    previous = Some(sources.clone());
                } else {
                    models = Some(sources.clone());
                }
                assert!(run(&schema, world, codec, Some(&dest)).status.success());
                assert_eq!(
                    sources,
                    names
                        .iter()
                        .map(|name| fs::read(dest.join(name)).unwrap())
                        .collect::<Vec<_>>()
                );
                for optimized in [false, true] {
                    for style in ["standalone", "package"] {
                        let mut cmd = Command::new(
                            std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()),
                        );
                        if optimized {
                            cmd.arg("-O");
                        }
                        cmd.arg(fixture().join("probe.py"))
                            .arg(&dest)
                            .arg(style)
                            .arg(if codec { "codec" } else { "model" });
                        if let Some(namespace) = namespace {
                            cmd.arg(namespace);
                        }
                        let result = cmd.output().unwrap();
                        assert!(result.status.success(), "{result:?}");
                        assert!(String::from_utf8_lossy(&result.stderr).contains("Ran 6 tests"));
                    }
                }
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn unsupported_facets_and_shadowing_fail_atomically() {
    let temp = std::env::temp_dir().join(format!("py013-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    let cases = [
        original.replace("<xs:restriction base=\"xs:hexBinary\"/>", "<xs:restriction base=\"xs:hexBinary\"><xs:pattern value=\"[A-F]*\"/></xs:restriction>"),
        original.replace("<xs:element name=\"Required\" type=\"xs:hexBinary\"/>", "<xs:element name=\"Required\"><xs:simpleType><xs:restriction base=\"xs:hexBinary\"><xs:length value=\"2\"/></xs:restriction></xs:simpleType></xs:element>"),
        original.replace("name=\"HexBinaryType\"", "name=\"bytes\"").replace("t:HexBinaryType", "t:bytes"),
        original.replace("name=\"Optional\"", "name=\"bytes\""),
    ];
    for (index, text) in cases.iter().enumerate() {
        let schema = temp.join(format!("negative-{index}.xsd"));
        fs::write(&schema, text).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            for codec in [false, true] {
                assert!(!run(&schema, world, codec, None).status.success());
                let dest = temp.join(format!("dest-{index}-{world}-{codec}"));
                assert!(!run(&schema, world, codec, Some(&dest)).status.success());
                assert!(!dest.exists());
                fs::create_dir(&dest).unwrap();
                fs::write(dest.join("sentinel"), b"unchanged").unwrap();
                assert!(!run(&schema, world, codec, Some(&dest)).status.success());
                assert_eq!(fs::read(dest.join("sentinel")).unwrap(), b"unchanged");
                assert_eq!(fs::read_dir(dest).unwrap().count(), 1);
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
