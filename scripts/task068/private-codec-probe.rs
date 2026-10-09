#[path = "../audit.rs"]
mod model;
#[path = "../service_codec.rs"]
mod service_codec;
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use serde_json::json;
fn main() {
    let body = json!({"{urn:audit}Value":{"$type":"{urn:audit}PrivateB","{urn:audit}Flag":true,"{urn:audit}Enabled":false}});
    let value: model::Holder = service_codec::ServiceCodec.decode_payload(&body).unwrap();
    assert_eq!(
        service_codec::ServiceCodec.encode_payload(&value).unwrap(),
        body
    );
    println!("PrivateB semantic round trip: {body}");
    for ty in ["{urn:audit}PublicA", "{urn:audit}PrivateC"] {
        let mut b = body.clone();
        b["{urn:audit}Value"]["$type"] = json!(ty);
        let e = <service_codec::ServiceCodec as OmsJsonCodec<model::Holder>>::decode_payload(
            &service_codec::ServiceCodec,
            &b,
        )
        .unwrap_err();
        println!("{ty}: {}", e.message());
    }
    let mut b = body.clone();
    b["{urn:audit}Value"]["{urn:audit}Secret"] = json!({"x":1});
    let e = <service_codec::ServiceCodec as OmsJsonCodec<model::Holder>>::decode_payload(
        &service_codec::ServiceCodec,
        &b,
    )
    .unwrap_err();
    println!("unknown member: {}", e.message());
}
