//! Task 064: world-aware plan/schema generated-support integrity.
//! Controlled synthetics exercise only real resolution, projection and readiness APIs.

use ams_gra_oms_codegen_core::{
    AbstractValueProjectionError, BackendLanguage, GeneratedSupportChange, GenerationWorld,
    PlanBindingMismatch, ServiceGenerationError, ServicePlan, ServiceReadinessError,
    analyze_service_readiness, project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, MessageDecl, NamespaceDecl, PrimitiveKind,
    QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, TypeRef,
};
use ams_gra_oms_service_contract::{Contract, parse_yaml};

const NS: &str = "urn:test";
const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;
const OPEN: GenerationWorld = GenerationWorld::OpenExtensions;

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
            record("Unrelated", vec![]),
        ],
        vec![message("SelectedReport", named("Holder"))],
    )
}
fn plan(s: &SchemaIr) -> ServicePlan {
    plan_for(s, &oms_exchange("e1", "SelectedReport"))
}
fn change(s: &mut SchemaIr, local: &str) {
    let d = s
        .types
        .iter_mut()
        .find(|d| d.name.local_name == local)
        .unwrap();
    if let TypeKind::Record { fields } = &mut d.kind {
        fields.push(field("Extra", TypeRef::primitive(PrimitiveKind::Boolean)));
    } else {
        panic!("record required");
    }
}
fn closure(p: &ServicePlan, s: &SchemaIr) -> Vec<QualifiedName> {
    p.selected_type_closure(s)
        .unwrap()
        .iter()
        .map(|d| d.name.clone())
        .collect()
}
fn reject(a: &SchemaIr, b: &SchemaIr, name: &str, change: GeneratedSupportChange) {
    a.validate().unwrap();
    b.validate().unwrap();
    let p = plan(a);
    assert_eq!(closure(&p, a), closure(&p, b));
    p.verify_schema_binding(b)
        .expect("selected semantics unchanged");
    let expected = PlanBindingMismatch::GeneratedSupport {
        name: qualified(name),
        change,
    };
    assert_eq!(
        project_service_generation_schema(&p, b, CLOSED),
        Err(ServiceGenerationError::PlanBinding(expected.clone()))
    );
    for language in BackendLanguage::ALL {
        assert_eq!(
            analyze_service_readiness(&p, b, language, CLOSED),
            Err(ServiceReadinessError::PlanBinding(expected.clone()))
        );
    }
}
#[test]
fn changed_support_rejected_with_readiness_generation_parity() {
    let a = fixture();
    let mut b = a.clone();
    change(&mut b, "ConcreteA");
    reject(&a, &b, "ConcreteA", GeneratedSupportChange::Changed);
}
#[test]
fn added_descendant_changes_closed_support_topology() {
    let a = fixture();
    let mut b = a.clone();
    b.types.push(derived("ConcreteB", "Base", vec![]));
    reject(&a, &b, "ConcreteB", GeneratedSupportChange::ClosureChanged);
}
#[test]
fn removed_descendant_is_typed_missing_support() {
    let a = fixture();
    let mut b = a.clone();
    b.types.retain(|d| d.name.local_name != "ConcreteA");
    reject(&a, &b, "ConcreteA", GeneratedSupportChange::Missing);
}
#[test]
fn transitive_support_dependency_is_bound() {
    let mut a = fixture();
    a.types[1].kind = TypeKind::Record {
        fields: vec![field("Leaf", named("SupportLeaf"))],
    };
    a.types.push(record("SupportLeaf", vec![]));
    let mut b = a.clone();
    change(&mut b, "SupportLeaf");
    reject(&a, &b, "SupportLeaf", GeneratedSupportChange::Changed);
    b = a.clone();
    b.types.retain(|d| d.name.local_name != "SupportLeaf");
    // Invalid IR deliberately: missing expected dependency must be binding,
    // not a projection lookup error or panic.
    assert_eq!(
        project_service_generation_schema(&plan(&a), &b, CLOSED),
        Err(ServiceGenerationError::PlanBinding(
            PlanBindingMismatch::GeneratedSupport {
                name: qualified("SupportLeaf"),
                change: GeneratedSupportChange::Missing
            }
        ))
    );
}
#[test]
fn nested_abstract_value_support_is_bound() {
    let mut a = fixture();
    a.types[1].kind = TypeKind::Record {
        fields: vec![field("Nested", named("NestedBase"))],
    };
    a.types.push(abstract_record("NestedBase", vec![]));
    a.types
        .push(derived("NestedConcrete", "NestedBase", vec![]));
    let mut b = a.clone();
    change(&mut b, "NestedConcrete");
    reject(&a, &b, "NestedConcrete", GeneratedSupportChange::Changed);
    b = a.clone();
    b.types.push(derived("NestedAdded", "NestedBase", vec![]));
    reject(
        &a,
        &b,
        "NestedAdded",
        GeneratedSupportChange::ClosureChanged,
    );
}
#[test]
fn task026_absent_only_topology_becoming_inhabited_is_bound() {
    let mut a = fixture();
    a.types.retain(|d| d.name.local_name != "ConcreteA");
    if let TypeKind::Record { fields } = &mut a.types[1].kind {
        fields[0].cardinality = Cardinality {
            min_occurs: 0,
            max_occurs: Some(1),
        };
    }
    let mut b = a.clone();
    b.types.push(derived("Concrete", "Base", vec![]));
    assert!(
        project_service_generation_schema(&plan(&a), &a, CLOSED)
            .unwrap()
            .generated_support_type_names()
            .is_empty()
    );
    reject(&a, &b, "Concrete", GeneratedSupportChange::ClosureChanged);
}
#[test]
fn simultaneous_support_changes_use_original_schema_order_not_names_or_stack() {
    let mut a = fixture();
    a.types.insert(1, derived("Zulu", "Base", vec![]));
    let mut b = a.clone();
    change(&mut b, "Zulu");
    change(&mut b, "ConcreteA");
    b.types.reverse();
    // Selected closure order changed after reversing; compare membership directly.
    let p = plan(&a);
    let expected = PlanBindingMismatch::GeneratedSupport {
        name: qualified("Zulu"),
        change: GeneratedSupportChange::Changed,
    };
    assert_eq!(
        project_service_generation_schema(&p, &b, CLOSED),
        Err(ServiceGenerationError::PlanBinding(expected))
    );
}
#[test]
fn added_descendants_use_supplied_schema_order() {
    let a = fixture();
    let mut b = a.clone();
    b.types.push(derived("Zulu", "Base", vec![]));
    b.types.push(derived("Alpha", "Base", vec![]));
    reject(&a, &b, "Zulu", GeneratedSupportChange::ClosureChanged);
}
#[test]
fn compatible_rebuild_and_provenance_and_unrelated_edits_are_accepted() {
    let a = fixture();
    let p = plan(&a);
    let original = project_service_generation_schema(&p, &a, CLOSED).unwrap();
    let mut b = fixture();
    change(&mut b, "Unrelated");
    b.types.push(record("AddedUnrelated", vec![]));
    for d in &mut b.types {
        d.source.document = "another/checkout.xsd".into();
        d.source.line = Some(900);
        d.documentation = Some("new annotation".into());
        if let TypeKind::Record { fields } = &mut d.kind {
            for f in fields {
                f.source = d.source.clone();
                f.documentation = d.documentation.clone();
            }
        }
    }
    for m in &mut b.messages {
        m.source.document = "other.xsd".into();
        m.source.line = Some(99);
        m.documentation = Some("other docs".into());
    }
    let projected = project_service_generation_schema(&p, &b, CLOSED).unwrap();
    assert_eq!(
        original.selected_type_names(),
        projected.selected_type_names()
    );
    assert_eq!(
        original.generated_support_type_names(),
        projected.generated_support_type_names()
    );
    for language in BackendLanguage::ALL {
        assert_eq!(
            analyze_service_readiness(&p, &a, language, CLOSED).unwrap(),
            analyze_service_readiness(&p, &b, language, CLOSED).unwrap()
        );
    }
    b.types.retain(|d| d.name.local_name != "Unrelated");
    project_service_generation_schema(&p, &b, CLOSED).unwrap();
}
#[test]
fn selected_only_concrete_service_has_no_support_binding() {
    let mut a = fixture();
    a.types[2].kind = TypeKind::Record {
        fields: vec![field("Flag", TypeRef::primitive(PrimitiveKind::Boolean))],
    };
    let p = plan(&a);
    let mut b = a.clone();
    change(&mut b, "ConcreteA");
    for world in [CLOSED, OPEN] {
        assert!(
            project_service_generation_schema(&p, &b, world)
                .unwrap()
                .generated_support_type_names()
                .is_empty()
        );
    }
}
#[test]
fn open_world_preserves_authoritative_abstract_value_failure() {
    let a = fixture();
    let mut b = a.clone();
    change(&mut b, "ConcreteA");
    b.types.push(derived("Added", "Base", vec![]));
    let p = plan(&a);
    assert_eq!(
        project_service_generation_schema(&p, &b, OPEN),
        Err(ServiceGenerationError::AbstractValue(
            AbstractValueProjectionError::NotClosedUnderOpenExtensions(qualified("Base"))
        ))
    );
    for language in BackendLanguage::ALL {
        assert_eq!(
            analyze_service_readiness(&p, &a, language, OPEN).unwrap(),
            analyze_service_readiness(&p, &b, language, OPEN).unwrap()
        );
    }
}
#[test]
fn member_wire_namespace_is_bound_for_selected_and_support_members() {
    let a = fixture();
    let mut b = a.clone();
    if let TypeKind::Record { fields } = &mut b.types[1].kind {
        fields[0].wire_namespace_uri = None;
    }
    reject(&a, &b, "ConcreteA", GeneratedSupportChange::Changed);
    b = a.clone();
    if let TypeKind::Record { fields } = &mut b.types[2].kind {
        fields[0].wire_namespace_uri = None;
    }
    assert_eq!(
        plan(&a).verify_schema_binding(&b),
        Err(PlanBindingMismatch::Changed {
            name: qualified("Holder"),
            role: ams_gra_oms_codegen_core::MismatchRole::TypeDeclaration
        })
    );
}
