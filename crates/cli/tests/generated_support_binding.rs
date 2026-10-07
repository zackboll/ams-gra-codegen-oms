//! Task 064: compatible-schema source/codec controls using production APIs.
//! This integration crate already depends on all three backend libraries.

use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, GenerationWorld, ServicePlan, analyze_service_readiness,
    build_service_api_model, project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, MessageDecl, NamespaceDecl, PrimitiveKind,
    QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, TypeRef,
};
use ams_gra_oms_service_contract::{Contract, parse_yaml};

const NS: &str = "urn:test";
const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn source() -> SourceRef {
    SourceRef {
        document: "fixture.xsd".to_owned(),
        line: Some(7),
    }
}

fn qualified(local: &str) -> QualifiedName {
    QualifiedName::new(NS, local)
}

fn named(local: &str) -> TypeRef {
    TypeRef::named(qualified(local))
}

fn field(name: &str, type_ref: TypeRef) -> FieldDecl {
    FieldDecl {
        name: name.to_owned(),
        wire_namespace_uri: Some(NS.to_owned()),
        type_ref,
        cardinality: Cardinality::REQUIRED_ONE,
        nillable: false,
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    }
}

fn record(local: &str, fields: Vec<FieldDecl>) -> TypeDecl {
    TypeDecl {
        name: qualified(local),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Record { fields },
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    }
}

fn abstract_record(local: &str, fields: Vec<FieldDecl>) -> TypeDecl {
    TypeDecl {
        is_abstract: true,
        ..record(local, fields)
    }
}

fn derived(local: &str, base: &str, fields: Vec<FieldDecl>) -> TypeDecl {
    TypeDecl {
        base_type: Some(named(base)),
        ..record(local, fields)
    }
}

fn message(local: &str, payload: TypeRef) -> MessageDecl {
    MessageDecl {
        name: qualified(local),
        payload_type: payload,
        documentation: None,
        source: source(),
    }
}

fn schema(types: Vec<TypeDecl>, messages: Vec<MessageDecl>) -> SchemaIr {
    SchemaIr {
        schema_version: Some("000.1.0".to_owned()),
        namespaces: vec![NamespaceDecl {
            uri: NS.to_owned(),
            preferred_prefix: Some("t".to_owned()),
        }],
        types,
        messages,
    }
}

fn contract(exchanges: &str) -> Contract {
    parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: Projection Test\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n{exchanges}"
    ))
    .expect("test contract should be portable-valid")
}

fn oms_exchange(id: &str, message: &str) -> String {
    format!(
        "      - id: {id}\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {message}\n        topic: t.{id}\n        timing:\n          kind: asynchronous\n"
    )
}

fn plan_for(schema: &SchemaIr, exchanges: &str) -> ServicePlan {
    resolve_service_plan(&contract(exchanges), schema).expect("plan should resolve")
}

fn fixture() -> SchemaIr {
    schema(
        vec![
            abstract_record("Base", vec![]),
            derived(
                "ConcreteA",
                "Base",
                vec![field("Code", TypeRef::primitive(PrimitiveKind::String))],
            ),
            record("Holder", vec![field("Value", named("Base"))]),
        ],
        vec![message("SelectedReport", named("Holder"))],
    )
}
fn codec(p: &ServicePlan, s: &SchemaIr) -> String {
    let projection = project_service_generation_schema(p, s, CLOSED).unwrap();
    let m = build_service_api_model(p, projection.schema(), CLOSED).unwrap();
    ams_gra_oms_backend_rust::generate_service_codec(&m, projection.schema(), CLOSED).unwrap()
}
#[test]
fn task064_compatible_schema_preserves_readiness_generated_source_and_codec() {
    let a = fixture();
    let p = plan_for(&a, &oms_exchange("e1", "SelectedReport"));
    let mut b = fixture();
    for d in &mut b.types {
        d.source.document = "other/checkout.xsd".into();
        d.source.line = Some(777);
        d.documentation = Some("different docs".into());
        if let TypeKind::Record { fields } = &mut d.kind {
            for f in fields {
                f.source = d.source.clone();
                f.documentation = d.documentation.clone();
            }
        }
    }
    b.messages[0].source.document = "reload.xsd".into();
    b.messages[0].documentation = Some("message docs".into());
    let pa = project_service_generation_schema(&p, &a, CLOSED).unwrap();
    let pb = project_service_generation_schema(&p, &b, CLOSED).unwrap();
    let backends: Vec<(BackendLanguage, Box<dyn Backend>)> = vec![
        (
            BackendLanguage::Ada,
            Box::new(ams_gra_oms_backend_ada::AdaBackend),
        ),
        (
            BackendLanguage::Rust,
            Box::new(ams_gra_oms_backend_rust::RustBackend),
        ),
        (
            BackendLanguage::Cpp,
            Box::new(ams_gra_oms_backend_cpp::CppBackend),
        ),
    ];
    for (language, backend) in backends {
        assert_eq!(
            analyze_service_readiness(&p, &a, language, CLOSED).unwrap(),
            analyze_service_readiness(&p, &b, language, CLOSED).unwrap()
        );
        assert_eq!(
            backend.generate(pa.schema(), CLOSED).unwrap(),
            backend.generate(pb.schema(), CLOSED).unwrap()
        );
    }
    assert_eq!(codec(&p, &a), codec(&p, &b));
    println!("TASK064 ZERO CAPABILITY SOURCE CODEC DELTA: PASSED");
}
#[test]
fn task064_wire_namespace_changes_actual_codec_spelling_and_invalidates_old_plan() {
    let mut a = fixture(); // Keep both namespaces in the production projection.
    a.types.push(record("Foreign", vec![]));
    a.types.last_mut().unwrap().name = QualifiedName::new("urn:other", "Foreign");
    a.namespaces.push(NamespaceDecl {
        uri: "urn:other".into(),
        preferred_prefix: None,
    });
    if let TypeKind::Record { fields } = &mut a.types[2].kind {
        fields.push(field(
            "Foreign",
            TypeRef::named(QualifiedName::new("urn:other", "Foreign")),
        ));
    }
    let p = plan_for(&a, &oms_exchange("e1", "SelectedReport"));
    let mut b = a.clone();
    if let TypeKind::Record { fields } = &mut b.types[1].kind {
        fields[0].wire_namespace_uri = Some("urn:other".into());
    }
    b.validate().unwrap();
    let fresh = plan_for(&b, &oms_exchange("e1", "SelectedReport"));
    let ca = codec(&p, &a);
    let cb = codec(&fresh, &b);
    assert_ne!(ca, cb);
    assert!(ca.contains("{urn:test}Code"));
    assert!(cb.contains("{urn:other}Code"));
    assert!(matches!(
        project_service_generation_schema(&p, &b, CLOSED),
        Err(ams_gra_oms_codegen_core::ServiceGenerationError::PlanBinding(_))
    ));
    println!("TASK064 WIRE NAMESPACE CODEC BINDING: PASSED");
}
