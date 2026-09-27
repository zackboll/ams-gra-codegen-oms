//! Task 051: GENERATED OMS JSON codecs for qualified NON-OAM schemas.
//!
//! OMSC-SPC-013 Rev B section 6.1.2: a particle member is keyed by the
//! ELEMENT declaration's target namespace, bare only for the OAM namespace,
//! otherwise `{namespace}local`. `$type` is the concrete complexType's QName
//! the same way. No spelling is aliased: a bare key where a Clark key is
//! required is an unknown member.

use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use ams_gra_oms_runtime_rust_facade_tests::{
    codec_choice, codec_inherit, codec_shape, runtime_test_codec,
};
use serde_json::{Value, json};

fn rejects<P>(codec: &impl OmsJsonCodec<P>, body: &Value, path: &str, detail: &str) {
    let error: CodecError = match codec.decode_payload(body) {
        Ok(_) => panic!("{body} must be rejected"),
        Err(error) => error,
    };
    assert!(
        error.message().starts_with(path) && error.message().contains(detail),
        "expected {path} ... {detail}, got: {}",
        error.message()
    );
}

mod record {
    use super::*;
    use runtime_test_codec::model as m;
    use runtime_test_codec::service_codec::ServiceCodec;

    fn shared(count: i64) -> m::SharedPayload {
        m::SharedPayload {
            count: m::BoundedI64::new(count).expect("xs:int"),
        }
    }

    /// The generated codec owns ONLY the inner (payload) key.
    #[test]
    fn task051_non_oam_record_uses_clark_member_keys() {
        let encoded = ServiceCodec.encode_payload(&shared(7)).expect("encode");
        assert_eq!(encoded, json!({ "{urn:test}Count": 7 }));
        assert_eq!(
            serde_json::to_string(&encoded).unwrap(),
            r#"{"{urn:test}Count":7}"#
        );
        let decoded: m::SharedPayload = ServiceCodec.decode_payload(&encoded).expect("decode");
        assert_eq!(decoded.count.get(), 7);
        let other = m::OtherPayload {
            label: "x y".to_owned(),
        };
        assert_eq!(
            ServiceCodec.encode_payload(&other).expect("encode"),
            json!({ "{urn:test}Label": "x y" })
        );
    }

    /// Runtime-owned outer key + codec-owned inner key = the complete
    /// OMS JSON document `{"{urn:test}MessageB":{"{urn:test}Count":7}}`.
    #[test]
    fn task051_complete_document_combines_runtime_and_codec_keys() {
        use ams_gra_oms_runtime_rust::{
            oms_global_element_name, unwrap_global_element, wrap_global_element,
        };
        let payload = ServiceCodec.encode_payload(&shared(7)).expect("encode");
        let document =
            wrap_global_element(oms_global_element_name("urn:test", "MessageB"), payload);
        assert_eq!(
            serde_json::to_string(&document).unwrap(),
            r#"{"{urn:test}MessageB":{"{urn:test}Count":7}}"#
        );
        let inner = unwrap_global_element(
            "{urn:test}MessageB",
            r#"{"{urn:test}MessageB":{"{urn:test}Count":7}}"#,
        )
        .expect("envelope");
        let decoded: m::SharedPayload = ServiceCodec.decode_payload(&inner).expect("decode");
        assert_eq!(decoded.count.get(), 7);
    }

    /// Strict negatives: no aliasing between bare and Clark spellings.
    #[test]
    fn task051_non_oam_record_rejects_wrong_member_spellings() {
        let codec = ServiceCodec;
        let p = "SharedPayload";
        let reject = |body: Value, detail: &str| {
            rejects::<m::SharedPayload>(&codec, &body, p, detail);
        };
        // bare member where the Clark member is required
        reject(json!({ "Count": 7 }), "unknown member \"Count\"");
        // wrong namespace
        reject(
            json!({ "{urn:other}Count": 7 }),
            "unknown member \"{urn:other}Count\"",
        );
        // OAM-namespace Clark spelling is not an alias either
        reject(
            json!({ "{https://www.vdl.afrl.af.mil/programs/oam}Count": 7 }),
            "unknown member",
        );
        // unknown Clark member
        reject(
            json!({ "{urn:test}Count": 7, "{urn:test}Extra": 1 }),
            "unknown member \"{urn:test}Extra\"",
        );
        // missing required Clark member
        reject(json!({}), "missing required member \"{urn:test}Count\"");
        // both correct and bare spellings present
        reject(
            json!({ "{urn:test}Count": 7, "Count": 7 }),
            "unknown member \"Count\"",
        );
        // $type is the owner's own type QName, never its bare name
        reject(
            json!({ "{urn:test}Count": 7, "$type": "SharedPayload" }),
            "is not \"{urn:test}SharedPayload\"",
        );
        // A value error keeps a useful semantic path.
        reject(
            json!({ "{urn:test}Count": "seven" }),
            "SharedPayload.{urn:test}Count: expected integer number",
        );
        // The exact $type of the concrete type itself is accepted.
        let ok: m::SharedPayload = codec
            .decode_payload(&json!({ "{urn:test}Count": 7, "$type": "{urn:test}SharedPayload" }))
            .expect("own $type");
        assert_eq!(ok.count.get(), 7);
    }
}

mod choice {
    use super::*;
    use codec_choice::model as m;
    use codec_choice::service_codec::ServiceCodec;

    fn payload(pick: m::Selection) -> m::ChoicePayload {
        m::ChoicePayload { pick }
    }

    /// Choice alternatives are keyed by their element QName, never by the
    /// Rust variant spelling.
    #[test]
    fn task051_non_oam_choice_uses_clark_alternative_keys() {
        let alpha = payload(m::Selection::Alpha(m::BoundedI64::new(5).expect("xs:int")));
        let encoded = ServiceCodec.encode_payload(&alpha).expect("encode");
        assert_eq!(
            encoded,
            json!({ "{urn:choice}Pick": { "{urn:choice}Alpha": 5 } })
        );
        let decoded: m::ChoicePayload = ServiceCodec.decode_payload(&encoded).expect("decode");
        assert_eq!(decoded, alpha);

        let beta = payload(m::Selection::Beta("b".to_owned()));
        let encoded = ServiceCodec.encode_payload(&beta).expect("encode");
        assert_eq!(
            encoded,
            json!({ "{urn:choice}Pick": { "{urn:choice}Beta": "b" } })
        );
        let decoded: m::ChoicePayload = ServiceCodec.decode_payload(&encoded).expect("decode");
        assert_eq!(decoded, beta);
    }

    #[test]
    fn task051_non_oam_choice_rejects_wrong_alternatives() {
        let codec = ServiceCodec;
        let reject = |pick: Value, detail: &str| {
            rejects::<m::ChoicePayload>(
                &codec,
                &json!({ "{urn:choice}Pick": pick }),
                "ChoicePayload.{urn:choice}Pick",
                detail,
            );
        };
        // bare alternative
        reject(json!({ "Alpha": 5 }), "unknown member \"Alpha\"");
        // wrong namespace
        reject(
            json!({ "{urn:other}Alpha": 5 }),
            "unknown member \"{urn:other}Alpha\"",
        );
        // two alternatives
        reject(
            json!({ "{urn:choice}Alpha": 5, "{urn:choice}Beta": "b" }),
            "exactly one selected member",
        );
        // none
        reject(json!({}), "exactly one selected member");
        // Rust variant spelling is not a wire name
        reject(json!({ "alpha": 5 }), "unknown member \"alpha\"");
        // the Choice owner's bare $type is not its $type
        reject(
            json!({ "{urn:choice}Alpha": 5, "$type": "Selection" }),
            "is not \"{urn:choice}Selection\"",
        );
        // bare outer member
        rejects::<m::ChoicePayload>(
            &codec,
            &json!({ "Pick": { "{urn:choice}Alpha": 5 } }),
            "ChoicePayload",
            "unknown member \"Pick\"",
        );
    }
}

mod inheritance {
    use super::*;
    use codec_inherit::model as m;
    use codec_inherit::service_codec::ServiceCodec;

    /// Both inherited and added members sit at ONE level, each with its own
    /// FieldDecl wire QName; there is no nested base object.
    #[test]
    fn task051_non_oam_inherited_record_is_flat_with_clark_keys() {
        let value = m::Derived {
            basevalue: m::BoundedI64::new(3).expect("xs:int"),
            extravalue: "extra".to_owned(),
        };
        let encoded = ServiceCodec.encode_payload(&value).expect("encode");
        assert_eq!(
            encoded,
            json!({
                "{urn:inherit}BaseValue": 3,
                "{urn:inherit}ExtraValue": "extra"
            })
        );
        let decoded: m::Derived = ServiceCodec.decode_payload(&encoded).expect("decode");
        assert_eq!(decoded, value);
        // A nested base object or a bare inherited member is rejected.
        rejects::<m::Derived>(
            &ServiceCodec,
            &json!({ "{urn:inherit}Base": { "{urn:inherit}BaseValue": 3 }, "{urn:inherit}ExtraValue": "x" }),
            "Derived",
            "unknown member \"{urn:inherit}Base\"",
        );
        rejects::<m::Derived>(
            &ServiceCodec,
            &json!({ "BaseValue": 3, "{urn:inherit}ExtraValue": "x" }),
            "Derived",
            "unknown member \"BaseValue\"",
        );
    }
}

mod abstract_type {
    use super::*;
    use codec_shape::model as m;
    use codec_shape::service_codec::ServiceCodec;

    fn boxed() -> m::ShapePayload {
        m::ShapePayload {
            shape: m::ShapeBase::BoxShape(m::BoxShape {
                tag: "t".to_owned(),
                width: m::BoundedI64::new(4).expect("xs:int"),
            }),
        }
    }

    /// `$type` is the concrete complexType's own QName in Clark notation.
    #[test]
    fn task051_non_oam_type_member_uses_the_concrete_type_qname() {
        let encoded = ServiceCodec.encode_payload(&boxed()).expect("encode");
        assert_eq!(
            encoded,
            json!({ "{urn:shape}Shape": {
                "$type": "{urn:shape}BoxShape",
                "{urn:shape}Tag": "t",
                "{urn:shape}Width": 4
            } })
        );
        let decoded: m::ShapePayload = ServiceCodec.decode_payload(&encoded).expect("decode");
        assert_eq!(decoded, boxed());
    }

    #[test]
    fn task051_non_oam_type_member_rejects_other_spellings() {
        let with_type = |ty: Value| {
            json!({ "{urn:shape}Shape": {
                "$type": ty,
                "{urn:shape}Tag": "t",
                "{urn:shape}Width": 4
            } })
        };
        let path = "ShapePayload.{urn:shape}Shape";
        for (ty, detail) in [
            (
                json!("BoxShape"),
                "$type \"BoxShape\" is not a known concrete ShapeBase",
            ),
            (
                json!("{urn:other}BoxShape"),
                "$type \"{urn:other}BoxShape\" is not a known concrete ShapeBase",
            ),
            (
                json!("{urn:shape}TriangleShape"),
                "is not a known concrete ShapeBase",
            ),
            // the abstract base itself is not concrete
            (
                json!("{urn:shape}ShapeBase"),
                "is not a known concrete ShapeBase",
            ),
        ] {
            rejects::<m::ShapePayload>(&ServiceCodec, &with_type(ty), path, detail);
        }
        // No field-based subtype selection: without $type it is rejected.
        rejects::<m::ShapePayload>(
            &ServiceCodec,
            &json!({ "{urn:shape}Shape": { "{urn:shape}Tag": "t", "{urn:shape}Width": 4 } }),
            path,
            "requires a $type member",
        );
        // A correct $type with a member of the OTHER concrete type fails.
        rejects::<m::ShapePayload>(
            &ServiceCodec,
            &json!({ "{urn:shape}Shape": {
                "$type": "{urn:shape}CircleShape",
                "{urn:shape}Tag": "t",
                "{urn:shape}Width": 4
            } }),
            path,
            "unknown member \"{urn:shape}Width\"",
        );
    }
}
