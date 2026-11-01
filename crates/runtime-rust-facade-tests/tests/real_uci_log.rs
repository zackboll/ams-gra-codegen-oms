//! External consumer of the independently generated authentic Log services.
//! No handwritten Log codec or transport: malformed PUBs use pinned CalClient.

#[cfg(ams_gra_real_uci_log25)]
mod uci25 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_log25 as generated;
    const VERSION: &str = "002.5.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
    fn set_data(value: &mut generated::model::LogMT, data: Option<Vec<u8>>) {
        value.messagedata.data = data;
    }
    fn data(value: &generated::model::LogMT) -> Option<&[u8]> {
        value.messagedata.data.as_deref()
    }
    const GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task093-real-log25.json");
    include!("real_log_cases/mod.rs");
}

#[cfg(ams_gra_real_uci_log26)]
mod uci26 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_log26 as generated;
    const VERSION: &str = "002.6.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_6_ROOT_BUILD");
    fn set_data(value: &mut generated::model::LogMT, data: Option<Vec<u8>>) {
        value.messagedata.data = data.map(generated::model::HexBinaryType::new);
    }
    fn data(value: &generated::model::LogMT) -> Option<&[u8]> {
        value.messagedata.data.as_ref().map(|data| data.as_slice())
    }
    const GOLDEN: &str =
        include_str!("../../../tests/fixtures/service-generate/task093-real-log26.json");
    include!("real_log_cases/mod.rs");
}

#[cfg(not(all(ams_gra_real_uci_log25, ams_gra_real_uci_log26)))]
#[test]
fn real_log_inputs_not_configured() {
    eprintln!(
        "SKIPPED: real Log requires both pinned UCI roots; explicit qualification sets AMS_GRA_REQUIRE_REAL_UCI_LOG"
    );
}
