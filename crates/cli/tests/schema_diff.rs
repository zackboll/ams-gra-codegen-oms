use ams_gra_oms_ir::*;
use ams_gra_oms_schema_diff::*;
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/schema-diff")
        .join(name)
}
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn task069_cli_usage_controls() {
    for args in [
        vec!["--after", "a"],
        vec!["--before", "b"],
        vec!["--before", "b", "--before", "c", "--after", "a"],
        vec!["--before", "b", "--after", "a", "--after", "c"],
        vec!["--before", "b", "--after", "a", "--unknown", "x"],
        vec!["--before", "b", "--after", "a", "--format", "json"],
        vec!["--before", "b", "--after", "a", "--world", "closed-schema"],
        vec!["--before", "b", "--after", "a", "--language", "rust"],
        vec!["--before", "b", "--after", "a", "--output", "out"],
        vec!["--before", "b", "--after", "a", "--contract", "x"],
        vec![
            "--before", "b", "--after", "a", "--format", "tsv", "--format", "text",
        ],
    ] {
        let mut full = vec!["schema-diff"];
        full.extend(args);
        let out = cli(&full);
        assert_eq!(out.status.code(), Some(2), "{full:?}");
        assert!(out.stdout.is_empty());
    }
}
#[test]
fn task069_cli_synthetic_exact_and_deterministic() {
    let a = fixture("before.xsd");
    let b = fixture("after.xsd");
    let noise = fixture("noise.xsd");
    let load = |p: &Path| {
        let s = load_schema_set_with_overlays(p, &[]).unwrap();
        s.validate().unwrap();
        s
    };
    let before = load(&a);
    let after = load(&b);
    let d = compare_schemas(&before, &after);
    assert_eq!(
        d.types,
        Counts {
            before: 2,
            after: 2,
            added: 0,
            removed: 0,
            changed: 2,
            unchanged: 0
        }
    );
    assert_eq!(
        d.messages,
        Counts {
            before: 1,
            after: 2,
            added: 1,
            removed: 0,
            changed: 0,
            unchanged: 1
        }
    );
    assert_eq!(
        d.changes,
        vec![
            Change {
                category: Category::Type,
                name: QualifiedName::new("urn:diff", "Record"),
                member: None,
                property: Property::Order,
                kind: ChangeKind::MemberOrderChanged,
                before: Some(Value::Order(vec!["a".into(), "b".into()])),
                after: Some(Value::Order(vec!["b".into(), "a".into()]))
            },
            Change {
                category: Category::Type,
                name: QualifiedName::new("urn:diff", "Text"),
                member: None,
                property: Property::MaxLength,
                kind: ChangeKind::ConstraintChanged,
                before: Some(Value::Unsigned(8)),
                after: Some(Value::Unsigned(9))
            },
            Change {
                category: Category::Message,
                name: QualifiedName::new("urn:diff", "NewMessage"),
                member: None,
                property: Property::Presence,
                kind: ChangeKind::MessageAdded,
                before: None,
                after: Some(Value::Reference(TypeRef::named(QualifiedName::new(
                    "urn:diff", "Text"
                ))))
            },
        ]
    );
    assert!(compare_schemas(&before, &load(&noise)).changes.is_empty());
    let args = [
        "schema-diff",
        "--before",
        a.to_str().unwrap(),
        "--after",
        b.to_str().unwrap(),
        "--format",
        "tsv",
    ];
    let one = cli(&args);
    let two = cli(&args);
    assert_eq!(one.status.code(), Some(0));
    assert_eq!(one.stdout, two.stdout);
    assert_eq!(one.stdout, d.to_tsv().as_bytes());
    let out = cli(&args[..5]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(out.stdout, d.to_text().as_bytes());
}
#[test]
fn task069_cli_errors_are_not_empty_diffs() {
    let a = fixture("before.xsd");
    for extra in [
        vec!["--before-overlay", "/nonexistent/task069.xsd"],
        vec!["--after-overlay", "/nonexistent/task069.xsd"],
    ] {
        let mut args = vec![
            "schema-diff",
            "--before",
            a.to_str().unwrap(),
            "--after",
            a.to_str().unwrap(),
        ];
        args.extend(extra);
        let out = cli(&args);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
    }
    let out = cli(&[
        "schema-diff",
        "--before",
        "/nonexistent/task069.xsd",
        "--after",
        a.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken output"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let args = [
        "schema-diff",
        "--before",
        a.to_str().unwrap(),
        "--after",
        a.to_str().unwrap(),
    ]
    .map(std::ffi::OsString::from);
    assert_eq!(
        ams_gra_codegen_oms::run(args, &mut Broken)
            .unwrap_err()
            .exit_code(),
        1
    );
}

#[test]
fn task069_cli_overlay_order_and_malformed_schema() {
    let dir = std::env::temp_dir().join(format!("task069-overlay-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let root = dir.join("root.xsd");
    let first = dir.join("first.xsd");
    let second = dir.join("second.xsd");
    let schema = |body: &str| {
        format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:diff" targetNamespace="urn:diff">{body}</xs:schema>"#
        )
    };
    std::fs::write(&root, schema("")).unwrap();
    let original = std::fs::read(&root).unwrap();
    std::fs::write(
        &first,
        schema(r#"<xs:simpleType name="First"><xs:restriction base="xs:string"/></xs:simpleType>"#),
    )
    .unwrap();
    std::fs::write(
        &second,
        schema(
            r#"<xs:simpleType name="Second"><xs:restriction base="xs:string"/></xs:simpleType>"#,
        ),
    )
    .unwrap();
    let s = load_schema_set_with_overlays(&root, &[second.clone(), first.clone()]).unwrap();
    assert_eq!(
        s.types
            .iter()
            .map(|t| t.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["Second", "First"]
    );
    let args = [
        "schema-diff",
        "--before",
        root.to_str().unwrap(),
        "--after",
        root.to_str().unwrap(),
        "--before-overlay",
        second.to_str().unwrap(),
        "--before-overlay",
        first.to_str().unwrap(),
        "--after-overlay",
        first.to_str().unwrap(),
        "--after-overlay",
        second.to_str().unwrap(),
        "--format",
        "tsv",
    ];
    let out = cli(&args);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        out.stdout,
        compare_schemas(
            &s,
            &load_schema_set_with_overlays(&root, &[first.clone(), second.clone()]).unwrap()
        )
        .to_tsv()
        .as_bytes()
    );
    assert_eq!(
        std::fs::read(&first).unwrap(),
        schema(r#"<xs:simpleType name="First"><xs:restriction base="xs:string"/></xs:simpleType>"#)
            .as_bytes()
    );
    assert_eq!(std::fs::read(&root).unwrap(), original);
    std::fs::write(&second, "not XML").unwrap();
    let out = cli(&args);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    std::fs::write(
        &second,
        schema(r#"<xs:simpleType name="Bad"><xs:restriction base="t:Missing"/></xs:simpleType>"#),
    )
    .unwrap();
    let out = cli(&args);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        3,
        "no output directory or generated files"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
