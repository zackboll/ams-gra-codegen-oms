//! Message serialization only, using unchanged independently authored Task101 inputs.

#[cfg(ams_gra_real_uci_dlz25)]
mod uci25 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_dlz25 as generated;
    const VERSION: &str = "002.5.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
    const GOLDEN: &str = include_str!("../../../tests/fixtures/task101/uci25-full-dlz.json");
    include!("real_dlz_cases/mod.rs");
}

#[cfg(ams_gra_real_uci_dlz26)]
mod uci26 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_dlz26 as generated;
    const VERSION: &str = "002.6.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_6_ROOT_BUILD");
    const GOLDEN: &str = include_str!("../../../tests/fixtures/task101/uci26-full-dlz.json");
    include!("real_dlz_cases/mod.rs");
}

#[cfg(all(ams_gra_real_uci_dlz25, ams_gra_real_uci_dlz26))]
#[test]
fn independent_version_types() {
    use ams_gra_oms_runtime_rust_facade_tests::{real_uci_dlz25, real_uci_dlz26};
    assert_ne!(
        std::any::TypeId::of::<real_uci_dlz25::model::DLZMT>(),
        std::any::TypeId::of::<real_uci_dlz26::model::DLZMT>()
    );
}

#[cfg(not(all(ams_gra_real_uci_dlz25, ams_gra_real_uci_dlz26)))]
#[test]
fn real_dlz_inputs_not_configured() {
    eprintln!(
        "SKIPPED: DLZ qualification requires both pinned UCI roots and AMS_GRA_REQUIRE_REAL_UCI_DLZ"
    );
}
