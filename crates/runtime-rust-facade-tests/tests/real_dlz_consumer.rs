//! Independent Cargo consumer: production crates and actual generated source only.

#[cfg(all(ams_gra_real_uci_dlz25, ams_gra_real_uci_dlz26))]
#[test]
fn external_cargo_consumer() {
    use std::fmt::Write;
    use std::path::PathBuf;
    use std::process::Command;

    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let directory = std::env::temp_dir().join(format!("dlz-consumer-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("src")).unwrap();
    let manifest = format!(
        "[package]\nname=\"dlz-external-consumer\"\nversion=\"0.1.0\"\nedition=\"2024\"\n\
         [workspace]\n[dependencies]\nserde_json=\"1\"\n\
         ams-gra-oms-runtime-api={{path={:?}}}\n\
         ams-gra-oms-runtime-rust={{path={:?}}}\n",
        repository.join("crates/runtime-api-rust"),
        repository.join("crates/runtime-rust"),
    );
    std::fs::write(directory.join("Cargo.toml"), manifest).unwrap();
    let mut source = String::from("use ams_gra_oms_runtime_rust::OmsJsonCodec;\n");
    for version in ["25", "26"] {
        writeln!(
            source,
            "#[allow(dead_code)] mod uci{version} {{ include!({:?}); }}",
            PathBuf::from(env!("OUT_DIR")).join(format!("real_uci_dlz{version}/service_api.rs"))
        )
        .unwrap();
        // This function is type checked against the production runtime, without
        // using the test-only facade crate or making a synthetic adapter.
        writeln!(source, r#"
        #[allow(dead_code)]
        fn endpoints{version}(runtime: &mut ams_gra_oms_runtime_rust::SleetRuntime<uci{version}::service_codec::ServiceCodec>, value: &uci{version}::model::DLZMT) {{
            use uci{version}::service_api::function_f as svc;
            let subscription = svc::exchange_e::subscribe(runtime, |_: &uci{version}::model::DLZMT| {{}}).unwrap();
            svc::exchange_publish::publish(runtime, value).unwrap();
            subscription.unsubscribe().unwrap();
        }}
        "#).unwrap();
    }
    source.push_str("fn main() {\n");
    for version in ["25", "26"] {
        writeln!(source, r#"
        {{
            use uci{version}::{{model, service_codec::ServiceCodec, service_api::function_f as svc}};
            let body: serde_json::Value = serde_json::from_str(include_str!({golden:?})).unwrap();
            let value: model::DLZMT = ServiceCodec.decode_payload(&body).unwrap();
            assert_eq!(value.messageheader.schemaversion.as_str(), "002.{minor}.0");
            let data = &value.messagedata.dlz_data;
            assert_eq!(data.rangeprobabilityofintercept.get().to_bits(), 0x3ff0000000000001);
            assert_eq!(data.loftangle.unwrap().get().to_bits(), 0x8000000000000000);
            assert_eq!(data.timeofintercept.as_str(), "12:34:56.5000Z");
            assert_eq!(data.timetorelease.as_str(), "24:00:00.000Z");
            assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), body);
            assert_eq!(svc::exchange_e::MESSAGE_NAME, "DLZ");
            assert_eq!(svc::exchange_publish::MESSAGE_NAME, "DLZ");
            println!("EXTERNAL DLZ {version}: PASSED");
        }}
        "#, golden=repository.join(format!("tests/fixtures/task101/uci{version}-full-dlz.json")), minor=&version[1..]).unwrap();
    }
    source.push_str("}\n");
    std::fs::write(directory.join("src/main.rs"), source).unwrap();
    let output = Command::new("cargo")
        .args(["+1.95.0", "run", "--offline", "--manifest-path"])
        .arg(directory.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", directory.join("target"))
        .output()
        .expect("external Cargo consumer");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for version in ["25", "26"] {
        assert!(
            stdout
                .lines()
                .any(|line| line == format!("EXTERNAL DLZ {version}: PASSED"))
        );
    }
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    eprint!("{stdout}");
    std::fs::remove_dir_all(directory).unwrap();
}
