use ams_gra_codegen_oms::schema_impact::SchemaImpact;
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(n: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/schema-diff")
        .join(n)
}
fn report(a: &str, b: &str) -> SchemaImpact {
    let load = |n| load_schema_set_with_overlays(&fixture(n), &[]).unwrap();
    SchemaImpact::analyze(&load(a), &load(b)).unwrap()
}

#[test]
fn exact_text_tsv_and_empty_reports() {
    for (before, after, stem) in [
        ("before.xsd", "after.xsd", "changed"),
        ("before.xsd", "before.xsd", "empty"),
    ] {
        let r = report(before, after);
        for (format, expected) in [("text", r.to_text()), ("tsv", r.to_tsv())] {
            let out = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
                .args([
                    "schema-impact",
                    "--before",
                    fixture(before).to_str().unwrap(),
                    "--after",
                    fixture(after).to_str().unwrap(),
                    "--format",
                    format,
                ])
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(0));
            assert_eq!(out.stdout, expected.as_bytes());
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../tests/fixtures/schema-impact/{stem}.{format}"
            ));
            assert_eq!(expected, std::fs::read_to_string(path).unwrap());
            if format == "tsv" {
                assert!(expected.lines().all(|l| l.split('\t').count() == 11));
            }
        }
    }
}

#[test]
fn noise_is_not_semantic_exposure() {
    let r = report("before.xsd", "noise.xsd");
    assert!(r.before.is_empty() && r.after.is_empty() && r.direct().next().is_none());
}

#[test]
fn namespace_identity_permutations_and_hostile_tsv_cells() {
    let load = || load_schema_set_with_overlays(&fixture("before.xsd"), &[]).unwrap();
    let before = load();
    let mut after = before.clone();
    let namespace = "urn:new\t\n\r\\";
    after.namespaces[0].uri = namespace.into();
    for t in &mut after.types {
        t.name.namespace_uri = namespace.into();
        if let ams_gra_oms_ir::TypeKind::Record { fields } = &mut t.kind {
            for f in fields {
                if let ams_gra_oms_ir::TypeRefTarget::Named(n) = &mut f.type_ref.target {
                    n.namespace_uri = namespace.into();
                }
                if f.wire_namespace_uri.as_deref() == Some("urn:diff") {
                    f.wire_namespace_uri = Some(namespace.into());
                }
            }
        }
    }
    for m in &mut after.messages {
        m.name.namespace_uri = namespace.into();
        if let ams_gra_oms_ir::TypeRefTarget::Named(n) = &mut m.payload_type.target {
            n.namespace_uri = namespace.into();
        }
    }
    let r = SchemaImpact::analyze(&before, &after).unwrap();
    assert_eq!(r.direct().count(), 2);
    assert!(r.before.iter().all(|e| e.seed.namespace_uri == "urn:diff"));
    assert!(r.after.iter().all(|e| e.seed.namespace_uri == namespace));
    assert!(r.to_tsv().lines().all(|l| l.split('\t').count() == 11));
    assert!(r.to_tsv().contains("urn:new\\t\\n\\r\\\\"));
    after.types.reverse();
    after.messages.reverse();
    assert_eq!(
        r.to_tsv(),
        SchemaImpact::analyze(&before, &after).unwrap().to_tsv()
    );
}

#[test]
fn failing_output_writer_is_execution_failure() {
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken output"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let path = fixture("before.xsd");
    let args = [
        "schema-impact",
        "--before",
        path.to_str().unwrap(),
        "--after",
        path.to_str().unwrap(),
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
fn multiple_properties_one_seed_and_independent_snapshots() {
    let dir = std::env::temp_dir().join(format!("task078-semantics-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let load = |name: &str, body: &str| {
        let path = dir.join(name);
        std::fs::write(&path, format!(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:i" xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam" targetNamespace="urn:i">{body}</xs:schema>"#)).unwrap();
        load_schema_set_with_overlays(&path, &[]).unwrap()
    };
    // Old Root -> Mid, new Mid -> Leaf: union would invent Root -> Mid -> Leaf.
    let mut before = load(
        "before.xsd",
        r#"<xs:complexType name="Root"><xs:sequence><xs:element name="x" type="t:Mid"/></xs:sequence></xs:complexType><xs:complexType name="Mid"><xs:sequence/></xs:complexType><xs:simpleType name="Leaf"><xs:restriction base="xs:string"><xs:minLength value="1"/><xs:maxLength value="8"/></xs:restriction></xs:simpleType><xs:complexType name="Removed"><xs:sequence/></xs:complexType><xs:element uci:version="1" name="M" type="t:Root"/><xs:element uci:version="1" name="Gone" type="t:Removed"/>"#,
    );
    let mut after = load(
        "after.xsd",
        r#"<xs:complexType name="Root"><xs:sequence/></xs:complexType><xs:complexType name="Mid"><xs:sequence><xs:element name="x" type="t:Leaf"/></xs:sequence></xs:complexType><xs:simpleType name="Leaf"><xs:restriction base="xs:string"><xs:minLength value="2"/><xs:maxLength value="9"/></xs:restriction></xs:simpleType><xs:complexType name="Added"><xs:sequence/></xs:complexType><xs:element uci:version="1" name="M" type="t:Root"/><xs:element uci:version="1" name="New" type="t:Added"/>"#,
    );
    let primitive = |kind| ams_gra_oms_ir::MessageDecl {
        name: ams_gra_oms_ir::QualifiedName::new("urn:i", "P"),
        payload_type: ams_gra_oms_ir::TypeRef::primitive(kind),
        documentation: None,
        source: ams_gra_oms_ir::SourceRef {
            document: "IR-only".into(),
            line: None,
        },
    };
    before
        .messages
        .push(primitive(ams_gra_oms_ir::PrimitiveKind::String));
    after
        .messages
        .push(primitive(ams_gra_oms_ir::PrimitiveKind::Boolean));
    let mut leaf_message = primitive(ams_gra_oms_ir::PrimitiveKind::String);
    leaf_message.name.local_name = "LeafMessage".into();
    leaf_message.payload_type =
        ams_gra_oms_ir::TypeRef::named(ams_gra_oms_ir::QualifiedName::new("urn:i", "Leaf"));
    before.messages.push(leaf_message.clone());
    after.messages.push(leaf_message);
    let r = SchemaImpact::analyze(&before, &after).unwrap();
    assert_eq!(r.direct().count(), 3);
    for rows in [&r.before, &r.after] {
        assert_eq!(
            rows.iter()
                .filter(|e| e.message.local_name == "LeafMessage" && e.seed.local_name == "Leaf")
                .count(),
            1
        );
    }
    assert!(
        !r.before
            .iter()
            .chain(&r.after)
            .any(|e| e.message.local_name == "M" && e.seed.local_name == "Leaf")
    );
    assert!(
        r.before
            .iter()
            .any(|e| e.message.local_name == "Gone" && e.seed.local_name == "Removed")
    );
    assert!(
        r.after
            .iter()
            .any(|e| e.message.local_name == "New" && e.seed.local_name == "Added")
    );
    for rows in [&r.before, &r.after] {
        let pairs: std::collections::BTreeSet<_> =
            rows.iter().map(|r| (&r.message, &r.seed)).collect();
        assert_eq!(rows.len(), pairs.len());
    }
    std::fs::remove_dir_all(dir).unwrap();
}
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn task078_cli_usage_controls() {
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
        let mut full = vec!["schema-impact"];
        full.extend(args);
        let out = cli(&full);
        assert_eq!(out.status.code(), Some(2), "{full:?}");
        assert!(out.stdout.is_empty());
    }
}
#[test]
fn task078_cli_overlay_order_and_malformed_schema() {
    let dir = std::env::temp_dir().join(format!("task078-overlay-{}", std::process::id()));
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
        "schema-impact",
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
        SchemaImpact::analyze(
            &s,
            &load_schema_set_with_overlays(&root, &[first.clone(), second.clone()]).unwrap()
        )
        .unwrap()
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
