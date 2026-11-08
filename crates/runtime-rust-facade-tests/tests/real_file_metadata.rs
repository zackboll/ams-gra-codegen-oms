//! RUST-OMS-010: independent lexical/binary oracles and authentic live transport.

#[cfg(any(ams_gra_real_uci_file_metadata25, ams_gra_real_uci_file_metadata26))]
mod common;

#[cfg(ams_gra_real_uci_file_metadata25)]
mod uci25 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_file_metadata25 as generated;
    const VERSION: &str = "002.5.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
    const NORMAL: &str = include_str!("../../../tests/fixtures/rust-oms-010/uci25-normal.json");
    const SECOND: &str = include_str!("../../../tests/fixtures/rust-oms-010/uci25-second.json");
    const HASH: &str = include_str!("../../../tests/fixtures/rust-oms-010/uci25-hash.json");
    const NEGATIVE: &str = include_str!("../../../tests/fixtures/rust-oms-010/uci25-negative.json");
    include!("real_file_metadata_cases/mod.rs");
}

#[cfg(ams_gra_real_uci_file_metadata26)]
mod uci26 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_file_metadata26 as generated;
    const VERSION: &str = "002.6.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_6_ROOT_BUILD");
    const NORMAL: &str = include_str!("../../../tests/fixtures/rust-oms-010/uci26-normal.json");
    const SECOND: &str = include_str!("../../../tests/fixtures/rust-oms-010/uci26-second.json");
    const HASH: &str = include_str!("../../../tests/fixtures/rust-oms-010/uci26-hash.json");
    const NEGATIVE: &str = include_str!("../../../tests/fixtures/rust-oms-010/uci26-negative.json");
    include!("real_file_metadata_cases/mod.rs");
}

#[cfg(all(ams_gra_real_uci_file_metadata25, ams_gra_real_uci_file_metadata26))]
#[test]
fn independent_version_types() {
    use ams_gra_oms_runtime_rust_facade_tests::{
        real_uci_file_metadata25, real_uci_file_metadata26,
    };
    assert_ne!(
        std::any::TypeId::of::<real_uci_file_metadata25::model::FileMetadataMT>(),
        std::any::TypeId::of::<real_uci_file_metadata26::model::FileMetadataMT>()
    );
}
