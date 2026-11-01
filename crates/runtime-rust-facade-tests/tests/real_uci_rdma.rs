//! Authentic version-separated RDMA consumers; only production-generated codecs.

#[cfg(ams_gra_real_uci_rdma25)]
mod uci25 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_rdma25 as generated;
    const VERSION: &str = "002.5.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
    const GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task092-real-rdma25.json");
    fn assert_version_surface() {
        use generated::model::WhitespaceVisibleString1024Type as Text;
        assert!(Text::new("").is_some());
        assert_eq!(Text::new("  RDMA  setup  ").unwrap().as_str(), "RDMA setup");
    }
    include!("real_rdma_cases/mod.rs");
}

#[cfg(ams_gra_real_uci_rdma26)]
mod uci26 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_rdma26 as generated;
    const VERSION: &str = "002.6.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_6_ROOT_BUILD");
    const GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task092-real-rdma26.json");
    fn assert_version_surface() {
        use generated::model::WhitespaceVisibleString1024Type as Text;
        assert!(Text::new("").is_none());
        assert_eq!(
            Text::new("  RDMA  setup  ").unwrap().as_str(),
            "  RDMA  setup  "
        );
    }
    include!("real_rdma_cases/mod.rs");
}

#[cfg(not(all(ams_gra_real_uci_rdma25, ams_gra_real_uci_rdma26)))]
#[test]
fn real_rdma_inputs_not_configured() {
    eprintln!(
        "SKIPPED: real RDMA requires both pinned UCI roots; qualification sets AMS_GRA_REQUIRE_REAL_UCI_RDMA"
    );
}
