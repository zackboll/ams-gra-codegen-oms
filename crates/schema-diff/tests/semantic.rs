use ams_gra_oms_ir::*;
use ams_gra_oms_schema_diff::*;

fn source() -> SourceRef {
    SourceRef {
        document: "synthetic".into(),
        line: Some(1),
    }
}
fn field(name: &str) -> FieldDecl {
    FieldDecl {
        name: name.into(),
        wire_namespace_uri: Some("urn:a".into()),
        type_ref: TypeRef::primitive(PrimitiveKind::String),
        cardinality: Cardinality::REQUIRED_ONE,
        nillable: false,
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    }
}
fn schema(kind: TypeKind) -> SchemaIr {
    SchemaIr {
        schema_version: None,
        namespaces: vec![NamespaceDecl {
            uri: "urn:a".into(),
            preferred_prefix: None,
        }],
        types: vec![TypeDecl {
            name: QualifiedName::new("urn:a", "T"),
            is_abstract: false,
            base_type: None,
            kind,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }],
        messages: vec![],
    }
}
fn primitive() -> SchemaIr {
    schema(TypeKind::Primitive(PrimitiveKind::String))
}
fn record() -> SchemaIr {
    schema(TypeKind::Record {
        fields: vec![field("a"), field("b")],
    })
}
fn fields(s: &mut SchemaIr) -> &mut Vec<FieldDecl> {
    match &mut s.types[0].kind {
        TypeKind::Record { fields } => fields,
        TypeKind::Choice { alternatives } => alternatives,
        _ => panic!("members"),
    }
}
fn expected(
    member: Option<&str>,
    property: Property,
    kind: ChangeKind,
    before: Option<Value>,
    after: Option<Value>,
) -> Change {
    Change {
        category: Category::Type,
        name: QualifiedName::new("urn:a", "T"),
        member: member.map(str::to_owned),
        property,
        kind,
        before,
        after,
    }
}
fn one(a: &SchemaIr, b: &SchemaIr, e: Change) {
    let d = compare_schemas(a, b);
    assert_eq!(d.changes, vec![e]);
    assert_eq!(
        d.types,
        Counts {
            before: 1,
            after: 1,
            added: 0,
            removed: 0,
            changed: 1,
            unchanged: 0
        }
    );
    assert_eq!(d.messages, Counts::default());
    let reverse = compare_schemas(b, a);
    assert_eq!(reverse.changes[0].before, d.changes[0].after);
    assert_eq!(reverse.changes[0].after, d.changes[0].before);
}

#[test]
fn task069_primitive_alias_base_abstract_and_kind() {
    let a = primitive();
    let mut b = a.clone();
    b.types[0].kind = TypeKind::Primitive(PrimitiveKind::Binary);
    one(
        &a,
        &b,
        expected(
            None,
            Property::Primitive,
            ChangeKind::TypeChanged,
            Some(Value::Primitive(PrimitiveKind::String)),
            Some(Value::Primitive(PrimitiveKind::Binary)),
        ),
    );
    let a = schema(TypeKind::Alias(TypeRef::primitive(PrimitiveKind::String)));
    let mut b = a.clone();
    b.types[0].kind = TypeKind::Alias(TypeRef::named(QualifiedName::new("urn:a", "Other")));
    one(
        &a,
        &b,
        expected(
            None,
            Property::Target,
            ChangeKind::TypeChanged,
            Some(Value::Reference(TypeRef::primitive(PrimitiveKind::String))),
            Some(Value::Reference(TypeRef::named(QualifiedName::new(
                "urn:a", "Other",
            )))),
        ),
    );
    let a = primitive();
    let mut b = a.clone();
    b.types[0].base_type = Some(TypeRef::primitive(PrimitiveKind::String));
    one(
        &a,
        &b,
        expected(
            None,
            Property::BaseType,
            ChangeKind::BaseTypeChanged,
            None,
            Some(Value::Reference(TypeRef::primitive(PrimitiveKind::String))),
        ),
    );
    let mut b = a.clone();
    b.types[0].is_abstract = true;
    one(
        &a,
        &b,
        expected(
            None,
            Property::Abstract,
            ChangeKind::AbstractChanged,
            Some(Value::Bool(false)),
            Some(Value::Bool(true)),
        ),
    );
    let b = record();
    one(
        &a,
        &b,
        expected(
            None,
            Property::Kind,
            ChangeKind::TypeChanged,
            Some(Value::Kind(KindFamily::Primitive)),
            Some(Value::Kind(KindFamily::Record)),
        ),
    );
}

#[test]
fn task069_members_and_choice_exact_properties() {
    for a in [
        record(),
        schema(TypeKind::Choice {
            alternatives: vec![field("a"), field("b")],
        }),
    ] {
        let mut b = a.clone();
        fields(&mut b).push(field("c"));
        one(
            &a,
            &b,
            expected(
                Some("c"),
                Property::Presence,
                ChangeKind::MemberAdded,
                None,
                Some(Value::Bool(true)),
            ),
        );
        one(
            &b,
            &a,
            expected(
                Some("c"),
                Property::Presence,
                ChangeKind::MemberRemoved,
                Some(Value::Bool(true)),
                None,
            ),
        );
        let mut b = a.clone();
        fields(&mut b)[0].type_ref = TypeRef::primitive(PrimitiveKind::Boolean);
        one(
            &a,
            &b,
            expected(
                Some("a"),
                Property::Target,
                ChangeKind::MemberChanged,
                Some(Value::Reference(TypeRef::primitive(PrimitiveKind::String))),
                Some(Value::Reference(TypeRef::primitive(PrimitiveKind::Boolean))),
            ),
        );
        let mut b = a.clone();
        fields(&mut b)[0].wire_namespace_uri = None;
        one(
            &a,
            &b,
            expected(
                Some("a"),
                Property::WireNamespace,
                ChangeKind::MemberChanged,
                Some(Value::Text("urn:a".into())),
                None,
            ),
        );
        let mut b = a.clone();
        fields(&mut b)[0].cardinality = Cardinality::OPTIONAL_ONE;
        one(
            &a,
            &b,
            expected(
                Some("a"),
                Property::Cardinality,
                ChangeKind::MemberChanged,
                Some(Value::Cardinality(Cardinality::REQUIRED_ONE)),
                Some(Value::Cardinality(Cardinality::OPTIONAL_ONE)),
            ),
        );
        let mut b = a.clone();
        fields(&mut b)[0].nillable = true;
        one(
            &a,
            &b,
            expected(
                Some("a"),
                Property::Nillable,
                ChangeKind::MemberChanged,
                Some(Value::Bool(false)),
                Some(Value::Bool(true)),
            ),
        );
        let mut b = a.clone();
        fields(&mut b)[0].constraints.length = Some(3);
        one(
            &a,
            &b,
            expected(
                Some("a"),
                Property::Length,
                ChangeKind::ConstraintChanged,
                None,
                Some(Value::Unsigned(3)),
            ),
        );
        let mut b = a.clone();
        fields(&mut b).reverse();
        one(
            &a,
            &b,
            expected(
                None,
                Property::Order,
                ChangeKind::MemberOrderChanged,
                Some(Value::Order(vec!["a".into(), "b".into()])),
                Some(Value::Order(vec!["b".into(), "a".into()])),
            ),
        );
    }
}

#[test]
fn task069_constraints_all_normalized_fields() {
    let a = primitive();
    for (property, set) in [
        (Property::MinInclusive, 0),
        (Property::MaxInclusive, 1),
        (Property::MinExclusive, 2),
        (Property::MaxExclusive, 3),
        (Property::Length, 4),
        (Property::MinLength, 5),
        (Property::MaxLength, 6),
        (Property::WhiteSpace, 7),
        (Property::Patterns, 8),
    ] {
        let mut b = a.clone();
        let c = &mut b.types[0].constraints;
        let n = NumericValue::Integer(7);
        let v = match set {
            0 => {
                c.min_inclusive = Some(n);
                Value::Numeric(n)
            }
            1 => {
                c.max_inclusive = Some(n);
                Value::Numeric(n)
            }
            2 => {
                c.min_exclusive = Some(n);
                Value::Numeric(n)
            }
            3 => {
                c.max_exclusive = Some(n);
                Value::Numeric(n)
            }
            4 => {
                c.length = Some(7);
                Value::Unsigned(7)
            }
            5 => {
                c.min_length = Some(7);
                Value::Unsigned(7)
            }
            6 => {
                c.max_length = Some(7);
                Value::Unsigned(7)
            }
            7 => {
                c.lexical.white_space = Some(WhiteSpacePolicy::Collapse);
                Value::WhiteSpace(WhiteSpacePolicy::Collapse)
            }
            _ => {
                c.lexical.pattern_groups = vec![PatternGroup {
                    alternatives: vec![
                        PatternExpression::xml_schema("a"),
                        PatternExpression::xml_schema("b"),
                    ],
                }];
                Value::Patterns(c.lexical.pattern_groups.clone())
            }
        };
        one(
            &a,
            &b,
            expected(
                None,
                property,
                ChangeKind::ConstraintChanged,
                if set == 8 {
                    Some(Value::Patterns(vec![]))
                } else {
                    None
                },
                Some(v),
            ),
        );
    }
    let mut a = a;
    a.types[0].constraints.lexical.pattern_groups = vec![PatternGroup {
        alternatives: vec![
            PatternExpression::xml_schema("a"),
            PatternExpression::xml_schema("b"),
        ],
    }];
    let mut b = a.clone();
    b.types[0].constraints.lexical.pattern_groups[0]
        .alternatives
        .reverse();
    one(
        &a,
        &b,
        expected(
            None,
            Property::Patterns,
            ChangeKind::ConstraintChanged,
            Some(Value::Patterns(
                a.types[0].constraints.lexical.pattern_groups.clone(),
            )),
            Some(Value::Patterns(
                b.types[0].constraints.lexical.pattern_groups.clone(),
            )),
        ),
    );
    let mut b = a.clone();
    b.types[0]
        .constraints
        .lexical
        .pattern_groups
        .push(PatternGroup {
            alternatives: vec![PatternExpression::xml_schema("c")],
        });
    one(
        &a,
        &b,
        expected(
            None,
            Property::Patterns,
            ChangeKind::ConstraintChanged,
            Some(Value::Patterns(
                a.types[0].constraints.lexical.pattern_groups.clone(),
            )),
            Some(Value::Patterns(
                b.types[0].constraints.lexical.pattern_groups.clone(),
            )),
        ),
    );
}

#[test]
fn task069_binary_and_list() {
    let old = TypeRef::binary(BinaryLexicalEncoding::HexBinary);
    let new = TypeRef::binary(BinaryLexicalEncoding::Base64Binary);
    let a = schema(TypeKind::Alias(old.clone()));
    let b = schema(TypeKind::Alias(new.clone()));
    one(
        &a,
        &b,
        expected(
            None,
            Property::Target,
            ChangeKind::TypeChanged,
            Some(Value::Reference(old.clone())),
            Some(Value::Reference(new.clone())),
        ),
    );
    let a = schema(TypeKind::List {
        item_type: old.clone(),
        cardinality: Cardinality::REQUIRED_ONE,
    });
    let b = schema(TypeKind::List {
        item_type: new.clone(),
        cardinality: Cardinality::REQUIRED_ONE,
    });
    one(
        &a,
        &b,
        expected(
            None,
            Property::ItemType,
            ChangeKind::TypeChanged,
            Some(Value::Reference(old.clone())),
            Some(Value::Reference(new)),
        ),
    );
    let b = schema(TypeKind::List {
        item_type: old,
        cardinality: Cardinality {
            min_occurs: 2,
            max_occurs: None,
        },
    });
    one(
        &a,
        &b,
        expected(
            None,
            Property::Cardinality,
            ChangeKind::TypeChanged,
            Some(Value::Cardinality(Cardinality::REQUIRED_ONE)),
            Some(Value::Cardinality(Cardinality {
                min_occurs: 2,
                max_occurs: None,
            })),
        ),
    );
}

fn enumeration(values: &[&str]) -> SchemaIr {
    schema(TypeKind::Enumeration {
        variants: values
            .iter()
            .map(|v| EnumVariant {
                wire_value: (*v).into(),
                documentation: None,
            })
            .collect(),
    })
}
#[test]
fn task069_enum_values_and_order() {
    let a = enumeration(&["a", "b"]);
    let b = enumeration(&["a", "b", "c"]);
    one(
        &a,
        &b,
        expected(
            Some("c"),
            Property::Presence,
            ChangeKind::EnumValueAdded,
            None,
            Some(Value::Text("c".into())),
        ),
    );
    one(
        &b,
        &a,
        expected(
            Some("c"),
            Property::Presence,
            ChangeKind::EnumValueRemoved,
            Some(Value::Text("c".into())),
            None,
        ),
    );
    let b = enumeration(&["b", "a"]);
    one(
        &a,
        &b,
        expected(
            None,
            Property::Order,
            ChangeKind::EnumOrderChanged,
            Some(Value::Order(vec!["a".into(), "b".into()])),
            Some(Value::Order(vec!["b".into(), "a".into()])),
        ),
    );
}

#[test]
fn task069_qnames_presence_messages_direction() {
    let a = primitive();
    let mut b = a.clone();
    b.types[0].name.namespace_uri = "urn:b".into();
    let d = compare_schemas(&a, &b);
    assert_eq!(
        d.types,
        Counts {
            before: 1,
            after: 1,
            added: 1,
            removed: 1,
            changed: 0,
            unchanged: 0
        }
    );
    assert_eq!(
        d.changes,
        vec![
            expected(
                None,
                Property::Presence,
                ChangeKind::TypeRemoved,
                Some(Value::Kind(KindFamily::Primitive)),
                None
            ),
            Change {
                name: QualifiedName::new("urn:b", "T"),
                ..expected(
                    None,
                    Property::Presence,
                    ChangeKind::TypeAdded,
                    None,
                    Some(Value::Kind(KindFamily::Primitive))
                )
            },
        ]
    );
    let mut b = a.clone();
    b.messages.push(MessageDecl {
        name: QualifiedName::new("urn:a", "M"),
        payload_type: TypeRef::named(a.types[0].name.clone()),
        documentation: None,
        source: source(),
    });
    let d = compare_schemas(&a, &b);
    let e = Change {
        category: Category::Message,
        name: QualifiedName::new("urn:a", "M"),
        member: None,
        property: Property::Presence,
        kind: ChangeKind::MessageAdded,
        before: None,
        after: Some(Value::Reference(b.messages[0].payload_type.clone())),
    };
    assert_eq!(d.changes, vec![e.clone()]);
    assert_eq!(d.messages.added, 1);
    assert_eq!(d.types.unchanged, 1);
    let reverse = compare_schemas(&b, &a);
    assert_eq!(
        reverse.changes,
        vec![Change {
            kind: ChangeKind::MessageRemoved,
            before: e.after.clone(),
            after: None,
            ..e.clone()
        }]
    );
    assert_eq!(reverse.messages.removed, 1);
    let mut c = b.clone();
    c.messages[0].payload_type = TypeRef::primitive(PrimitiveKind::String);
    let d = compare_schemas(&b, &c);
    assert_eq!(
        d.changes,
        vec![Change {
            property: Property::Payload,
            kind: ChangeKind::MessageChanged,
            before: e.after,
            after: Some(Value::Reference(c.messages[0].payload_type.clone())),
            ..e
        }]
    );
    assert_eq!(d.messages.changed, 1);
    let mut c = b.clone();
    c.types[0].is_abstract = true;
    assert_eq!(compare_schemas(&b, &c).messages.unchanged, 1);
}

#[test]
fn task069_noise_declaration_order_and_deterministic_tsv() {
    let mut a = record();
    let mut t = enumeration(&["x", "y"]).types.remove(0);
    t.name.local_name = "E".into();
    a.types.push(t);
    a.messages.push(MessageDecl {
        name: QualifiedName::new("urn:a", "M"),
        payload_type: TypeRef::named(a.types[0].name.clone()),
        documentation: None,
        source: source(),
    });
    let mut b = a.clone();
    b.types.reverse();
    b.schema_version = Some("new".into());
    b.namespaces[0].preferred_prefix = Some("other".into());
    for t in &mut b.types {
        t.documentation = Some("doc".into());
        t.source = SourceRef {
            document: "/different/path".into(),
            line: Some(900),
        };
        match &mut t.kind {
            TypeKind::Record { fields } => {
                for f in fields {
                    f.documentation = Some("doc".into());
                    f.source = t.source.clone();
                }
            }
            TypeKind::Enumeration { variants } => {
                for v in variants {
                    v.documentation = Some("doc".into());
                }
            }
            _ => {}
        }
    }
    b.messages[0].documentation = Some("doc".into());
    b.messages[0].source.document = "other".into();
    assert!(compare_schemas(&a, &a).changes.is_empty());
    let d = compare_schemas(&a, &b);
    assert!(d.changes.is_empty());
    assert_eq!(d.types.unchanged, 2);
    assert_eq!(d.messages.unchanged, 1);
    b.types[0].is_abstract = true;
    let d = compare_schemas(&a, &b);
    b.types.reverse();
    assert_eq!(d, compare_schemas(&a, &b));
    assert_eq!(d.to_tsv(), compare_schemas(&a, &b).to_tsv());
    assert!(d.to_tsv().lines().all(|l| l.split('\t').count() == 7));
    assert!(!d.to_tsv().contains("/different/path"));
    assert_eq!(escape_tsv("\t\n\r\\\0\u{7f}"), "\\t\\n\\r\\\\\\u{0}\\u{7f}");
}
