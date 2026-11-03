//! PY-017 independent lexical and generated service qualification.
use ams_gra_oms_codegen_core::Ipv6AddressModel;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "../../../tests/task062_cases.rs"]
mod task062;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/python-network-addresses")
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
fn independent_corpora_models_codecs_worlds_imports_optimization_and_paths() {
    let temp = std::env::temp_dir().join(format!("py017-positive-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    // Directly serialize the unchanged independent Task 062 expectations.
    // The shared production scanner is an additional gate, never the oracle.
    let cases = task062::cases();
    assert!(cases.len() > 26_000);
    let frozen: Vec<(String, bool)> =
        serde_json::from_slice(&fs::read(fixture().join("ipv6-cases.json")).unwrap()).unwrap();
    assert_eq!(cases, frozen, "independent expected fixture changed");
    for (text, expected) in &cases {
        assert_eq!(
            task062::independent_accepts(text),
            *expected,
            "oracle {text:?}"
        );
        assert_eq!(
            Ipv6AddressModel::accepts(text.as_bytes()),
            *expected,
            "shared {text:?}"
        );
    }
    fs::write(
        temp.join("ipv6-cases.json"),
        serde_json::to_vec(&cases).unwrap(),
    )
    .unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    for qualified in [false, true] {
        let schema = temp.join(format!("schema-{qualified}.xsd"));
        let source = if qualified {
            original
                .replace(
                    "https://www.vdl.afrl.af.mil/programs/oam",
                    "urn:python:network",
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
        let mut previous = None;
        let mut previous_codec = None;
        for world in ["closed-schema", "open-extensions"] {
            for codec in [false, true] {
                let dest = temp.join(format!("q{qualified}_{}_c{codec}", world.replace('-', "_")));
                let result = run(&schema, world, codec, None);
                assert!(result.status.success(), "{result:?}");
                let result = run(&schema, world, codec, Some(&dest));
                assert!(result.status.success(), "{result:?}");
                let sources =
                    ["model.py", "service_api.py"].map(|name| fs::read(dest.join(name)).unwrap());
                if let Some(previous) = &previous {
                    assert_eq!(previous, &sources);
                }
                previous = Some(sources.clone());
                assert!(run(&schema, world, codec, Some(&dest)).status.success());
                assert_eq!(
                    sources,
                    ["model.py", "service_api.py"].map(|name| fs::read(dest.join(name)).unwrap())
                );
                assert_eq!(dest.join("service_codec.py").exists(), codec);
                if codec {
                    let bytes = fs::read(dest.join("service_codec.py")).unwrap();
                    if let Some(previous) = &previous_codec {
                        assert_eq!(previous, &bytes);
                    }
                    previous_codec = Some(bytes);
                }
                for optimized in [false, true] {
                    for style in ["standalone", "package"] {
                        let mut command = Command::new(
                            std::env::var("PYTHON").unwrap_or_else(|_| "python3".into()),
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
fn exact_profiles_and_named_edges_reject_neighbors_atomically() {
    let temp = std::env::temp_dir().join(format!("py017-negative-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let original = fs::read_to_string(fixture().join("oam.xsd")).unwrap();
    let cases = [
        ("ipv4-min", original.replace("minLength value=\"7\"", "minLength value=\"6\"")),
        ("ipv4-max", original.replace("maxLength value=\"15\"", "maxLength value=\"16\"")),
        ("ipv6-min", original.replace("minLength value=\"2\"", "minLength value=\"3\"")),
        ("ipv6-max", original.replace("maxLength value=\"45\"", "maxLength value=\"44\"")),
        ("whitespace", original.replace("<xs:minLength", "<xs:whiteSpace value=\"preserve\"/><xs:minLength")),
        ("extra-facet", original.replace("<xs:minLength", "<xs:length value=\"7\"/><xs:minLength")),
        ("quantifier", original.replace("{0,5}", "{0,6}")),
        ("octet", original.replace("25[0-5]", "25[0-6]")),
        ("extra-pattern", original.replace("<xs:minLength", "<xs:pattern value=\".*\"/><xs:minLength")),
        ("unknown", original.replace("[0-9a-fA-F]", "[0-9a-f]")),
        ("incompatible", original.replace("base=\"t:Derived\"/>", "base=\"t:IPv6_AddressType\"><xs:minLength value=\"7\"/><xs:maxLength value=\"15\"/></xs:restriction>")),
        ("cycle", original.replace("base=\"t:Derived\"", "base=\"t:Leaf\"")),
        ("unresolved", original.replace("base=\"t:Derived\"", "base=\"t:Missing\"")),
        ("collision", original.replace("IPv4_AddressType", "str")),
        ("field-local", original.replace("name=\"Required\" type=\"t:IPv4_AddressType\"/>", "name=\"Required\"><xs:simpleType><xs:restriction base=\"xs:string\"><xs:minLength value=\"7\"/><xs:maxLength value=\"15\"/><xs:pattern value=\".*\"/></xs:restriction></xs:simpleType></xs:element>")),
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
                assert!(!run(&schema, world, codec, Some(&dest)).status.success());
                assert_eq!(fs::read(dest.join("sentinel")).unwrap(), b"unchanged");
                assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
            }
        }
    }
    // Classification must not depend on either conventional address name.
    let schema = temp.join("renamed.xsd");
    fs::write(
        &schema,
        original
            .replace("IPv4_AddressType", "DottedText")
            .replace("IPv6_AddressType", "ColonText"),
    )
    .unwrap();
    for world in ["closed-schema", "open-extensions"] {
        for codec in [false, true] {
            assert!(run(&schema, world, codec, None).status.success());
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
