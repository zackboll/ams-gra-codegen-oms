//! Three authentic message families, independently generated for each release.

#[cfg(ams_gra_real_uci_survivability25)]
mod uci25 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_survivability25 as generated;
    const VERSION: &str = "002.5.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
    const CORPUS: &str = include_str!("../../../tests/fixtures/rust-oms-004/corpus25.json");
    const LOG_GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task093-real-log25.json");
    const RDMA_GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task092-real-rdma25.json");
    fn data(value: &generated::model::LogMT) -> Option<&[u8]> {
        value.messagedata.data.as_deref()
    }
    fn version_surface(value: &generated::model::SurvivabilityRiskLevelMT, expected: &Value) {
        use generated::model as m;
        assert!(m::WhitespaceVisibleString1024Type::new("").is_some());
        assert!(m::WhitespaceVisibleString4096Type::new("").is_some());
        let declass: &Option<m::XmlSchemaDateTime> = &value.securityinformation.declassdate;
        let decontrol: &Option<m::XmlSchemaDateTime> = &value.securityinformation.cui_decontroldate;
        assert!(declass.is_none() && decontrol.is_none());
        assert!(m::XmlSchemaDateTime::new("2026-01-02T03:04:05+00:00").is_some());
        assert!(value.securityinformation.declassdate.is_none());
        assert!(expected["SecurityInformation"].get("DeclassDate").is_none());
    }
    include!("real_survivability_cases/mod.rs");
}

#[cfg(ams_gra_real_uci_survivability26)]
mod uci26 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_survivability26 as generated;
    const VERSION: &str = "002.6.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_6_ROOT_BUILD");
    const CORPUS: &str = include_str!("../../../tests/fixtures/rust-oms-004/corpus26.json");
    const LOG_GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task093-real-log26.json");
    const RDMA_GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task092-real-rdma26.json");
    fn data(value: &generated::model::LogMT) -> Option<&[u8]> {
        value.messagedata.data.as_ref().map(|v| v.as_slice())
    }
    fn version_surface(value: &generated::model::SurvivabilityRiskLevelMT, expected: &Value) {
        use generated::model as m;
        assert!(m::WhitespaceVisibleString1024Type::new("").is_none());
        assert!(m::WhitespaceVisibleString4096Type::new("").is_none());
        assert!(m::DateTimeType::new("2026-01-02T03:04:05+00:00").is_none());
        assert_eq!(
            m::DateTimeType::new(" \t2026-01-02T03:04:05Z\n")
                .unwrap()
                .as_str(),
            "2026-01-02T03:04:05Z"
        );
        let security = &value.securityinformation;
        // These annotations deliberately require the 2.6 named model carrier.
        let declass: &Option<m::DateTimeType> = &security.declassdate;
        let decontrol: &Option<m::DateTimeType> = &security.cui_decontroldate;
        assert_eq!(
            declass.as_ref().map(m::DateTimeType::as_str),
            expected["SecurityInformation"]["DeclassDate"].as_str()
        );
        assert_eq!(
            decontrol.as_ref().map(m::DateTimeType::as_str),
            expected["SecurityInformation"]["CUI_DecontrolDate"].as_str()
        );
        assert_eq!(
            security
                .classifiedby
                .as_ref()
                .map(m::WhitespaceVisibleString1024Type::as_str),
            expected["SecurityInformation"]["ClassifiedBy"].as_str()
        );
        assert_eq!(
            security.sar_identifier.as_slice().len(),
            expected["SecurityInformation"]["SAR_Identifier"]
                .as_array()
                .map_or(0, Vec::len)
        );
        if let Some(expected) = expected["SecurityInformation"]["SAR_Identifier"].as_array() {
            for (actual, expected) in security.sar_identifier.as_slice().iter().zip(expected) {
                assert_eq!(Some(actual.as_str()), expected.as_str());
            }
        }
    }
    include!("real_survivability_cases/mod.rs");
}

#[cfg(not(all(ams_gra_real_uci_survivability25, ams_gra_real_uci_survivability26)))]
#[test]
fn real_survivability_inputs_not_configured() {
    eprintln!(
        "SKIPPED: three-message qualification requires both pinned roots; set AMS_GRA_REQUIRE_REAL_UCI_SURVIVABILITY"
    );
}
