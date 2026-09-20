//! Contract-selected generation-projection tests.
//!
//! Every schema below is a controlled synthetic. The point of each is to make
//! one projection rule observable in isolation: selectivity, schema ordering,
//! the selected/support classification, fixed-point support expansion, Task
//! 024 closed-sum descendants, Task 026 zero-descendant retention, and Task
//! 028 open-world defensiveness.

use ams_gra_oms_codegen_core::{
    GenerationWorld, MismatchRole, PlanBindingMismatch, ServiceGenerationError, ServicePlan,
    project_service_generation_schema, resolve_service_plan,
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

fn abstract_derived(local: &str, base: &str, fields: Vec<FieldDecl>) -> TypeDecl {
    TypeDecl {
        is_abstract: true,
        ..derived(local, base, fields)
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

fn locals(names: &[QualifiedName]) -> Vec<&str> {
    names.iter().map(|name| name.local_name.as_str()).collect()
}

fn type_locals(schema: &SchemaIr) -> Vec<&str> {
    schema
        .types
        .iter()
        .map(|declaration| declaration.name.local_name.as_str())
        .collect()
}

// ---------------------------------------------------------------------
// Sections 9/10/22/41/73 -- selectivity and ordering
// ---------------------------------------------------------------------

/// Section 73: the projection retains selected declarations in ORIGINAL
/// schema order and omits the rest. Discovery order would put SelectedRoot
/// before SelectedDependencyA; schema order does not.
#[test]
fn projection_keeps_schema_order_and_omits_unselected() {
    let schema = schema(
        vec![
            record("SelectedDependencyA", vec![]),
            record("UnselectedX", vec![]),
            record(
                "SelectedRoot",
                vec![field("Dep", named("SelectedDependencyA"))],
            ),
            record("UnselectedY", vec![field("Dep", named("UnselectedX"))]),
        ],
        vec![
            message("SelectedReport", named("SelectedRoot")),
            message("UnselectedReport", named("UnselectedY")),
        ],
    );
    let plan = plan_for(&schema, &oms_exchange("e1", "SelectedReport"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(
        type_locals(projection.schema()),
        ["SelectedDependencyA", "SelectedRoot"]
    );
    assert_eq!(
        locals(projection.selected_type_names()),
        ["SelectedDependencyA", "SelectedRoot"]
    );
    assert!(projection.generated_support_type_names().is_empty());
}

/// Section 41: only SELECTED messages survive. `UnselectedReport` shares the
/// selected payload type here, so retaining messages by payload reachability
/// (rather than by selection) would wrongly keep it.
#[test]
fn projection_keeps_only_selected_messages() {
    let schema = schema(
        vec![record("Payload", vec![])],
        vec![
            message("SelectedReport", named("Payload")),
            message("UnselectedReport", named("Payload")),
        ],
    );
    let plan = plan_for(&schema, &oms_exchange("e1", "SelectedReport"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(
        locals(projection.selected_message_names()),
        ["SelectedReport"]
    );
    assert_eq!(projection.schema().messages.len(), 1);
}

/// Section 39: one message selected from several exchanges yields each
/// required declaration exactly once.
#[test]
fn repeated_message_selection_does_not_duplicate_types() {
    let schema = schema(
        vec![record("Payload", vec![])],
        vec![message("Report", named("Payload"))],
    );
    let plan = plan_for(
        &schema,
        &format!(
            "{}{}",
            oms_exchange("e1", "Report"),
            oms_exchange("e2", "Report")
        ),
    );
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(type_locals(projection.schema()), ["Payload"]);
    assert_eq!(projection.schema().messages.len(), 1);
}

/// Section 40: reversing exchange order while selecting the same message set
/// produces an identical projection, because type order follows SchemaIr.
#[test]
fn contract_exchange_order_does_not_reorder_the_type_model() {
    let schema = schema(
        vec![record("A", vec![]), record("B", vec![])],
        vec![
            message("ReportA", named("A")),
            message("ReportB", named("B")),
        ],
    );
    let forward = plan_for(
        &schema,
        &format!(
            "{}{}",
            oms_exchange("e1", "ReportA"),
            oms_exchange("e2", "ReportB")
        ),
    );
    let reverse = plan_for(
        &schema,
        &format!(
            "{}{}",
            oms_exchange("e1", "ReportB"),
            oms_exchange("e2", "ReportA")
        ),
    );

    assert_eq!(
        project_service_generation_schema(&forward, &schema, CLOSED)
            .expect("forward")
            .schema(),
        project_service_generation_schema(&reverse, &schema, CLOSED)
            .expect("reverse")
            .schema()
    );
}

/// Sections 18/20/24: metadata and provenance are preserved verbatim, and the
/// caller's schema is never mutated.
#[test]
fn projection_preserves_metadata_and_leaves_the_original_schema_alone() {
    let schema = schema(
        vec![record("Payload", vec![]), record("Unselected", vec![])],
        vec![message("Report", named("Payload"))],
    );
    let before = schema.clone();
    let plan = plan_for(&schema, &oms_exchange("e1", "Report"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(schema, before, "the source schema must not be mutated");
    assert_eq!(
        projection.schema().schema_version,
        Some("000.1.0".to_owned())
    );
    assert_eq!(projection.schema().namespaces, schema.namespaces);
    // Cloned, not re-authored: the SourceRef survives.
    assert_eq!(projection.schema().types[0].source, source());
    assert_eq!(projection.schema().messages[0].source, source());
}

// ---------------------------------------------------------------------
// Sections 11/12/13/21/42/43/44/74 -- generated support closure
// ---------------------------------------------------------------------

/// A Task 024 fixture covering every descendant shape at once:
///
/// * `ConcreteA`      concrete leaf
/// * `ConcreteParent` concrete NON-leaf (still a variant)
/// * `ConcreteLeaf`   concrete leaf under a concrete parent
/// * `AbstractMiddle` abstract intermediate (NOT a variant)
/// * `MiddleLeaf`     concrete leaf under the abstract intermediate
fn closed_sum_schema() -> SchemaIr {
    schema(
        vec![
            abstract_record(
                "Base",
                vec![field("Id", TypeRef::primitive(PrimitiveKind::String))],
            ),
            derived("ConcreteA", "Base", vec![]),
            derived("ConcreteParent", "Base", vec![]),
            derived("ConcreteLeaf", "ConcreteParent", vec![]),
            abstract_derived("AbstractMiddle", "Base", vec![]),
            derived("MiddleLeaf", "AbstractMiddle", vec![]),
            record("Unrelated", vec![]),
            record("Holder", vec![field("Value", named("Base"))]),
        ],
        vec![message("HolderReport", named("Holder"))],
    )
}

/// Sections 21/42: the raw semantic closure stays honest -- it contains
/// `Holder` and `Base` only -- while the projection supplies every concrete
/// descendant as GENERATED SUPPORT. This is the pairing the whole task turns
/// on, so both halves are asserted together.
#[test]
fn closed_sum_descendants_are_support_not_contract_selection() {
    let schema = closed_sum_schema();
    let plan = plan_for(&schema, &oms_exchange("e1", "HolderReport"));

    // Task 030's semantics are unchanged: generation needs never leaked in.
    let raw = plan.selected_type_closure(&schema).expect("closure");
    assert_eq!(
        raw.iter()
            .map(|declaration| declaration.name.local_name.as_str())
            .collect::<Vec<_>>(),
        ["Base", "Holder"]
    );

    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");
    assert_eq!(locals(projection.selected_type_names()), ["Base", "Holder"]);
    // Section 43: the concrete NON-leaf is a variant too, so leaf-only
    // collapsing is wrong. Section 44: the abstract intermediate is retained
    // as structural ancestry, classified as support rather than selection.
    assert_eq!(
        locals(projection.generated_support_type_names()),
        [
            "ConcreteA",
            "ConcreteParent",
            "ConcreteLeaf",
            "AbstractMiddle",
            "MiddleLeaf"
        ]
    );
    // Section 22: schema order throughout, and the unrelated declaration is
    // absent despite sitting between support types in the source schema.
    assert_eq!(
        type_locals(projection.schema()),
        [
            "Base",
            "ConcreteA",
            "ConcreteParent",
            "ConcreteLeaf",
            "AbstractMiddle",
            "MiddleLeaf",
            "Holder"
        ]
    );
}

/// Section 74: the two identity lists are disjoint, and their union is
/// exactly the projected type set.
#[test]
fn selected_and_support_lists_are_disjoint_and_complete() {
    let schema = closed_sum_schema();
    let plan = plan_for(&schema, &oms_exchange("e1", "HolderReport"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    for name in projection.selected_type_names() {
        assert!(
            !projection.generated_support_type_names().contains(name),
            "{name:?} appears in both lists"
        );
    }
    assert_eq!(
        projection.selected_type_names().len() + projection.generated_support_type_names().len(),
        projection.schema().types.len()
    );
}

/// Sections 13/14/75: support expansion must iterate. `ConcreteA` is reached
/// only through `BaseA`'s closed sum; its `Nested` field reaches a SECOND
/// abstract value whose own descendant reaches `DeepNote`. A one-generation
/// expansion would stop two steps short.
#[test]
fn support_expansion_reaches_a_nested_closed_sum_fixed_point() {
    let schema = schema(
        vec![
            record("DeepNote", vec![]),
            abstract_record("BaseB", vec![]),
            derived("ConcreteB", "BaseB", vec![field("Note", named("DeepNote"))]),
            abstract_record("BaseA", vec![]),
            derived("ConcreteA", "BaseA", vec![field("Nested", named("BaseB"))]),
            record("Holder", vec![field("Value", named("BaseA"))]),
        ],
        vec![message("HolderReport", named("Holder"))],
    );
    let plan = plan_for(&schema, &oms_exchange("e1", "HolderReport"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(
        locals(projection.selected_type_names()),
        ["BaseA", "Holder"]
    );
    assert_eq!(
        locals(projection.generated_support_type_names()),
        ["DeepNote", "BaseB", "ConcreteB", "ConcreteA"]
    );
}

/// Sections 11/12: a message whose payload is DIRECTLY an abstract structural
/// value is a value position too, so its descendants are support.
#[test]
fn abstract_message_payload_is_a_value_position() {
    let schema = schema(
        vec![
            abstract_record("Base", vec![]),
            derived("Concrete", "Base", vec![]),
        ],
        vec![message("Report", named("Base"))],
    );
    let plan = plan_for(&schema, &oms_exchange("e1", "Report"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(locals(projection.selected_type_names()), ["Base"]);
    assert_eq!(
        locals(projection.generated_support_type_names()),
        ["Concrete"]
    );
}

/// Section 11: base-type ancestry alone is NOT an abstract value position. An
/// abstract base that nothing stores by value pulls in no sibling descendants.
#[test]
fn abstract_ancestry_alone_admits_no_descendants() {
    let schema = schema(
        vec![
            abstract_record("Base", vec![]),
            derived("SelectedChild", "Base", vec![]),
            derived("UnselectedSibling", "Base", vec![]),
        ],
        vec![message("Report", named("SelectedChild"))],
    );
    let plan = plan_for(&schema, &oms_exchange("e1", "Report"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(type_locals(projection.schema()), ["Base", "SelectedChild"]);
    assert!(projection.generated_support_type_names().is_empty());
}

// ---------------------------------------------------------------------
// Sections 15/16/17/58/59/76 -- world behavior and failure modes
// ---------------------------------------------------------------------

/// Section 15: a Task 026 optional zero-known-descendant abstract value keeps
/// its declaration (it is part of the source model) and gains NO invented
/// concrete support type.
#[test]
fn zero_descendant_abstract_value_gains_no_invented_support() {
    let schema = schema(
        vec![
            abstract_record("SidecarPoint", vec![]),
            record(
                "Holder",
                vec![FieldDecl {
                    cardinality: Cardinality::OPTIONAL_ONE,
                    ..field("Widget", named("SidecarPoint"))
                }],
            ),
        ],
        vec![message("Report", named("Holder"))],
    );
    let plan = plan_for(&schema, &oms_exchange("e1", "Report"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(type_locals(projection.schema()), ["SidecarPoint", "Holder"]);
    assert!(projection.generated_support_type_names().is_empty());
}

/// Section 16: under `OpenExtensions` a selected abstract structural VALUE
/// has no closed representation, so the projection fails closed rather than
/// emitting a partial sum or quietly switching worlds.
#[test]
fn open_world_abstract_value_fails_closed() {
    let schema = closed_sum_schema();
    let plan = plan_for(&schema, &oms_exchange("e1", "HolderReport"));
    let error = project_service_generation_schema(&plan, &schema, OPEN)
        .expect_err("an open-world abstract value has no representation");

    assert!(matches!(error, ServiceGenerationError::AbstractValue(_)));
    assert!(error.to_string().contains("open-extensions"));
}

/// Section 17: with no abstract structural value anywhere in the selection,
/// the two worlds project identically. `GenerationWorld` is not stored in
/// `SchemaIr`, so there is nothing left to differ.
#[test]
fn concrete_only_selection_projects_identically_in_both_worlds() {
    let schema = schema(
        vec![record("Payload", vec![]), record("Unselected", vec![])],
        vec![message("Report", named("Payload"))],
    );
    let plan = plan_for(&schema, &oms_exchange("e1", "Report"));

    assert_eq!(
        project_service_generation_schema(&plan, &schema, CLOSED).expect("closed"),
        project_service_generation_schema(&plan, &schema, OPEN).expect("open")
    );
}

/// Section 76: a plan resolved against schema A, projected against schema B,
/// fails deterministically instead of panicking on a failed lookup.
#[test]
fn plan_schema_mismatch_fails_without_panicking() {
    let schema_a = schema(
        vec![record("Payload", vec![])],
        vec![message("Report", named("Payload"))],
    );
    let plan = plan_for(&schema_a, &oms_exchange("e1", "Report"));

    // Schema B declares the payload but not the selected message.
    let schema_b = schema(vec![record("Payload", vec![])], vec![]);
    let error = project_service_generation_schema(&plan, &schema_b, CLOSED)
        .expect_err("a mismatched schema must fail");
    // Now diagnosed by the shared semantic binding, which runs before the
    // identity-only lookups and reports the absent selected message.
    assert!(
        matches!(
            error,
            ServiceGenerationError::PlanBinding(PlanBindingMismatch::Missing {
                role: MismatchRole::Message,
                ..
            })
        ),
        "{error:?}"
    );
    assert!(error.to_string().contains("different schema set"));
}

/// Section 58: a contract with zero OMS Message exchanges projects an empty
/// type model rather than fabricating one.
#[test]
fn non_uci_only_contract_projects_nothing() {
    let schema = schema(
        vec![record("Payload", vec![])],
        vec![message("Report", named("Payload"))],
    );
    let exchanges = "      - id: ss\n        kind: special_signal\n        direction: input\n        mandate: mandatory\n        name: Pulse\n        timing:\n          kind: asynchronous\n";
    let plan = plan_for(&schema, exchanges);
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert!(projection.selected_message_names().is_empty());
    assert!(projection.selected_type_names().is_empty());
    assert!(projection.generated_support_type_names().is_empty());
    assert!(projection.schema().types.is_empty());
}

/// Section 59: a primitive message payload has no named closure, and none is
/// invented. Task 032 generates selected TYPES, not placeholders.
#[test]
fn primitive_message_payload_invents_no_named_type() {
    let schema = schema(
        vec![record("Unselected", vec![])],
        vec![message("Report", TypeRef::primitive(PrimitiveKind::String))],
    );
    let plan = plan_for(&schema, &oms_exchange("e1", "Report"));
    let projection =
        project_service_generation_schema(&plan, &schema, CLOSED).expect("projection should build");

    assert_eq!(locals(projection.selected_message_names()), ["Report"]);
    assert!(projection.schema().types.is_empty());
}
