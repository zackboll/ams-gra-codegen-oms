//! Shared Task 050 sample value for the synthetic `codec-oam` fixture.
#![allow(dead_code)]

pub use ams_gra_oms_runtime_rust_facade_tests::codec_oam::model as m;
use serde_json::{Value, json};

pub fn label(text: &str) -> m::Label {
    m::Label::new(text).expect("label")
}

fn circle(tag: &str, radius: f64) -> m::ShapeBase {
    m::ShapeBase::CircleShape(m::CircleShape {
        tag: tag.to_owned(),
        radius: m::Measure::new(radius),
    })
}

/// A value touching every supported construct.
pub fn sample() -> m::CodecPayload {
    m::CodecPayload {
        enabled: true,
        level: m::Level::new(-10).expect("level"),
        percent: m::Percent::new(100).expect("percent"),
        offset: m::BoundedI64::new(i64::MIN).expect("offset"),
        count: m::BoundedU64::new(65535).expect("count"),
        heading: m::Angle::new(-180.0).expect("heading"),
        gain: m::Ratio::new(0.1),
        reading: f64::NEG_INFINITY,
        note: "plain \"text\"".to_owned(),
        callsign: label("ALPHA 1"),
        identity: m::VersionedIdentity {
            uuid: m::UniversallyUniqueIdentifierType::new("550e8400-e29b-41d4-a716-446655440000")
                .expect("uuid"),
            name: Some(label("unit")),
            version: Some(m::BoundedU64::new(4_294_967_295).expect("version")),
        },
        created: m::Instant::new("2026-01-01T00:00:00Z").expect("instant"),
        observed: m::XmlSchemaDateTime::new("2026-01-01T05:30:00+05:30").expect("dateTime"),
        signal: m::SignalCode::Value5G,
        comment: None,
        tags: m::BoundedVec::new(vec![]).expect("empty 0..3"),
        signals: m::BoundedVec::new(vec![m::SignalCode::SOMEVALUE, m::SignalCode::ValueSelf])
            .expect("2..4"),
        history: m::UnboundedVec::new(vec![
            m::Measure::new(0.0),
            m::Measure::new(-0.0),
            m::Measure::new(f64::NAN),
            m::Measure::new(f64::INFINITY),
            m::Measure::new(2.5),
        ])
        .expect("unbounded"),
        source: m::SourceChoice::Levels(
            m::BoundedVec::new(vec![
                m::Level::new(1).expect("1"),
                m::Level::new(10).expect("10"),
            ])
            .expect("1..3"),
        ),
        shape: m::ShapeBase::BoxShape(m::BoxShape {
            tag: "box".to_owned(),
            width: 1.5,
            height: -2.0,
        }),
        shapes: m::UnboundedVec::new(vec![circle("c", 3.0)]).expect("1.."),
    }
}

/// The exact OMS JSON body `sample()` must encode to.
pub fn sample_json() -> Value {
    json!({
        "Enabled": true,
        "Level": -10,
        "Percent": 100,
        "Offset": i64::MIN,
        "Count": 65535,
        "Heading": -180.0,
        "Gain": 0.1,
        "Reading": "-Infinity",
        "Note": "plain \"text\"",
        "Callsign": "ALPHA 1",
        "Identity": {
            "UUID": "550e8400-e29b-41d4-a716-446655440000",
            "Name": "unit",
            "Version": 4_294_967_295_u64
        },
        "Created": "2026-01-01T00:00:00Z",
        "Observed": "2026-01-01T05:30:00+05:30",
        "Signal": "5G",
        "Signals": ["SOME_VALUE", "Self"],
        "History": [0.0, -0.0, "NaN", "Infinity", 2.5],
        "Source": { "Levels": [1, 10] },
        "Shape": { "$type": "BoxShape", "Tag": "box", "Width": 1.5, "Height": -2.0 },
        "Shapes": [ { "$type": "CircleShape", "Tag": "c", "Radius": 3.0 } ]
    })
}
