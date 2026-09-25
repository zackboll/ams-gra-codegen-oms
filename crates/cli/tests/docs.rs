use std::ffi::OsString;
use std::path::PathBuf;

fn run(args: &[&str]) -> Result<String, ams_gra_codegen_oms::CliError> {
    let mut output = Vec::new();
    ams_gra_codegen_oms::run(args.iter().map(OsString::from), &mut output)?;
    Ok(String::from_utf8(output).unwrap())
}

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-plan")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn help_and_usage() {
    assert!(
        run(&["docs", "--help"])
            .unwrap()
            .contains("offline HTML browser")
    );
    for (args, diagnostic) in [
        (vec!["docs"], "--schema"),
        (vec!["docs", "--schema", "root.xsd"], "--output"),
        (vec!["docs", "--world", "closed-schema"], "unknown option"),
        (vec!["docs", "--language", "rust"], "unknown option"),
        (vec!["docs", "--bogus", "value"], "unknown option"),
    ] {
        assert!(run(&args).unwrap_err().to_string().contains(diagnostic));
    }
}

#[test]
fn overlays_and_failure_paths() {
    let destination = std::env::temp_dir().join(format!("task043-docs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&destination);
    let root = fixture("root.xsd");
    let overlay = fixture("private-overlay.xsd");
    let dest = destination.to_string_lossy().into_owned();
    let result = run(&[
        "docs",
        "-s",
        &root,
        "--overlay",
        &overlay,
        "--overlay",
        &overlay,
        "-o",
        &dest,
    ])
    .unwrap();
    assert!(result.contains("documentation file(s)"));
    let index = std::fs::read_to_string(destination.join("index.html")).unwrap();
    assert!(index.contains("PrivateReportType"));
    assert!(index.contains("PrivateReport"));
    let bad = destination.join("invalid");
    let bad_path = bad.to_string_lossy().into_owned();
    assert!(run(&["docs", "-s", "missing.xsd", "-o", &bad_path]).is_err());
    assert!(!bad.exists());
    let file = destination.join("index.html");
    assert!(run(&["docs", "-s", &root, "-o", &file.to_string_lossy()]).is_err());
    std::fs::remove_dir_all(destination).unwrap();
}
