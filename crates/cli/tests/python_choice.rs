//! PY-015 exercises the actual CLI admission and executable checked carriers.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-choice")
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
fn choice_models_codecs_worlds_imports_and_optimization() {
    let temp = std::env::temp_dir().join(format!("py015-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let mut previous = None;
    for world in ["closed-schema", "open-extensions"] {
        for codec in [false, true] {
            let dest = temp.join(format!("{}_{}", world.replace('-', "_"), codec));
            let check = run(&fixture().join("oam.xsd"), world, codec, None);
            assert!(check.status.success(), "{check:?}");
            let generated = run(&fixture().join("oam.xsd"), world, codec, Some(&dest));
            assert!(generated.status.success(), "{generated:?}");
            let sources =
                ["model.py", "service_api.py"].map(|name| fs::read(dest.join(name)).unwrap());
            if let Some(previous) = &previous {
                assert_eq!(previous, &sources);
            }
            previous = Some(sources);
            let before = fs::read(dest.join("model.py")).unwrap();
            assert!(
                run(&fixture().join("oam.xsd"), world, codec, Some(&dest))
                    .status
                    .success()
            );
            assert_eq!(before, fs::read(dest.join("model.py")).unwrap());
            assert_eq!(dest.join("service_codec.py").exists(), codec);
            for optimized in [false, true] {
                for style in ["standalone", "package"] {
                    let mut command = Command::new(
                        std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()),
                    );
                    if optimized {
                        command.arg("-O");
                    }
                    let result = command
                        .arg(fixture().join("probe.py"))
                        .arg(&dest)
                        .arg(style)
                        .env("PYTHONDONTWRITEBYTECODE", "1")
                        .output()
                        .unwrap();
                    assert!(
                        result.status.success(),
                        "{world} codec={codec} optimized={optimized} {style}: {result:?}"
                    );
                }
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn unsupported_choices_fail_atomically() {
    let temp = std::env::temp_dir().join(format!("py015-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    let cases = [
        ("optional-alternative", original.replace("name=\"Text\" type=\"xs:string\"", "name=\"Text\" type=\"xs:string\" minOccurs=\"0\"")),
        ("repeated-alternative", original.replace("name=\"Text\" type=\"xs:string\"", "name=\"Text\" type=\"xs:string\" maxOccurs=\"2\"")),
        ("unbounded-alternative", original.replace("name=\"Text\" type=\"xs:string\"", "name=\"Text\" type=\"xs:string\" maxOccurs=\"unbounded\"")),
        ("nillable-alternative", original.replace("name=\"Text\" type=\"xs:string\"", "name=\"Text\" type=\"xs:string\" nillable=\"true\"")),
        ("collision", original.replace("name=\"OtherText\"", "name=\"Text\"")),
        ("normalized-collision", original.replace("name=\"Text\"", "name=\"A-B\"").replace("name=\"OtherText\"", "name=\"A_B\"")),
        ("storage-collision", original.replace("name=\"Text\"", "name=\"value\"")),
        ("type-collision", original.replace("name=\"Text\"", "name=\"Country\"")),
        ("builtin-collision", original.replace("name=\"Text\"", "name=\"staticmethod\"")),
        ("recursive", original.replace("name=\"Text\" type=\"xs:string\"", "name=\"Text\" type=\"t:Selection\"")),
        ("unresolved", original.replace("name=\"Text\" type=\"xs:string\"", "name=\"Text\" type=\"t:Missing\"")),
        ("list", original.replace("<xs:restriction base=\"xs:hexBinary\"/>", "<xs:list itemType=\"xs:string\"/>")),
        ("unsupported", original.replace("name=\"Text\" type=\"xs:string\"", "name=\"Text\" type=\"xs:float\"")),
        ("facets", original.replace("<xs:element name=\"Text\" type=\"xs:string\"/>", "<xs:element name=\"Text\"><xs:simpleType><xs:restriction base=\"xs:string\"><xs:pattern value=\"X.*\"/></xs:restriction></xs:simpleType></xs:element>")),
        ("abstract", original.replace("name=\"Selection\"><xs:choice>", "name=\"Selection\" abstract=\"true\"><xs:choice>")),
        ("mixed-ancestry", original.replace("name=\"Selection\"><xs:choice>", "name=\"Selection\"><xs:complexContent><xs:extension base=\"t:Detail\"><xs:choice>").replace("</xs:choice></xs:complexType>\n  <xs:complexType name=\"Ancestor\"", "</xs:choice></xs:extension></xs:complexContent></xs:complexType>\n  <xs:complexType name=\"Ancestor\"")),
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

#[test]
fn clark_qualified_choice_and_repetition_paths() {
    let temp = std::env::temp_dir().join(format!("py015-clark-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    let schema = temp.join("qualified.xsd");
    fs::write(
        &schema,
        original
            .replace(
                "https://www.vdl.afrl.af.mil/programs/oam",
                "urn:python:choice",
            )
            .replace(
                "xmlns:t=",
                "xmlns:uci=\"https://www.vdl.afrl.af.mil/programs/oam\" xmlns:t=",
            )
            .replace("t:version=", "uci:version="),
    )
    .unwrap();
    let script = "import service_codec as c,unittest\nclass Paths(unittest.TestCase):\n def test_paths(self):\n  text='{\"{urn:python:choice}Required\":{\"{urn:python:choice}Text\":\"x\"},\"{urn:python:choice}Positive\":[{\"{urn:python:choice}GovernmentIdentifier\":\"USA\"}],\"{urn:python:choice}Repeated\":[{\"{urn:python:choice}Number\":true}]}'\n  with self.assertRaises(c.CodecError) as caught:c.decode_Payload(text)\n  self.assertIn('$.{urn:python:choice}Repeated[0].{urn:python:choice}Number:',str(caught.exception))\nunittest.main()";
    for world in ["closed-schema", "open-extensions"] {
        let dest = temp.join(world.replace('-', "_"));
        let result = run(&schema, world, true, Some(&dest));
        assert!(result.status.success(), "{result:?}");
        for optimized in [false, true] {
            let mut command =
                Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
            if optimized {
                command.arg("-O");
            }
            let result = command
                .args(["-c", script])
                .current_dir(&dest)
                .env("PYTHONDONTWRITEBYTECODE", "1")
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
