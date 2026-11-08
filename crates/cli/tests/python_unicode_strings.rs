//! PY-018 independent corpus, shared Rust oracle and actual generated Python.
use ams_gra_oms_codegen_core::UnicodeStringProfile;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-unicode31")
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
fn independent_lexical_models_codecs_worlds_imports_and_optimization() {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".into());
    let corpus = Command::new(&python)
        .arg(fixture().join("cases.py"))
        .output()
        .unwrap();
    assert!(corpus.status.success(), "{corpus:?}");
    let cases: Vec<(usize, String, bool)> = serde_json::from_slice(&corpus.stdout).unwrap();
    assert!(cases.len() > 3000);
    for (profile, text, expected) in &cases {
        assert_eq!(
            UnicodeStringProfile::ALL[*profile].accepts(text),
            *expected,
            "{profile}: {text:?}"
        );
    }
    println!("PY018 independent Rust/Python cases={}", cases.len());
    let temp = std::env::temp_dir().join(format!("py018-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    for qualified in [false, true] {
        let schema = temp.join(format!("schema-{qualified}.xsd"));
        let source = if qualified {
            original
                .replace(
                    "https://www.vdl.afrl.af.mil/programs/oam",
                    "urn:python:unicode",
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
        for world in ["closed-schema", "open-extensions"] {
            for codec in [false, true] {
                let dest = temp.join(format!("q{qualified}_{}_c{codec}", world.replace('-', "_")));
                let check = run(&schema, world, codec, None);
                assert!(check.status.success(), "{check:?}");
                let result = run(&schema, world, codec, Some(&dest));
                assert!(result.status.success(), "{result:?}");
                for optimized in [false, true] {
                    for style in ["standalone", "package"] {
                        let mut command = Command::new(&python);
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
                            "{}\n{}",
                            String::from_utf8_lossy(&result.stdout),
                            String::from_utf8_lossy(&result.stderr)
                        );
                    }
                }
            }
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn neighboring_profiles_reject_atomically_and_names_do_not_authorize() {
    let temp = std::env::temp_dir().join(format!("py018-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    for (name, source) in [
        (
            "length",
            original.replace("length value=\"8\"", "length value=\"9\""),
        ),
        (
            "whitespace",
            original.replace(
                "<xs:length",
                "<xs:whiteSpace value=\"preserve\"/><xs:length",
            ),
        ),
        (
            "extra",
            original.replace("<xs:length", "<xs:maxLength value=\"50\"/><xs:length"),
        ),
        ("punctuation", original.replace("(N|S)", "(N|S|X)")),
        ("spaces", original.replace(" {8}", " {7}")),
        (
            "cycle",
            original.replace("base=\"t:Derived\"", "base=\"t:Leaf\""),
        ),
        (
            "unresolved",
            original.replace("base=\"t:Derived\"", "base=\"t:Missing\""),
        ),
        (
            "incompatible",
            original.replace(
                "base=\"t:Derived\"/>",
                "base=\"t:NITF_DateAndTimeType\"><xs:length value=\"8\"/></xs:restriction>",
            ),
        ),
        ("collision", original.replace("NITF_DateType", "ord")),
        (
            "named-only",
            original.replace("<xs:pattern value=\" {8}\"/>", "<xs:pattern value=\".*\"/>"),
        ),
    ] {
        assert_ne!(source, original);
        let schema = temp.join(format!("{name}.xsd"));
        fs::write(&schema, source).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            for codec in [false, true] {
                assert!(!run(&schema, world, codec, None).status.success(), "{name}");
                let dest = temp.join(format!("{name}-{world}-{codec}"));
                assert!(
                    !run(&schema, world, codec, Some(&dest)).status.success(),
                    "{name}"
                );
                assert!(!dest.exists());
                fs::create_dir(&dest).unwrap();
                fs::write(dest.join("sentinel"), b"unchanged").unwrap();
                assert!(!run(&schema, world, codec, Some(&dest)).status.success());
                assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
                assert_eq!(fs::read(dest.join("sentinel")).unwrap(), b"unchanged");
            }
        }
    }
    let renamed = temp.join("renamed.xsd");
    fs::write(
        &renamed,
        original.replace("NITF_DateType", "UnrelatedSpelling"),
    )
    .unwrap();
    assert!(run(&renamed, "closed-schema", true, None).status.success());
    fs::remove_dir_all(temp).unwrap();
}
