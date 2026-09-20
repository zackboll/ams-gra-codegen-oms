//! ServicePlan/schema semantic binding tests.
//!
//! The public library API takes a `ServicePlan` and a `SchemaIr` separately
//! and claims to diagnose wrong-schema reuse. A QName-only check cannot
//! deliver that: two schemas can agree on every message and type NAME while
//! disagreeing about what those declarations mean. These tests pin the
//! guarantee as real, and pin its scope -- selected-service only.

use ams_gra_oms_codegen_core::{
    MismatchRole, PlanBindingMismatch, ServicePlan, resolve_service_plan,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, MessageDecl, NamespaceDecl, PrimitiveKind,
    QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, TypeRef,
};
use ams_gra_oms_service_contract::parse_yaml;

const NS: &str = "urn:test";

fn source() -> SourceRef {
    SourceRef {
        document: "fixture.xsd".to_owned(),
        line: None,
    }
}

fn named(local: &str) -> TypeRef {
    TypeRef::named(QualifiedName::new(NS, local))
}

fn primitive(kind: PrimitiveKind) -> TypeRef {
    TypeRef::primitive(kind)
}

fn field(name: &str, type_ref: TypeRef) -> FieldDecl {
    FieldDecl {
        name: name.to_owned(),
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
        name: QualifiedName::new(NS, local),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Record { fields },
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    }
}

fn declared_message(local: &str, payload: TypeRef) -> MessageDecl {
    MessageDecl {
        name: QualifiedName::new(NS, local),
        payload_type: payload,
        documentation: None,
        source: source(),
    }
}

/// `ReportMessage -> PayloadA -> NestedType`, plus an entirely unselected
/// `UnusedType`/`UnusedReport` pair used to pin the plan's scope.
fn schema() -> SchemaIr {
    SchemaIr {
        schema_version: Some("000.1.0".to_owned()),
        namespaces: vec![NamespaceDecl {
            uri: NS.to_owned(),
            preferred_prefix: Some("t".to_owned()),
        }],
        types: vec![
            record(
                "NestedType",
                vec![field("Value", primitive(PrimitiveKind::Float64))],
            ),
            record("PayloadA", vec![field("Nested", named("NestedType"))]),
            record(
                "PayloadB",
                vec![field("Other", primitive(PrimitiveKind::Boolean))],
            ),
            record(
                "UnusedType",
                vec![field("Filler", primitive(PrimitiveKind::String))],
            ),
        ],
        messages: vec![
            declared_message("ReportMessage", named("PayloadA")),
            declared_message("UnusedReport", named("UnusedType")),
        ],
    }
}

fn plan_for(schema: &SchemaIr) -> ServicePlan {
    let contract = parse_yaml(
        "contract_version: \"0.1\"\nservice:\n  name: Binding Test\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n  - id: mission-data\n    name: Mission Data\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: ReportMessage\n        topic: mission.report\n        timing:\n          kind: asynchronous\n",
    )
    .expect("test contract should be portable-valid");
    resolve_service_plan(&contract, schema).expect("resolution should succeed")
}

/// A semantically identical schema rebuilt from scratch is compatible. Object
/// identity is never required, so a caller that legitimately rebuilds its
/// schema is not punished.
#[test]
fn a_separately_rebuilt_equal_schema_is_compatible() {
    plan_for(&schema())
        .verify_schema_binding(&schema())
        .expect("an equal schema built separately must be accepted");
}

/// Same message QName, different payload TypeRef. Every name matches, so only
/// a semantic binding can see this.
#[test]
fn a_changed_selected_message_payload_is_rejected() {
    let plan = plan_for(&schema());
    let mut other = schema();
    other
        .messages
        .iter_mut()
        .find(|message| message.name.local_name == "ReportMessage")
        .expect("ReportMessage must exist")
        .payload_type = named("PayloadB");

    let mismatch = plan
        .verify_schema_binding(&other)
        .expect_err("a repointed payload must be rejected");
    assert!(
        matches!(
            mismatch,
            PlanBindingMismatch::Changed {
                role: MismatchRole::Message,
                ref name
            } if name.local_name == "ReportMessage"
        ),
        "{mismatch:?}"
    );
}

/// Same type QName, different declaration body.
#[test]
fn a_changed_selected_type_body_is_rejected() {
    let plan = plan_for(&schema());
    let mut other = schema();
    other
        .types
        .iter_mut()
        .find(|declaration| declaration.name.local_name == "PayloadA")
        .expect("PayloadA must exist")
        .kind = TypeKind::Record {
        fields: vec![field("Renamed", named("NestedType"))],
    };

    let mismatch = plan
        .verify_schema_binding(&other)
        .expect_err("a changed declaration body must be rejected");
    assert!(
        matches!(
            mismatch,
            PlanBindingMismatch::Changed {
                role: MismatchRole::TypeDeclaration,
                ref name
            } if name.local_name == "PayloadA"
        ),
        "{mismatch:?}"
    );
}

/// A transitive dependency whose constraints or cardinality changed, with no
/// name anywhere altered.
#[test]
fn changed_transitive_constraints_or_cardinality_are_rejected() {
    let mutations: [fn(&mut TypeDecl); 2] = [
        |declaration| {
            if let TypeKind::Record { fields } = &mut declaration.kind {
                fields[0].cardinality = Cardinality::OPTIONAL_ONE;
            }
        },
        |declaration| {
            if let TypeKind::Record { fields } = &mut declaration.kind {
                fields[0].constraints.max_length = Some(8);
            }
        },
    ];
    for mutate in mutations {
        let plan = plan_for(&schema());
        let mut other = schema();
        mutate(
            other
                .types
                .iter_mut()
                .find(|declaration| declaration.name.local_name == "NestedType")
                .expect("NestedType must exist"),
        );

        let mismatch = plan
            .verify_schema_binding(&other)
            .expect_err("a transitive dependency change must be rejected");
        assert!(
            matches!(
                mismatch,
                PlanBindingMismatch::Changed {
                    role: MismatchRole::TypeDeclaration,
                    ref name
                } if name.local_name == "NestedType"
            ),
            "{mismatch:?}"
        );
    }
}

/// A `ServicePlan` is selected-service scoped, so changing or removing a
/// declaration outside the selected closure must NOT invalidate it.
#[test]
fn unrelated_unselected_declaration_changes_do_not_invalidate_the_plan() {
    let plan = plan_for(&schema());

    // Mutate an unselected declaration.
    let mut mutated = schema();
    mutated
        .types
        .iter_mut()
        .find(|declaration| declaration.name.local_name == "UnusedType")
        .expect("UnusedType must exist")
        .kind = TypeKind::Record {
        fields: vec![field("Different", primitive(PrimitiveKind::Boolean))],
    };
    plan.verify_schema_binding(&mutated)
        .expect("an unselected declaration change is out of scope for this plan");

    // Remove an unselected message and its type entirely.
    let mut removed = schema();
    removed
        .messages
        .retain(|message| message.name.local_name != "UnusedReport");
    removed
        .types
        .retain(|declaration| declaration.name.local_name != "UnusedType");
    plan.verify_schema_binding(&removed)
        .expect("an unselected message is out of scope for this plan");
}

/// A selected identity that is simply absent is reported as missing rather
/// than as changed, so the two remain distinguishable.
#[test]
fn an_absent_selected_message_is_reported_as_missing() {
    let plan = plan_for(&schema());
    let mut other = schema();
    other
        .messages
        .retain(|message| message.name.local_name != "ReportMessage");

    let mismatch = plan
        .verify_schema_binding(&other)
        .expect_err("an absent selected message must be rejected");
    assert!(
        matches!(
            mismatch,
            PlanBindingMismatch::Missing {
                role: MismatchRole::Message,
                ..
            }
        ),
        "{mismatch:?}"
    );
}
