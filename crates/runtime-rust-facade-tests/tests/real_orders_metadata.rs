//! RUST-OMS-012: independent authentic OrdersMetadata qualification per release.

#[cfg(any(ams_gra_real_uci_orders_metadata25, ams_gra_real_uci_orders_metadata26))]
mod common;

#[cfg(ams_gra_real_uci_orders_metadata25)]
mod uci25 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_orders_metadata25 as generated;
    const VERSION: &str = "002.5.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_5_ROOT_BUILD");
    const NORMAL: &str = include_str!("../../../tests/fixtures/rust-oms-012/uci25-normal.json");
    const SECOND: &str = include_str!("../../../tests/fixtures/rust-oms-012/uci25-second.json");
    const NEGATIVE: &str = include_str!("../../../tests/fixtures/rust-oms-012/uci25-negative.json");
    include!("real_orders_metadata_cases/mod.rs");
}

#[cfg(ams_gra_real_uci_orders_metadata26)]
mod uci26 {
    use ams_gra_oms_runtime_rust_facade_tests::real_uci_orders_metadata26 as generated;
    const VERSION: &str = "002.6.0";
    const ROOT: &str = env!("AMS_GRA_UCI_2_6_ROOT_BUILD");
    const NORMAL: &str = include_str!("../../../tests/fixtures/rust-oms-012/uci26-normal.json");
    const SECOND: &str = include_str!("../../../tests/fixtures/rust-oms-012/uci26-second.json");
    const NEGATIVE: &str = include_str!("../../../tests/fixtures/rust-oms-012/uci26-negative.json");
    include!("real_orders_metadata_cases/mod.rs");
}

#[cfg(all(ams_gra_real_uci_orders_metadata25, ams_gra_real_uci_orders_metadata26))]
#[test]
fn independent_version_types() {
    use ams_gra_oms_runtime_rust_facade_tests::{
        real_uci_orders_metadata25 as u25, real_uci_orders_metadata26 as u26,
    };
    assert_ne!(
        std::any::TypeId::of::<u25::model::OrdersMetadataMT>(),
        std::any::TypeId::of::<u26::model::OrdersMetadataMT>()
    );
    // SYSTEM_SCHEDULE was added to MessageTypeEnum in 2.6, not backported.
    let mut body: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/rust-oms-012/uci26-second.json"
    ))
    .unwrap();
    body["MessageHeader"]["SchemaVersion"] = serde_json::json!("002.5.0");
    use ams_gra_oms_runtime_rust::OmsJsonCodec;
    let result: Result<u25::model::OrdersMetadataMT, _> =
        u25::service_codec::ServiceCodec.decode_payload(&body);
    assert!(result.unwrap_err().message().contains("MessageType"));
}
