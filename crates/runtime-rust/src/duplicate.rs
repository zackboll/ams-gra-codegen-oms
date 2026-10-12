//! Structural first pass; Value construction remains the unchanged second pass.
use crate::MessageDecodeError;
use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use std::{collections::HashSet, fmt};

struct Check<'a>(&'a mut bool);

impl<'de> DeserializeSeed<'de> for Check<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Check<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_bool<E: Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while seq.next_element_seed(Check(self.0))?.is_some() {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        // Only keys of currently open objects remain live. No subtree Values
        // or raw payload copies are retained; serde_json retains its depth limit.
        let mut names = HashSet::new();
        while let Some(name) = map.next_key::<String>()? {
            if !names.insert(name) {
                *self.0 = true;
                return Err(A::Error::custom("duplicate object member"));
            }
            map.next_value_seed(Check(self.0))?;
        }
        Ok(())
    }
}

pub(crate) fn check(raw: &str) -> Result<(), MessageDecodeError> {
    let mut duplicate = false;
    let mut parser = serde_json::Deserializer::from_str(raw);
    let result = Check(&mut duplicate).deserialize(&mut parser);
    if duplicate {
        return Err(MessageDecodeError::DuplicateMember);
    }
    result.map_err(MessageDecodeError::InvalidJson)?;
    parser.end().map_err(MessageDecodeError::InvalidJson)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DuplicateMemberPolicy, envelope::unwrap_with_policy, unwrap_global_element};

    #[test]
    fn task077_duplicate_names() {
        for raw in [
            r#"{"M":{},"M":{}}"#,
            r#"{"M":{"x":1,"x":2}}"#,
            r#"{"M":{"x":1,"x":1}}"#,
            r#"{"M":{"$type":"a","\u0024type":"b"}}"#,
            r#"{"M":{"x":1,"\u0078":2}}"#,
            r#"{"M":{"unknown":{"nested":{"x":1,"x":2}}}}"#,
            r#"{"M":[{"x":1,"x":2}]}"#,
            include_str!("../../../tests/fixtures/task076/duplicate.json"),
            include_str!("../../../tests/fixtures/task076/duplicate-escaped.json"),
        ] {
            assert!(
                matches!(check(raw), Err(MessageDecodeError::DuplicateMember)),
                "{raw}"
            );
        }
        println!("TASK077 DUPLICATE NAMES: PASSED");
    }

    #[test]
    fn task077_value_parity() {
        for body in [
            r#"{"a":{"x":1},"b":{"x":2},"items":[{"x":1},{"x":2},1,1]}"#,
            r#"{"x":1,"X":2,"é":3,"e\u0301":4,"empty":{},"array":[]}"#,
            r#"[-9223372036854775808,9223372036854775807,18446744073709551615,9007199254740993,-9223372036854775809,18446744073709551616,-0,-0.0,0.25,1e3,1E-3,1.7976931348623157e308,5e-324,true,false,null,"雪\n\u0024\"\\"]"#,
        ] {
            let raw = format!("{{\"M\":{body}}}");
            let old = unwrap_global_element("M", &raw).unwrap();
            let strict = unwrap_with_policy("M", &raw, DuplicateMemberPolicy::Reject).unwrap();
            assert_eq!(old, strict);
            assert_eq!(
                serde_json::to_string(&old).unwrap(),
                serde_json::to_string(&strict).unwrap()
            );
        }
        println!("TASK077 VALUE PARITY: PASSED");
    }

    #[test]
    fn task077_error_precedence() {
        for raw in ["{", "true false", r#"{"M":01}"#, r#"{"M":{,}}"#] {
            assert!(matches!(
                check(raw),
                Err(MessageDecodeError::InvalidJson(_))
            ));
        }
        let deep = format!("{}0{}", "[".repeat(256), "]".repeat(256));
        assert!(matches!(
            check(&deep),
            Err(MessageDecodeError::InvalidJson(_))
        ));
        // A duplicate observed first wins even over malformed remainder or wrong envelope.
        for raw in [
            r#"{"wrong":{"x":1,"x":}}"#,
            r#"{"M":{"x":1,"x":2}} trailing"#,
        ] {
            assert!(matches!(
                unwrap_with_policy("M", raw, DuplicateMemberPolicy::Reject),
                Err(MessageDecodeError::DuplicateMember)
            ));
        }
        assert!(matches!(
            check(r#"{"M":{"x":,"x":2}}"#),
            Err(MessageDecodeError::InvalidJson(_))
        ));
        assert!(matches!(
            unwrap_with_policy("M", r#"{"wrong":{}}"#, DuplicateMemberPolicy::Reject),
            Err(MessageDecodeError::UnexpectedMember { .. })
        ));
        assert!(matches!(
            unwrap_with_policy("M", r#"{"M":{},"other":{}}"#, DuplicateMemberPolicy::Reject),
            Err(MessageDecodeError::MemberCount { found: 2 })
        ));
        assert!(matches!(
            unwrap_with_policy("M", "[]", DuplicateMemberPolicy::Reject),
            Err(MessageDecodeError::NotAnObject)
        ));
        println!("TASK077 ERROR PRECEDENCE: PASSED");
    }
}
