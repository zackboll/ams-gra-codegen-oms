//! Tasks 047-048: the language-neutral `ServiceApiModel`, its lowering from a
//! `ServicePlan`, its shared naming preflight, and its readiness integration.
//!
//! Every schema here is a controlled synthetic, built directly as `SchemaIr`.

use ams_gra_oms_codegen_core::{
    BackendLanguage, GenerationWorld, ServiceApiError, ServiceApiExchangeKind, ServiceApiModel,
    ServiceApiNameError, ServiceApiNameOwner, ServiceApiRegion, UnboundPayload,
    UnboundPayloadReason, analyze_service_readiness, build_service_api_model,
    project_service_generation_schema, resolve_service_plan, service_api_preflight,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, MessageDecl, NamespaceDecl, PrimitiveKind,
    QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, TypeRef,
};
use ams_gra_oms_service_contract::{Direction, Mandate, ServiceKind, parse_yaml};

const NS: &str = "urn:test";
const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn source() -> SourceRef {
    SourceRef {
        document: "fixture.xsd".to_owned(),
        line: None,
    }
}

fn qualified(local: &str) -> QualifiedName {
    QualifiedName::new(NS, local)
}

fn record(local: &str, fields: Vec<(&str, TypeRef)>) -> TypeDecl {
    TypeDecl {
        name: qualified(local),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Record {
            fields: fields
                .into_iter()
                .map(|(name, type_ref)| FieldDecl {
                    name: name.to_owned(),
                    type_ref,
                    cardinality: Cardinality::REQUIRED_ONE,
                    nillable: false,
                    constraints: ConstraintSet::default(),
                    documentation: None,
                    source: source(),
                })
                .collect(),
        },
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

/// `ReportA -> PayloadA`, `ReportB -> PayloadB`, and a primitive-payload
/// `TextReport`. Message and payload local names differ on purpose.
fn schema() -> SchemaIr {
    SchemaIr {
        schema_version: Some("000.1.0".to_owned()),
        namespaces: vec![NamespaceDecl {
            uri: NS.to_owned(),
            preferred_prefix: Some("t".to_owned()),
        }],
        types: vec![
            record(
                "PayloadA",
                vec![("Text", TypeRef::primitive(PrimitiveKind::String))],
            ),
            record(
                "PayloadB",
                vec![("Flag", TypeRef::primitive(PrimitiveKind::Boolean))],
            ),
        ],
        messages: vec![
            message("ReportA", TypeRef::named(qualified("PayloadA"))),
            message("ReportB", TypeRef::named(qualified("PayloadB"))),
            message("TextReport", TypeRef::primitive(PrimitiveKind::String)),
        ],
    }
}

fn contract(functions: &str) -> ams_gra_oms_service_contract::Contract {
    parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: \"Api Test: Service!\"\n  version: \"2.0.1\"\n  kind: isolator\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n{functions}"
    ))
    .expect("test contract should be portable-valid")
}

fn function(id: &str, name: &str, exchanges: &str) -> String {
    format!(
        "  - id: {id}\n    name: \"{name}\"\n    category: specific\n    applicability: applicable\n    exchanges:\n{exchanges}"
    )
}

fn oms(id: &str, direction: &str, mandate: &str, message: &str, topic: &str) -> String {
    format!(
        "      - id: {id}\n        kind: oms_message\n        direction: {direction}\n        mandate: {mandate}\n        message: {message}\n        topic: {topic}\n        timing:\n          kind: asynchronous\n"
    )
}

fn non_uci(id: &str, kind: &str, direction: &str, mandate: &str) -> String {
    let extra = if kind == "data_transfer" {
        "        protocol: p\n        data_type: d\n        data_format: f\n        sharing_pattern: s\n"
    } else {
        ""
    };
    format!(
        "      - id: {id}\n        kind: {kind}\n        direction: {direction}\n        mandate: {mandate}\n        name: N\n{extra}        timing:\n          kind: asynchronous\n"
    )
}

/// Resolve, project, and lower in one step, as `service-generate` does.
fn lower(functions: &str) -> Result<ServiceApiModel, ServiceApiError> {
    let schema = schema();
    let plan = resolve_service_plan(&contract(functions), &schema).expect("plan");
    let projection = project_service_generation_schema(&plan, &schema, CLOSED).expect("project");
    build_service_api_model(&plan, projection.schema(), CLOSED)
}

/// Contract order, every occurrence, all five kinds, and the per-kind
/// metadata survive lowering exactly. Only OMS exchanges get a binding.
#[test]
fn lowering_preserves_contract_order_and_all_five_kinds() {
    let exchanges = [
        oms("oms-in", "input", "mandatory", "ReportA", "topic.a"),
        non_uci("dt", "data_transfer", "output", "optional"),
        non_uci("sig", "special_signal", "input", "mandatory"),
        non_uci("sec", "security_exchange", "output", "mandatory"),
        non_uci("legacy", "non_oms_message", "input", "optional"),
    ]
    .concat();
    let model = lower(&function("first-fn", "First: Fn!", &exchanges)).expect("lower");

    assert_eq!(model.service_name(), "Api Test: Service!");
    assert_eq!(model.service_version(), "2.0.1");
    assert_eq!(model.service_kind(), ServiceKind::Isolator);
    assert!(model.emits_type_model());
    assert_eq!(model.functions().len(), 1);
    let function = &model.functions()[0];
    assert_eq!(function.id(), "first-fn");
    assert_eq!(function.name(), "First: Fn!");

    let observed = function
        .exchanges()
        .iter()
        .map(|exchange| {
            (
                exchange.id(),
                exchange.kind().as_str(),
                exchange.direction(),
                exchange.mandate(),
                exchange.oms_binding().is_some(),
            )
        })
        .collect::<Vec<_>>();
    use Direction::{Input, Output};
    use Mandate::{Mandatory, Optional};
    assert_eq!(
        observed,
        [
            ("oms-in", "oms_message", Input, Mandatory, true),
            ("dt", "data_transfer", Output, Optional, false),
            ("sig", "special_signal", Input, Mandatory, false),
            ("sec", "security_exchange", Output, Mandatory, false),
            ("legacy", "non_oms_message", Input, Optional, false),
        ]
    );
    // No payload is fabricated for the four non-UCI kinds.
    for exchange in &function.exchanges()[1..] {
        assert!(!matches!(
            exchange.kind(),
            ServiceApiExchangeKind::OmsMessage(_)
        ));
    }
}

/// The OMS binding carries the plan's resolved identities verbatim: the
/// message is `ReportA` and the payload is `PayloadA`, not a local-name guess.
#[test]
fn oms_binding_uses_the_resolved_payload_not_the_message_name() {
    let model = lower(&function(
        "f",
        "F",
        &oms("e", "output", "optional", "ReportA", "a.topic"),
    ))
    .expect("lower");
    let binding = model.functions()[0].exchanges()[0]
        .oms_binding()
        .expect("OMS binding");
    assert_eq!(binding.topic(), "a.topic");
    assert_eq!(binding.message_name(), &qualified("ReportA"));
    assert_eq!(binding.payload_name(), &qualified("PayloadA"));
    assert_eq!(
        binding.payload_type(),
        &TypeRef::named(qualified("PayloadA"))
    );
}

/// Two occurrences of one message in two functions stay two endpoints, while
/// the plan's selected_messages() still deduplicates them for the closure.
#[test]
fn repeated_message_selection_stays_two_endpoints() {
    let functions = [
        function(
            "fn-a",
            "A",
            &oms("input-1", "input", "mandatory", "ReportA", "topic.a"),
        ),
        function(
            "fn-b",
            "B",
            &oms("input-2", "output", "optional", "ReportA", "topic.b"),
        ),
    ]
    .concat();
    let schema = schema();
    let plan = resolve_service_plan(&contract(&functions), &schema).expect("plan");
    assert_eq!(plan.selected_messages().len(), 1, "type selection dedupes");

    let model = lower(&functions).expect("lower");
    let endpoints = model
        .functions()
        .iter()
        .flat_map(|function| {
            function.exchanges().iter().map(move |exchange| {
                let binding = exchange.oms_binding().expect("OMS");
                (
                    function.id(),
                    exchange.id(),
                    binding.topic(),
                    binding.payload_name(),
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        endpoints,
        [
            ("fn-a", "input-1", "topic.a", &qualified("PayloadA")),
            ("fn-b", "input-2", "topic.b", &qualified("PayloadA")),
        ]
    );
}

/// A service with only non-UCI exchanges lowers to a real model with no type
/// model to import and no fabricated payload.
#[test]
fn zero_oms_service_lowers_without_a_type_model() {
    let model = lower(&function(
        "only-signals",
        "Only Signals",
        &non_uci("s", "special_signal", "input", "mandatory"),
    ))
    .expect("lower");
    assert!(!model.emits_type_model());
    assert_eq!(model.functions()[0].exchanges().len(), 1);
    assert!(model.functions()[0].exchanges()[0].oms_binding().is_none());
}

/// A primitive message payload has no generated model type, so no Payload
/// alias can be bound and none is invented: lowering fails closed.
#[test]
fn primitive_payload_cannot_be_bound() {
    let error = lower(&function(
        "f",
        "F",
        &oms("text", "input", "mandatory", "TextReport", "t"),
    ))
    .expect_err("primitive payload must not bind");
    assert_eq!(
        error,
        ServiceApiError::UnboundPayload(Box::new(UnboundPayload {
            function: "f".into(),
            exchange: "text".into(),
            message: qualified("TextReport"),
            reason: UnboundPayloadReason::Primitive(PrimitiveKind::String),
        }))
    );
}

/// Lowering against a schema that does not carry the resolved message is
/// rejected rather than silently producing an unbound Payload.
#[test]
fn unprojected_message_is_rejected() {
    let schema = schema();
    let plan = resolve_service_plan(
        &contract(&function(
            "f",
            "F",
            &oms("e", "input", "mandatory", "ReportA", "t"),
        )),
        &schema,
    )
    .expect("plan");
    let mut stripped = schema.clone();
    stripped.messages.clear();
    stripped.types.clear();
    let error = build_service_api_model(&plan, &stripped, CLOSED).expect_err("must fail");
    assert!(matches!(
        &error,
        ServiceApiError::UnboundPayload(unbound)
            if unbound.reason == UnboundPayloadReason::MessageNotProjected
    ));
}

// ---------------------------------------------------------------------
// Readiness integration
// ---------------------------------------------------------------------

/// Colliding function IDs: the contract is portable-valid, every selected
/// type is renderable, and yet readiness is NOT READY in every language,
/// with the typed cause reported ONLY in `service_api_blocker`. Counts are
/// unchanged, and no fake unsupported type or blocked message appears.
#[test]
fn normalization_collision_makes_readiness_not_ready_without_touching_counts() {
    let functions = [
        function(
            "foo-bar",
            "Dash",
            &oms("a", "input", "mandatory", "ReportA", "t.a"),
        ),
        function(
            "foo_bar",
            "Underscore",
            &oms("b", "input", "mandatory", "ReportB", "t.b"),
        ),
    ]
    .concat();
    let schema = schema();
    let plan = resolve_service_plan(&contract(&functions), &schema).expect("valid contract");
    for language in BackendLanguage::ALL {
        let readiness = analyze_service_readiness(&plan, &schema, language, CLOSED).unwrap();
        assert!(!readiness.is_ready(), "{language:?}");
        assert_eq!(readiness.selected_types_total, 2);
        assert_eq!(readiness.selected_types_renderable, 2);
        assert_eq!(readiness.selected_messages_total, 2);
        assert_eq!(readiness.selected_messages_renderable, 2);
        assert!(readiness.unsupported_types.is_empty());
        assert!(readiness.blocked_messages.is_empty());
        assert!(readiness.backend_blocker.is_none());
        let Some(ServiceApiError::Name(ServiceApiNameError::Collision(collision))) =
            &readiness.service_api_blocker
        else {
            panic!("{language:?}: {:?}", readiness.service_api_blocker);
        };
        assert_eq!(collision.language, language);
        assert_eq!(collision.region, ServiceApiRegion::Service);
        assert_eq!(
            collision.first,
            ServiceApiNameOwner::Function("foo-bar".into())
        );
        assert_eq!(
            collision.second,
            ServiceApiNameOwner::Function("foo_bar".into())
        );
    }
}

/// A safe service's readiness is unchanged by Task 047: READY, no blocker,
/// and the full preflight agrees by returning the model.
#[test]
fn safe_service_stays_ready_and_preflight_agrees() {
    let functions = function(
        "position-input-example",
        "Position Input Example",
        &oms(
            "position-report-input",
            "input",
            "mandatory",
            "ReportA",
            "PositionReport",
        ),
    );
    let schema = schema();
    let plan = resolve_service_plan(&contract(&functions), &schema).expect("plan");
    let projection = project_service_generation_schema(&plan, &schema, CLOSED).expect("project");
    for language in BackendLanguage::ALL {
        let readiness = analyze_service_readiness(&plan, &schema, language, CLOSED).unwrap();
        assert!(readiness.is_ready(), "{language:?}");
        assert!(readiness.service_api_blocker.is_none());
        assert!(service_api_preflight(&plan, projection.schema(), language, CLOSED).is_ok());
    }
}

/// A primitive-payload message is renderable as a TYPE selection (there is
/// nothing to render), but it has no Payload to bind, so readiness reports a
/// service API boundary instead of claiming READY and failing in generation.
#[test]
fn primitive_payload_is_a_service_api_boundary_in_readiness() {
    let schema = schema();
    let plan = resolve_service_plan(
        &contract(&function(
            "f",
            "F",
            &oms("text", "input", "mandatory", "TextReport", "t"),
        )),
        &schema,
    )
    .expect("plan");
    let readiness =
        analyze_service_readiness(&plan, &schema, BackendLanguage::Rust, CLOSED).unwrap();
    assert!(readiness.blocked_messages.is_empty());
    assert!(!readiness.is_ready());
    assert!(matches!(
        &readiness.service_api_blocker,
        Some(ServiceApiError::UnboundPayload(unbound))
            if unbound.reason == UnboundPayloadReason::Primitive(PrimitiveKind::String)
    ));
}

/// `schema()` moved into another namespace URI.
fn schema_in(uri: &str) -> SchemaIr {
    let mut schema = schema();
    schema.namespaces[0].uri = uri.to_owned();
    let rename = |name: &mut QualifiedName| name.namespace_uri = uri.to_owned();
    for declaration in &mut schema.types {
        rename(&mut declaration.name);
    }
    for message in &mut schema.messages {
        rename(&mut message.name);
        if let ams_gra_oms_ir::TypeRefTarget::Named(name) = &mut message.payload_type.target {
            rename(name);
        }
    }
    schema
}

/// Task 047 corrective: READY now covers the complete artifact set. An Ada
/// model package hidden from the wrapper is NOT READY for Ada only, with a
/// typed `ModelWrapperNameCollision` in `service_api_blocker`, every count
/// unchanged, and no fake UCI blocker. Namespaces that merely look like the
/// wrapper stay READY everywhere.
#[test]
fn artifact_boundary_is_part_of_readiness() {
    use ams_gra_oms_codegen_core::{ServiceApiModelConflict, ServiceApiModelEntity};
    let functions = function(
        "mission-data",
        "Mission Data",
        &oms("a-input", "input", "mandatory", "ReportA", "t.a"),
    );
    let contract = contract(&functions);

    let hidden = schema_in("urn:topic:model");
    let plan = resolve_service_plan(&contract, &hidden).expect("plan");
    for language in BackendLanguage::ALL {
        let readiness = analyze_service_readiness(&plan, &hidden, language, CLOSED).unwrap();
        assert_eq!(readiness.selected_types_total, 1, "{language:?}");
        assert_eq!(readiness.selected_types_renderable, 1, "{language:?}");
        assert_eq!(readiness.selected_messages_total, 1, "{language:?}");
        assert_eq!(readiness.selected_messages_renderable, 1, "{language:?}");
        assert!(readiness.unsupported_types.is_empty());
        assert!(readiness.blocked_messages.is_empty());
        assert!(readiness.backend_blocker.is_none());
        if language != BackendLanguage::Ada {
            assert!(readiness.is_ready(), "{language:?}");
            continue;
        }
        assert!(!readiness.is_ready());
        let Some(ServiceApiError::ModelWrapperNameCollision(collision)) =
            &readiness.service_api_blocker
        else {
            panic!("{:?}", readiness.service_api_blocker);
        };
        assert_eq!(collision.language, BackendLanguage::Ada);
        assert_eq!(collision.generated, "Topic");
        assert_eq!(
            collision.model,
            ServiceApiModelEntity::AdaPackage("Topic.Model".into())
        );
        assert_eq!(collision.wrapper, ServiceApiNameOwner::Fixed("Topic"));
        assert_eq!(
            collision.region,
            ServiceApiRegion::Exchange {
                function: "mission-data".into(),
                exchange: "a-input".into(),
            }
        );
        assert_eq!(
            collision.conflict,
            ServiceApiModelConflict::Hides {
                referenced: "Topic".into()
            }
        );
    }

    for uri in [
        "urn:test:serviceApi",
        "urn:serviceApi:serviceName",
        "urn:service_api:service_name",
        "https://www.vdl.afrl.af.mil/programs/oam",
    ] {
        let schema = schema_in(uri);
        let plan = resolve_service_plan(&contract, &schema).expect("plan");
        for language in BackendLanguage::ALL {
            let readiness = analyze_service_readiness(&plan, &schema, language, CLOSED).unwrap();
            assert!(readiness.is_ready(), "{language:?} {uri}");
        }
    }
}

// ---------------------------------------------------------------------
// Task 048: operation classification and subscription group
// ---------------------------------------------------------------------

fn oms_grouped(id: &str, direction: &str, mandate: &str, topic: &str, group: &str) -> String {
    format!(
        "      - id: {id}\n        kind: oms_message\n        direction: {direction}\n        mandate: {mandate}\n        message: ReportA\n        topic: {topic}\n        subscription_group: '{group}'\n        timing:\n          kind: periodic\n          nominal_rate_hz: 2\n"
    )
}

/// Direction alone decides the operation: both mandates of each direction
/// get the same operation, a periodic output is still an ordinary Publish,
/// IDs that SAY `input`/`output` are ignored, and the authored group is
/// carried verbatim (absent stays absent). Same payload type throughout.
#[test]
fn operation_is_decided_by_direction_only() {
    let exchanges = [
        // IDs deliberately contradict their directions.
        oms("x-output", "input", "mandatory", "ReportA", "t.in.m"),
        oms_grouped("y-output", "input", "optional", "t.in.o", "Pool 7: \"hot\""),
        oms_grouped(
            "x-input",
            "output",
            "mandatory",
            "t.out.m",
            "ignored-for-publish",
        ),
        oms("y-input", "output", "optional", "ReportA", "t.out.o"),
    ]
    .concat();
    let model = lower(&function("f", "F", &exchanges)).expect("lower");
    use ams_gra_oms_codegen_core::ServiceApiOmsOperation::{Publish, Subscribe};
    let observed = model.functions()[0]
        .exchanges()
        .iter()
        .map(|exchange| {
            let binding = exchange.oms_binding().expect("OMS");
            (
                exchange.id(),
                exchange.mandate(),
                binding.operation(),
                binding.topic(),
                binding.subscription_group(),
                binding.payload_name().local_name.as_str(),
                binding.message_name().local_name.as_str(),
            )
        })
        .collect::<Vec<_>>();
    use Mandate::{Mandatory, Optional};
    assert_eq!(
        observed,
        [
            (
                "x-output", Mandatory, Subscribe, "t.in.m", None, "PayloadA", "ReportA"
            ),
            (
                "y-output",
                Optional,
                Subscribe,
                "t.in.o",
                Some("Pool 7: \"hot\""),
                "PayloadA",
                "ReportA"
            ),
            (
                "x-input",
                Mandatory,
                Publish,
                "t.out.m",
                Some("ignored-for-publish"),
                "PayloadA",
                "ReportA"
            ),
            (
                "y-input", Optional, Publish, "t.out.o", None, "PayloadA", "ReportA"
            ),
        ]
    );
    // The resolved message identity stays structured, never flattened.
    let binding = model.functions()[0].exchanges()[0].oms_binding().unwrap();
    assert_eq!(binding.message_name(), &qualified("ReportA"));
    assert_eq!(binding.message_name().namespace_uri, NS);
}

/// The four non-OMS kinds carry no binding, hence no operation and no
/// subscription group, whatever their direction.
#[test]
fn non_oms_kinds_have_no_operation() {
    let exchanges = [
        non_uci("dt", "data_transfer", "input", "mandatory"),
        non_uci("sig", "special_signal", "output", "mandatory"),
        non_uci("sec", "security_exchange", "input", "optional"),
        non_uci("legacy", "non_oms_message", "output", "optional"),
    ]
    .concat();
    let model = lower(&function("f", "F", &exchanges)).expect("lower");
    assert!(!model.has_oms_exchanges());
    for exchange in model.functions()[0].exchanges() {
        assert!(exchange.oms_binding().is_none(), "{}", exchange.id());
    }
}
