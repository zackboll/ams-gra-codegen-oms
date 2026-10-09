//! Task068 evidence only: generated closed codecs do not retain unknown values.
use ams_gra_oms_runtime_rust::OmsJsonCodec;
use ams_gra_oms_runtime_rust_facade_tests::codec_shape::{model, service_codec::ServiceCodec};
use serde_json::json;

#[test]
fn unknown_subtype_and_unknown_payload_fail_closed() {
    let known = json!({"{urn:shape}Shape": {
        "$type": "{urn:shape}BoxShape",
        "{urn:shape}Tag": "t", "{urn:shape}Width": 4
    }});
    let decoded: model::ShapePayload = ServiceCodec.decode_payload(&known).unwrap();
    assert_eq!(ServiceCodec.encode_payload(&decoded).unwrap(), known);

    let mut unknown = known.clone();
    unknown["{urn:shape}Shape"]["$type"] = json!("{urn:private}NeverGenerated");
    let error = <ServiceCodec as OmsJsonCodec<model::ShapePayload>>::decode_payload(
        &ServiceCodec,
        &unknown,
    )
    .unwrap_err();
    assert_eq!(
        error.message(),
        "ShapePayload.{urn:shape}Shape: $type \"{urn:private}NeverGenerated\" is not a known concrete ShapeBase"
    );

    let mut extra = known.clone();
    extra["{urn:shape}Shape"]["{urn:private}Payload"] = json!({"nested": [1, true, null]});
    let error =
        <ServiceCodec as OmsJsonCodec<model::ShapePayload>>::decode_payload(&ServiceCodec, &extra)
            .unwrap_err();
    assert_eq!(
        error.message(),
        "ShapePayload.{urn:shape}Shape: unknown member \"{urn:private}Payload\""
    );

    let mut wrong = known;
    wrong["{urn:shape}Shape"]["$type"] = json!("{urn:shape}CircleShape");
    let error =
        <ServiceCodec as OmsJsonCodec<model::ShapePayload>>::decode_payload(&ServiceCodec, &wrong)
            .unwrap_err();
    assert_eq!(
        error.message(),
        "ShapePayload.{urn:shape}Shape: unknown member \"{urn:shape}Width\""
    );
}
