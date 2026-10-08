use ams_gra_oms_codegen_core::{
    OAM_NAMESPACE, ResolvedExchange, ServicePlan, resolve_service_plan,
};
use ams_gra_oms_ir::{MessageDecl, PrimitiveKind, QualifiedName, SchemaIr, SourceRef, TypeRef};
use ams_gra_oms_service_contract::{Mandate, parse_yaml};
use ams_gra_oms_service_routes::*;

const UUID: &str = "550e8400-e29b-41d4-a716-446655440071";

fn schema(namespace: &str) -> SchemaIr {
    SchemaIr {
        schema_version: None,
        namespaces: vec![],
        types: vec![],
        messages: ["MessageA", "MessageB"]
            .map(|name| MessageDecl {
                name: QualifiedName::new(namespace, name),
                payload_type: TypeRef::primitive(PrimitiveKind::String),
                documentation: None,
                source: SourceRef {
                    document: "synthetic".into(),
                    line: None,
                },
            })
            .to_vec(),
    }
}

fn plan(namespace: &str) -> ServicePlan {
    let contract = parse_yaml("contract_version: '0.1'\nservice:\n  name: Human readable name\n  version: '1'\n  kind: service\nstandards:\n  oms_version: '2.5'\n  uci_schema_version: '2.5'\nfunctions:\n  - id: first\n    name: First\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: receive\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: MessageA\n        topic: z-topic\n        timing:\n          kind: asynchronous\n      - id: send\n        kind: oms_message\n        direction: output\n        mandate: optional\n        message: MessageB\n        topic: z-topic\n        timing:\n          kind: asynchronous\n").unwrap();
    resolve_service_plan(&contract, &schema(namespace)).unwrap()
}

#[test]
fn occurrences_and_first_occurrence_aggregation() {
    let mut plan = plan(OAM_NAMESPACE);
    let mut second = plan.functions[0].clone();
    second.id = "second".into();
    second.exchanges = vec![second.exchanges[0].clone(), second.exchanges[0].clone()];
    if let ResolvedExchange::OmsMessage(e) = &mut second.exchanges[1] {
        e.topic = "a-topic".into();
        e.direction = ams_gra_oms_service_contract::Direction::Output;
        e.subscription_group = Some("group".into());
    }
    plan.functions.push(second);
    assert_eq!(plan.selected_messages().len(), 2);
    let manifest = build_route_manifest(&plan);
    assert_eq!(manifest.occurrences().len(), 4);
    let topics = manifest.topics();
    assert_eq!(
        topics.iter().map(|t| t.topic.as_str()).collect::<Vec<_>>(),
        ["z-topic", "a-topic"]
    );
    assert_eq!(
        topics.iter().map(|t| t.messages.len()).collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(manifest.occurrences()[0].operation.as_str(), "subscribe");
    assert_eq!(manifest.occurrences()[1].operation.as_str(), "publish");
    assert_eq!(manifest.occurrences()[0].mandate, Mandate::Mandatory);
    assert_eq!(manifest.occurrences()[1].mandate, Mandate::Optional);
    assert_eq!(
        manifest.occurrences()[3].subscription_group.as_deref(),
        Some("group")
    );
    let expected = format!(
        "service_id = \"explicit-id\"\nservice_uuid = \"{UUID}\"\nallowed_topics = [\"z-topic\", \"a-topic\"]\n\n[[topic_bindings]]\ntopic = \"z-topic\"\nallowed_messages = [\"MessageA\", \"MessageB\"]\n\n[[topic_bindings]]\ntopic = \"a-topic\"\nallowed_messages = [\"MessageA\"]\n"
    );
    assert_eq!(
        render_sleet_toml(&manifest, "explicit-id", UUID).unwrap(),
        expected
    );
    assert_eq!(
        render_sleet_toml(&manifest, "explicit-id", UUID).unwrap(),
        expected
    );
    assert_eq!(render_route_tsv(&manifest), render_route_tsv(&manifest));
}

#[test]
fn exact_tsv_escaping_and_group_presence() {
    let mut plan = plan("urn:other");
    if let ResolvedExchange::OmsMessage(e) = &mut plan.functions[0].exchanges[0] {
        e.topic = "a\tb\nc\rd\\e\0\u{7f}".into();
        e.subscription_group = Some(String::new());
    }
    let manifest = build_route_manifest(&plan);
    assert_eq!(
        render_route_tsv(&manifest),
        format!(
            "{TSV_HEADER}first\treceive\tinput\tsubscribe\tmandatory\ta\\tb\\nc\\rd\\\\e\\u{{0}}\\u{{7f}}\turn:other\tMessageA\ttrue\t\nfirst\tsend\toutput\tpublish\toptional\tz-topic\turn:other\tMessageB\tfalse\t\n"
        )
    );
    assert_eq!(render_route_tsv(&manifest).lines().count(), 3);
}

#[test]
fn empty_routes_are_valid() {
    let mut plan = plan(OAM_NAMESPACE);
    plan.functions.clear();
    let manifest = build_route_manifest(&plan);
    assert!(manifest.occurrences().is_empty());
    assert!(manifest.topics().is_empty());
    assert_eq!(render_route_tsv(&manifest), TSV_HEADER);
    assert_eq!(
        render_sleet_toml(&manifest, "empty", UUID).unwrap(),
        format!("service_id = \"empty\"\nservice_uuid = \"{UUID}\"\nallowed_topics = []\n")
    );
}

#[test]
fn sleet_validation_fails_closed() {
    let manifest = build_route_manifest(&plan(OAM_NAMESPACE));
    for invalid in ["", "with space", "a\"b", "a\\b", "é", "a:b", "a/b", "a\n"] {
        assert!(matches!(
            render_sleet_toml(&manifest, invalid, UUID),
            Err(RouteError::InvalidServiceId(_))
        ));
        let mut plan = plan(OAM_NAMESPACE);
        if let ResolvedExchange::OmsMessage(e) = &mut plan.functions[0].exchanges[1] {
            e.topic = invalid.into();
        }
        let error = render_sleet_toml(&build_route_manifest(&plan), "svc", UUID).unwrap_err();
        assert!(matches!(error, RouteError::InvalidTopic { .. }));
        assert!(error.to_string().contains("send"));
        if let ResolvedExchange::OmsMessage(e) = &mut plan.functions[0].exchanges[1] {
            e.topic = "z-topic".into();
            e.message_name.local_name = invalid.into();
        }
        assert!(matches!(
            render_sleet_toml(&build_route_manifest(&plan), "svc", UUID),
            Err(RouteError::InvalidMessage { .. })
        ));
    }
    for invalid in ["", "550e8400-e29b-41d4-a716-44665544007z", "not-a-uuid"] {
        assert!(matches!(
            render_sleet_toml(&manifest, "svc", invalid),
            Err(RouteError::InvalidServiceUuid(_))
        ));
    }
    assert!(matches!(
        render_sleet_toml(&build_route_manifest(&plan("urn:other")), "svc", UUID),
        Err(RouteError::UnsupportedNamespace { .. })
    ));
    assert!(is_sleet_identifier("9_A-z.foo"));
    assert_eq!(
        render_sleet_toml(
            &manifest,
            "svc",
            "urn:uuid:550e8400-e29b-41d4-a716-446655440071"
        )
        .unwrap(),
        render_sleet_toml(&manifest, "svc", UUID).unwrap()
    );
}

#[test]
fn exact_pair_duplicates_and_mixed_direction() {
    let mut plan = plan(OAM_NAMESPACE);
    let mut exchange = plan.functions[0].exchanges[0].clone();
    if let ResolvedExchange::OmsMessage(e) = &mut exchange {
        e.direction = ams_gra_oms_service_contract::Direction::Output;
    }
    plan.functions[0].exchanges = vec![plan.functions[0].exchanges[0].clone(), exchange];
    let manifest = build_route_manifest(&plan);
    assert_eq!(manifest.occurrences().len(), 2);
    assert_eq!(manifest.topics().len(), 1);
    assert_eq!(manifest.topics()[0].messages.len(), 1);
    assert_eq!(
        render_sleet_toml(&manifest, "svc", UUID).unwrap(),
        format!(
            "service_id = \"svc\"\nservice_uuid = \"{UUID}\"\nallowed_topics = [\"z-topic\"]\n\n[[topic_bindings]]\ntopic = \"z-topic\"\nallowed_messages = [\"MessageA\"]\n"
        )
    );
}

#[test]
fn aggregation_preserves_namespace_and_topic_equality() {
    let mut plan = plan(OAM_NAMESPACE);
    let mut exchange = plan.functions[0].exchanges[0].clone();
    if let ResolvedExchange::OmsMessage(e) = &mut exchange {
        e.message_name.namespace_uri = "urn:private".into();
    }
    plan.functions[0].exchanges.push(exchange.clone());
    if let ResolvedExchange::OmsMessage(e) = &mut exchange {
        e.topic = "Z-topic".into();
    }
    plan.functions[0].exchanges.push(exchange);
    let manifest = build_route_manifest(&plan);
    assert_eq!(manifest.topics().len(), 2);
    assert_eq!(manifest.topics()[0].messages.len(), 3);
    assert!(render_route_tsv(&manifest).contains("urn:private\tMessageA"));
    assert!(matches!(
        render_sleet_toml(&manifest, "svc", UUID),
        Err(RouteError::UnsupportedNamespace { .. })
    ));
}

#[test]
fn all_non_oms_kinds_are_counted_not_invented() {
    use ams_gra_oms_service_contract::*;
    let mut plan = plan(OAM_NAMESPACE);
    let direction = Direction::Input;
    let mandate = Mandate::Mandatory;
    let timing = Timing::Asynchronous {};
    plan.functions[0].exchanges = vec![
        ResolvedExchange::DataTransfer(DataTransferExchange {
            id: "data".into(),
            direction,
            mandate,
            name: "Data".into(),
            protocol: "p".into(),
            data_type: "t".into(),
            data_format: "f".into(),
            sharing_pattern: "s".into(),
            timing: timing.clone(),
            traceability: vec![],
        }),
        ResolvedExchange::SpecialSignal(SpecialSignalExchange {
            id: "signal".into(),
            direction,
            mandate,
            name: "Signal".into(),
            details: None,
            reference: None,
            timing: timing.clone(),
            traceability: vec![],
        }),
        ResolvedExchange::SecurityExchange(SecurityExchange {
            id: "security".into(),
            direction,
            mandate,
            name: "Security".into(),
            details: None,
            reference: None,
            timing: timing.clone(),
            traceability: vec![],
        }),
        ResolvedExchange::NonOmsMessage(NonOmsMessageExchange {
            id: "non-oms".into(),
            direction,
            mandate,
            name: "Other".into(),
            details: None,
            reference: None,
            timing,
            traceability: vec![],
        }),
    ];
    let manifest = build_route_manifest(&plan);
    assert_eq!(manifest.excluded_non_oms_occurrences(), 4);
    assert!(manifest.occurrences().is_empty());
    assert!(manifest.topics().is_empty());
    assert_eq!(render_route_tsv(&manifest), TSV_HEADER);
}

#[test]
fn unknown_and_ambiguous_names_remain_resolver_errors() {
    let contract = parse_yaml("contract_version: '0.1'\nservice:\n  name: Test\n  version: '1'\n  kind: service\nstandards:\n  oms_version: '2.5'\n  uci_schema_version: '2.5'\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: MessageA\n        topic: tracks\n        timing:\n          kind: asynchronous\n").unwrap();
    let mut schema = schema(OAM_NAMESPACE);
    schema.messages.clear();
    assert!(matches!(
        resolve_service_plan(&contract, &schema),
        Err(ams_gra_oms_codegen_core::ServicePlanError::UnknownOmsMessage { .. })
    ));
    schema = self::schema(OAM_NAMESPACE);
    let mut duplicate = schema.messages[0].clone();
    duplicate.name.namespace_uri = "urn:other".into();
    schema.messages.push(duplicate);
    assert!(matches!(
        resolve_service_plan(&contract, &schema),
        Err(ams_gra_oms_codegen_core::ServicePlanError::AmbiguousOmsMessage { .. })
    ));
}
