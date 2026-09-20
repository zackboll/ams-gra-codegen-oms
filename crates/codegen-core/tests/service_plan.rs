//! Resolved Service Plan tests: the Contract IR + Schema IR join.

use ams_gra_oms_codegen_core::{ResolvedExchange, ServicePlanError, resolve_service_plan};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, MessageDecl, NamespaceDecl, PrimitiveKind,
    QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind, TypeRef,
};
use ams_gra_oms_service_contract::{Contract, Direction, Mandate, Timing, parse_yaml};

const NS: &str = "urn:test";
const OTHER_NS: &str = "urn:other";

fn source() -> SourceRef {
    SourceRef {
        document: "fixture.xsd".to_owned(),
        line: None,
    }
}

fn named(namespace: &str, local: &str) -> TypeRef {
    TypeRef::named(QualifiedName::new(namespace, local))
}

fn primitive(kind: PrimitiveKind) -> TypeRef {
    TypeRef::primitive(kind)
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

fn message(namespace: &str, local: &str, payload: TypeRef) -> MessageDecl {
    MessageDecl {
        name: QualifiedName::new(namespace, local),
        payload_type: payload,
        documentation: None,
        source: source(),
    }
}

/// The controlled schema set mirroring tests/fixtures/service-plan/root.xsd.
fn schema() -> SchemaIr {
    SchemaIr {
        schema_version: Some("000.1.0".to_owned()),
        namespaces: vec![NamespaceDecl {
            uri: NS.to_owned(),
            preferred_prefix: Some("t".to_owned()),
        }],
        types: vec![
            record(
                "PositionType",
                vec![
                    field("Latitude", primitive(PrimitiveKind::Float64)),
                    field("Longitude", primitive(PrimitiveKind::Float64)),
                ],
            ),
            TypeDecl {
                name: QualifiedName::new(NS, "TimestampType"),
                is_abstract: false,
                base_type: None,
                kind: TypeKind::Primitive(PrimitiveKind::String),
                constraints: ConstraintSet::default(),
                documentation: None,
                source: source(),
            },
            record(
                "PositionReportType",
                vec![
                    field("Position", named(NS, "PositionType")),
                    field("Timestamp", named(NS, "TimestampType")),
                ],
            ),
            record(
                "MeasurementType",
                vec![field("Value", primitive(PrimitiveKind::Float64))],
            ),
            record(
                "ObservationMeasurementReportType",
                vec![field("Measurement", named(NS, "MeasurementType"))],
            ),
            record(
                "UnusedType",
                vec![field("Filler", primitive(PrimitiveKind::String))],
            ),
        ],
        messages: vec![
            message(NS, "PositionReport", named(NS, "PositionReportType")),
            message(
                NS,
                "ObservationMeasurementReport",
                named(NS, "ObservationMeasurementReportType"),
            ),
            message(NS, "UnusedReport", named(NS, "UnusedType")),
        ],
    }
}

fn contract(functions: &str) -> Contract {
    parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: Contract Codegen Test\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\n{functions}"
    ))
    .expect("test contract should be portable-valid")
}

fn oms_exchange(id: &str, direction: &str, message: &str, topic: &str) -> String {
    format!(
        "      - id: {id}\n        kind: oms_message\n        direction: {direction}\n        mandate: mandatory\n        message: {message}\n        topic: {topic}\n        timing:\n          kind: asynchronous\n"
    )
}

#[test]
fn resolves_position_report() {
    let contract = contract(&format!(
        "functions:\n  - id: mission-data\n    name: Mission Data\n    category: specific\n    applicability: applicable\n    exchanges:\n{}",
        oms_exchange(
            "position-input",
            "input",
            "PositionReport",
            "mission.position-report"
        )
    ));
    let plan = resolve_service_plan(&contract, &schema()).expect("resolution should succeed");
    let ResolvedExchange::OmsMessage(oms) = &plan.functions[0].exchanges[0] else {
        panic!("expected a resolved OMS message exchange");
    };
    assert_eq!(oms.contract_message, "PositionReport");
    assert_eq!(oms.message_name, QualifiedName::new(NS, "PositionReport"));
    assert_eq!(oms.payload_type, named(NS, "PositionReportType"));
    // Every contract fact survives resolution.
    assert_eq!(oms.direction, Direction::Input);
    assert_eq!(oms.mandate, Mandate::Mandatory);
    assert_eq!(oms.topic, "mission.position-report");
    assert_eq!(oms.timing, Timing::Asynchronous {});
}

#[test]
fn resolves_observation_measurement_report() {
    let contract = contract(&format!(
        "functions:\n  - id: mission-data\n    name: Mission Data\n    category: specific\n    applicability: applicable\n    exchanges:\n{}",
        oms_exchange(
            "observation-output",
            "output",
            "ObservationMeasurementReport",
            "mission.observation-measurement-report"
        )
    ));
    let plan = resolve_service_plan(&contract, &schema()).expect("resolution should succeed");
    let ResolvedExchange::OmsMessage(oms) = &plan.functions[0].exchanges[0] else {
        panic!("expected a resolved OMS message exchange");
    };
    assert_eq!(
        oms.message_name,
        QualifiedName::new(NS, "ObservationMeasurementReport")
    );
    assert_eq!(oms.direction, Direction::Output);
}

#[test]
fn unknown_oms_message_fails_with_function_and_exchange_context() {
    let contract = contract(&format!(
        "functions:\n  - id: mission-data\n    name: Mission Data\n    category: specific\n    applicability: applicable\n    exchanges:\n{}",
        oms_exchange("position-input", "input", "AbsentReport", "t")
    ));
    match resolve_service_plan(&contract, &schema()) {
        Err(ServicePlanError::UnknownOmsMessage {
            function,
            exchange,
            message,
        }) => {
            assert_eq!(function, "mission-data");
            assert_eq!(exchange, "position-input");
            assert_eq!(message, "AbsentReport");
        }
        other => panic!("expected unknown message, got {other:?}"),
    }
}

#[test]
fn resolution_is_exact_and_never_fuzzy() {
    // Case variants, prefixes, suffixes, and substrings must all fail: a
    // near-miss is a contract/schema mismatch, not a hint.
    for spelling in [
        "positionreport",
        "POSITIONREPORT",
        "Position",
        "Report",
        "PositionReportType",
        "MyPositionReport",
    ] {
        let contract = contract(&format!(
            "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n{}",
            oms_exchange("e1", "input", spelling, "t")
        ));
        assert!(
            matches!(
                resolve_service_plan(&contract, &schema()),
                Err(ServicePlanError::UnknownOmsMessage { .. })
            ),
            "{spelling} must not resolve"
        );
    }
}

#[test]
fn ambiguous_same_local_name_fails_and_reports_candidates() {
    let mut schema = schema();
    schema.namespaces.push(NamespaceDecl {
        uri: OTHER_NS.to_owned(),
        preferred_prefix: Some("o".to_owned()),
    });
    // A second message with the SAME local name in a different namespace.
    // There is deliberately no first-match-wins fallback.
    schema.messages.push(message(
        OTHER_NS,
        "PositionReport",
        primitive(PrimitiveKind::String),
    ));
    let contract = contract(&format!(
        "functions:\n  - id: mission-data\n    name: Mission Data\n    category: specific\n    applicability: applicable\n    exchanges:\n{}",
        oms_exchange("position-input", "input", "PositionReport", "t")
    ));
    match resolve_service_plan(&contract, &schema) {
        Err(ServicePlanError::AmbiguousOmsMessage {
            function,
            exchange,
            message,
            candidates,
        }) => {
            assert_eq!(function, "mission-data");
            assert_eq!(exchange, "position-input");
            assert_eq!(message, "PositionReport");
            assert_eq!(
                candidates,
                vec![
                    QualifiedName::new(NS, "PositionReport"),
                    QualifiedName::new(OTHER_NS, "PositionReport"),
                ]
            );
        }
        other => panic!("expected ambiguity, got {other:?}"),
    }
}

/// Two functions whose exchanges deliberately share one message, plus a third
/// message introduced later, to pin dedup and first-occurrence ordering.
fn shared_message_contract() -> Contract {
    contract(&format!(
        "functions:\n  - id: function-a\n    name: Function A\n    category: specific\n    applicability: applicable\n    exchanges:\n{}  - id: function-b\n    name: Function B\n    category: specific\n    applicability: applicable\n    exchanges:\n{}{}",
        oms_exchange("exchange-x", "input", "PositionReport", "topic.a"),
        oms_exchange("exchange-y", "output", "PositionReport", "topic.b"),
        oms_exchange(
            "exchange-z",
            "output",
            "ObservationMeasurementReport",
            "topic.c"
        ),
    ))
}

#[test]
fn two_exchange_occurrences_may_share_one_message() {
    let plan = resolve_service_plan(&shared_message_contract(), &schema()).expect("should resolve");
    // Both occurrences survive, with their own distinct topics and directions.
    assert_eq!(plan.exchange_occurrence_count(), 3);
    assert_eq!(plan.oms_message_exchange_count(), 3);
    let ResolvedExchange::OmsMessage(first) = &plan.functions[0].exchanges[0] else {
        panic!("expected an OMS message");
    };
    let ResolvedExchange::OmsMessage(second) = &plan.functions[1].exchanges[0] else {
        panic!("expected an OMS message");
    };
    assert_eq!(first.message_name, second.message_name);
    assert_eq!(first.topic, "topic.a");
    assert_eq!(second.topic, "topic.b");
    assert_eq!(first.direction, Direction::Input);
    assert_eq!(second.direction, Direction::Output);
}

#[test]
fn selected_messages_are_deduplicated() {
    let plan = resolve_service_plan(&shared_message_contract(), &schema()).expect("should resolve");
    assert_eq!(plan.selected_messages().len(), 2);
}

#[test]
fn selected_messages_use_first_occurrence_order() {
    // Reverse the contract so the alphabetically-later message comes first.
    // A sorted-set ordering would silently reorder; first-occurrence does not.
    let reversed = contract(&format!(
        "functions:\n  - id: function-a\n    name: Function A\n    category: specific\n    applicability: applicable\n    exchanges:\n{}  - id: function-b\n    name: Function B\n    category: specific\n    applicability: applicable\n    exchanges:\n{}",
        oms_exchange(
            "exchange-x",
            "input",
            "ObservationMeasurementReport",
            "topic.a"
        ),
        oms_exchange("exchange-y", "output", "PositionReport", "topic.b"),
    ));
    let plan = resolve_service_plan(&reversed, &schema()).expect("should resolve");
    let order = plan
        .selected_messages()
        .iter()
        .map(|selected| selected.name.local_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        order,
        vec!["ObservationMeasurementReport", "PositionReport"]
    );

    let forward =
        resolve_service_plan(&shared_message_contract(), &schema()).expect("should resolve");
    let forward_order = forward
        .selected_messages()
        .iter()
        .map(|selected| selected.name.local_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        forward_order,
        vec!["PositionReport", "ObservationMeasurementReport"]
    );
}

#[test]
fn function_and_exchange_order_are_preserved() {
    // 'zulu' before 'alpha' in the contract must stay that way in the plan.
    let contract = contract(&format!(
        "functions:\n  - id: zulu-function\n    name: Zulu\n    category: specific\n    applicability: applicable\n    exchanges:\n{}{}  - id: alpha-function\n    name: Alpha\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
        oms_exchange("zulu-exchange", "input", "PositionReport", "t1"),
        oms_exchange(
            "alpha-exchange",
            "output",
            "ObservationMeasurementReport",
            "t2"
        ),
    ));
    let plan = resolve_service_plan(&contract, &schema()).expect("should resolve");
    let functions = plan
        .functions
        .iter()
        .map(|function| function.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(functions, vec!["zulu-function", "alpha-function"]);
    let exchanges = plan.functions[0]
        .exchanges
        .iter()
        .map(ResolvedExchange::id)
        .collect::<Vec<_>>();
    assert_eq!(exchanges, vec!["zulu-exchange", "alpha-exchange"]);
}

#[test]
fn non_uci_exchanges_are_preserved_and_never_resolved() {
    // None of these names exist in the schema set; that must not be an error.
    let contract = contract(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e-transfer\n        kind: data_transfer\n        direction: output\n        mandate: optional\n        name: ImageryProduct\n        protocol: sftp\n        data_type: imagery\n        data_format: nitf\n        sharing_pattern: push\n        timing:\n          kind: asynchronous\n      - id: e-signal\n        kind: special_signal\n        direction: input\n        mandate: mandatory\n        name: DiscreteLine\n        timing:\n          kind: asynchronous\n      - id: e-security\n        kind: security_exchange\n        direction: output\n        mandate: mandatory\n        name: KeyLoad\n        timing:\n          kind: asynchronous\n      - id: e-legacy\n        kind: non_oms_message\n        direction: input\n        mandate: optional\n        name: ExampleLegacyMessage\n        timing:\n          kind: asynchronous\n",
    );
    let plan = resolve_service_plan(&contract, &schema()).expect("non-UCI exchanges must not fail");
    let exchanges = &plan.functions[0].exchanges;
    assert_eq!(exchanges.len(), 4);
    assert!(
        exchanges
            .iter()
            .all(|exchange| exchange.as_oms_message().is_none())
    );
    assert_eq!(plan.oms_message_exchange_count(), 0);
    assert!(plan.selected_messages().is_empty());

    let ResolvedExchange::DataTransfer(transfer) = &exchanges[0] else {
        panic!("expected a data transfer");
    };
    assert_eq!(transfer.name, "ImageryProduct");
    assert_eq!(transfer.protocol, "sftp");
    assert_eq!(transfer.data_format, "nitf");
    assert_eq!(transfer.sharing_pattern, "push");

    let ResolvedExchange::NonOmsMessage(legacy) = &exchanges[3] else {
        panic!("expected a non-OMS message");
    };
    assert_eq!(legacy.name, "ExampleLegacyMessage");
    assert_eq!(legacy.mandate, Mandate::Optional);
}

#[test]
fn selected_type_closure_is_deterministic_and_selective() {
    let contract = contract(&format!(
        "functions:\n  - id: mission-data\n    name: Mission Data\n    category: specific\n    applicability: applicable\n    exchanges:\n{}{}",
        oms_exchange("position-input", "input", "PositionReport", "t1"),
        oms_exchange(
            "observation-output",
            "output",
            "ObservationMeasurementReport",
            "t2"
        ),
    ));
    let schema = schema();
    let plan = resolve_service_plan(&contract, &schema).expect("should resolve");
    let closure = plan
        .selected_type_closure(&schema)
        .expect("closure should resolve");
    let names = closure
        .iter()
        .map(|declaration| declaration.name.local_name.as_str())
        .collect::<Vec<_>>();
    // Schema declaration order, transitively closed, and NOT every declared
    // type: UnusedType is reachable from no selected message.
    assert_eq!(
        names,
        vec![
            "PositionType",
            "TimestampType",
            "PositionReportType",
            "MeasurementType",
            "ObservationMeasurementReportType",
        ]
    );
    assert!(!names.contains(&"UnusedType"));

    // Determinism: the same inputs give byte-identical output every time.
    let again = plan
        .selected_type_closure(&schema)
        .expect("closure should resolve");
    assert_eq!(closure, again);
}

#[test]
fn selected_type_closure_narrows_with_the_contract() {
    let contract = contract(&format!(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n{}",
        oms_exchange("e1", "input", "PositionReport", "t")
    ));
    let schema = schema();
    let plan = resolve_service_plan(&contract, &schema).expect("should resolve");
    let names = plan
        .selected_type_closure(&schema)
        .expect("closure should resolve")
        .iter()
        .map(|declaration| declaration.name.local_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["PositionType", "TimestampType", "PositionReportType"]
    );
}

#[test]
fn selected_type_closure_follows_base_types_and_choices_and_lists() {
    // The closure must use the same dependency categories as the rest of
    // codegen planning, not a second private model.
    let mut schema = schema();
    schema.types.push(TypeDecl {
        name: QualifiedName::new(NS, "BaseType"),
        is_abstract: true,
        base_type: None,
        kind: TypeKind::Record { fields: Vec::new() },
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    });
    schema.types.push(TypeDecl {
        name: QualifiedName::new(NS, "ItemType"),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Primitive(PrimitiveKind::String),
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    });
    schema.types.push(TypeDecl {
        name: QualifiedName::new(NS, "ListType"),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::List {
            item_type: named(NS, "ItemType"),
            cardinality: Cardinality::REQUIRED_ONE,
        },
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    });
    schema.types.push(TypeDecl {
        name: QualifiedName::new(NS, "AliasType"),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Alias(named(NS, "ListType")),
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    });
    schema.types.push(TypeDecl {
        name: QualifiedName::new(NS, "ChoiceType"),
        is_abstract: false,
        base_type: None,
        kind: TypeKind::Choice {
            alternatives: vec![field("Alias", named(NS, "AliasType"))],
        },
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    });
    schema.types.push(TypeDecl {
        name: QualifiedName::new(NS, "DerivedReportType"),
        is_abstract: false,
        base_type: Some(named(NS, "BaseType")),
        kind: TypeKind::Record {
            fields: vec![field("Choice", named(NS, "ChoiceType"))],
        },
        constraints: ConstraintSet::default(),
        documentation: None,
        source: source(),
    });
    schema
        .messages
        .push(message(NS, "DerivedReport", named(NS, "DerivedReportType")));

    let contract = contract(&format!(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n{}",
        oms_exchange("e1", "input", "DerivedReport", "t")
    ));
    let plan = resolve_service_plan(&contract, &schema).expect("should resolve");
    let names = plan
        .selected_type_closure(&schema)
        .expect("closure should resolve")
        .iter()
        .map(|declaration| declaration.name.local_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "BaseType",
            "ItemType",
            "ListType",
            "AliasType",
            "ChoiceType",
            "DerivedReportType",
        ]
    );
}

#[test]
fn capability_states_and_standard_roles_survive_into_the_plan() {
    let contract = contract(
        "capabilities:\n  - id: esm\n    name: ESM\n    requires_position_information: true\n  - id: radar\n    name: Radar\n    requires_position_information: false\nfunctions:\n  - id: position-information\n    name: Position Information Processing\n    standard_role: position_information_processing\n    category: required\n    required_group: capability\n    applicability: applicable\n    exchanges: []\n  - id: esm-status\n    name: ESM Status\n    standard_role: capability_status\n    capability: esm\n    category: required\n    required_group: capability\n    applicability: applicable\n    exchanges: []\n",
    );
    let plan = resolve_service_plan(&contract, &schema()).expect("should resolve");
    let capabilities = plan.capabilities.as_ref().expect("declared capabilities");
    assert_eq!(capabilities.len(), 2);
    assert_eq!(capabilities[0].id, "esm");
    assert!(capabilities[0].requires_position_information);
    assert_eq!(capabilities[1].id, "radar");
    assert!(!capabilities[1].requires_position_information);

    // No inference: the component-level function keeps NO capability owner,
    // and no Section 3.3 function is synthesized for 'radar'.
    assert_eq!(plan.functions[0].capability, None);
    assert_eq!(plan.functions[1].capability.as_deref(), Some("esm"));
    assert_eq!(plan.functions.len(), 2);
}

#[test]
fn omitted_and_empty_capability_states_survive_into_the_plan() {
    let omitted = resolve_service_plan(
        &contract(
            "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
        ),
        &schema(),
    )
    .expect("should resolve");
    assert_eq!(omitted.capabilities, None);

    let empty = resolve_service_plan(
        &contract(
            "capabilities: []\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
        ),
        &schema(),
    )
    .expect("should resolve");
    assert_eq!(empty.capabilities.as_deref(), Some(&[][..]));
    assert_ne!(omitted.capabilities, empty.capabilities);
}

#[test]
fn both_uci_version_strings_are_retained_without_comparison() {
    // The contract says "2.5"; the schema root says "000.1.0" here (real UCI
    // 2.5 says "002.5.0"). Mismatched spellings must NOT fail the plan.
    let plan = resolve_service_plan(
        &contract(
            "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
        ),
        &schema(),
    )
    .expect("differing version spellings must not fail resolution");
    assert_eq!(plan.standards.uci_schema_version, "2.5");
    assert_eq!(
        plan.standards.schema_root_version.as_deref(),
        Some("000.1.0")
    );
}
