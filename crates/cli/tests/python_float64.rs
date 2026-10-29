//! PY-008 production CLI -> selected schema -> checked generated Python sources.
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
    .arg(root().join("tests/fixtures/python-float64/service.yaml"))
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
fn generated_float64_models_and_codecs_execute_in_both_worlds() {
    let temp = std::env::temp_dir().join(format!("py008-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-float64");
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
        for (index, name) in ["model.py", "service_api.py"].iter().enumerate() {
            assert_eq!(sources[index], fs::read(no_codec.join(name)).unwrap());
        }
        for optimized in [false, true] {
            for style in ["standalone", "package"] {
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
                println!("PY008 world={world} optimized={optimized} style={style}\n{stderr}");
                assert!(output.status.success(), "{output:?}");
                assert!(
                    stderr.contains("Ran 11 tests") && stderr.contains("\nOK\n"),
                    "{stderr}"
                );
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn unsupported_float64_selections_fail_without_artifacts() {
    let temp = std::env::temp_dir().join(format!("py008-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original =
        fs::read_to_string(root().join("tests/fixtures/python-float64/oam.xsd")).unwrap();
    let cases = [
        ("float32", original.replace("xs:double", "xs:float")),
        ("decimal", original.replace("xs:double", "xs:decimal")),
        (
            "pattern",
            original.replace(
                "base=\"xs:double\"/>",
                "base=\"xs:double\"><xs:pattern value=\".*\"/></xs:restriction>",
            ),
        ),
        (
            "empty",
            original.replace("maxExclusive value=\"2\"", "maxExclusive value=\"-1\""),
        ),
        (
            "contradiction",
            original.replace("maxExclusive value=\"2\"", "maxExclusive value=\"-2\""),
        ),
        (
            "nonfinite",
            original.replace("minInclusive value=\"-1\"", "minInclusive value=\"INF\""),
        ),
        (
            "abstract-value",
            original.replace(
                "name=\"Nested\" type=\"t:Nested\"",
                "name=\"Nested\" type=\"t:Ancestor\"",
            ),
        ),
        (
            "choice",
            original
                .replace(
                    "<xs:complexType name=\"Nested\"><xs:sequence>",
                    "<xs:complexType name=\"Nested\"><xs:choice>",
                )
                .replace(
                    "</xs:sequence></xs:complexType>\n  <xs:complexType name=\"Payload\">",
                    "</xs:choice></xs:complexType>\n  <xs:complexType name=\"Payload\">",
                ),
        ),
        (
            "float-name",
            original.replace("name=\"Direct\"", "name=\"float\""),
        ),
        (
            "float64-helper-name",
            original
                .replace("name=\"Scalar\"", "name=\"float64\"")
                .replace("t:Scalar", "t:float64"),
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
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
