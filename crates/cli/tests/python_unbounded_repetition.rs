//! PY-012: production CLI selection, atomic generation and executable Python oracles.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
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
    .arg(root().join("tests/fixtures/python-unbounded-repetition/service.yaml"))
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
fn generated_unbounded_models_and_codecs_execute_in_both_worlds() {
    let temp = std::env::temp_dir().join(format!("py012-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-unbounded-repetition");
    let schema = fixture.join("oam.xsd");
    let mut previous = None;
    for world in ["closed-schema", "open-extensions"] {
        let mut model_sources = None;
        for codec in [false, true] {
            let dest = temp.join(format!("{}_{codec}", world.replace('-', "_")));
            let check = run(&schema, world, codec, None);
            assert!(check.status.success(), "{check:?}");
            let generated = run(&schema, world, codec, Some(&dest));
            assert!(generated.status.success(), "{generated:?}");
            let names = if codec {
                vec!["model.py", "service_api.py", "service_codec.py"]
            } else {
                vec!["model.py", "service_api.py"]
            };
            let sources: Vec<_> = names
                .iter()
                .map(|name| fs::read(dest.join(name)).unwrap())
                .collect();
            if codec {
                assert_eq!(model_sources.as_ref().unwrap(), &sources[..2]);
                if let Some(previous) = &previous {
                    assert_eq!(previous, &sources);
                }
                previous = Some(sources.clone());
            } else {
                assert!(!dest.join("service_codec.py").exists());
                model_sources = Some(sources.clone());
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
                    let result = cmd
                        .arg(fixture.join("probe.py"))
                        .arg(&dest)
                        .arg(style)
                        .arg(if codec { "codec" } else { "model" })
                        .output()
                        .unwrap();
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!(
                        "PY012 world={world} codec={codec} optimized={optimized} style={style}\n{stderr}"
                    );
                    assert!(result.status.success(), "{result:?}");
                    assert!(
                        stderr.contains(if codec { "Ran 18 tests" } else { "Ran 9 tests" })
                            && stderr.contains("\nOK\n"),
                        "{stderr}"
                    );
                }
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn unsupported_repeated_elements_fail_atomically() {
    let temp = std::env::temp_dir().join(format!("py012-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original =
        fs::read_to_string(root().join("tests/fixtures/python-unbounded-repetition/oam.xsd"))
            .unwrap();
    let cases = [
        ("binary-pattern", original.replace("<xs:element name=\"Strings\" type=\"xs:string\" minOccurs=\"0\" maxOccurs=\"unbounded\"/>", "<xs:element name=\"Strings\" minOccurs=\"0\" maxOccurs=\"unbounded\"><xs:simpleType><xs:restriction base=\"xs:hexBinary\"><xs:pattern value=\"[A-F]*\"/></xs:restriction></xs:simpleType></xs:element>")),
        ("nillable", original.replace("name=\"Strings\"", "name=\"Strings\" nillable=\"true\"")),
        ("abstract", original.replace("name=\"Groups\" type=\"t:Group\"", "name=\"Groups\" type=\"t:Ancestor\"")),
        ("choice", original.replace("<xs:complexType name=\"Group\"><xs:sequence>", "<xs:complexType name=\"Group\"><xs:choice>").replace("</xs:sequence></xs:complexType>\n  <xs:complexType name=\"Payload\">", "</xs:choice></xs:complexType>\n  <xs:complexType name=\"Payload\">")),
        ("list", original.replace("<xs:element name=\"Strings\" type=\"xs:string\" minOccurs=\"0\" maxOccurs=\"unbounded\"/>", "<xs:element name=\"Strings\" minOccurs=\"0\" maxOccurs=\"unbounded\"><xs:simpleType><xs:list itemType=\"xs:string\"/></xs:simpleType></xs:element>")),
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
                let generation = run(&schema, world, codec, Some(&dest));
                assert!(!generation.status.success(), "{name}: {generation:?}");
                assert!(!dest.exists());
                fs::create_dir(&dest).unwrap();
                fs::write(dest.join("sentinel"), b"unchanged").unwrap();
                assert!(!run(&schema, world, codec, Some(&dest)).status.success());
                assert_eq!(fs::read(dest.join("sentinel")).unwrap(), b"unchanged");
                assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn qualified_inherited_and_nested_member_paths_preserve_indices() {
    let temp = std::env::temp_dir().join(format!("py012-qualified-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original =
        fs::read_to_string(root().join("tests/fixtures/python-unbounded-repetition/oam.xsd"))
            .unwrap();
    let schema = temp.join("qualified.xsd");
    fs::write(
        &schema,
        original
            .replace("https://www.vdl.afrl.af.mil/programs/oam", "urn:py012")
            .replace(
                "xmlns:t=",
                "xmlns:oam=\"https://www.vdl.afrl.af.mil/programs/oam\" xmlns:t=",
            )
            .replace("t:version=", "oam:version="),
    )
    .unwrap();
    for world in ["closed-schema", "open-extensions"] {
        let dest = temp.join(world.replace('-', "_"));
        let output = run(&schema, world, true, Some(&dest));
        assert!(output.status.success(), "{output:?}");
        let script = r#"import model as m, service_codec as c, unittest
class Qualified(unittest.TestCase):
 def test_wire_and_paths(self):
  text='{"{urn:py012}Required":[7],"{urn:py012}At-Least-Three":["a","b","a"],"{urn:py012}Inherited-Values":["z","a","z"]}'
  value=m.Payload(required=(7,),at_least_three=('a','b','a'),inherited_values=('z','a','z'))
  self.assertEqual(c.decode_Payload(text),value)
  self.assertEqual(c.decode_Payload(c.encode_Payload(value)),value)
  with self.assertRaises(c.CodecError) as caught:
   c.decode_Payload(text.replace('["z","a","z"]','["z",3,"z"]'))
  self.assertIn('$.{urn:py012}Inherited-Values[1]',str(caught.exception))
  child=m.Child(words=(m.Word('NATO:A'),))
  object.__setattr__(child.words[0],'value','NATO:1')
  value=m.Payload(required=(7,),at_least_three=('a','b','a'),groups=(m.Group(children=(child,)),))
  with self.assertRaises(c.CodecError) as caught: c.encode_Payload(value)
  self.assertIn('$.{urn:py012}Groups[0].{urn:py012}Children[0].{urn:py012}Words[0]',str(caught.exception))
unittest.main()
"#;
        for optimized in [false, true] {
            let mut cmd =
                Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
            if optimized {
                cmd.arg("-O");
            }
            let output = cmd
                .args(["-c", script])
                .current_dir(&dest)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("Ran 1 test"));
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
