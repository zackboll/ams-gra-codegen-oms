use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

fn verify(path: &Path, expected: &str) -> Result<(), String> {
    println!("cargo:rerun-if-changed={}", path.display());
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .map_err(|error| format!("install sha256sum (GNU coreutils): {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot read {}: {}; fetch the complete pinned schema set using scripts/fetch-pinned-uci-2.5.sh or scripts/fetch-pinned-uci-2.6.sh",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let digest = String::from_utf8_lossy(&output.stdout);
    if digest.split_whitespace().next() != Some(expected) {
        return Err(format!(
            "{} SHA-256 mismatch: expected {expected}, got {}; AMS_GRA_UCI_ROOT and AMS_GRA_UCI_RELEASE must select the same authentic pinned release; refetch with scripts/fetch-pinned-uci-<release>.sh",
            path.display(),
            digest.split_whitespace().next().unwrap_or("<none>")
        ));
    }
    Ok(())
}

fn generate() -> Result<(), String> {
    println!("cargo:rerun-if-env-changed=AMS_GRA_UCI_ROOT");
    println!("cargo:rerun-if-env-changed=AMS_GRA_UCI_RELEASE");
    let release = std::env::var("AMS_GRA_UCI_RELEASE")
        .map_err(|_| "set AMS_GRA_UCI_RELEASE to 2.5 or 2.6".to_owned())?;
    // Source identities are the existing fetch-script pins:
    // v2.5: 093610b7753944059360d3236770ab446d039556
    // v2.6: 78eb61b6112c8bffa40820c33124b57787fc5bd9
    let (minor, root_hash, security_hash) = match release.as_str() {
        "2.5" => (
            "5",
            "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
            "4a8056f1503234d423a0c2495844ab65387febced62bea58b87d4f343e9e7a0b",
        ),
        "2.6" => (
            "6",
            "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
            "ee6e58d5db9fd80d526bc964b8244ec296d106301640d34c3cb2c5c00b710b88",
        ),
        _ => {
            return Err(format!(
                "unsupported AMS_GRA_UCI_RELEASE={release:?}; use 2.5 or 2.6"
            ));
        }
    };
    let selected = PathBuf::from(std::env::var_os("AMS_GRA_UCI_ROOT").ok_or(
        "set AMS_GRA_UCI_ROOT to the MessageDefinitions XSD printed by scripts/fetch-pinned-uci-<release>.sh",
    )?);
    // Watch both the authored path (including a possible symlink) and resolved
    // files. Changing either selection or either schema invalidates generation.
    println!("cargo:rerun-if-changed={}", selected.display());
    let root = selected.canonicalize().map_err(|error| {
        format!(
            "AMS_GRA_UCI_ROOT {}: {error}; fetch pinned inputs first",
            selected.display()
        )
    })?;
    verify(&root, root_hash)?;
    verify(
        &root
            .parent()
            .ok_or("schema root has no parent")?
            .join(format!("UCI_SecurityMarkings_v2_{minor}_0.xsd")),
        security_hash,
    )?;
    let contract =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("contracts/uci2{minor}-service.yaml"));
    println!("cargo:rerun-if-changed={}", contract.display());
    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("Cargo did not set OUT_DIR")?);
    let generated = out.join("generated");
    if generated.exists() {
        std::fs::remove_dir_all(&generated).map_err(|error| error.to_string())?;
    }
    let arguments: Vec<OsString> = vec![
        "service-generate".into(),
        "--schema".into(),
        root.clone().into(),
        "--contract".into(),
        contract.into(),
        "--language".into(),
        "rust".into(),
        "--world".into(),
        "closed-schema".into(),
        "--with-codec".into(),
        "--output".into(),
        generated.join("service").into(),
    ];
    let mut report = Vec::new();
    ams_gra_codegen_oms::run(arguments, &mut report).map_err(|error| {
        format!(
            "production service-generate failed: {error}\n{}",
            String::from_utf8_lossy(&report)
        )
    })?;
    // A path module compiles the actual generated source and its siblings. No
    // rewriting, copied model, facade-test dependency, or handwritten codec.
    std::fs::write(
        out.join("generated.rs"),
        format!(
            "#[allow(dead_code, clippy::all, clippy::pedantic)]\n#[path = {:?}]\npub mod generated;\n",
            generated.join("service/service_api.rs")
        ),
    )
    .map_err(|error| error.to_string())?;
    println!("cargo:rustc-env=AMS_GRA_UCI_RELEASE_BUILD={release}");
    println!("cargo:rustc-env=AMS_GRA_SCHEMA_VERSION_BUILD=002.{minor}.0");
    println!("cargo:rustc-env=AMS_GRA_UCI_ROOT_BUILD={}", root.display());
    Ok(())
}

fn main() {
    if let Err(error) = generate() {
        panic!("rust-uci-client build configuration: {error}");
    }
}
