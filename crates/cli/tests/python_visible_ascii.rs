//! PY-010 production CLI -> selected schema -> checked generated Python sources.
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
    .arg(root().join("tests/fixtures/python-visible-ascii/service.yaml"))
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
fn generated_visible_ascii_models_and_codecs_execute_in_both_worlds() {
    let temp = std::env::temp_dir().join(format!("py010-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-visible-ascii");
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
            .args(["-O", "-c", "import model; value = model.Text256('  kept  ');\nif value.value != '  kept  ': raise RuntimeError('lexical preservation')"])
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
                println!("PY010 world={world} optimized={optimized} style={style}\n{stderr}");
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
fn unsupported_visible_ascii_selections_fail_without_artifacts() {
    let temp = std::env::temp_dir().join(format!("py010-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original =
        fs::read_to_string(root().join("tests/fixtures/python-visible-ascii/oam.xsd")).unwrap();
    let cases = [
        ("unobserved", original.replace("{1,256}", "{1,255}").replace("value=\"256\"", "value=\"255\"")),
        ("quantifier", original.replace("{1,256}", "{1,128}")),
        ("extra-facet", original.replace("<xs:minLength value=\"1\"/>", "<xs:length value=\"1\"/><xs:minLength value=\"1\"/>")),
        ("collapse", original.replace("<xs:pattern", "<xs:whiteSpace value=\"collapse\"/><xs:pattern")),
        ("alternatives", original.replace("[ -~]{1,256}", "[ -~]{1,256}|x")),
        ("whitespace-visible", original.replace("[ -~]{1,256}", "[&#xA;&#xD; -~]{1,256}")),
        ("nato", original.replace("[ -~]{1,256}", "NATO:[a-zA-Z\\-_]{1,256}")),
        ("structured", original.replace("[ -~]{1,256}", "[0-9]{1,3}([.][0-9]{1,3}){3}")),
        ("unicode", original.replace("[ -~]{1,256}", "[\\p{L}]{1,256}")),
        ("multiple-patterns", original.replace("<xs:pattern value=\"[ -~]{1,256}\"/>", "<xs:pattern value=\"[ -~]{1,256}\"/><xs:pattern value=\"x\"/>")),
        ("missing-bound", original.replace("<xs:maxLength value=\"256\"/>", "")),
        ("list", original.replace("<xs:restriction base=\"t:Text256\"/>", "<xs:list itemType=\"t:Text256\"/>")),
        ("field-local", original.replace("name=\"Direct\" type=\"xs:string\"/>", "name=\"Direct\"><xs:simpleType><xs:restriction base=\"xs:string\"><xs:maxLength value=\"20\"/></xs:restriction></xs:simpleType></xs:element>")),
        ("abstract-value", original.replace("name=\"Nested\" type=\"t:Nested\"", "name=\"Nested\" type=\"t:Ancestor\"")),
        ("collision", original.replace("Text256", "str")),
        ("unproven-base", original.replace("base=\"t:Text256\"", "base=\"xs:token\"")),
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
