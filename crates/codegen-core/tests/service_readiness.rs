//! Contract-selected backend/world readiness tests.
//!
//! Every schema below is a controlled synthetic. The point of each is to make
//! one readiness rule observable in isolation: selection scoping, ordering,
//! deduplication, blocker attribution, and Task 024/026/028 world semantics.

use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, MismatchRole, PlanBindingMismatch,
    ServiceBackendReadiness, ServiceMessageBlocker, ServiceReadinessError,
    analyze_service_readiness, project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, MessageDecl, NamespaceDecl, PrimitiveKind,
    QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, TypeRef,
};
use ams_gra_oms_service_contract::{Contract, parse_yaml};

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

fn qualified(local: &str) -> QualifiedName {
    QualifiedName::new(NS, local)
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

fn optional_field(name: &str, type_ref: TypeRef) -> FieldDecl {
    FieldDecl {
        cardinality: Cardinality::OPTIONAL_ONE,
        ..field(name, type_ref)
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

fn derived_record(local: &str, base: &str, fields: Vec<FieldDecl>) -> TypeDecl {
    TypeDecl {
        base_type: Some(named(base)),
        ..record(local, fields)
    }
}

/// A declaration no backend can render today: an unconstrained `Duration`
/// primitive. Used as the controlled "unsupported construct" throughout.
fn unsupported(local: &str) -> TypeDecl {
    TypeDecl {
        name: qualified(local),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Primitive(PrimitiveKind::Duration),
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
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
        "contract_version: \"0.1\"\nservice:\n  name: Readiness Test\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n{exchanges}"
    ))
    .expect("test contract should be portable-valid")
}

fn oms_exchange(id: &str, message: &str) -> String {
    format!(
        "      - id: {id}\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {message}\n        topic: t.{id}\n        timing:\n          kind: asynchronous\n"
    )
}

fn readiness(
    schema: &SchemaIr,
    contract: &Contract,
    language: BackendLanguage,
    world: GenerationWorld,
) -> ServiceBackendReadiness {
    let plan = resolve_service_plan(contract, schema).expect("plan should resolve");
    analyze_service_readiness(&plan, schema, language, world).expect("readiness should compute")
}

fn blockers(readiness: &ServiceBackendReadiness) -> Vec<(&str, &ServiceMessageBlocker)> {
    readiness
        .blocked_messages
        .iter()
        .map(|blocked| (blocked.contract_message.as_str(), &blocked.blocker))
        .collect()
}

/// A schema whose SELECTED message closure is fully supported while several
/// UNSELECTED declarations are not. This is the Task 031 value proposition
/// fixture: full-schema generation would fail, the service still is ready.
fn mixed_schema() -> SchemaIr {
    schema(
        vec![
            record(
                "GoodPayload",
                vec![field("Value", primitive(PrimitiveKind::Float64))],
            ),
            // Two unrelated unsupported declarations. Neither is selected by
            // the ready contract, so neither may ever appear in its report.
            unsupported("UnsupportedA"),
            record(
                "UnrelatedA",
                vec![field("Occurred_At", named("UnsupportedA"))],
            ),
            unsupported("UnsupportedB"),
            record(
                "UnrelatedB",
                vec![field("Occurred_At", named("UnsupportedB"))],
            ),
        ],
        vec![
            message("GoodReport", named("GoodPayload")),
            message("BadReportA", named("UnrelatedA")),
            message("BadReportB", named("UnrelatedB")),
        ],
    )
}

// ---------------------------------------------------------------------
// Sections 25/26/27 -- selected-only scope
// ---------------------------------------------------------------------

/// Section 25: a contract whose selected closure is supported is READY even
/// though the same schema set contains unrenderable declarations. This is the
/// whole reason contract-selected analysis exists.
#[test]
fn selected_supported_closure_is_ready_despite_unselected_unsupported_declarations() {
    let schema = mixed_schema();
    let contract = contract(&oms_exchange("e1", "GoodReport"));
    for language in BackendLanguage::ALL {
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let result = readiness(&schema, &contract, language, world);
            assert!(result.is_ready(), "{language:?} {world:?} should be ready");
            assert_eq!(result.selected_messages_total, 1);
            assert_eq!(result.selected_messages_renderable, 1);
            assert_eq!(result.selected_types_total, 1);
            assert_eq!(result.selected_types_renderable, 1);
        }
    }
}

/// Section 27: blockers outside the selected closure must not leak into the
/// report, in either direction.
#[test]
fn unselected_unsupported_declarations_are_never_reported() {
    let result = readiness(
        &mixed_schema(),
        &contract(&oms_exchange("e1", "GoodReport")),
        BackendLanguage::Rust,
        GenerationWorld::ClosedSchemaSet,
    );
    assert!(result.unsupported_types.is_empty());
    assert!(result.blocked_messages.is_empty());
}

/// Section 26: selecting a message whose closure reaches the unsupported
/// declaration flips the verdict, names the declaration, and attributes it as
/// that message's blocker.
#[test]
fn selected_unsupported_closure_is_not_ready_with_deterministic_blocker() {
    let schema = mixed_schema();
    let contract = contract(&oms_exchange("e1", "BadReportA"));
    for language in BackendLanguage::ALL {
        let result = readiness(
            &schema,
            &contract,
            language,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(!result.is_ready(), "{language:?} should not be ready");
        assert_eq!(result.unsupported_types, vec![qualified("UnsupportedA")]);
        assert_eq!(result.selected_messages_renderable, 0);
        assert_eq!(
            blockers(&result),
            vec![(
                "BadReportA",
                &ServiceMessageBlocker::Declaration(qualified("UnsupportedA"))
            )]
        );
        // The other unsupported declaration is not in this closure.
        assert!(
            !result
                .unsupported_types
                .contains(&qualified("UnsupportedB"))
        );
    }
}

// ---------------------------------------------------------------------
// Sections 28/29/30/31 -- ordering, deduplication, non-UCI exchanges
// ---------------------------------------------------------------------

/// Section 28: a closure with several non-renderable declarations reports all
/// of them in schema order, and attributes the EARLIEST one as the first
/// blocker -- not an arbitrary map member.
#[test]
fn multiple_blockers_report_schema_order_and_earliest_first_blocker() {
    // `LateBad` appears FIRST among the payload's fields but LAST in the
    // schema, so field-order and schema-order implementations disagree here.
    let schema = schema(
        vec![
            unsupported("EarlyBad"),
            record(
                "MultiPayload",
                vec![
                    field("Late", named("LateBad")),
                    field("Early", named("EarlyBad")),
                ],
            ),
            unsupported("LateBad"),
        ],
        vec![message("MultiReport", named("MultiPayload"))],
    );
    let result = readiness(
        &schema,
        &contract(&oms_exchange("e1", "MultiReport")),
        BackendLanguage::Rust,
        GenerationWorld::ClosedSchemaSet,
    );
    assert!(!result.is_ready());
    // Schema declaration order, not field order and not alphabetical.
    assert_eq!(
        result.unsupported_types,
        vec![qualified("EarlyBad"), qualified("LateBad")]
    );
    assert_eq!(
        blockers(&result),
        vec![(
            "MultiReport",
            &ServiceMessageBlocker::Declaration(qualified("EarlyBad"))
        )]
    );
}

/// Section 29: blocked messages follow contract first-occurrence order while
/// unsupported types stay in schema order. Reversing the contract's exchange
/// order reverses only the former.
#[test]
fn blocked_message_order_follows_contract_while_type_order_follows_schema() {
    let schema = mixed_schema();
    let forward = readiness(
        &schema,
        &contract(&format!(
            "{}{}",
            oms_exchange("e1", "BadReportA"),
            oms_exchange("e2", "BadReportB")
        )),
        BackendLanguage::Rust,
        GenerationWorld::ClosedSchemaSet,
    );
    let reversed = readiness(
        &schema,
        &contract(&format!(
            "{}{}",
            oms_exchange("e1", "BadReportB"),
            oms_exchange("e2", "BadReportA")
        )),
        BackendLanguage::Rust,
        GenerationWorld::ClosedSchemaSet,
    );

    let names = |result: &ServiceBackendReadiness| {
        result
            .blocked_messages
            .iter()
            .map(|blocked| blocked.contract_message.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&forward), ["BadReportA", "BadReportB"]);
    assert_eq!(names(&reversed), ["BadReportB", "BadReportA"]);
    // Schema order is contract-independent, so it does NOT reverse.
    let schema_order = vec![qualified("UnsupportedA"), qualified("UnsupportedB")];
    assert_eq!(forward.unsupported_types, schema_order);
    assert_eq!(reversed.unsupported_types, schema_order);
    // Each message keeps its own blocker.
    assert_eq!(
        forward.blocked_messages[0].blocker,
        ServiceMessageBlocker::Declaration(qualified("UnsupportedA"))
    );
    assert_eq!(
        forward.blocked_messages[1].blocker,
        ServiceMessageBlocker::Declaration(qualified("UnsupportedB"))
    );
}

/// Section 30: several exchanges may select one message. The plan keeps both
/// occurrences; readiness analyzes the unique message once.
#[test]
fn repeated_message_selection_is_analyzed_once() {
    let schema = mixed_schema();
    let contract = contract(&format!(
        "{}{}",
        oms_exchange("e1", "BadReportA"),
        oms_exchange("e2", "BadReportA")
    ));
    let plan = resolve_service_plan(&contract, &schema).expect("plan should resolve");
    assert_eq!(plan.oms_message_exchange_count(), 2);
    assert_eq!(plan.selected_messages().len(), 1);

    let result = analyze_service_readiness(
        &plan,
        &schema,
        BackendLanguage::Rust,
        GenerationWorld::ClosedSchemaSet,
    )
    .expect("readiness should compute");
    assert_eq!(result.selected_messages_total, 1);
    assert_eq!(result.blocked_messages.len(), 1);
    assert_eq!(result.unsupported_types, vec![qualified("UnsupportedA")]);
}

/// Section 31: a contract with only non-UCI exchanges selects no UCI type
/// model at all, so it is vacuously ready for every backend and world.
#[test]
fn non_uci_only_contract_is_vacuously_ready() {
    let schema = mixed_schema();
    let timing = "        timing:\n          kind: asynchronous\n";
    let data_transfer = format!(
        "      - id: dt\n        kind: data_transfer\n        direction: input\n        mandate: mandatory\n        name: Bulk\n        protocol: sftp\n        data_type: imagery\n        data_format: nitf\n        sharing_pattern: push\n{timing}"
    );
    let named_only = |id: &str, kind: &str, name: &str| {
        format!(
            "      - id: {id}\n        kind: {kind}\n        direction: input\n        mandate: mandatory\n        name: {name}\n{timing}"
        )
    };
    let contract = contract(&format!(
        "{data_transfer}{}{}{}",
        named_only("ss", "special_signal", "Pulse"),
        named_only("sx", "security_exchange", "Creds"),
        named_only("nm", "non_oms_message", "Legacy")
    ));
    for language in BackendLanguage::ALL {
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let result = readiness(&schema, &contract, language, world);
            assert_eq!(result.selected_messages_total, 0);
            assert_eq!(result.selected_types_total, 0);
            assert!(result.is_ready(), "{language:?} {world:?} should be ready");
        }
    }
}

// ---------------------------------------------------------------------
// Corrective cleanup -- readiness must not swallow projection failures
// ---------------------------------------------------------------------

/// The shared fixture for the projection-error tests: a selected closure
/// holding an abstract structural VALUE, which projects cleanly in the closed
/// world and fails closed in the open world.
fn abstract_value_schema() -> SchemaIr {
    schema(
        vec![
            abstract_record("AbstractBase", vec![]),
            derived_record(
                "ConcreteOne",
                "AbstractBase",
                vec![field("Value", primitive(PrimitiveKind::Float64))],
            ),
            record("ValuePayload", vec![field("Item", named("AbstractBase"))]),
        ],
        vec![message("ValueReport", named("ValuePayload"))],
    )
}

/// The abstract-value projection failure is the one class readiness is
/// allowed to treat as already-reported, and this pins the invariant that
/// permits it: the result is genuinely NOT READY, with the failure attributed
/// per declaration and per message, before the projection error is ignored.
///
/// If that stopped holding, ignoring the projection error would produce a
/// false READY -- so it is proven rather than assumed.
#[test]
fn an_abstract_value_projection_failure_is_already_reported_as_not_ready() {
    let schema = abstract_value_schema();
    let contract = contract(&oms_exchange("e1", "ValueReport"));
    let plan = resolve_service_plan(&contract, &schema).expect("plan should resolve");

    // The projection really does fail in the open world.
    assert!(
        project_service_generation_schema(&plan, &schema, GenerationWorld::OpenExtensions).is_err(),
        "the fixture must really fail projection, or this proves nothing"
    );

    for language in BackendLanguage::ALL {
        // Readiness nonetheless computes, and is NOT READY with attribution,
        // which is exactly what makes ignoring the duplicate error safe.
        let result =
            analyze_service_readiness(&plan, &schema, language, GenerationWorld::OpenExtensions)
                .expect("an already-reported capability failure must not become an error");
        assert!(!result.is_ready(), "{language:?} must not be READY");
        assert!(
            !result.unsupported_types.is_empty() || !result.blocked_messages.is_empty(),
            "{language:?} must attribute the failure per declaration or message"
        );
    }
}

/// A plan resolved against a different schema set must surface as a typed
/// binding error from the projection path rather than being swallowed into
/// "no backend blocker".
#[test]
fn a_projection_binding_mismatch_is_propagated_not_swallowed() {
    let original = schema(
        vec![record(
            "Payload",
            vec![field("Value", primitive(PrimitiveKind::Float64))],
        )],
        vec![message("Report", named("Payload"))],
    );
    let contract = contract(&oms_exchange("e1", "Report"));
    let plan = resolve_service_plan(&contract, &original).expect("plan should resolve");

    // Same names, different semantics: only the semantic binding sees this.
    let changed = schema(
        vec![record(
            "Payload",
            vec![field("Value", primitive(PrimitiveKind::Boolean))],
        )],
        vec![message("Report", named("Payload"))],
    );

    for language in BackendLanguage::ALL {
        let error =
            analyze_service_readiness(&plan, &changed, language, GenerationWorld::ClosedSchemaSet)
                .expect_err("a wrong-schema plan must not silently report readiness");
        assert!(
            matches!(
                error,
                ServiceReadinessError::PlanBinding(PlanBindingMismatch::Changed { .. })
            ),
            "{language:?}: {error:?}"
        );
    }
}

/// Readiness must never report READY for a selection whose projection cannot
/// be produced, under any world or backend, and must not panic. This is the
/// property the removed catch-all projection arm put at risk.
#[test]
fn readiness_is_never_ready_when_projection_fails() {
    let schema = abstract_value_schema();
    let contract = contract(&oms_exchange("e1", "ValueReport"));
    let plan = resolve_service_plan(&contract, &schema).expect("plan should resolve");

    for language in BackendLanguage::ALL {
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let projection_failed =
                project_service_generation_schema(&plan, &schema, world).is_err();
            match analyze_service_readiness(&plan, &schema, language, world) {
                Ok(result) => assert!(
                    !projection_failed || !result.is_ready(),
                    "{language:?} {world:?} reported READY despite a failed projection"
                ),
                Err(error) => assert!(
                    projection_failed,
                    "{language:?} {world:?} errored without a projection failure: {error}"
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------
// Sections 32/33/34 -- Task 024/026/028 world semantics, inherited
// ---------------------------------------------------------------------

/// Section 32: a selected closure containing an abstract structural VALUE with
/// known concrete descendants is ready under `ClosedSchemaSet` (Task 024 closed
/// sum) and not ready under `OpenExtensions` (Task 028 fail-closed).
#[test]
fn abstract_value_readiness_differs_by_world() {
    let schema = schema(
        vec![
            abstract_record("AbstractBase", vec![]),
            derived_record(
                "ConcreteOne",
                "AbstractBase",
                vec![field("Value", primitive(PrimitiveKind::Float64))],
            ),
            record("ValuePayload", vec![field("Item", named("AbstractBase"))]),
        ],
        vec![message("ValueReport", named("ValuePayload"))],
    );
    let contract = contract(&oms_exchange("e1", "ValueReport"));

    for language in BackendLanguage::ALL {
        let closed = readiness(
            &schema,
            &contract,
            language,
            GenerationWorld::ClosedSchemaSet,
        );
        assert!(closed.is_ready(), "{language:?} closed should be ready");

        let open = readiness(
            &schema,
            &contract,
            language,
            GenerationWorld::OpenExtensions,
        );
        assert!(!open.is_ready(), "{language:?} open should not be ready");
        // `AbstractBase` itself is still a legitimate inheritance ancestor, so
        // the declaration that cannot be rendered is the one holding the
        // abstract VALUE position -- exactly the Task 028 distinction.
        assert_eq!(open.unsupported_types, vec![qualified("ValuePayload")]);
        assert_eq!(
            blockers(&open),
            vec![(
                "ValueReport",
                &ServiceMessageBlocker::Declaration(qualified("ValuePayload"))
            )]
        );
    }
}

/// Section 33: a `0..1` field typed as a zero-known-descendant abstract target
/// is elided under `ClosedSchemaSet` (Task 026) and fails closed under
/// `OpenExtensions`, where zero known descendants does not prove uninhabited.
#[test]
fn zero_descendant_optional_readiness_differs_by_world() {
    let schema = schema(
        vec![
            abstract_record("NoDescendants", vec![]),
            record(
                "OptionalPayload",
                vec![
                    field("Value", primitive(PrimitiveKind::Float64)),
                    optional_field("Maybe", named("NoDescendants")),
                ],
            ),
        ],
        vec![message("OptionalReport", named("OptionalPayload"))],
    );
    let contract = contract(&oms_exchange("e1", "OptionalReport"));

    for language in BackendLanguage::ALL {
        assert!(
            readiness(
                &schema,
                &contract,
                language,
                GenerationWorld::ClosedSchemaSet
            )
            .is_ready(),
            "{language:?} closed should elide and be ready"
        );
        assert!(
            !readiness(
                &schema,
                &contract,
                language,
                GenerationWorld::OpenExtensions
            )
            .is_ready(),
            "{language:?} open should not be ready"
        );
    }
}

/// Section 34: an abstract declaration used ONLY as inheritance ancestry, with
/// no value position typed as it, is unaffected by the world. This is the
/// Task 028 base-only distinction.
#[test]
fn base_only_abstract_ancestry_is_world_independent() {
    let schema = schema(
        vec![
            abstract_record("AncestorOnly", vec![]),
            derived_record(
                "Concrete",
                "AncestorOnly",
                vec![field("Value", primitive(PrimitiveKind::Float64))],
            ),
            // The value position names the CONCRETE type, never the ancestor.
            record("AncestryPayload", vec![field("Item", named("Concrete"))]),
        ],
        vec![message("AncestryReport", named("AncestryPayload"))],
    );
    let contract = contract(&oms_exchange("e1", "AncestryReport"));

    for language in BackendLanguage::ALL {
        let closed = readiness(
            &schema,
            &contract,
            language,
            GenerationWorld::ClosedSchemaSet,
        );
        let open = readiness(
            &schema,
            &contract,
            language,
            GenerationWorld::OpenExtensions,
        );
        assert!(closed.is_ready(), "{language:?} closed should be ready");
        assert!(open.is_ready(), "{language:?} open should be ready");
        assert_eq!(closed.unsupported_types, open.unsupported_types);
        assert_eq!(
            closed.selected_types_renderable,
            open.selected_types_renderable
        );
    }
}

// ---------------------------------------------------------------------
// Sections 11/18/21/49 -- blocker model, actual capability, defensiveness
// ---------------------------------------------------------------------

/// Section 11: a primitive payload the backend cannot render is reported as a
/// primitive blocker rather than as a declaration.
#[test]
fn unsupported_primitive_payload_is_reported_as_a_primitive_blocker() {
    let schema = schema(
        Vec::new(),
        vec![message(
            "DurationReport",
            primitive(PrimitiveKind::Duration),
        )],
    );
    let result = readiness(
        &schema,
        &contract(&oms_exchange("e1", "DurationReport")),
        BackendLanguage::Rust,
        GenerationWorld::ClosedSchemaSet,
    );
    assert!(!result.is_ready());
    // A primitive payload pulls in no named declaration at all.
    assert_eq!(result.selected_types_total, 0);
    assert_eq!(
        blockers(&result),
        vec![(
            "DurationReport",
            &ServiceMessageBlocker::Primitive(PrimitiveKind::Duration)
        )]
    );
}

/// Section 18: readiness measures capability TODAY. A construct that only a
/// hypothetical `FeatureFamily` would enable must stay NOT READY; there is no
/// "ready if X were implemented" answer.
#[test]
fn readiness_never_enables_hypothetical_features() {
    let result = readiness(
        &mixed_schema(),
        &contract(&oms_exchange("e1", "BadReportA")),
        BackendLanguage::Rust,
        GenerationWorld::ClosedSchemaSet,
    );
    // `PrimitiveExpansion` would make `Duration` renderable; it is not enabled.
    assert!(!result.is_ready());
}

/// Section 21: analyzing a plan against a DIFFERENT schema set than the one
/// that produced it fails deterministically instead of panicking.
#[test]
fn plan_schema_mismatch_fails_deterministically() {
    let original = mixed_schema();
    let plan = resolve_service_plan(&contract(&oms_exchange("e1", "GoodReport")), &original)
        .expect("plan should resolve");
    // A schema set that declares the payload but not the message.
    let other = schema(
        vec![record(
            "GoodPayload",
            vec![field("Value", primitive(PrimitiveKind::Float64))],
        )],
        Vec::new(),
    );
    let error = analyze_service_readiness(
        &plan,
        &other,
        BackendLanguage::Rust,
        GenerationWorld::ClosedSchemaSet,
    )
    .expect_err("a mismatched schema set must fail");
    // Now diagnosed by the shared semantic binding, which runs first and
    // names the absent selected message.
    assert!(
        matches!(
            error,
            ServiceReadinessError::PlanBinding(PlanBindingMismatch::Missing {
                role: MismatchRole::Message,
                ..
            })
        ),
        "{error:?}"
    );
    assert!(error.to_string().contains("GoodReport"));
}

/// Section 49: repeated analyses of identical inputs produce identical
/// results, including list ordering. Nothing may depend on hash iteration.
#[test]
fn readiness_results_are_deterministic() {
    let schema = mixed_schema();
    let contract = contract(&format!(
        "{}{}",
        oms_exchange("e1", "BadReportA"),
        oms_exchange("e2", "BadReportB")
    ));
    let first = readiness(
        &schema,
        &contract,
        BackendLanguage::Cpp,
        GenerationWorld::ClosedSchemaSet,
    );
    for _ in 0..4 {
        assert_eq!(
            readiness(
                &schema,
                &contract,
                BackendLanguage::Cpp,
                GenerationWorld::ClosedSchemaSet
            ),
            first
        );
    }
}

/// The reported language and world are exactly what the caller requested, so
/// saved readiness evidence can never become ambiguous about what it measured.
#[test]
fn readiness_records_the_requested_language_and_world() {
    let schema = mixed_schema();
    let contract = contract(&oms_exchange("e1", "GoodReport"));
    for language in BackendLanguage::ALL {
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let result = readiness(&schema, &contract, language, world);
            assert_eq!(result.language, language);
            assert_eq!(result.world, world);
        }
    }
}

// ---------------------------------------------------------------------
// Sections 13/15/17 -- one shared capability model
// ---------------------------------------------------------------------

/// Sections 13/17: readiness and full-schema `BackendCoverage` must agree,
/// because they are the same computation. When the contract happens to select
/// EVERY message in the schema, the renderable message counts must match
/// exactly -- if they ever diverged, a second capability model would exist.
#[test]
fn readiness_agrees_with_full_schema_coverage_when_everything_is_selected() {
    let schema = mixed_schema();
    let contract = contract(&format!(
        "{}{}{}",
        oms_exchange("e1", "GoodReport"),
        oms_exchange("e2", "BadReportA"),
        oms_exchange("e3", "BadReportB")
    ));
    for language in BackendLanguage::ALL {
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let result = readiness(&schema, &contract, language, world);
            let analysis = CoverageAnalysis::new(&schema, world).expect("analysis should build");
            // Parity is only meaningful while the schema passes global
            // preflight: once it does not, coverage reports zero generable
            // message closures by design and the comparison below would be
            // measuring the preflight failure rather than the capability
            // model. Asserting it keeps this test honest about what it
            // covers.
            assert!(
                analysis.backend_preflight_error(language).is_none(),
                "{language:?} {world:?}: the parity fixture must pass global preflight"
            );
            let coverage = analysis
                .backend_coverage(language)
                .expect("coverage should compute");
            assert_eq!(result.selected_messages_total, coverage.messages_total);
            assert_eq!(
                result.selected_messages_renderable, coverage.message_closures_renderable,
                "{language:?} {world:?} must use one capability model"
            );
        }
    }
}
