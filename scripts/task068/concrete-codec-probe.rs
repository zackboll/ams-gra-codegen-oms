#[path = "../audit.rs"]
mod model;
#[path = "../service_codec.rs"]
mod service_codec;
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use serde_json::json;
fn main() {
    for ty in [
        "{urn:audit}PublicA",
        "{urn:audit}PrivateB",
        "{urn:audit}PrivateC",
    ] {
        let body = json!({"{urn:audit}Value":{"$type":ty,"{urn:audit}PublicFlag":true}});
        match <service_codec::ServiceCodec as OmsJsonCodec<model::Holder>>::decode_payload(
            &service_codec::ServiceCodec,
            &body,
        ) {
            Ok(v) => println!(
                "{ty}: accepted {}",
                service_codec::ServiceCodec.encode_payload(&v).unwrap()
            ),
            Err(e) => println!("{ty}: {}", e.message()),
        }
    }
}
