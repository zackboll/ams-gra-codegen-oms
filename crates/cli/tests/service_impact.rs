use ams_gra_codegen_oms::{schema_impact::SchemaImpact, service_impact::ServiceImpact};
use ams_gra_oms_codegen_core::resolve_service_plan;
use ams_gra_oms_ir::*;
use ams_gra_oms_service_contract::{Contract, load_contract, parse_yaml};
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}
fn fixture(name: &str) -> PathBuf {
    fixtures().join("service-impact").join(name)
}
fn schema(name: &str) -> SchemaIr {
    load_schema_set_with_overlays(&fixture(name), &[]).unwrap()
}
fn contract() -> Contract {
    load_contract(&fixture("service.yaml")).unwrap()
}
fn cli(after: &str, extra: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["service-impact", "--contract"])
        .arg(fixture("service.yaml"))
        .arg("--before")
        .arg(fixture("before.xsd"))
        .arg("--after")
        .arg(fixture(after))
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn baseline_join_counts_order_and_independent_task078_subset() {
    let (before, after, c) = (schema("before.xsd"), schema("after.xsd"), contract());
    let r = ServiceImpact::analyze(&c, &before, &after).unwrap();
    assert_eq!(r.plan, resolve_service_plan(&c, &before).unwrap());
    assert_eq!(
        (r.total_occurrences(), r.oms_occurrences(), r.selected.len()),
        (4, 3, 1)
    );
    assert_eq!(
        (r.direct_unique_messages(), r.before.len(), r.after.len()),
        (0, 1, 1)
    );
    assert_eq!(r.selected[0].status, "SAME_PAYLOAD");
    for (side, actual) in [(true, &r.before), (false, &r.after)] {
        let whole = SchemaImpact::analyze(&before, &after).unwrap();
        let expected: Vec<_> = if side { whole.before } else { whole.after }
            .into_iter()
            .filter(|e| e.message == QualifiedName::new("urn:impact", "Selected"))
            .collect();
        assert_eq!(actual, &expected);
        assert_eq!(
            actual[0]
                .witness
                .iter()
                .map(|n| n.local_name.as_str())
                .collect::<Vec<_>>(),
            ["Payload", "Nested"]
        );
    }
    let tsv = r.to_tsv();
    let bindings: Vec<_> = tsv
        .lines()
        .filter(|l| l.starts_with("OCCURRENCE"))
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .collect();
    assert_eq!(
        bindings
            .iter()
            .map(|r| (r[2], r[3], r[4], r[6]))
            .collect::<Vec<_>>(),
        [
            ("z-first", "input", "input", "first.topic"),
            ("z-first", "output", "output", "second.topic"),
            ("a-second", "input", "input", "third.topic")
        ]
    );
    assert!(!tsv.contains("UnselectedType"));
    assert!(tsv.contains("logical-uci") && tsv.contains("old-root") && tsv.contains("new-root"));
    for format in ["text", "tsv"] {
        let out = cli("after.xsd", &["--format", format]);
        assert_eq!(out.status.code(), Some(0), "{:?}", out.stderr);
        let expected = if format == "text" {
            r.to_text()
        } else {
            r.to_tsv()
        };
        assert_eq!(out.stdout, expected.as_bytes());
        assert_eq!(out.stdout, cli("after.xsd", &["--format", format]).stdout);
        assert_eq!(
            expected,
            std::fs::read_to_string(fixture(&format!("changed.{format}"))).unwrap()
        );
    }
}

#[test]
fn valid_removal_and_full_qname_tracking_without_after_planning() {
    let before = schema("before.xsd");
    let c = contract();
    let out = cli("removed.xsd", &[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("MESSAGE_REMOVED")
    );
    let mut after = before.clone();
    let mut other = after
        .messages
        .iter()
        .find(|m| m.name.local_name == "Selected")
        .unwrap()
        .clone();
    other.name.namespace_uri = "urn:other".into();
    after.namespaces.push(NamespaceDecl {
        uri: "urn:other".into(),
        preferred_prefix: None,
    });
    after.messages.push(other);
    assert!(resolve_service_plan(&c, &after).is_err());
    let r = ServiceImpact::analyze(&c, &before, &after).unwrap();
    assert_eq!(r.selected[0].status, "SAME_PAYLOAD");
    assert!(r.direct.is_empty());
    after.messages.retain(|m| m.name != r.selected[0].message);
    let r = ServiceImpact::analyze(&c, &before, &after).unwrap();
    assert_eq!(r.selected[0].status, "MISSING");
    assert_eq!(r.direct_unique_messages(), 1);
    assert!(r.after.is_empty());
    let error = ServiceImpact::analyze(&c, &after, &before).unwrap();
    assert_eq!(error.selected[0].message.namespace_uri, "urn:other");
    let replacement = cli("other.xsd", &["--format", "tsv"]);
    assert_eq!(replacement.status.code(), Some(0));
    let output = String::from_utf8(replacement.stdout).unwrap();
    assert!(output.contains("MISSING") && output.contains("MESSAGE_REMOVED"));
    assert!(!output.contains("MESSAGE_ADDED"));
    let additional = cli("additional.xsd", &["--format", "tsv"]);
    assert_eq!(additional.status.code(), Some(0), "{:?}", additional.stderr);
    assert!(
        String::from_utf8(additional.stdout)
            .unwrap()
            .contains("SAME_PAYLOAD")
    );
}

#[test]
fn direct_payload_change_and_separate_side_paths() {
    let before = schema("before.xsd");
    let mut after = schema("after.xsd");
    after
        .messages
        .iter_mut()
        .find(|m| m.name.local_name == "Selected")
        .unwrap()
        .payload_type = TypeRef::named(QualifiedName::new("urn:impact", "UnselectedType"));
    let r = ServiceImpact::analyze(&contract(), &before, &after).unwrap();
    assert_eq!(r.selected[0].status, "CHANGED_PAYLOAD");
    assert_eq!(r.direct_unique_messages(), 1);
    assert_eq!(r.before[0].seed.local_name, "Nested");
    assert_eq!(r.after[0].seed.local_name, "UnselectedType");
    assert_eq!(r.after[0].witness.len(), 1);
}

#[test]
fn unknown_ambiguous_and_invalid_closures_fail() {
    let before = schema("before.xsd");
    let mut c = std::fs::read_to_string(fixture("service.yaml")).unwrap();
    c = c.replace("message: Selected", "message: Missing");
    let error = ServiceImpact::analyze(&parse_yaml(&c).unwrap(), &before, &before).unwrap_err();
    assert!(error.contains("z-first") && error.contains("input") && error.contains("Missing"));
    let mut ambiguous = before.clone();
    let mut other = ambiguous.messages[1].clone();
    other.name.namespace_uri = "urn:other".into();
    ambiguous.namespaces.push(NamespaceDecl {
        uri: "urn:other".into(),
        preferred_prefix: None,
    });
    ambiguous.messages.push(other);
    assert!(
        ServiceImpact::analyze(&contract(), &ambiguous, &before)
            .unwrap_err()
            .contains("ambiguous")
    );
    let mut broken = before.clone();
    broken.types.retain(|t| t.name.local_name != "Nested");
    assert!(ServiceImpact::analyze(&contract(), &before, &broken).is_err());
    assert_eq!(cli("broken.xsd", &[]).status.code(), Some(1));
    for (baseline, term) in [
        ("removed.xsd", "not found"),
        ("additional.xsd", "ambiguous"),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(["service-impact", "--contract"])
            .arg(fixture("service.yaml"))
            .arg("--before")
            .arg(fixture(baseline))
            .arg("--after")
            .arg(fixture("after.xsd"))
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8(out.stderr).unwrap().contains(term));
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn abstract_descendants_are_not_static_exposure() {
    let mut before = schema("before.xsd");
    before
        .types
        .iter_mut()
        .find(|t| t.name.local_name == "Payload")
        .unwrap()
        .is_abstract = true;
    let mut after = before.clone();
    let mut child = before
        .types
        .iter()
        .find(|t| t.name.local_name == "Payload")
        .unwrap()
        .clone();
    child.name.local_name = "Child".into();
    child.is_abstract = false;
    child.base_type = Some(TypeRef::named(QualifiedName::new("urn:impact", "Payload")));
    child.kind = TypeKind::Record { fields: vec![] };
    after.types.push(child);
    let r = ServiceImpact::analyze(&contract(), &before, &after).unwrap();
    assert!(r.before.is_empty() && r.after.is_empty() && r.direct.is_empty());
    assert!(
        r.to_text()
            .contains("No service-selected static dependency impact")
    );
}

#[test]
fn hostile_cells_optional_presence_empty_selection_and_writer_failure() {
    let before = schema("before.xsd");
    let mut c = contract();
    c.service.name = "hostile\t\n\r\\\u{1}".into();
    c.standards.ams_gra_version = Some(String::new());
    let r = ServiceImpact::analyze(&c, &before, &before).unwrap();
    assert!(r.to_tsv().lines().all(|l| l.split('\t').count() == 21));
    assert!(r.to_text().contains("hostile\\t\\n\\r\\\\\\u{1}"));
    assert!(r.to_tsv().contains("ams_gra_version\ttrue\t"));
    c.standards.ams_gra_version = None;
    assert!(
        ServiceImpact::analyze(&c, &before, &before)
            .unwrap()
            .to_tsv()
            .contains("ams_gra_version\tfalse\t")
    );
    let empty = load_contract(&fixtures().join("service-routes/empty.yaml")).unwrap();
    let r = ServiceImpact::analyze(&empty, &before, &before).unwrap();
    assert_eq!(
        (r.selected.len(), r.oms_occurrences(), r.total_occurrences()),
        (0, 0, 1)
    );
    for (format, expected) in [("text", r.to_text()), ("tsv", r.to_tsv())] {
        assert_eq!(
            expected,
            std::fs::read_to_string(fixture(&format!("empty.{format}"))).unwrap()
        );
        let out = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(["service-impact", "--contract"])
            .arg(fixtures().join("service-routes/empty.yaml"))
            .arg("--before")
            .arg(fixture("before.xsd"))
            .arg("--after")
            .arg(fixture("before.xsd"))
            .args(["--format", format])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(out.stdout, expected.as_bytes());
    }
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken writer"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let args = [
        "service-impact".into(),
        "--contract".into(),
        fixture("service.yaml").into_os_string(),
        "--before".into(),
        fixture("before.xsd").into_os_string(),
        "--after".into(),
        fixture("after.xsd").into_os_string(),
    ];
    assert!(
        ams_gra_codegen_oms::run(args, &mut Broken)
            .unwrap_err()
            .to_string()
            .contains("broken writer")
    );
}

#[test]
fn actual_cli_usage_errors_and_command_help() {
    for args in [
        vec!["service-impact"],
        vec!["service-impact", "--before", "file"],
        vec!["service-impact", "--contract", "file", "--before", "file"],
    ] {
        assert_eq!(
            Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
                .args(args)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2)
        );
    }
    for extra in [
        vec!["--contract", "duplicate"],
        vec!["--before", "duplicate"],
        vec!["--after", "duplicate"],
        vec!["--format", "text", "--format", "tsv"],
        vec!["--format", "json"],
        vec!["--before-extension", "broken"],
        vec!["--after-extension", "=path"],
        vec!["--before-extension", "extra=file"],
        vec!["--after-extension", "x=file", "--after-extension", "x=file"],
        vec!["--overlay", "file"],
        vec!["--extension", "id=file"],
        vec!["--world", "closed-schema"],
        vec!["--language", "rust"],
        vec!["--output", "/tmp/task079/forbidden"],
        vec!["--unknown", "x"],
        vec!["--format"],
    ] {
        let out = cli("after.xsd", &extra);
        assert_eq!(out.status.code(), Some(2), "{extra:?}: {:?}", out.stderr);
        assert!(out.stdout.is_empty());
    }
    let out = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(["service-impact", "--help"])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    for term in [
        "BEFORE_CONTRACT",
        "STATIC_NAMED_REFERENCES",
        "Removed",
        "EACH",
        "No AFTER",
    ] {
        assert!(text.contains(term));
    }
    assert!(!Path::new("/tmp/task079/forbidden").exists());
}

#[test]
fn extension_sets_independent_contract_order_and_failures() {
    let f = |name| fixtures().join("service-plan").join(name);
    let a = format!("ext-a-1.0={}", f("private-overlay.xsd").display());
    let b = format!("ext-b-2.0={}", f("private-overlay-b.xsd").display());
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .args(["service-impact", "--contract"])
            .arg(f("service-extension.yaml"))
            .arg("--before")
            .arg(f("root.xsd"))
            .arg("--after")
            .arg(f("root.xsd"))
            .args(args)
            .output()
            .unwrap()
    };
    let first = run(&[
        "--before-extension",
        &a,
        "--before-extension",
        &b,
        "--after-extension",
        &a,
        "--after-extension",
        &b,
    ]);
    assert_eq!(first.status.code(), Some(0), "{:?}", first.stderr);
    assert_eq!(
        first.stdout,
        run(&[
            "--before-extension",
            &b,
            "--before-extension",
            &a,
            "--after-extension",
            &b,
            "--after-extension",
            &a
        ])
        .stdout
    );
    // Explicitly different valid AFTER input for the same logical ID: the
    // private message disappears, but extension requirements still apply.
    let removed = format!("ext-a-1.0={}", f("root.xsd").display());
    let out = run(&[
        "--before-extension",
        &a,
        "--before-extension",
        &b,
        "--after-extension",
        &removed,
        "--after-extension",
        &b,
    ]);
    assert_eq!(out.status.code(), Some(0), "{:?}", out.stderr);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("MISSING") && text.contains("MESSAGE_REMOVED"));
    for args in [
        vec![],
        vec!["--before-extension", &a, "--before-extension", &b],
        vec!["--after-extension", &a, "--after-extension", &b],
        vec!["--before-extension", &a, "--before-extension", &a],
    ] {
        assert_eq!(run(&args).status.code(), Some(2));
    }
    let bad = "ext-a-1.0=/tmp/task079/absent.xsd";
    assert_eq!(
        run(&[
            "--before-extension",
            &a,
            "--before-extension",
            &b,
            "--after-extension",
            bad,
            "--after-extension",
            &b
        ])
        .status
        .code(),
        Some(1)
    );
    let duplicate = format!("ext-a-1.0={}", f("duplicate-private-aux.xsd").display());
    assert_eq!(
        run(&[
            "--before-extension",
            &a,
            "--before-extension",
            &b,
            "--after-extension",
            &b,
            "--after-extension",
            &duplicate
        ])
        .status
        .code(),
        Some(1)
    );
}
