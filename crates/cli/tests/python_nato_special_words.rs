//! PY-011 production CLI -> selected schema -> checked generated Python sources.
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
    .arg(root().join("tests/fixtures/python-nato-special-words/service.yaml"))
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
fn generated_nato_special_words_models_and_codecs_execute_in_both_worlds() {
    let temp = std::env::temp_dir().join(format!("py011-positive-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-nato-special-words");
    let schema = fixture.join("oam.xsd");
    let mut previous = None;
    for world in ["closed-schema", "open-extensions"] {
        let dest = temp.join(world.replace('-', "_"));
        let check = run(&schema, world, true, None);
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
        let generated = run(&schema, world, true, Some(&dest));
        assert!(generated.status.success(), "{generated:?}");
        let sources = ["model.py", "service_api.py", "service_codec.py"]
            .map(|name| fs::read(dest.join(name)).unwrap());
        if let Some(previous) = &previous {
            assert_eq!(previous, &sources);
        }
        previous = Some(sources.clone());
        assert!(run(&schema, world, true, Some(&dest)).status.success());
        assert_eq!(
            sources,
            ["model.py", "service_api.py", "service_codec.py"]
                .map(|name| fs::read(dest.join(name)).unwrap())
        );
        let no_codec = temp.join(format!("no_codec_{}", world.replace('-', "_")));
        assert!(run(&schema, world, false, Some(&no_codec)).status.success());
        assert!(!no_codec.join("service_codec.py").exists());
        for (index, name) in ["model.py", "service_api.py"].iter().enumerate() {
            assert_eq!(sources[index], fs::read(no_codec.join(name)).unwrap());
        }
        let output = Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()))
            .args(["-O", "-c", "import model; value = model.SpecialWord('NATO:aBc-X_');\nif value.value != 'NATO:aBc-X_': raise RuntimeError('lexical preservation')"])
            .current_dir(&no_codec).output().unwrap();
        assert!(output.status.success(), "{output:?}");
        for optimized in [false, true] {
            for style in ["standalone", "package"] {
                let mut no_codec_probe =
                    Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
                if optimized {
                    no_codec_probe.arg("-O");
                }
                let result = no_codec_probe
                    .arg(fixture.join("model_probe.py"))
                    .arg(&no_codec)
                    .arg(style)
                    .output()
                    .unwrap();
                let stderr = String::from_utf8_lossy(&result.stderr);
                assert!(
                    result.status.success()
                        && stderr.contains("Ran 3 tests")
                        && stderr.contains("\nOK\n"),
                    "{result:?}"
                );
                let mut cmd =
                    Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
                if optimized {
                    cmd.arg("-O");
                }
                let output = cmd
                    .arg(fixture.join("probe.py"))
                    .arg(&dest)
                    .arg(style)
                    .output()
                    .unwrap();
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("PY011 world={world} optimized={optimized} style={style}\n{stderr}");
                assert!(output.status.success(), "{output:?}");
                assert!(
                    stderr.contains("Ran 6 tests") && stderr.contains("\nOK\n"),
                    "{stderr}"
                );
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn qualified_member_diagnostics_preserve_namespace_and_index() {
    let temp = std::env::temp_dir().join(format!("py011-qualified-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let original =
        fs::read_to_string(root().join("tests/fixtures/python-nato-special-words/oam.xsd"))
            .unwrap();
    let schema = temp.join("qualified.xsd");
    fs::write(
        &schema,
        original
            .replace("https://www.vdl.afrl.af.mil/programs/oam", "urn:py011")
            .replace(
                "xmlns:t=",
                "xmlns:oam=\"https://www.vdl.afrl.af.mil/programs/oam\" xmlns:t=",
            )
            .replace("t:version=", "oam:version="),
    )
    .unwrap();
    let destination = temp.join("models");
    let generated = run(&schema, "closed-schema", true, Some(&destination));
    assert!(generated.status.success(), "{generated:?}");
    for optimized in [false, true] {
        let mut command =
            Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
        if optimized {
            command.arg("-O");
        }
        let output = command.args(["-c", r#"
import model as m
import service_codec as c
value = m.Payload(inherited=m.SpecialWord('NATO:A'), direct='raw',
                  named=m.SpecialWord('NATO:aBc-X_'), values=(m.SpecialWord('NATO:_'),),
                  nested=m.Nested(value=m.SpecialWord('NATO:Z'), values=(m.SpecialWord('NATO:A'),)))
text = c.encode_Payload(value)
if c.decode_Payload(text) != value: raise RuntimeError('roundtrip')
object.__setattr__(value.nested.values[0], 'value', 'NATO:1')
for action in [lambda: c.encode_Payload(value), lambda: c.decode_Payload(text.replace('NATO:Z', 'NATO:1'))]:
    try: action()
    except c.CodecError as error:
        if '$.{urn:py011}Nested.' not in str(error): raise RuntimeError(str(error))
    else: raise RuntimeError('invalid marking accepted')
try: c.encode_Payload(value)
except c.CodecError as error:
    if '$.{urn:py011}Nested.{urn:py011}Values[0]' not in str(error): raise RuntimeError(str(error))
"#]).current_dir(&destination).output().unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn unsupported_nato_special_words_selections_fail_without_artifacts() {
    let temp = std::env::temp_dir().join(format!("py011-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original =
        fs::read_to_string(root().join("tests/fixtures/python-nato-special-words/oam.xsd"))
            .unwrap();
    let cases = [
        ("wrong-min", original.replace("value=\"6\"", "value=\"5\"")),
        (
            "wrong-max",
            original.replace("value=\"261\"", "value=\"260\""),
        ),
        ("wrong-prefix", original.replace("NATO:", "nato:")),
        ("quantifier", original.replace("{1,256}", "{1,255}")),
        (
            "extra-facet",
            original.replace("<xs:minLength", "<xs:length value=\"6\"/><xs:minLength"),
        ),
        (
            "collapse",
            original.replace(
                "<xs:pattern",
                "<xs:whiteSpace value=\"collapse\"/><xs:pattern",
            ),
        ),
        (
            "preserve",
            original.replace(
                "<xs:pattern",
                "<xs:whiteSpace value=\"preserve\"/><xs:pattern",
            ),
        ),
        ("alternatives", original.replace("{1,256}", "{1,256}|x")),
        (
            "multiple-patterns",
            original.replace("<xs:pattern", "<xs:pattern value=\"x\"/><xs:pattern"),
        ),
        (
            "missing-bound",
            original.replace("<xs:maxLength value=\"261\"/>", ""),
        ),
        (
            "list",
            original.replace(
                "<xs:restriction base=\"t:SpecialWord\"/>",
                "<xs:list itemType=\"t:SpecialWord\"/>",
            ),
        ),
        (
            "abstract-value",
            original.replace(
                "name=\"Nested\" type=\"t:Nested\"",
                "name=\"Nested\" type=\"t:Ancestor\"",
            ),
        ),
        ("collision", original.replace("SpecialWord", "str")),
        (
            "unproven-base",
            original.replace("base=\"t:SpecialWord\"", "base=\"xs:token\""),
        ),
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
                let generated = run(&schema, world, codec, Some(&dest));
                assert!(!generated.status.success(), "{name}: {generated:?}");
                assert!(!dest.exists(), "partial artifacts: {name}");
                fs::create_dir(&dest).unwrap();
                fs::write(dest.join("sentinel.txt"), b"preexisting output").unwrap();
                let repeated = run(&schema, world, codec, Some(&dest));
                assert!(!repeated.status.success(), "{name}: {repeated:?}");
                assert_eq!(
                    fs::read(dest.join("sentinel.txt")).unwrap(),
                    b"preexisting output"
                );
                assert_eq!(
                    fs::read_dir(&dest).unwrap().count(),
                    1,
                    "partial overwrite: {name}"
                );
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
