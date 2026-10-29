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
        ("abstract-payload", original.replace("name=\"InheritedNotice\" type=\"t:Leaf\"", "name=\"InheritedNotice\" type=\"t:Base\"")),
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
        ("inherited-unbounded", original.replace("maxOccurs=\"3\"", "maxOccurs=\"unbounded\"")),
        ("inherited-nillable", original.replace(field, "<xs:element name=\"Base_Required\" type=\"xs:int\" nillable=\"true\"/>")),
    ];
    assert_eq!(cases.len(), 14);
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
    println!("PY007 ATOMIC NEGATIVE SELECTIONS: PASSED cases=14 selections=56 checks=112");
    fs::remove_dir_all(temp).unwrap();
}
