//! Standalone public-API consumer: generated modules are supplied by the driver.
#[cfg(test)]
use super::{DOCUMENT25, DOCUMENT26};
use super::{uci25, uci26};
#[cfg(test)]
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use ams_gra_oms_runtime_rust::SleetRuntime;

// These bodies are compiled against the real production runtime and generated
// endpoint signatures; no facade-test crate or handwritten API adapter is used.
#[allow(dead_code)]
fn endpoints25(
    runtime: &mut SleetRuntime<uci25::service_codec::ServiceCodec>,
    value: &uci25::model::OrdersMetadataMT,
) {
    use uci25::service_api::function_f as svc;
    let subscription =
        svc::exchange_e::subscribe(runtime, |_: &uci25::model::OrdersMetadataMT| {}).unwrap();
    svc::exchange_publish::publish(runtime, value).unwrap();
    subscription.unsubscribe().unwrap();
}

#[allow(dead_code)]
fn endpoints26(
    runtime: &mut SleetRuntime<uci26::service_codec::ServiceCodec>,
    value: &uci26::model::OrdersMetadataMT,
) {
    use uci26::service_api::function_f as svc;
    let subscription =
        svc::exchange_e::subscribe(runtime, |_: &uci26::model::OrdersMetadataMT| {}).unwrap();
    svc::exchange_publish::publish(runtime, value).unwrap();
    subscription.unsubscribe().unwrap();
}

#[cfg(test)]
macro_rules! qualification {
    ($test:ident, $module:ident, $version:literal, $document:ident) => {
        #[test]
        fn $test() {
            use $module::{model, service_api::function_f as svc, service_codec::ServiceCodec};
            let body: serde_json::Value = serde_json::from_str($document).unwrap();
            let value: model::OrdersMetadataMT = ServiceCodec.decode_payload(&body).unwrap();
            assert_eq!(value.messageheader.schemaversion.as_str(), $version);
            let qualifier = value.messagedata.ordersassociation.as_slice()[0]
                .orders
                .ato
                .as_slice()[0]
                .identifier
                .as_ref()
                .unwrap()
                .qualifierserialnumber
                .unwrap();
            assert_eq!(qualifier.get(), 999);
            assert_eq!(
                model::USMTFSerialNumberOfQualifierType::new(1)
                    .unwrap()
                    .get(),
                1
            );
            for invalid in [0, 1000, -1] {
                assert!(model::USMTFSerialNumberOfQualifierType::new(invalid).is_none());
            }
            assert_eq!(ServiceCodec.encode_payload(&value).unwrap(), body);
            assert_eq!(svc::exchange_e::MESSAGE_NAME, "OrdersMetadata");
            assert_eq!(svc::exchange_publish::MESSAGE_NAME, "OrdersMetadata");
            assert_eq!(svc::exchange_e::TOPIC, "orders.metadata");
            assert_eq!(svc::exchange_publish::TOPIC, "orders.metadata");
            assert_eq!(svc::exchange_e::SUBSCRIPTION_GROUP, None);
            println!("EXTERNAL ORDERSMETADATA {}: PASSED", $version);
        }
    };
}

#[cfg(test)]
qualification!(orders25, uci25, "002.5.0", DOCUMENT25);
#[cfg(test)]
qualification!(orders26, uci26, "002.6.0", DOCUMENT26);
