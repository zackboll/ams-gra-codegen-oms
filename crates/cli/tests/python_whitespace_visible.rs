//! PY-016 exact shared profile admission and executable runtime qualification.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-whitespace-visible")
}

fn run(schema: &Path, world: &str, codec: bool, dest: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    command
        .arg(if dest.is_some() {
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
        command.arg("--with-codec");
    }
    if let Some(dest) = dest {
        command.arg("--output").arg(dest);
    }
    command.output().unwrap()
}

#[test]
fn models_codecs_worlds_imports_optimization_and_clark_paths() {
    let temp = std::env::temp_dir().join(format!("py016-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    for qualified in [false, true] {
        let schema = temp.join(format!("schema-{qualified}.xsd"));
        let source = if qualified {
            original
                .replace(
                    "https://www.vdl.afrl.af.mil/programs/oam",
                    "urn:python:whitespace",
                )
                .replace(
                    "xmlns:t=",
                    "xmlns:uci=\"https://www.vdl.afrl.af.mil/programs/oam\" xmlns:t=",
                )
                .replace("t:version=", "uci:version=")
        } else {
            original.clone()
        };
        fs::write(&schema, source).unwrap();
        let mut previous = None;
        let mut previous_codec = None;
        for world in ["closed-schema", "open-extensions"] {
            for codec in [false, true] {
                let dest = temp.join(format!("q{qualified}_{}_c{codec}", world.replace('-', "_")));
                let result = run(&schema, world, codec, None);
                assert!(result.status.success(), "{result:?}");
                let result = run(&schema, world, codec, Some(&dest));
                assert!(result.status.success(), "{result:?}");
                let sources =
                    ["model.py", "service_api.py"].map(|name| fs::read(dest.join(name)).unwrap());
                if let Some(previous) = &previous {
                    assert_eq!(previous, &sources);
                }
                previous = Some(sources.clone());
                assert!(run(&schema, world, codec, Some(&dest)).status.success());
                assert_eq!(
                    sources,
                    ["model.py", "service_api.py"].map(|name| fs::read(dest.join(name)).unwrap())
                );
                assert_eq!(dest.join("service_codec.py").exists(), codec);
                if codec {
                    let bytes = fs::read(dest.join("service_codec.py")).unwrap();
                    if let Some(previous) = &previous_codec {
                        assert_eq!(previous, &bytes);
                    }
                    previous_codec = Some(bytes);
                }
                for optimized in [false, true] {
                    for style in ["standalone", "package"] {
                        let mut command = Command::new(
                            std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()),
                        );
                        if optimized {
                            command.arg("-O");
                        }
                        let output = command
                            .arg(fixture().join("probe.py"))
                            .arg(&dest)
                            .arg(style)
                            .arg(if qualified {
                                "urn:python:whitespace"
                            } else {
                                ""
                            })
                            .env("PYTHONDONTWRITEBYTECODE", "1")
                            .output()
                            .unwrap();
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        assert!(output.status.success(), "{output:?}");
                        assert!(
                            stderr.contains("Ran 5 tests") && stderr.contains("\nOK\n"),
                            "{stderr}"
                        );
                    }
                }
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn neighboring_profiles_and_bad_ancestry_reject_atomically() {
    let temp = std::env::temp_dir().join(format!("py016-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    let collapse_short = "<xs:whiteSpace value=\"collapse\"/><xs:minLength value=\"0\"/><xs:maxLength value=\"1024\"/>\n    <xs:pattern value=\"[ -~\\n\\r]{0,1024}\"/>";
    let collapse_long = collapse_short.replace("1024", "4096");
    let cases = [
        (
            "collapse-one-short",
            original.replace(collapse_short, &collapse_short.replace("value=\"0\"", "value=\"1\"").replace("{0,", "{1,")),
        ),
        (
            "collapse-one-long",
            original.replace(&collapse_long, &collapse_long.replace("value=\"0\"", "value=\"1\"").replace("{0,", "{1,")),
        ),
        ("collapse-min-one", original.replace("value=\"0\"", "value=\"1\"").replace("{0,", "{1,")),
        ("preserve-zero-short", original.replace("<xs:whiteSpace value=\"collapse\"/>", "")),
        ("explicit-preserve", original.replace("value=\"collapse\"", "value=\"preserve\"")),
        ("replace", original.replace("value=\"collapse\"", "value=\"replace\"")),
        ("extra", original.replace("<xs:minLength value=\"0\"/>", "<xs:length value=\"0\"/><xs:minLength value=\"0\"/>")),
        ("quantifier", original.replace("{0,1024}", "{1,1024}")),
        ("pattern", original.replace("[ -~\\n\\r]", "[ -~]")),
        ("incorrect-min", original.replace("<xs:minLength value=\"1\"/>", "<xs:minLength value=\"2\"/>")),
        ("named-policy-change", original.replace("base=\"t:FoldShort\"/>", "base=\"t:KeepShort\"><xs:whiteSpace value=\"collapse\"/></xs:restriction>")),
        ("cycle", original.replace("base=\"t:FoldShort\"", "base=\"t:Leaf\"")),
        ("unresolved", original.replace("base=\"t:FoldShort\"", "base=\"t:Missing\"")),
        ("field-local", original.replace("name=\"Required\" type=\"t:FoldShort\"/>", "name=\"Required\"><xs:simpleType><xs:restriction base=\"xs:string\"><xs:whiteSpace value=\"collapse\"/><xs:minLength value=\"0\"/><xs:maxLength value=\"1024\"/><xs:pattern value=\"[ -~\\n\\r]{0,1024}\"/></xs:restriction></xs:simpleType></xs:element>")),
        ("uci-name-invalid", original.replace("FoldShort", "WhitespaceVisibleString1024Type").replace("{0,1024}", "{0,1023}")),
        ("unsafe-name", original.replace("FoldShort", "str")),
    ];
    for (name, source) in cases {
        assert_ne!(source, original, "ineffective mutation {name}");
        let schema = temp.join(format!("{name}.xsd"));
        fs::write(&schema, source).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            for codec in [false, true] {
                let check = run(&schema, world, codec, None);
                assert!(!check.status.success(), "{name}: {check:?}");
                let dest = temp.join(format!("{name}-{world}-{codec}"));
                let result = run(&schema, world, codec, Some(&dest));
                assert!(!result.status.success(), "{name}: {result:?}");
                assert!(!dest.exists(), "{name}: partial output");
                fs::create_dir(&dest).unwrap();
                fs::write(dest.join("sentinel"), b"unchanged").unwrap();
                let result = run(&schema, world, codec, Some(&dest));
                assert!(!result.status.success(), "{name}: {result:?}");
                assert_eq!(fs::read(dest.join("sentinel")).unwrap(), b"unchanged");
                assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
