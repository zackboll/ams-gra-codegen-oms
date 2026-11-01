//! Two independently typed generated endpoints on one schema-specific runtime.

#[cfg(ams_gra_real_uci_multimessage25)]
mod uci25 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_multimessage25 as generated;
    const VERSION: &str = "002.5.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
    const LOG_GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task093-real-log25.json");
    const RDMA_GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task092-real-rdma25.json");
    fn data(value: &generated::model::LogMT) -> Option<&[u8]> {
        value.messagedata.data.as_deref()
    }
    fn assert_version_surface() {
        assert!(generated::model::WhitespaceVisibleString1024Type::new("").is_some());
    }
    include!("real_multimessage_cases/mod.rs");
}

#[cfg(ams_gra_real_uci_multimessage26)]
mod uci26 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_multimessage26 as generated;
    const VERSION: &str = "002.6.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_6_ROOT_BUILD");
    const LOG_GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task093-real-log26.json");
    const RDMA_GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task092-real-rdma26.json");
    fn data(value: &generated::model::LogMT) -> Option<&[u8]> {
        value.messagedata.data.as_ref().map(|v| v.as_slice())
    }
    fn assert_version_surface() {
        assert!(generated::model::WhitespaceVisibleString1024Type::new("").is_none());
    }
    include!("real_multimessage_cases/mod.rs");
}

#[cfg(not(all(ams_gra_real_uci_multimessage25, ams_gra_real_uci_multimessage26)))]
#[test]
fn real_multimessage_inputs_not_configured() {
    eprintln!(
        "SKIPPED: combined real UCI requires both pinned roots; qualification sets AMS_GRA_REQUIRE_REAL_UCI_MULTIMESSAGE"
    );
}
