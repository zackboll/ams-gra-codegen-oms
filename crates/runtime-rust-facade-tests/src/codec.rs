//! A handwritten, TEST-ONLY payload codec for the Task 049 fixtures.
//!
//! One provider type implements `OmsJsonCodec<P>` for three distinct
//! generated payload types. It encodes and decodes payload BODIES only;
//! the runtime adds and checks the global-element key. Generated model types
//! do not implement Serde.

use crate::{runtime_oam, runtime_test};
use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use serde_json::{Map, Value, json};

/// Codec provider for every payload type the fixtures select.
#[derive(Debug, Default, Clone, Copy)]
pub struct TestCodec;

/// The single member `name` of a JSON object that has exactly that member.
fn only_member<'a>(value: &'a Value, name: &str) -> Result<&'a Value, CodecError> {
    let object: &Map<String, Value> = value
        .as_object()
        .ok_or_else(|| CodecError::new("payload is not a JSON object"))?;
    if object.len() != 1 {
        return Err(CodecError::new(format!(
            "payload must have exactly the member {name:?}"
        )));
    }
    object
        .get(name)
        .ok_or_else(|| CodecError::new(format!("payload lacks {name:?}")))
}

fn count(value: &Value) -> Result<i64, CodecError> {
    only_member(value, "Count")?
        .as_i64()
        .ok_or_else(|| CodecError::new("Count is not an integer"))
}

impl OmsJsonCodec<runtime_test::model::SharedPayload> for TestCodec {
    fn encode_payload(
        &self,
        value: &runtime_test::model::SharedPayload,
    ) -> Result<Value, CodecError> {
        Ok(json!({ "Count": value.count.get() }))
    }

    fn decode_payload(
        &self,
        value: &Value,
    ) -> Result<runtime_test::model::SharedPayload, CodecError> {
        let count = runtime_test::model::BoundedI64::new(count(value)?)
            .ok_or_else(|| CodecError::new("Count is outside xs:int"))?;
        Ok(runtime_test::model::SharedPayload { count })
    }
}

impl OmsJsonCodec<runtime_test::model::OtherPayload> for TestCodec {
    fn encode_payload(
        &self,
        value: &runtime_test::model::OtherPayload,
    ) -> Result<Value, CodecError> {
        Ok(json!({ "Label": value.label }))
    }

    fn decode_payload(
        &self,
        value: &Value,
    ) -> Result<runtime_test::model::OtherPayload, CodecError> {
        let label = only_member(value, "Label")?
            .as_str()
            .ok_or_else(|| CodecError::new("Label is not a string"))?;
        Ok(runtime_test::model::OtherPayload {
            label: label.to_owned(),
        })
    }
}

impl OmsJsonCodec<runtime_oam::model::SharedPayload> for TestCodec {
    fn encode_payload(
        &self,
        value: &runtime_oam::model::SharedPayload,
    ) -> Result<Value, CodecError> {
        Ok(json!({ "Count": value.count.get() }))
    }

    fn decode_payload(
        &self,
        value: &Value,
    ) -> Result<runtime_oam::model::SharedPayload, CodecError> {
        let count = runtime_oam::model::BoundedI64::new(count(value)?)
            .ok_or_else(|| CodecError::new("Count is outside xs:int"))?;
        Ok(runtime_oam::model::SharedPayload { count })
    }
}

/// A `runtime-test` `SharedPayload` holding `count`.
#[must_use]
pub fn shared(count: i64) -> runtime_test::model::SharedPayload {
    runtime_test::model::SharedPayload {
        count: runtime_test::model::BoundedI64::new(count).expect("xs:int"),
    }
}

/// A `runtime-oam` `SharedPayload` holding `count`.
#[must_use]
pub fn oam_shared(count: i64) -> runtime_oam::model::SharedPayload {
    runtime_oam::model::SharedPayload {
        count: runtime_oam::model::BoundedI64::new(count).expect("xs:int"),
    }
}
