//! Compile and execute an independent Cargo package against actual output files.
//! The facade crate is deliberately NOT a dependency of that package.

#[cfg(all(ams_gra_real_uci_rdma25, ams_gra_real_uci_rdma26))]
#[test]
fn external_cargo_consumer() {
    use std::fmt::Write;
    use std::path::PathBuf;
    use std::process::Command;

    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let directory =
        std::env::temp_dir().join(format!("rust-oms-002-consumer-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("src")).unwrap();
    let manifest = format!(
        "[package]\nname=\"rdma-external-consumer\"\nversion=\"0.1.0\"\nedition=\"2024\"\n\
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
            PathBuf::from(env!("OUT_DIR")).join(format!("real_uci_rdma{version}/service_api.rs"))
        )
        .unwrap();
    }
    source.push_str("fn main() {\n");
    for version in ["25", "26"] {
        writeln!(source, r#"
        {{
            use uci{version}::{{model, service_codec::ServiceCodec, service_api::function_rdma as svc}};
            let expected: serde_json::Value = serde_json::from_str(include_str!({golden:?})).unwrap();
            let value: model::RDMAInitializeSetupMT = ServiceCodec.decode_payload(&expected).unwrap();
            assert_eq!(value.messagedata.endpoint.as_slice().len(), 2);
            assert_eq!(value.messagedata.rdma_provider, model::RDMAProviderEnum::TCP);
            assert_eq!(value.messageheader.schemaversion.as_str(), "002.{minor}.0");
            assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), expected);
            assert_eq!(svc::exchange_receive::MESSAGE_NAME, "RDMA_InitializeSetup");
            assert_eq!(svc::exchange_publish::MESSAGE_NAME, "RDMA_InitializeSetup");
            println!("EXTERNAL RDMA {version}: PASSED");
        }}
        "#, golden = repository.join(format!("tests/fixtures/service-generate/task092-real-rdma{version}.json")),
            minor = &version[1..]).unwrap();
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
                .any(|line| line == format!("EXTERNAL RDMA {version}: PASSED"))
        );
    }
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    eprint!("{stdout}");
    std::fs::remove_dir_all(directory).unwrap();
}
