fn corpus() -> Value {
    serde_json::from_str(CORPUS).unwrap()
}

fn risk_golden() -> Value {
    corpus()["positives"][0][1].clone()
}

fn inspect_risk(value: &m::SurvivabilityRiskLevelMT, expected: &Value) {
    assert_eq!(value.messageheader.systemid.uuid.as_str(), UUID);
    assert_eq!(value.messageheader.schemaversion.as_str(), VERSION);
    assert_eq!(
        value.messageheader.timestamp.as_str(),
        "2026-01-02T03:04:05Z"
    );
    assert_eq!(value.messageheader.mode, m::MessageModeEnum::LIVE);
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
                assert_eq!(
                    format!("{id:?}"),
                    expected["GovernmentIdentifier"].as_str().unwrap()
                );
            }
            m::OwnerProducerChoiceType::NATOSpecialWord(word) => {
                assert_eq!(Some(word.as_str()), expected["NATO_SpecialWord"].as_str());
            }
        }
    }
    let data = &value.messagedata;
    assert_eq!(data.survivabilityrisklevelid.uuid.as_str(), UUID);
    let systems = expected["MessageData"]["AppliesToSystemID"].as_array();
    assert_eq!(
        data.appliestosystemid.as_slice().len(),
        systems.map_or(0, Vec::len)
    );
    if let Some(systems) = systems {
        for (actual, expected) in data.appliestosystemid.as_slice().iter().zip(systems) {
            assert_eq!(Some(actual.uuid.as_str()), expected["UUID"].as_str());
        }
    }
    let settings = data.risksetting.as_slice();
    let expected_settings = expected["MessageData"]["RiskSetting"].as_array().unwrap();
    assert_eq!(settings.len(), expected_settings.len());
    for (setting, expected) in settings.iter().zip(expected_settings) {
        // Authentic records: inspect required members, never invented variants.
        assert_eq!(
            format!("{:?}", setting.riskprioritization),
            expected["RiskPrioritization"].as_str().unwrap()
        );
        let levels = setting.risklevel.as_slice();
        let expected_levels = expected["RiskLevel"].as_array().unwrap();
        assert_eq!(levels.len(), expected_levels.len());
        for (level, expected) in levels.iter().zip(expected_levels) {
            assert_eq!(
                format!("{:?}", level.allowedrisk),
                expected["AllowedRisk"].as_str().unwrap()
            );
            match (
                &level.maximumcontinuousexposure,
                expected.get("MaximumContinuousExposure"),
            ) {
                (Some(exposure), Some(expected)) => {
                    assert_eq!(
                        Some(exposure.compositeexposuretime.as_str()),
                        expected["CompositeExposureTime"].as_str()
                    );
                    assert_eq!(
                        Some(exposure.cumulativeexposuretime.as_str()),
                        expected["CumulativeExposureTime"].as_str()
                    );
                }
                (None, None) => {}
                other => panic!("exposure presence mismatch: {other:?}"),
            }
        }
    }
    version_surface(value, expected);
}

#[test]
fn authentic_model_and_codec() {
    fn compatible<
        C: OmsJsonCodec<m::LogMT>
            + OmsJsonCodec<m::RDMAInitializeSetupMT>
            + OmsJsonCodec<m::SurvivabilityRiskLevelMT>,
    >() {
    }
    compatible::<ServiceCodec>();
    for case in corpus()["positives"].as_array().unwrap() {
        let value: m::SurvivabilityRiskLevelMT = ServiceCodec.decode_payload(&case[1]).unwrap();
        inspect_risk(&value, &case[2]);
        assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), case[2]);
    }
    for case in corpus()["invalid"].as_array().unwrap() {
        let result: Result<m::SurvivabilityRiskLevelMT, _> = ServiceCodec.decode_payload(&case[1]);
        assert!(result.is_err(), "independent negative {}", case[0]);
    }
    assert!(
        m::UniversallyUniqueIdentifierType::new("12345678-1234-1234-1234-123456789abc").is_none()
    );
    assert!(m::UCISchemaVersionStringType::new("002.6").is_none());
    assert!(m::BoundedVec::<m::SurvivabilityRiskSettingType, 1, 4>::new(vec![]).is_none());
    assert!(m::BoundedVec::<m::RiskSettingType, 1, 5>::new(vec![]).is_none());
    let value: m::SurvivabilityRiskLevelMT = ServiceCodec.decode_payload(&risk_golden()).unwrap();
    let setting = value.messagedata.risksetting.as_slice()[0].clone();
    let level = setting.risklevel.as_slice()[0].clone();
    assert!(
        m::BoundedVec::<m::SurvivabilityRiskSettingType, 1, 4>::new(vec![setting; 5]).is_none()
    );
    assert!(m::BoundedVec::<m::RiskSettingType, 1, 5>::new(vec![level; 6]).is_none());
    for missing in ["CompositeExposureTime", "CumulativeExposureTime"] {
        let mut body = risk_golden();
        let mut exposure = json!({"CompositeExposureTime":"PT1S", "CumulativeExposureTime":"PT2S"});
        exposure.as_object_mut().unwrap().remove(missing);
        body["MessageData"]["RiskSetting"][0]["RiskLevel"][0]["MaximumContinuousExposure"] =
            exposure;
        let result: Result<m::SurvivabilityRiskLevelMT, _> = ServiceCodec.decode_payload(&body);
        assert!(result.unwrap_err().to_string().contains(missing));
    }
    // 2.5 collapses whitespace-visible strings; 2.6 preserves their text.
    let text = "  authority  text  ";
    let normalized = if VERSION == "002.5.0" {
        "authority text"
    } else {
        text
    };
    assert_eq!(
        m::WhitespaceVisibleString1024Type::new(text)
            .unwrap()
            .as_str(),
        normalized
    );
    assert_eq!(m::DurationType::new(" \tPT1S\n").unwrap().as_str(), "PT1S");
    let mut input = risk_golden();
    input["MessageHeader"]["Timestamp"] = json!(" \t2026-01-02T03:04:05Z\n");
    input["SecurityInformation"]["ClassifiedBy"] = json!(text);
    input["MessageData"]["RiskSetting"][0]["RiskLevel"][0]["MaximumContinuousExposure"] =
        json!({"CompositeExposureTime":" \tPT1S\n", "CumulativeExposureTime":" PT2S "});
    let mut expected = input.clone();
    expected["SecurityInformation"]["ClassifiedBy"] = json!(normalized);
    expected["MessageHeader"]["Timestamp"] = json!("2026-01-02T03:04:05Z");
    expected["MessageData"]["RiskSetting"][0]["RiskLevel"][0]["MaximumContinuousExposure"] =
        json!({"CompositeExposureTime":"PT1S", "CumulativeExposureTime":"PT2S"});
    let value: m::SurvivabilityRiskLevelMT = ServiceCodec.decode_payload(&input).unwrap();
    assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), expected);
    eprintln!("RUST-OMS-004 {VERSION} MODEL AND CODEC: PASSED");
}
