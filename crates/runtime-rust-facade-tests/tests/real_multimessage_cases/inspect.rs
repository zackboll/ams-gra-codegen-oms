fn inspect_log(value: &m::LogMT) {
    assert_eq!(
        value.securityinformation.classification,
        m::ClassificationEnum::U
    );
    assert!(matches!(
        value.securityinformation.ownerproducer.as_slice(),
        [m::OwnerProducerChoiceType::GovernmentIdentifier(m::OwnerProducerEnum::USA),
         m::OwnerProducerChoiceType::NATOSpecialWord(word)] if word.as_str() == "NATO:EXAMPLE"
    ));
    let header = &value.messageheader;
    assert_eq!(
        header.systemid.uuid.as_str(),
        "12345678-1234-1234-8234-123456789abc"
    );
    assert_eq!(header.timestamp.as_str(), "2026-01-02T03:04:05Z");
    assert_eq!(header.schemaversion.as_str(), VERSION);
    assert_eq!(header.mode, m::MessageModeEnum::LIVE);
    let data = &value.messagedata;
    assert_eq!(data.timestamp.as_str(), "2026-01-02T03:04:06Z");
    assert_eq!(data.serviceuptime.as_ref().unwrap().as_str(), "PT01.2300S");
    assert_eq!(data.severity, m::LogSeverityEnum::INFO);
    assert_eq!(data.label.as_str(), "Task093");
    assert_eq!(
        data.details.as_str(),
        "Independent real Log Binary codec qualification"
    );
}

fn inspect_rdma(value: &m::RDMAInitializeSetupMT, expected: &Value) {
    assert_eq!(
        value.securityinformation.classification,
        m::ClassificationEnum::U
    );
    let owners = value.securityinformation.ownerproducer.as_slice();
    let expected_owners = expected["SecurityInformation"]["OwnerProducer"]
        .as_array()
        .unwrap();
    assert_eq!(owners.len(), expected_owners.len());
    for (owner, expected) in owners.iter().zip(expected_owners) {
        match owner {
            m::OwnerProducerChoiceType::GovernmentIdentifier(id) => {
                let id = match id {
                    m::OwnerProducerEnum::USA => "USA",
                    m::OwnerProducerEnum::CAN => "CAN",
                    other => panic!("unexpected government {other:?}"),
                };
                assert_eq!(expected, &json!({"GovernmentIdentifier":id}));
            }
            m::OwnerProducerChoiceType::NATOSpecialWord(word) => {
                assert_eq!(word.as_str(), "NATO:EXAMPLE");
                assert_eq!(expected, &json!({"NATO_SpecialWord":"NATO:EXAMPLE"}));
            }
        }
    }
    assert_eq!(value.messageheader.systemid.uuid.as_str(), UUID);
    assert_eq!(
        value.messageheader.timestamp.as_str(),
        "2026-01-02T03:04:05Z"
    );
    assert_eq!(value.messageheader.schemaversion.as_str(), VERSION);
    assert_eq!(value.messageheader.mode, m::MessageModeEnum::LIVE);
    let data = &value.messagedata;
    assert_eq!(data.rdma_initializesetupid.uuid.as_str(), UUID);
    assert_eq!(data.rdma_provider, m::RDMAProviderEnum::TCP);
    let endpoints = data.endpoint.as_slice();
    let expected_endpoints = expected["MessageData"]
        .get("Endpoint")
        .map(|v| v.as_array().unwrap().as_slice())
        .unwrap_or(&[]);
    assert_eq!(endpoints.len(), expected_endpoints.len());
    for (endpoint, expected) in endpoints.iter().zip(expected_endpoints) {
        match endpoint {
            m::NetworkEndpointType::IPv4Endpoint(ip) => {
                assert_eq!(
                    Some(ip.ipv4_address.as_str()),
                    expected["IPv4_Endpoint"]["IPv4_Address"].as_str()
                );
                assert!(ip.prefix.is_none() && ip.description.is_none());
            }
            m::NetworkEndpointType::IPv6Endpoint(ip) => {
                assert_eq!(
                    Some(ip.ipv6_address.as_str()),
                    expected["IPv6_Endpoint"]["IPv6_Address"].as_str()
                );
                assert!(ip.prefix.is_none() && ip.description.is_none());
            }
        }
    }
}

fn positives() -> Vec<(&'static str, Value)> {
    let mut cases = vec![("golden", rdma_golden())];
    for (name, endpoints) in [
        (
            "ipv4",
            json!([{"IPv4_Endpoint":{"IPv4_Address":"198.51.100.42"}}]),
        ),
        (
            "ipv6",
            json!([{"IPv6_Endpoint":{"IPv6_Address":"2001:db8::42"}}]),
        ),
        (
            "reversed",
            json!([
                {"IPv6_Endpoint":{"IPv6_Address":"2001:db8::1"}},
                {"IPv4_Endpoint":{"IPv4_Address":"192.0.2.10"}}
            ]),
        ),
        (
            "alternating",
            json!([
                {"IPv4_Endpoint":{"IPv4_Address":"198.51.100.42"}},
                {"IPv6_Endpoint":{"IPv6_Address":"2001:db8::42"}},
                {"IPv4_Endpoint":{"IPv4_Address":"192.0.2.10"}},
                {"IPv6_Endpoint":{"IPv6_Address":"2001:db8::1"}}
            ]),
        ),
        ("empty", json!([])),
    ] {
        let mut body = rdma_golden();
        body["MessageData"]["Endpoint"] = endpoints;
        if name == "alternating" {
            body["SecurityInformation"]["OwnerProducer"] = json!([
                {"GovernmentIdentifier":"USA"}, {"NATO_SpecialWord":"NATO:EXAMPLE"},
                {"GovernmentIdentifier":"CAN"}, {"NATO_SpecialWord":"NATO:EXAMPLE"}
            ]);
        }
        cases.push((name, body));
    }
    cases
}
