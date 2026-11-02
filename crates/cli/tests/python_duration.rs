//! PY-014 production CLI -> selected schema -> checked generated Python sources.
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
    .arg(root().join("tests/fixtures/python-duration/service.yaml"))
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
fn generated_duration_models_and_codecs_execute_in_both_worlds() {
    let temp = std::env::temp_dir().join(format!("py014-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-duration");
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
            .args(["-O", "-c", "import model; value = model.Span(' -PT0.00S ');\nif value.value != '-PT0.00S': raise RuntimeError('normalization')"])
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
                        && stderr.contains("Ran 2 tests")
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
                println!("PY014 world={world} optimized={optimized} style={style}\n{stderr}");
                assert!(output.status.success(), "{output:?}");
                assert!(
                    stderr.contains("Ran 8 tests") && stderr.contains("\nOK\n"),
                    "{stderr}"
                );
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn unsupported_duration_selections_fail_without_artifacts() {
    let temp = std::env::temp_dir().join(format!("py014-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original =
        fs::read_to_string(root().join("tests/fixtures/python-duration/oam.xsd")).unwrap();
    let cases = [
        ("time", original.replace("xs:duration", "xs:time")),
        ("pattern", original.replace("base=\"xs:duration\"/>", "base=\"xs:duration\"><xs:pattern value=\"P.*\"/></xs:restriction>")),
        ("length", original.replace("base=\"xs:duration\"/>", "base=\"xs:duration\"><xs:maxLength value=\"30\"/></xs:restriction>")),
        ("field-local", original.replace("name=\"Direct\" type=\"xs:duration\"/>", "name=\"Direct\"><xs:simpleType><xs:restriction base=\"xs:duration\"><xs:pattern value=\"P.*\"/></xs:restriction></xs:simpleType></xs:element>")),
        ("abstract-value", original.replace("name=\"Nested\" type=\"t:Nested\"", "name=\"Nested\" type=\"t:Ancestor\"")),
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

#[test]
fn non_oam_duration_errors_retain_clark_member_paths() {
    let temp = std::env::temp_dir().join(format!("py014-clark-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original =
        fs::read_to_string(root().join("tests/fixtures/python-duration/oam.xsd")).unwrap();
    let schema = temp.join("non-oam.xsd");
    fs::write(
        &schema,
        original
            .replace(
                "https://www.vdl.afrl.af.mil/programs/oam",
                "urn:python:duration",
            )
            .replace(
                "xmlns:t=",
                "xmlns:uci=\"https://www.vdl.afrl.af.mil/programs/oam\" xmlns:t=",
            )
            .replace("t:version=", "uci:version="),
    )
    .unwrap();
    for world in ["closed-schema", "open-extensions"] {
        let dest = temp.join(world.replace('-', "_"));
        let output = run(&schema, world, true, Some(&dest));
        assert!(output.status.success(), "{output:?}");
        let script = "import model as m, service_codec as c, unittest\nclass Clark(unittest.TestCase):\n def test_paths(self):\n  with self.assertRaisesRegex(c.CodecError, r'\\$\\.\\{urn:python:duration\\}Inherited:'):\n   c.decode_Payload('{\"{urn:python:duration}Inherited\":\"PT\"}')\nunittest.main()";
        for optimized in [false, true] {
            let mut command =
                Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
            if optimized {
                command.arg("-O");
            }
            let result = command
                .args(["-c", script])
                .current_dir(&dest)
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
