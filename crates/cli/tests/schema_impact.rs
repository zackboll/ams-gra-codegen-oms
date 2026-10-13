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
        for (format, expected) in [
            ("text", r.to_text()),
            ("tsv", r.to_tsv()),
            ("json", r.to_json().unwrap()),
        ] {
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
    let json = r.to_json().unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["directMessageChanges"][1]["message"][0], namespace);
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
    let j: serde_json::Value = serde_json::from_str(&r.to_json().unwrap()).unwrap();
    let changes = j["directMessageChanges"].as_array().unwrap();
    assert_eq!(
        changes[0],
        serde_json::json!({"message":["urn:i","Gone"],"change":"MESSAGE_REMOVED","before":{"kind":"named","qname":["urn:i","Removed"],"binaryEncoding":null},"after":null})
    );
    assert_eq!(changes[1]["before"], serde_json::Value::Null);
    assert_eq!(
        changes[1]["after"]["qname"],
        serde_json::json!(["urn:i", "Added"])
    );
    assert_eq!(changes[2]["before"]["primitive"], "String");
    assert_eq!(changes[2]["after"]["primitive"], "Boolean");
    for (side, label, seed) in [
        ("before", "REMOVED", "Removed"),
        ("after", "ADDED", "Added"),
    ] {
        assert!(
            j[side]["exposures"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |e| e["seed"] == serde_json::json!(["urn:i", seed]) && e["seedChange"] == label
                )
        );
        assert!(
            !j[side]["exposures"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["message"][1] == "P")
        );
    }

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
        vec!["--before", "b", "--after", "a", "--format", "xml"],
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
    let mut json_args = args;
    json_args[14] = "json";
    let json_out = cli(&json_args);
    assert_eq!(json_out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(json["before"]["types"], 2);
    assert_eq!(json["after"]["types"], 2);
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
    let json_out = cli(&json_args);
    assert_eq!(json_out.status.code(), Some(1));
    assert!(json_out.stdout.is_empty());
    let out = cli(&args);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    std::fs::write(
        &second,
        schema(r#"<xs:simpleType name="Bad"><xs:restriction base="t:Missing"/></xs:simpleType>"#),
    )
    .unwrap();
    let json_out = cli(&json_args);
    assert_eq!(json_out.status.code(), Some(1));
    assert!(json_out.stdout.is_empty());
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

#[test]
fn json_cli_default_rejections_failures_and_writer() {
    let path = fixture("before.xsd");
    let base = [
        "schema-impact",
        "--before",
        path.to_str().unwrap(),
        "--after",
        path.to_str().unwrap(),
    ];
    assert_eq!(
        cli(&base).stdout,
        report("before.xsd", "before.xsd").to_text().as_bytes()
    );
    assert_eq!(
        cli(&[
            "schema-diff",
            "--before",
            "b",
            "--after",
            "a",
            "--format",
            "json"
        ])
        .status
        .code(),
        Some(2)
    );
    let mut args = base.to_vec();
    args.extend(["--format", "json", "--format", "json"]);
    assert_eq!(cli(&args).status.code(), Some(2));
    let out = cli(&[
        "schema-impact",
        "--before",
        "/nonexistent-task083",
        "--after",
        path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    struct Broken;
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut args = base.to_vec();
    args.extend(["--format", "json"]);
    assert_eq!(
        ams_gra_codegen_oms::run(args.into_iter().map(std::ffi::OsString::from), &mut Broken)
            .unwrap_err()
            .exit_code(),
        1
    );
    let help = cli(&["schema-impact", "--help"]);
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("schema-impact-json-v1")
    );
}

#[test]
fn json_binary_provenance_and_all_primitive_labels() {
    use ams_gra_oms_ir::*;
    let original = load_schema_set_with_overlays(&fixture("before.xsd"), &[]).unwrap();
    for (payload, label) in [
        (
            TypeRef::primitive(PrimitiveKind::Binary),
            serde_json::Value::Null,
        ),
        (
            TypeRef::binary(BinaryLexicalEncoding::HexBinary),
            serde_json::json!("HexBinary"),
        ),
        (
            TypeRef::binary(BinaryLexicalEncoding::Base64Binary),
            serde_json::json!("Base64Binary"),
        ),
    ] {
        let mut before = original.clone();
        let mut after = original.clone();
        before.messages[0].payload_type = payload;
        after.messages[0].payload_type = TypeRef::primitive(PrimitiveKind::Boolean);
        let j: serde_json::Value = serde_json::from_str(
            &SchemaImpact::analyze(&before, &after)
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            j["directMessageChanges"][0]["before"],
            serde_json::json!({"kind":"primitive","primitive":"Binary","binaryEncoding":label})
        );
        assert_eq!(j["before"]["exposures"], serde_json::json!([]));
    }
    for (p, label) in [
        (PrimitiveKind::Boolean, "Boolean"),
        (PrimitiveKind::SignedInteger, "SignedInteger"),
        (PrimitiveKind::UnsignedInteger, "UnsignedInteger"),
        (PrimitiveKind::Decimal, "Decimal"),
        (PrimitiveKind::Float32, "Float32"),
        (PrimitiveKind::Float64, "Float64"),
        (PrimitiveKind::String, "String"),
        (PrimitiveKind::DateTime, "DateTime"),
        (PrimitiveKind::Time, "Time"),
        (PrimitiveKind::Duration, "Duration"),
    ] {
        let mut after = original.clone();
        after.messages[0].payload_type = TypeRef::primitive(p);
        let j: serde_json::Value = serde_json::from_str(
            &SchemaImpact::analyze(&original, &after)
                .unwrap()
                .to_json()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(j["directMessageChanges"][0]["after"]["primitive"], label);
    }
}

#[test]
fn json_nested_seed_counts_cycles_ties_and_no_derived_expansion() {
    use ams_gra_oms_ir::*;
    let mut before = load_schema_set_with_overlays(&fixture("before.xsd"), &[]).unwrap();
    // Record -> Text is optional/repeated; add a duplicate edge and a cycle.
    let TypeKind::Record { fields } = &mut before.types[1].kind else {
        panic!()
    };
    fields[0].cardinality = Cardinality {
        min_occurs: 0,
        max_occurs: None,
    };
    let mut duplicate = fields[0].clone();
    duplicate.name = "duplicate".into();
    fields.push(duplicate);
    let cycle = fields[0].clone();
    let mut back = cycle;
    back.name = "back".into();
    back.type_ref = TypeRef::named(QualifiedName::new("urn:diff", "Record"));
    before.types[0].kind = TypeKind::Record { fields: vec![back] };
    before.types[0].base_type = None;
    let mut second = before.messages[0].clone();
    second.name.local_name = "Second".into();
    before.messages.push(second);
    let mut after = before.clone();
    after.types[0].constraints.min_length = Some(1);
    after.types[0].constraints.max_length = Some(9);
    let r = SchemaImpact::analyze(&before, &after).unwrap();
    let bytes = r.to_json().unwrap();
    let j: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    for side in ["before", "after"] {
        assert_eq!(j[side]["exposedUniqueMessages"], 2);
        assert_eq!(j[side]["exposurePairs"], 2);
        assert_eq!(
            j[side]["exposures"][0],
            serde_json::json!({"message":["urn:diff","Message"],"seed":["urn:diff","Text"],"seedChange":"CHANGED","witness":[["urn:diff","Record"],["urn:diff","Text"]]})
        );
    }
    assert_eq!(j["directUniqueMessages"], 0);
    after.types.reverse();
    after.messages.reverse();
    before.types.reverse();
    before.messages.reverse();
    assert_eq!(
        bytes,
        SchemaImpact::analyze(&before, &after)
            .unwrap()
            .to_json()
            .unwrap()
    );
    assert_eq!(bytes, r.to_json().unwrap());
    let mut derived = before.types[0].clone();
    derived.name.local_name = "Derived".into();
    derived.base_type = Some(TypeRef::named(QualifiedName::new("urn:diff", "Record")));
    let mut with_derived = before.clone();
    with_derived.types.push(derived);
    with_derived
        .types
        .iter_mut()
        .find(|t| t.name.local_name == "Record")
        .unwrap()
        .is_abstract = true;
    let mut abstract_before = before.clone();
    abstract_before
        .types
        .iter_mut()
        .find(|t| t.name.local_name == "Record")
        .unwrap()
        .is_abstract = true;
    let j: serde_json::Value = serde_json::from_str(
        &SchemaImpact::analyze(&abstract_before, &with_derived)
            .unwrap()
            .to_json()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(j["after"]["exposures"], serde_json::json!([]));
}

#[test]
fn json_hostile_utf8_empty_version_and_namespace_identity() {
    let before = load_schema_set_with_overlays(&fixture("before.xsd"), &[]).unwrap();
    let mut after = before.clone();
    let hostile = "雪\u{0000}\t\r\n\"</script>\\../../tmp/task083";
    let mut namespace = after.namespaces[0].clone();
    namespace.uri = hostile.into();
    after.namespaces.push(namespace);
    after.schema_version = Some(String::new());
    after.messages[0].name.namespace_uri = hostile.into();
    // Same local name in two namespaces must remain two independent identities.
    after.messages.push(before.messages[0].clone());
    let r = SchemaImpact::analyze(&before, &after).unwrap();
    let j: serde_json::Value = serde_json::from_str(&r.to_json().unwrap()).unwrap();
    assert_eq!(j["after"]["schemaVersion"], "");
    assert_eq!(j["before"]["schemaVersion"], serde_json::Value::Null);
    assert_eq!(
        j["directMessageChanges"][0]["message"],
        serde_json::json!([hostile, "Message"])
    );
    assert_eq!(j["directUniqueMessages"], 1);
}

#[test]
fn json_shortest_full_qname_tie_and_invalid_typed_value() {
    use ams_gra_oms_ir::*;
    let mut before = load_schema_set_with_overlays(&fixture("before.xsd"), &[]).unwrap();
    let mut a = before.types[1].clone();
    a.name.local_name = "A".into();
    let TypeKind::Record { fields } = &mut a.kind else {
        panic!()
    };
    fields.truncate(1);
    let mut b = a.clone();
    b.name.local_name = "B".into();
    let TypeKind::Record { fields } = &mut before.types[1].kind else {
        panic!()
    };
    fields[0].type_ref = TypeRef::named(a.name.clone());
    fields[1].type_ref = TypeRef::named(b.name.clone());
    before.types.extend([b, a]);
    let mut after = before.clone();
    after.types[0].constraints.max_length = Some(10);
    let mut r = SchemaImpact::analyze(&before, &after).unwrap();
    let j: serde_json::Value = serde_json::from_str(&r.to_json().unwrap()).unwrap();
    for side in ["before", "after"] {
        assert_eq!(
            j[side]["exposures"][0]["witness"],
            serde_json::json!([
                ["urn:diff", "Record"],
                ["urn:diff", "A"],
                ["urn:diff", "Text"]
            ])
        );
    }
    before.types.reverse();
    after.types.reverse();
    assert_eq!(
        r.to_json().unwrap(),
        SchemaImpact::analyze(&before, &after)
            .unwrap()
            .to_json()
            .unwrap()
    );
    // Public typed reports may be manually altered: reject unexpected values,
    // rather than serialize a diagnostic or conflate an authored empty string.
    r.diff.changes.push(ams_gra_oms_schema_diff::Change {
        category: ams_gra_oms_schema_diff::Category::Message,
        name: QualifiedName::new("urn:diff", "Message"),
        member: None,
        property: ams_gra_oms_schema_diff::Property::Payload,
        kind: ams_gra_oms_schema_diff::ChangeKind::MessageChanged,
        before: Some(ams_gra_oms_schema_diff::Value::Text(String::new())),
        after: None,
    });
    assert!(r.to_json().is_err());
}
