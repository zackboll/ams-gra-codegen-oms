//! Session reuse must preserve standalone reports, failures and source bytes.
use ams_gra_codegen_oms::{ServiceSchemaSession, run};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-check")
        .join(name)
}

fn request(command: &str, schema: &Path, output: Option<&Path>, codec: bool) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        command.into(),
        "--schema".into(),
        schema.into(),
        "--contract".into(),
        fixture("parity.yaml").into(),
        "--language".into(),
        "rust".into(),
        "--world".into(),
        "closed-schema".into(),
    ];
    if let Some(output) = output {
        args.extend(["--output".into(), output.into()]);
    }
    if codec {
        args.push("--with-codec".into());
    }
    args
}

fn files(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut result: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (
                path.file_name().unwrap().into(),
                std::fs::read(path).unwrap(),
            )
        })
        .collect();
    result.sort();
    result
}

#[test]
fn repeated_requests_match_standalone_including_failure_and_codec_parity() {
    let scratch = std::env::temp_dir().join(format!("schema-session-{}", std::process::id()));
    std::fs::create_dir(&scratch).unwrap();
    let mut session = ServiceSchemaSession::default();
    for round in 0..2 {
        for name in ["parity-ready.xsd", "parity-unready.xsd"] {
            for codec in [false, true] {
                for command in ["service-check", "service-generate"] {
                    let output = scratch.join(format!("{round}-{name}-{codec}-{command}"));
                    let args = request(
                        command,
                        &fixture(name),
                        (command == "service-generate").then_some(output.as_path()),
                        codec,
                    );
                    let mut expected = Vec::new();
                    let standalone = run(args.clone(), &mut expected);
                    let expected_files = output.exists().then(|| files(&output));
                    if output.exists() {
                        std::fs::remove_dir_all(&output).unwrap();
                    }
                    let mut actual = Vec::new();
                    let reused = session.run(args, &mut actual);
                    assert_eq!(
                        standalone.as_ref().err().map(ToString::to_string),
                        reused.as_ref().err().map(ToString::to_string)
                    );
                    assert_eq!(expected, actual);
                    assert_eq!(expected_files, output.exists().then(|| files(&output)));
                }
            }
        }
    }
    std::fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn new_session_observes_changed_schema_and_contracts_are_not_cached() {
    let scratch =
        std::env::temp_dir().join(format!("schema-session-inputs-{}", std::process::id()));
    std::fs::create_dir(&scratch).unwrap();
    let schema = scratch.join("schema.xsd");
    std::fs::copy(fixture("parity-ready.xsd"), &schema).unwrap();
    let args = request("service-check", &schema, None, false);
    ServiceSchemaSession::default()
        .run(args.clone(), &mut Vec::new())
        .unwrap();
    std::fs::write(&schema, "not XML").unwrap();
    assert!(
        ServiceSchemaSession::default()
            .run(args, &mut Vec::new())
            .is_err()
    );
    let contract = scratch.join("contract.yaml");
    std::fs::copy(fixture("parity.yaml"), &contract).unwrap();
    let mut args = request("service-check", &fixture("parity-ready.xsd"), None, false);
    args[4] = contract.clone().into();
    let mut session = ServiceSchemaSession::default();
    session.run(args.clone(), &mut Vec::new()).unwrap();
    std::fs::write(contract, "invalid: [").unwrap();
    assert!(session.run(args, &mut Vec::new()).is_err());
    std::fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn unsupported_requests_fail_before_creating_output() {
    let mut session = ServiceSchemaSession::default();
    assert!(
        session
            .run(vec![OsString::from("validate")], &mut Vec::new())
            .is_err()
    );
    let args = request("service-check", &fixture("parity-ready.xsd"), None, false);
    for language in ["ada", "cpp", "python"] {
        let mut unsupported = args.clone();
        unsupported[6] = language.into();
        assert!(session.run(unsupported, &mut Vec::new()).is_err());
    }
    let mut unsupported = args;
    unsupported.extend(["--extension-schema".into(), "example=/absent.xsd".into()]);
    assert!(session.run(unsupported, &mut Vec::new()).is_err());
}
