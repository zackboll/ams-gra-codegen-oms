//! Portable v0.1 Service Contract parsing and validation tests.

use ams_gra_oms_service_contract::{
    Applicability, ContractError, Direction, Exchange, FunctionCategory, Mandate, RequiredGroup,
    SemanticError, ServiceKind, StandardRole, Timing, load_contract, parse_json, parse_yaml,
};
use std::path::Path;

/// A minimal portable-valid contract with one applicable specific function.
fn minimal_yaml(body: &str) -> String {
    format!(
        "contract_version: \"0.1\"\nservice:\n  name: Test\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\n{body}"
    )
}

fn one_function(extra: &str) -> String {
    minimal_yaml(&format!(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n{extra}"
    ))
}

#[test]
fn parses_yaml_contract() {
    let contract = parse_yaml(&one_function("")).expect("minimal contract should parse");
    assert_eq!(contract.contract_version, "0.1");
    assert_eq!(contract.service.name, "Test");
    assert_eq!(contract.service.kind, ServiceKind::Service);
    assert_eq!(contract.standards.uci_schema_version, "2.5");
    assert!(contract.standards.uci_extension_schemas.is_empty());
    assert_eq!(contract.functions.len(), 1);
    assert_eq!(contract.functions[0].category, FunctionCategory::Specific);
}

#[test]
fn parses_equivalent_json_contract() {
    let json = r#"{
      "contract_version": "0.1",
      "service": {"name": "Test", "version": "0.1", "kind": "subsystem"},
      "standards": {"oms_version": "2.5", "uci_schema_version": "2.5"},
      "functions": [
        {"id": "f1", "name": "F1", "category": "specific",
         "applicability": "applicable", "exchanges": []}
      ]
    }"#;
    let contract = parse_json(json).expect("json contract should parse");
    assert_eq!(contract.service.kind, ServiceKind::Subsystem);
    assert_eq!(contract.functions[0].id, "f1");
}

#[test]
fn yaml_and_json_produce_identical_contract_ir() {
    // The Contract IR is a property of the contract, not of its syntax.
    let yaml = parse_yaml(&one_function("")).expect("yaml should parse");
    let json = parse_json(
        r#"{"contract_version":"0.1",
            "service":{"name":"Test","version":"0.1","kind":"service"},
            "standards":{"oms_version":"2.5","uci_schema_version":"2.5"},
            "functions":[{"id":"f1","name":"F1","category":"specific",
              "applicability":"applicable","exchanges":[]}]}"#,
    )
    .expect("json should parse");
    assert_eq!(yaml, json);
}

#[test]
fn rejects_unknown_top_level_property() {
    let text = one_function("") + "future_field: 1\n";
    assert!(matches!(parse_yaml(&text), Err(ContractError::Parse(_))));
}

#[test]
fn rejects_unknown_nested_property() {
    // A future portable field must fail loudly rather than be dropped.
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    future_field: x\n    exchanges: []\n",
    );
    assert!(matches!(parse_yaml(&text), Err(ContractError::Parse(_))));
}

#[test]
fn rejects_unknown_exchange_property() {
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: M\n        topic: t\n        future_field: x\n        timing:\n          kind: asynchronous\n",
    );
    assert!(matches!(parse_yaml(&text), Err(ContractError::Parse(_))));
}

#[test]
fn rejects_unsupported_contract_version() {
    let text = one_function("").replace("\"0.1\"", "\"0.2\"");
    match parse_yaml(&text) {
        Err(ContractError::UnsupportedVersion { found }) => assert_eq!(found, "0.2"),
        other => panic!("expected unsupported version, got {other:?}"),
    }
}

#[test]
fn rejects_malformed_identifiers() {
    for bad in ["F1", "1f", "f-", "f--g", "f.g"] {
        let text = one_function("").replace("id: f1", &format!("id: {bad}"));
        assert!(
            matches!(
                parse_yaml(&text),
                Err(ContractError::Semantic(
                    SemanticError::InvalidIdentifier { .. }
                )) | Err(ContractError::Parse(_))
            ),
            "{bad} should be rejected"
        );
    }
}

#[test]
fn rejects_duplicate_source_id() {
    let text = minimal_yaml(
        "sources:\n  - id: s1\n    title: T\n    uri: https://example.invalid/a\n  - id: s1\n    title: U\n    uri: https://example.invalid/b\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
    );
    assert!(matches!(
        parse_yaml(&text),
        Err(ContractError::Semantic(
            SemanticError::DuplicateSourceId { .. }
        ))
    ));
}

#[test]
fn rejects_duplicate_capability_id() {
    let text = minimal_yaml(
        "capabilities:\n  - id: esm\n    name: ESM\n    requires_position_information: true\n  - id: esm\n    name: ESM Again\n    requires_position_information: false\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
    );
    assert!(matches!(
        parse_yaml(&text),
        Err(ContractError::Semantic(
            SemanticError::DuplicateCapabilityId { .. }
        ))
    ));
}

#[test]
fn rejects_duplicate_function_id() {
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n  - id: f1\n    name: F1 Again\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
    );
    assert!(matches!(
        parse_yaml(&text),
        Err(ContractError::Semantic(
            SemanticError::DuplicateFunctionId { .. }
        ))
    ));
}

#[test]
fn rejects_duplicate_exchange_id_within_one_function() {
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: M\n        topic: t1\n        timing:\n          kind: asynchronous\n      - id: e1\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: N\n        topic: t2\n        timing:\n          kind: asynchronous\n",
    );
    assert!(matches!(
        parse_yaml(&text),
        Err(ContractError::Semantic(
            SemanticError::DuplicateExchangeId { .. }
        ))
    ));
}

#[test]
fn allows_same_exchange_id_in_different_functions() {
    // Exchange ids are scoped per function, so this is legal.
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: status-output\n        kind: special_signal\n        direction: output\n        mandate: mandatory\n        name: S\n        timing:\n          kind: asynchronous\n  - id: f2\n    name: F2\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: status-output\n        kind: special_signal\n        direction: output\n        mandate: mandatory\n        name: S\n        timing:\n          kind: asynchronous\n",
    );
    parse_yaml(&text).expect("per-function exchange id scoping should be accepted");
}

#[test]
fn rejects_unknown_capability_ownership() {
    let text = minimal_yaml(
        "capabilities:\n  - id: esm\n    name: ESM\n    requires_position_information: true\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    capability: radar\n    exchanges: []\n",
    );
    match parse_yaml(&text) {
        Err(ContractError::Semantic(SemanticError::UnknownCapabilityReference {
            function,
            capability,
        })) => {
            assert_eq!(function, "f1");
            assert_eq!(capability, "radar");
        }
        other => panic!("expected unknown capability reference, got {other:?}"),
    }
}

#[test]
fn rejects_capability_ownership_when_inventory_is_omitted() {
    // An omitted inventory is not permission to invent a Capability.
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    capability: esm\n    exchanges: []\n",
    );
    assert!(matches!(
        parse_yaml(&text),
        Err(ContractError::Semantic(
            SemanticError::UnknownCapabilityReference { .. }
        ))
    ));
}

#[test]
fn rejects_unknown_traceability_source() {
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    traceability:\n      - source: absent-source\n    exchanges: []\n",
    );
    match parse_yaml(&text) {
        Err(ContractError::Semantic(SemanticError::UnknownSourceReference { source, .. })) => {
            assert_eq!(source, "absent-source");
        }
        other => panic!("expected unknown source reference, got {other:?}"),
    }
}

#[test]
fn rejects_specific_function_with_required_group() {
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    required_group: service\n    applicability: applicable\n    exchanges: []\n",
    );
    assert!(matches!(
        parse_yaml(&text),
        Err(ContractError::Semantic(
            SemanticError::SpecificFunctionWithRequiredGroup { .. }
        ))
    ));
}

#[test]
fn accepts_required_function_with_required_group() {
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: required\n    required_group: subsystem\n    applicability: applicable\n    exchanges: []\n",
    );
    let contract = parse_yaml(&text).expect("required functions may declare a group");
    assert_eq!(
        contract.functions[0].required_group,
        Some(RequiredGroup::Subsystem)
    );
}

#[test]
fn omitted_capabilities_stay_omitted() {
    // Omission must never be normalized into an explicit empty assertion.
    let contract = parse_yaml(&one_function("")).expect("contract should parse");
    assert_eq!(contract.capabilities, None);
}

#[test]
fn explicitly_empty_capabilities_stay_explicitly_empty() {
    let text = minimal_yaml(
        "capabilities: []\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
    );
    let contract = parse_yaml(&text).expect("contract should parse");
    assert_eq!(contract.capabilities.as_deref(), Some(&[][..]));
}

#[test]
fn nonempty_capabilities_are_preserved_in_order() {
    let text = minimal_yaml(
        "capabilities:\n  - id: esm\n    name: ESM\n    requires_position_information: true\n  - id: radar\n    name: Radar Processing\n    requires_position_information: false\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    capability: esm\n    exchanges: []\n",
    );
    let contract = parse_yaml(&text).expect("contract should parse");
    let capabilities = contract
        .capabilities
        .as_ref()
        .expect("capabilities were declared");
    assert_eq!(capabilities.len(), 2);
    assert_eq!(capabilities[0].id, "esm");
    assert!(capabilities[0].requires_position_information);
    assert_eq!(capabilities[1].id, "radar");
    assert!(!capabilities[1].requires_position_information);
    assert_eq!(contract.functions[0].capability.as_deref(), Some("esm"));
}

#[test]
fn the_three_capability_states_are_mutually_distinguishable() {
    let omitted = parse_yaml(&one_function("")).expect("omitted should parse");
    let empty = parse_yaml(&minimal_yaml(
        "capabilities: []\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
    ))
    .expect("empty should parse");
    let populated = parse_yaml(&minimal_yaml(
        "capabilities:\n  - id: esm\n    name: ESM\n    requires_position_information: true\nfunctions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges: []\n",
    ))
    .expect("populated should parse");
    assert_ne!(omitted.capabilities, empty.capabilities);
    assert_ne!(empty.capabilities, populated.capabilities);
    assert_ne!(omitted.capabilities, populated.capabilities);
}

#[test]
fn all_five_exchange_variants_parse() {
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e-oms\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: PositionReport\n        topic: t\n        operational_attribute: SOAC-1\n        subscription_group: g\n        appendix_c_mapping: C-1\n        timing:\n          kind: asynchronous\n      - id: e-transfer\n        kind: data_transfer\n        direction: output\n        mandate: optional\n        name: Imagery\n        protocol: sftp\n        data_type: imagery\n        data_format: nitf\n        sharing_pattern: push\n        timing:\n          kind: asynchronous\n      - id: e-signal\n        kind: special_signal\n        direction: input\n        mandate: mandatory\n        name: Discrete\n        details: A discrete line.\n        reference: Table X\n        timing:\n          kind: asynchronous\n      - id: e-security\n        kind: security_exchange\n        direction: output\n        mandate: mandatory\n        name: KeyLoad\n        timing:\n          kind: asynchronous\n      - id: e-legacy\n        kind: non_oms_message\n        direction: input\n        mandate: optional\n        name: ExampleLegacyMessage\n        timing:\n          kind: asynchronous\n",
    );
    let contract = parse_yaml(&text).expect("all five exchange kinds should parse");
    let exchanges = &contract.functions[0].exchanges;
    assert_eq!(exchanges.len(), 5);
    assert!(matches!(exchanges[0], Exchange::OmsMessage(_)));
    assert!(matches!(exchanges[1], Exchange::DataTransfer(_)));
    assert!(matches!(exchanges[2], Exchange::SpecialSignal(_)));
    assert!(matches!(exchanges[3], Exchange::SecurityExchange(_)));
    assert!(matches!(exchanges[4], Exchange::NonOmsMessage(_)));
    assert_eq!(exchanges[0].direction(), Direction::Input);
    assert_eq!(exchanges[1].mandate(), Mandate::Optional);
    let Exchange::OmsMessage(oms) = &exchanges[0] else {
        panic!("first exchange should be an OMS message");
    };
    // The message name is kept exactly as authored; no resolution happens here.
    assert_eq!(oms.message, "PositionReport");
    assert_eq!(oms.operational_attribute.as_deref(), Some("SOAC-1"));
    let Exchange::DataTransfer(transfer) = &exchanges[1] else {
        panic!("second exchange should be a data transfer");
    };
    assert_eq!(transfer.protocol, "sftp");
    assert_eq!(transfer.sharing_pattern, "push");
}

#[test]
fn rejects_exchange_missing_kind_required_field() {
    // A Data Transfer without sharing_pattern is not a valid Data Transfer,
    // and the enum shape makes that a parse failure rather than a None.
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: data_transfer\n        direction: output\n        mandate: optional\n        name: Imagery\n        protocol: sftp\n        data_type: imagery\n        data_format: nitf\n        timing:\n          kind: asynchronous\n",
    );
    assert!(matches!(parse_yaml(&text), Err(ContractError::Parse(_))));
}

#[test]
fn rejects_oms_message_field_on_a_data_transfer() {
    let text = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: data_transfer\n        direction: output\n        mandate: optional\n        name: Imagery\n        message: PositionReport\n        protocol: sftp\n        data_type: imagery\n        data_format: nitf\n        sharing_pattern: push\n        timing:\n          kind: asynchronous\n",
    );
    assert!(matches!(parse_yaml(&text), Err(ContractError::Parse(_))));
}

fn with_timing(timing: &str) -> String {
    minimal_yaml(&format!(
        "functions:\n  - id: f1\n    name: F1\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: M\n        topic: t\n        timing:\n{timing}"
    ))
}

#[test]
fn all_timing_variants_parse() {
    let asynchronous = parse_yaml(&with_timing("          kind: asynchronous\n"))
        .expect("asynchronous timing should parse");
    assert_eq!(
        asynchronous.functions[0].exchanges[0].timing(),
        &Timing::Asynchronous {}
    );

    let on_demand = parse_yaml(&with_timing(
        "          kind: on_demand\n          nominal_response_seconds: 0.5\n          max_response_seconds: 3.0\n",
    ))
    .expect("on-demand timing should parse");
    assert_eq!(
        on_demand.functions[0].exchanges[0].timing(),
        &Timing::OnDemand {
            nominal_response_seconds: Some(0.5),
            max_response_seconds: Some(3.0),
        }
    );

    let periodic = parse_yaml(&with_timing(
        "          kind: periodic\n          nominal_rate_hz: 1.0\n",
    ))
    .expect("periodic timing should parse");
    assert_eq!(
        periodic.functions[0].exchanges[0].timing(),
        &Timing::Periodic {
            nominal_rate_hz: Some(1.0),
            max_rate_hz: None,
        }
    );
}

#[test]
fn timing_values_may_be_absent_without_being_defaulted() {
    // "Unspecified" is a legal author statement and must stay unspecified.
    let contract = parse_yaml(&with_timing("          kind: on_demand\n"))
        .expect("on-demand with no values should parse");
    assert_eq!(
        contract.functions[0].exchanges[0].timing(),
        &Timing::OnDemand {
            nominal_response_seconds: None,
            max_response_seconds: None,
        }
    );
}

#[test]
fn rejects_non_positive_timing_values() {
    for timing in [
        "          kind: periodic\n          nominal_rate_hz: 0\n",
        "          kind: periodic\n          max_rate_hz: -1.0\n",
        "          kind: on_demand\n          nominal_response_seconds: 0.0\n",
        "          kind: on_demand\n          max_response_seconds: -0.5\n",
    ] {
        assert!(
            matches!(
                parse_yaml(&with_timing(timing)),
                Err(ContractError::Semantic(
                    SemanticError::NonPositiveTimingValue { .. }
                ))
            ),
            "{timing} should be rejected"
        );
    }
}

#[test]
fn rejects_unknown_timing_kind_and_stray_timing_field() {
    assert!(matches!(
        parse_yaml(&with_timing("          kind: sporadic\n")),
        Err(ContractError::Parse(_))
    ));
    assert!(matches!(
        parse_yaml(&with_timing(
            "          kind: asynchronous\n          nominal_rate_hz: 1.0\n"
        )),
        Err(ContractError::Parse(_))
    ));
}

#[test]
fn not_applicable_requires_reason_and_no_exchanges() {
    let valid = minimal_yaml(
        "functions:\n  - id: f1\n    name: F1\n    category: required\n    required_group: subsystem\n    applicability: not_applicable\n    not_applicable_reason: Not applicable to this illustrative subsystem.\n    exchanges: []\n",
    );
    let contract = parse_yaml(&valid).expect("valid not-applicable function should parse");
    assert_eq!(
        contract.functions[0].applicability,
        Applicability::NotApplicable
    );
    assert!(contract.functions[0].not_applicable_reason.is_some());

    let missing_reason = valid.replace(
        "    not_applicable_reason: Not applicable to this illustrative subsystem.\n",
        "",
    );
    assert!(matches!(
        parse_yaml(&missing_reason),
        Err(ContractError::Semantic(
            SemanticError::NotApplicableWithoutReason { .. }
        ))
    ));

    let with_exchange = valid.replace(
        "    exchanges: []\n",
        "    exchanges:\n      - id: e1\n        kind: special_signal\n        direction: input\n        mandate: mandatory\n        name: S\n        timing:\n          kind: asynchronous\n",
    );
    assert!(matches!(
        parse_yaml(&with_exchange),
        Err(ContractError::Semantic(
            SemanticError::NotApplicableWithExchanges { .. }
        ))
    ));
}

#[test]
fn preserves_task_040_capability_roles_and_position_processing() {
    // The merged Task 040 contract shape: a Capability inventory, per-Capability
    // required functions with standard roles and ownership, and a
    // component-level position_information_processing function.
    let path = Path::new("tests/fixtures/upstream-capability-function-profile.yaml");
    let contract = load_contract(path).expect("upstream Task 040 shape should parse");

    let capabilities = contract
        .capabilities
        .as_ref()
        .expect("capabilities were declared");
    assert_eq!(capabilities.len(), 2);
    assert_eq!(capabilities[0].id, "esm");
    assert_eq!(capabilities[1].id, "radar");

    // Function order is the contract's, not alphabetical.
    let ids = contract
        .functions
        .iter()
        .map(|function| function.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "position-information",
            "esm-status",
            "esm-enable-disable",
            "esm-operations",
            "radar-status",
            "radar-enable-disable",
            "radar-operations",
        ]
    );

    // Component-level position information processing: role present, and with
    // NO capability ownership. Nothing invents one.
    let position = &contract.functions[0];
    assert_eq!(
        position.standard_role,
        Some(StandardRole::PositionInformationProcessing)
    );
    assert_eq!(position.capability, None);
    assert_eq!(position.category, FunctionCategory::Required);
    assert_eq!(position.required_group, Some(RequiredGroup::Capability));

    // Per-Capability required functions keep both role and ownership.
    let esm_status = &contract.functions[1];
    assert_eq!(
        esm_status.standard_role,
        Some(StandardRole::CapabilityStatus)
    );
    assert_eq!(esm_status.capability.as_deref(), Some("esm"));
    let radar_operations = &contract.functions[6];
    assert_eq!(
        radar_operations.standard_role,
        Some(StandardRole::CapabilityOperations)
    );
    assert_eq!(radar_operations.capability.as_deref(), Some("radar"));
}

#[test]
fn parses_upstream_minimal_example_unchanged() {
    // Cross-repository compatibility: an upstream-authored v0.1 contract is
    // consumed by this independent implementation with no edits to its body.
    let contract = load_contract(Path::new("tests/fixtures/upstream-minimal.yaml"))
        .expect("upstream minimal example should parse");
    assert_eq!(contract.contract_version, "0.1");
    assert_eq!(contract.functions.len(), 1);
    let Exchange::OmsMessage(oms) = &contract.functions[0].exchanges[0] else {
        panic!("expected an OMS message exchange");
    };
    assert_eq!(oms.message, "PositionReport");
}

#[test]
fn rejects_unsupported_file_extension() {
    assert!(matches!(
        load_contract(Path::new("tests/fixtures/upstream-minimal.txt")),
        Err(ContractError::Io(_))
    ));
}
