//! PY-006 production CLI qualification, independent Python literal oracles.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(schema: &Path, contract: &Path, world: &str, dest: Option<&Path>) -> Output {
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
    .args(["--language", "python", "--world", world, "--with-codec"]);
    if let Some(dest) = dest {
        cmd.arg("--output").arg(dest);
    }
    cmd.output().unwrap()
}

#[test]
fn exact_profiles_execute_and_neighbours_fail_atomically() {
    let temp = std::env::temp_dir().join(format!("py006-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let fixture = root().join("tests/fixtures/python-header-profiles");
    let original = fs::read_to_string(fixture.join("oam.xsd")).unwrap();
    let contract = fs::read_to_string(fixture.join("service.yaml")).unwrap();
    let mut previous = None;
    for world in ["closed-schema", "open-extensions"] {
        let dest = temp.join(world.replace('-', "_"));
        let check = run(
            &fixture.join("oam.xsd"),
            &fixture.join("service.yaml"),
            world,
            None,
        );
        assert!(check.status.success(), "{check:?}");
        let report = String::from_utf8_lossy(&check.stdout);
        for verdict in [
            "model status: READY",
            "service api status: READY",
            "codec status: READY",
            "\nstatus: READY",
        ] {
            assert!(report.contains(verdict), "{report}");
        }
        let generated = run(
            &fixture.join("oam.xsd"),
            &fixture.join("service.yaml"),
            world,
            Some(&dest),
        );
        assert!(generated.status.success(), "{generated:?}");
        let sources = ["model.py", "service_api.py", "service_codec.py"]
            .map(|name| fs::read(dest.join(name)).unwrap());
        if let Some(previous) = &previous {
            assert_eq!(previous, &sources);
        }
        previous = Some(sources.clone());
        assert!(
            run(
                &fixture.join("oam.xsd"),
                &fixture.join("service.yaml"),
                world,
                Some(&dest)
            )
            .status
            .success()
        );
        assert_eq!(
            sources,
            ["model.py", "service_api.py", "service_codec.py"]
                .map(|name| fs::read(dest.join(name)).unwrap())
        );
        for optimized in [false, true] {
            for style in ["standalone", "package"] {
                let mut cmd =
                    Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
                if optimized {
                    cmd.arg("-O");
                }
                let output = cmd
                    .arg(root().join("crates/backend-python/tests/header_profiles.py"))
                    .arg(&dest)
                    .arg(style)
                    .arg(root())
                    .output()
                    .expect("Python is mandatory, no skip");
                assert!(
                    output.status.success(),
                    "{world} {style} optimized={optimized}: {output:?}"
                );
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(stderr.contains("Ran 7 tests"), "{stderr}");
                assert!(stderr.lines().any(|line| line == "OK"), "{stderr}");
                eprintln!("{world} {style} optimized={optimized}:\n{stderr}");
            }
        }
    }
    // The private namespace uses ordinary String mapping, never OAM identity.
    let schema = temp.join("private.xsd");
    fs::write(
        &schema,
        original
            .replace(
                "https://www.vdl.afrl.af.mil/programs/oam",
                "urn:py006:private",
            )
            .replace(
                "xmlns:uci=\"urn:py006:private\"",
                "xmlns:uci=\"https://www.vdl.afrl.af.mil/programs/oam\"",
            ),
    )
    .unwrap();
    assert!(
        run(
            &schema,
            &fixture.join("service.yaml"),
            "closed-schema",
            Some(&temp.join("private"))
        )
        .status
        .success()
    );
    // A foreign namespace's similarly named unconstrained String must use the
    // ordinary String mapping, not acquire OAM UUID restrictions by local name.
    let private_contract = temp.join("private.yaml");
    fs::write(
        &private_contract,
        contract
            .replace("HeaderNotice", "UUIDValue")
            .replace("HeaderOther", "UUIDValue")
            .replace("VersionValue", "UUIDValue"),
    )
    .unwrap();
    let private_source = fs::read_to_string(&schema).unwrap().replace(
        "<xs:length value=\"36\"/>", "").replace(
        "<xs:pattern value=\"(0{8}(-0{4}){3}-0{12})|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})\"/>", "");
    fs::write(&schema, private_source).unwrap();
    let foreign = temp.join("foreign");
    let ordinary = run(&schema, &private_contract, "closed-schema", Some(&foreign));
    assert!(ordinary.status.success(), "{ordinary:?}");
    for optimized in [false, true] {
        let mut cmd = Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
        if optimized {
            cmd.arg("-O");
        }
        let output = cmd.current_dir(&foreign).args(["-c", "import model as m,service_codec as c,unittest\nt=unittest.TestCase()\nv=m.UniversallyUniqueIdentifierType('ordinary String, not a UUID')\nt.assertEqual(c.encode_UniversallyUniqueIdentifierType(v), '\"ordinary String, not a UUID\"')\nt.assertEqual(c.decode_UniversallyUniqueIdentifierType('\"ordinary String, not a UUID\"'),v)"]).output().unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    for (label, source, contract) in [
        ("uuid-length", original.replace("value=\"36\"", "value=\"35\""), contract.clone()),
        ("version-bound", original.replace("value=\"57\"", "value=\"58\""), contract.clone()),
        ("uuid-pattern", original.replace("[1-5]", "[1-6]"), contract.clone()),
        ("uuid-alternative", original.replace("(0{8}(-0{4}){3}-0{12})|", ""), contract.clone()),
        ("whitespace", original.replace("<xs:length value=\"36\"/>", "<xs:length value=\"36\"/><xs:whiteSpace value=\"preserve\"/>"), contract.clone()),
        ("near-selection", original.clone(), contract.replace("HeaderNotice", "Unsupported")),
        ("oam-name-only", original.replace("<xs:length value=\"36\"/>", "").replace(
            "<xs:pattern value=\"(0{8}(-0{4}){3}-0{12})|([a-fA-F0-9]{8}-[a-fA-F0-9]{4}-[1-5][a-fA-F0-9]{3}-[89abAB][a-fA-F0-9]{3}-[a-fA-F0-9]{12})\"/>", ""), contract),
    ] {
        let schema = temp.join(format!("{label}.xsd"));
        let contract_path = temp.join(format!("{label}.yaml"));
        fs::write(&schema, source).unwrap(); fs::write(&contract_path, contract).unwrap();
        for world in ["closed-schema", "open-extensions"] {
            let dest = temp.join(format!("absent-{label}-{world}"));
            let blocked = run(&schema, &contract_path, world, Some(&dest));
            assert_eq!(blocked.status.code(), Some(1), "{label}: {blocked:?}");
            assert!(String::from_utf8_lossy(&blocked.stdout).contains("status: NOT READY"), "{blocked:?}");
            assert!(!dest.exists());
            fs::create_dir(&dest).unwrap(); fs::write(dest.join("model.py"), "sentinel").unwrap();
            assert_eq!(run(&schema, &contract_path, world, Some(&dest)).status.code(), Some(1));
            assert_eq!(fs::read_to_string(dest.join("model.py")).unwrap(), "sentinel");
            assert_eq!(fs::read_dir(&dest).unwrap().count(), 1);
        }
    }
    fs::remove_dir_all(temp).unwrap();
}
