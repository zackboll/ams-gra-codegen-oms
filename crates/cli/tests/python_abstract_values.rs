//! PY-019: actual CLI, independently authored expectations, exact closed universe.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-abstract-values")
}

fn run(schema: &Path, world: &str, codec: bool, destination: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"));
    command
        .arg(if destination.is_some() {
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
    if let Some(destination) = destination {
        command.arg("--output").arg(destination);
    }
    command.output().unwrap()
}

#[test]
fn closed_models_codecs_imports_and_optimization() {
    let temp = std::env::temp_dir().join(format!("py019-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    for codec in [false, true] {
        let dest = temp.join(format!("models_{codec}"));
        let result = run(
            &fixture().join("oam.xsd"),
            "closed-schema",
            codec,
            Some(&dest),
        );
        assert!(result.status.success(), "{result:?}");
        let source = fs::read_to_string(dest.join("model.py")).unwrap();
        assert!(!source.contains("class Middle:"));
        let start = source.find("class Base:").unwrap();
        let factories = &source[start..];
        assert!(
            factories.find("def from_Zeta").unwrap()
                < factories.find("def from_Deep_Leaf").unwrap()
        );
        assert!(
            factories.find("def from_Deep_Leaf").unwrap()
                < factories.find("def from_Alpha").unwrap()
        );
        assert_eq!(dest.join("service_codec.py").exists(), codec);
        for optimized in [false, true] {
            for style in ["standalone", "package"] {
                let mut command =
                    Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
                if optimized {
                    command.arg("-O");
                }
                let result = command
                    .args([
                        "-c",
                        include_str!("../../../tests/fixtures/python-abstract-values/runtime.py"),
                    ])
                    .arg(&dest)
                    .arg(style)
                    .arg(if codec { "codec" } else { "model" })
                    .env("PYTHONDONTWRITEBYTECODE", "1")
                    .output()
                    .unwrap();
                assert!(result.status.success(), "{result:?}");
                assert!(
                    String::from_utf8_lossy(&result.stderr).contains(if codec {
                        "Ran 14 tests"
                    } else {
                        "Ran 6 tests"
                    }),
                    "{result:?}"
                );
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn negative_schemas_and_open_world_fail_atomically() {
    let temp = std::env::temp_dir().join(format!("py019-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    let cases = [
        (
            "missing",
            original
                .replace("name=\"Zeta\"", "name=\"Zeta\" abstract=\"true\"")
                .replace("name=\"Alpha\"", "name=\"Alpha\" abstract=\"true\"")
                .replace("name=\"Deep-Leaf\"", "name=\"Deep-Leaf\" abstract=\"true\""),
        ),
        (
            "duplicate-names",
            original
                .replace("name=\"Alpha\"", "name=\"Deep_Leaf\"")
                .replace("type=\"t:Alpha\"", "type=\"t:Deep_Leaf\""),
        ),
        (
            "recursive",
            original.replace(
                "name=\"Text\" type=\"xs:string\"",
                "name=\"Text\" type=\"t:Base\"",
            ),
        ),
        (
            "incompatible-ancestry",
            original.replace("base=\"t:Base\"", "base=\"t:Selection\""),
        ),
        (
            "unsupported-descendant",
            original.replace(
                "name=\"Count\" type=\"xs:int\"",
                "name=\"Count\" type=\"xs:decimal\"",
            ),
        ),
        (
            "invalid-descendant",
            original.replace("name=\"Count\"", "name=\"Inherited\""),
        ),
    ];
    for (name, source) in cases {
        assert_ne!(source, original, "ineffective mutation {name}");
        let schema = temp.join(format!("{name}.xsd"));
        fs::write(&schema, source).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            for codec in [false, true] {
                let dest = temp.join(format!("{name}-{world}-{codec}"));
                let result = run(&schema, world, codec, None);
                assert!(!result.status.success(), "{name}: {result:?}");
                let result = run(&schema, world, codec, Some(&dest));
                assert!(!result.status.success(), "{name}: {result:?}");
                assert!(!dest.exists());
                fs::create_dir(&dest).unwrap();
                fs::write(dest.join("sentinel"), b"unchanged").unwrap();
                assert!(!run(&schema, world, codec, Some(&dest)).status.success());
                assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
                assert_eq!(fs::read(dest.join("sentinel")).unwrap(), b"unchanged");
            }
        }
    }
    for codec in [false, true] {
        let result = run(&fixture().join("oam.xsd"), "open-extensions", codec, None);
        assert!(!result.status.success());
        let text = String::from_utf8_lossy(&result.stdout);
        assert!(
            text.contains("abstract value Base is not closed under open-extensions"),
            "{result:?}"
        );
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn non_oam_qnames_are_not_python_identifiers() {
    let temp = std::env::temp_dir().join(format!("py019-clark-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    let schema = temp.join("clark.xsd");
    fs::write(
        &schema,
        original
            .replace("https://www.vdl.afrl.af.mil/programs/oam", "urn:py019")
            .replace(
                "xmlns:t=",
                "xmlns:uci=\"https://www.vdl.afrl.af.mil/programs/oam\" xmlns:t=",
            )
            .replace("t:version=", "uci:version="),
    )
    .unwrap();
    let dest = temp.join("models");
    let result = run(&schema, "closed-schema", true, Some(&dest));
    assert!(result.status.success(), "{result:?}");
    for optimized in [false, true] {
        let mut command =
            Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
        if optimized {
            command.arg("-O");
        }
        let result = command.args(["-c", "import model as m,service_codec as c,unittest\nclass Q(unittest.TestCase):\n def test_q(self):\n  v=m.Base.from_Deep_Leaf(m.Deep_Leaf(inherited=2,enabled=True,count=3))\n  wire={'{urn:py019}Inherited':2,'{urn:py019}Enabled':True,'{urn:py019}Count':3,'$type':'{urn:py019}Deep-Leaf'}\n  self.assertEqual(c._encode_Base(v,'$'),wire)\n  self.assertEqual(c._decode_Base(wire,'$'),v)\n  with self.assertRaises(c.CodecError):c._decode_Base(dict(wire,**{'$type':'Deep_Leaf'}),'$.Nested[0]')\nunittest.main()"])
            .current_dir(&dest).env("PYTHONDONTWRITEBYTECODE", "1").output().unwrap();
        assert!(result.status.success(), "{result:?}");
    }
    fs::remove_dir_all(temp).unwrap();
}
