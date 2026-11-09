//! PY-007 actual CLI -> frontend -> service projection -> generated execution.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
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
fn inherited_services_execute_literals_in_both_worlds_and_import_styles() {
    let temp = std::env::temp_dir().join(format!("py007-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-inheritance");
    let schema = fixture.join("oam.xsd");
    let contract = fixture.join("service.yaml");
    let mut previous = None;
    for world in ["closed-schema", "open-extensions"] {
        let dest = temp.join(world.replace('-', "_"));
        let check = run(&schema, &contract, world, true, None);
        assert!(check.status.success(), "{check:?}");
        for stage in [
            "model status: READY",
            "service api status: READY",
            "codec status: READY",
        ] {
            assert!(
                String::from_utf8_lossy(&check.stdout).contains(stage),
                "{check:?}"
            );
        }
        let generated = run(&schema, &contract, world, true, Some(&dest));
        assert!(generated.status.success(), "{generated:?}");
        let sources = ["model.py", "service_api.py", "service_codec.py"]
            .map(|name| fs::read(dest.join(name)).unwrap());
        if let Some(previous) = &previous {
            assert_eq!(previous, &sources);
        }
        previous = Some(sources.clone());
        assert!(
            run(&schema, &contract, world, true, Some(&dest))
                .status
                .success()
        );
        assert_eq!(
            sources,
            ["model.py", "service_api.py", "service_codec.py"]
                .map(|name| fs::read(dest.join(name)).unwrap())
        );
        let no_codec = temp.join(format!("no_codec_{}", world.replace('-', "_")));
        assert!(
            run(&schema, &contract, world, false, Some(&no_codec))
                .status
                .success()
        );
        for (index, file) in ["model.py", "service_api.py"].iter().enumerate() {
            assert_eq!(sources[index], fs::read(no_codec.join(file)).unwrap());
        }
        for optimized in [false, true] {
            for style in ["standalone", "package"] {
                let mut command =
                    Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
                if optimized {
                    command.arg("-O");
                }
                let output = command
                    .arg(fixture.join("probe.py"))
                    .arg(&dest)
                    .arg(style)
                    .output()
                    .unwrap();
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("PY007 world={world} optimized={optimized} style={style}\n{stderr}");
                assert!(output.status.success(), "{output:?}");
                assert!(
                    stderr.contains("Ran 12 tests") && stderr.contains("\nOK\n"),
                    "{stderr}"
                );
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn unsupported_inherited_service_selections_fail_atomically() {
    let temp = std::env::temp_dir().join(format!("py007-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-inheritance");
    let original = fs::read_to_string(fixture.join("oam.xsd")).unwrap();
    let contract = fixture.join("service.yaml");
    let field = "<xs:element name=\"Base_Required\" type=\"xs:int\"/>";
    let cases = [
        ("abstract-field", original.replace("name=\"Linked\" type=\"t:Token\"", "name=\"Linked\" type=\"t:Base\"")),
        ("choice-segment", original.replacen("<xs:sequence>", "<xs:choice>", 1).replacen("</xs:sequence>", "</xs:choice>", 1)),
        ("abstract-choice", original.replace("<xs:sequence><xs:element name=\"Linked\" type=\"t:Token\"/></xs:sequence>", "<xs:choice><xs:element name=\"Abstract\" type=\"t:Base\"/></xs:choice>")),
        ("missing-base", original.replace("base=\"t:Third\"", "base=\"t:Absent\"")),
        ("cycle", original.replace("base=\"t:Middle\"", "base=\"t:Leaf\"")),
        ("original-collision", original.replace("name=\"Local\"", "name=\"Base_Required\"")),
        ("normalized-collision", original.replace("name=\"Local\"", "name=\"base_required\"")),
        ("builtin-annotation", original.replace("name=\"Base_Required\"", "name=\"int\"")),
        ("generated-annotation", original.replace("name=\"Token\"", "name=\"token\"").replace("t:Token", "t:token").replace("name=\"Base_Required\"", "name=\"token\"")),
        ("inherited-primitive", original.replace(field, "<xs:element name=\"Base_Required\" type=\"xs:decimal\"/>")),
        ("inherited-facet", original.replace(field, "<xs:element name=\"Base_Required\"><xs:simpleType><xs:restriction base=\"xs:string\"><xs:pattern value=\"[A-Z]+\"/></xs:restriction></xs:simpleType></xs:element>")),
        ("inherited-nillable", original.replace(field, "<xs:element name=\"Base_Required\" type=\"xs:int\" nillable=\"true\"/>")),
    ];
    assert_eq!(cases.len(), 12);
    for (name, source) in cases {
        let schema = temp.join(format!("{name}.xsd"));
        fs::write(&schema, source).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            for codec in [false, true] {
                let check = run(&schema, &contract, world, codec, None);
                assert!(!check.status.success(), "{name}: {check:?}");
                let dest = temp.join(format!("{name}-{world}-{codec}"));
                let generation = run(&schema, &contract, world, codec, Some(&dest));
                assert!(!generation.status.success(), "{name}: {generation:?}");
                assert!(
                    !dest.exists() || fs::read_dir(&dest).unwrap().count() == 0,
                    "partial files: {name}"
                );
            }
        }
    }
    println!("PY019 INHERITED NEGATIVE SELECTIONS: PASSED cases=12 selections=48 checks=96");
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn inhabited_abstract_service_payload_executes_closed_and_rejects_open() {
    let temp = std::env::temp_dir().join(format!("py019-inherited-payload-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-inheritance");
    let schema = temp.join("abstract-payload.xsd");
    let original = fs::read_to_string(fixture.join("oam.xsd")).unwrap();
    let changed = original.replace(
        "name=\"InheritedNotice\" type=\"t:Leaf\"",
        "name=\"InheritedNotice\" type=\"t:Base\"",
    );
    assert_ne!(changed, original);
    fs::write(&schema, changed).unwrap();
    let contract = fixture.join("service.yaml");
    for codec in [false, true] {
        let checked = run(&schema, &contract, "closed-schema", codec, None);
        assert!(checked.status.success(), "{checked:?}");
        let dest = temp.join(format!("models_{codec}"));
        let generated = run(&schema, &contract, "closed-schema", codec, Some(&dest));
        assert!(generated.status.success(), "{generated:?}");
        assert_eq!(
            fs::read_dir(&dest).unwrap().count(),
            if codec { 3 } else { 2 }
        );
        let script = r#"import model as m,service_api as api,unittest,json
class ClosedPayload(unittest.TestCase):
 def test_inventory_and_all_variants(self):
  common=dict(base_required=1,required_values=(2,))
  values=[m.Base.from_Middle(m.Middle(**common)),m.Base.from_Third(m.Third(**common,linked=m.Token(3))),m.Base.from_Leaf(m.Leaf(**common,linked=m.Token(3),local='x',nested=m.Nested(count=4,label='n'))),m.Base.from_Sibling(m.Sibling(**common,other=True))]
  wires=[{'Base_Required':1,'Required_Values':[2],'$type':'Middle'},{'Base_Required':1,'Required_Values':[2],'Linked':3,'$type':'Third'},{'Base_Required':1,'Required_Values':[2],'Linked':3,'Local':'x','Nested':{'Count':4,'Label':'n'},'$type':'Leaf'},{'Base_Required':1,'Required_Values':[2],'Other':True,'$type':'Sibling'}]
  self.assertIs(api.function_loop.exchange_input.Payload,m.Base)
  for value,wire in zip(values,wires):
   self.assertEqual(value.alternative,('https://www.vdl.afrl.af.mil/programs/oam',wire['$type']))
   self.assertEqual(value.value.base_required,1)
   if WITH_CODEC:
    import service_codec as c
    self.assertEqual(json.loads(c.encode_Base(value)),wire)
    self.assertEqual(c.decode_Base(json.dumps(wire)),value)
  with self.assertRaises(TypeError):m.Base.from_Middle(values[1].value)
  with self.assertRaises(ValueError):m.Base(('https://www.vdl.afrl.af.mil/programs/oam','Base'),values[0].value)
  if WITH_CODEC:
   import service_codec as c
   for bad in [{'$type':'Private'},{'$type':'Middle'},{'Base_Required':True,'Required_Values':[2],'$type':'Middle'}]:
    with self.assertRaises(c.CodecError):c.decode_Base(json.dumps(bad))
unittest.main()
"#;
        for optimized in [false, true] {
            let mut python =
                Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
            if optimized {
                python.arg("-O");
            }
            let script = format!(
                "WITH_CODEC={}\n{script}",
                if codec { "True" } else { "False" }
            );
            let result = python
                .args(["-c", &script])
                .current_dir(&dest)
                .env("PYTHONDONTWRITEBYTECODE", "1")
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
            assert!(String::from_utf8_lossy(&result.stderr).contains("Ran 1 test"));
        }
        let open = temp.join(format!("open-{codec}"));
        let check = run(&schema, &contract, "open-extensions", codec, None);
        assert!(!check.status.success());
        assert!(
            String::from_utf8_lossy(&check.stdout)
                .contains("abstract value Base is not closed under open-extensions generation"),
            "{check:?}"
        );
        assert!(
            !run(&schema, &contract, "open-extensions", codec, Some(&open))
                .status
                .success()
        );
        assert!(!open.exists());
        fs::create_dir(&open).unwrap();
        fs::write(open.join("sentinel"), b"unchanged").unwrap();
        assert!(
            !run(&schema, &contract, "open-extensions", codec, Some(&open))
                .status
                .success()
        );
        assert_eq!(fs::read_dir(&open).unwrap().count(), 1);
        assert_eq!(fs::read(open.join("sentinel")).unwrap(), b"unchanged");
    }
    fs::remove_dir_all(temp).unwrap();
}
