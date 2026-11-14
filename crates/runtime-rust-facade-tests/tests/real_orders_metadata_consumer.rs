//! External Cargo package links only production crates and generated APIs.

#[cfg(all(ams_gra_real_uci_orders_metadata25, ams_gra_real_uci_orders_metadata26))]
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
        std::env::temp_dir().join(format!("orders-metadata-consumer-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("Cargo.toml"),
        format!(
            "[package]\nname=\"orders-metadata-consumer\"\nversion=\"0.1.0\"\nedition=\"2024\"\n\
         [workspace]\n[dependencies]\nserde_json=\"1\"\n\
         ams-gra-oms-runtime-api={{path={:?}}}\n\
         ams-gra-oms-runtime-rust={{path={:?}}}\n",
            repository.join("crates/runtime-api-rust"),
            repository.join("crates/runtime-rust"),
        ),
    )
    .unwrap();
    let mut source = String::new();
    for version in ["25", "26"] {
        writeln!(source,
            "#[allow(dead_code, clippy::all)] #[path = {:?}] mod uci{version};\n#[cfg(test)] const DOCUMENT{version}: &str = include_str!({:?});",
            PathBuf::from(env!("OUT_DIR")).join(format!("real_uci_orders_metadata{version}/service_api.rs")),
            repository.join(format!("tests/fixtures/rust-oms-012/uci{version}-second.json")),
        ).unwrap();
    }
    // Keep the small consumer reviewable as a source fixture, not a generated
    // handwritten imitation of the service API. Include the fixture as a module
    // so its inner documentation remains valid.
    writeln!(
        source,
        "#[path = {:?}] mod consumer;",
        repository.join("tests/fixtures/rust-oms-012/consumer.rs")
    )
    .unwrap();
    // The included source uses sibling generated modules and authored documents.
    source.push('\n');
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
    // This independent package's lock is the Cargo-pruned subset of the
    // repository lock, with identical versions/sources/checksums. The runner's
    // adversarial controls verify that subset; every command here is locked.
    let lock = include_str!("../../../tests/fixtures/rust-oms-012/consumer.Cargo.lock");
    std::fs::write(directory.join("Cargo.lock"), lock).unwrap();
    for arguments in [
        vec!["test", "--all-targets", "--", "--nocapture"],
        vec!["check", "--all-targets"],
        vec!["clippy", "--all-targets", "--", "-D", "warnings"],
    ] {
        let output = Command::new("cargo")
            .arg("+1.95.0")
            .args(&arguments[..1])
            .args(["--locked", "--offline", "--manifest-path"])
            .arg(directory.join("Cargo.toml"))
            .args(&arguments[1..])
            .env("CARGO_TARGET_DIR", directory.join("target"))
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprint!("{stderr}");
        eprintln!("{stdout}");
        assert!(
            output.status.success(),
            "external consumer {}: {stdout}\n{stderr}",
            arguments[0]
        );
        if arguments[0] == "test" {
            for version in ["002.5.0", "002.6.0"] {
                assert!(
                    stdout
                        .lines()
                        .any(|line| line == format!("EXTERNAL ORDERSMETADATA {version}: PASSED"))
                );
            }
            assert!(stdout.contains("test result: ok. 2 passed; 0 failed; 0 ignored;"));
        }
    }
    assert_eq!(
        std::fs::read_to_string(directory.join("Cargo.lock")).unwrap(),
        lock,
        "consumer must not silently resolve different dependencies"
    );
    std::fs::remove_dir_all(directory).unwrap();
    eprintln!("RUST-OMS-012 INDEPENDENT CARGO CONSUMER: PASSED");
}
