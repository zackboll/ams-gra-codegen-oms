use ams_gra_oms_codegen_core::static_dependency_exposures;
use ams_gra_oms_ir::*;
use std::collections::BTreeSet;

fn q(n: &str) -> QualifiedName {
    QualifiedName::new("urn:test", n)
}
fn source() -> SourceRef {
    SourceRef {
        document: "fixture".into(),
        line: None,
    }
}
fn ty(n: &str, kind: TypeKind) -> TypeDecl {
    TypeDecl {
        name: q(n),
        kind,
        base_type: None,
        is_abstract: false,
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    }
}
fn field(n: &str, target: &str) -> FieldDecl {
    FieldDecl {
        name: n.into(),
        type_ref: TypeRef::named(q(target)),
        cardinality: Cardinality {
            min_occurs: 0,
            max_occurs: None,
        },
        nillable: false,
        wire_namespace_uri: None,
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    }
}
fn record(n: &str, targets: &[&str]) -> TypeDecl {
    ty(
        n,
        TypeKind::Record {
            fields: targets
                .iter()
                .enumerate()
                .map(|(i, t)| field(&format!("f{i}"), t))
                .collect(),
        },
    )
}
fn schema(types: Vec<TypeDecl>, root: &str) -> SchemaIr {
    SchemaIr {
        schema_version: None,
        namespaces: vec![NamespaceDecl {
            uri: "urn:test".into(),
            preferred_prefix: None,
        }],
        types,
        messages: vec![MessageDecl {
            name: q("M"),
            payload_type: TypeRef::named(q(root)),
            documentation: None,
            source: source(),
        }],
    }
}
fn paths(s: &SchemaIr, seeds: &[&str]) -> Vec<Vec<String>> {
    static_dependency_exposures(s, &seeds.iter().map(|n| q(n)).collect())
        .unwrap()
        .into_iter()
        .map(|r| r.witness.into_iter().map(|n| n.local_name).collect())
        .collect()
}

#[test]
fn direct_nested_diamond_duplicate_edges_shortest_and_permutation() {
    let mut s = schema(
        vec![
            record("Root", &["B", "A", "B"]),
            record("A", &["Leaf"]),
            record("B", &["Leaf"]),
            record("Leaf", &[]),
        ],
        "Root",
    );
    assert_eq!(
        paths(&s, &["Root", "Leaf", "Leaf"]),
        vec![vec!["Root", "A", "Leaf"], vec!["Root"]]
    );
    let expected = static_dependency_exposures(&s, &BTreeSet::from([q("Leaf")])).unwrap();
    s.types.reverse();
    if let TypeKind::Record { fields } = &mut s.types.last_mut().unwrap().kind {
        fields.reverse();
    }
    assert_eq!(
        static_dependency_exposures(&s, &BTreeSet::from([q("Leaf")])).unwrap(),
        expected
    );
    s.types
        .iter_mut()
        .find(|t| t.name == q("Root"))
        .unwrap()
        .kind = TypeKind::Record {
        fields: vec![field("direct", "Leaf"), field("indirect", "A")],
    };
    assert_eq!(paths(&s, &["Leaf"]), vec![vec!["Root", "Leaf"]]);
}

#[test]
fn base_alias_choice_list_and_optional_repeated_edges() {
    let mut derived = record("Derived", &["Choice"]);
    derived.base_type = Some(TypeRef::named(q("Base")));
    let s = schema(
        vec![
            derived,
            record("Base", &[]),
            ty(
                "Choice",
                TypeKind::Choice {
                    alternatives: vec![field("a", "List")],
                },
            ),
            ty(
                "List",
                TypeKind::List {
                    item_type: TypeRef::named(q("Alias")),
                    cardinality: Cardinality::REQUIRED_ONE,
                },
            ),
            ty("Alias", TypeKind::Alias(TypeRef::named(q("Leaf")))),
            ty("Leaf", TypeKind::Primitive(PrimitiveKind::String)),
        ],
        "Derived",
    );
    assert_eq!(
        paths(&s, &["Base", "Leaf"]),
        vec![
            vec!["Derived", "Base"],
            vec!["Derived", "Choice", "List", "Alias", "Leaf"]
        ]
    );
}

#[test]
fn accepted_record_cycle_and_unrelated_seed() {
    let s = schema(
        vec![
            record("A", &["B"]),
            record("B", &["A"]),
            record("Other", &[]),
        ],
        "A",
    );
    s.validate().unwrap();
    assert_eq!(paths(&s, &["B", "Other"]), vec![vec!["A", "B"]]);
}

#[test]
fn full_qname_identity_and_primitive_root() {
    let mut s = schema(vec![record("Leaf", &[])], "Leaf");
    let other = QualifiedName::new("urn:other", "Leaf");
    s.namespaces.push(NamespaceDecl {
        uri: "urn:other".into(),
        preferred_prefix: None,
    });
    let mut t = record("Leaf", &[]);
    t.name = other.clone();
    s.types.push(t);
    assert!(
        static_dependency_exposures(&s, &BTreeSet::from([other]))
            .unwrap()
            .is_empty()
    );
    s.messages[0].payload_type = TypeRef::primitive(PrimitiveKind::String);
    assert!(paths(&s, &["Leaf"]).is_empty());
}

#[test]
fn abstract_descendants_are_not_reverse_edges() {
    let mut base = record("Base", &[]);
    base.is_abstract = true;
    let mut descendant = record("Descendant", &[]);
    descendant.base_type = Some(TypeRef::named(q("Base")));
    let s = schema(vec![base, descendant], "Base");
    assert!(paths(&s, &["Descendant"]).is_empty());
}

#[test]
fn invalid_declarations_references_and_seeds_fail() {
    let mut s = schema(vec![record("A", &[])], "A");
    assert!(static_dependency_exposures(&s, &BTreeSet::from([q("Missing")])).is_err());
    s.types.push(s.types[0].clone());
    assert!(static_dependency_exposures(&s, &BTreeSet::new()).is_err());
    s.types.pop();
    s.messages[0].payload_type = TypeRef::named(q("Missing"));
    assert!(static_dependency_exposures(&s, &BTreeSet::new()).is_err());
}
