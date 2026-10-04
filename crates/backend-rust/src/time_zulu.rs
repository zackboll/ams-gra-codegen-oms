//! Named Time uses only carrier-private helpers, with no generated global name.
pub(super) const CARRIER: &str = r#"#[derive(Clone, Debug)]
pub struct {name} {
    lexical: String,
}

impl {name} {
    /// Collapse XML whitespace, validate XSD 1.0 Time and require literal Z.
    /// Preserve the collapsed spelling, including hour 24 and fractional zeros.
    pub fn new(value: &str) -> Option<Self> {
        let lexical = value.split([' ', '\t', '\n', '\r'])
            .filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ");
        Self::valid(&lexical).then_some(Self { lexical })
    }
    pub fn as_str(&self) -> &str { &self.lexical }

    fn valid(text: &str) -> bool {
        let bytes = text.as_bytes();
        if bytes.len() < 9 || bytes.last() != Some(&b'Z') { return false; }
        let body = &bytes[..bytes.len() - 1];
        if body[2] != b':' || body[5] != b':' { return false; }
        let (Some(hour), Some(minute), Some(second)) = (
            Self::two(&body[..2]), Self::two(&body[3..5]), Self::two(&body[6..8]),
        ) else { return false; };
        // XSD 1.0 Appendix D admits second 60, including fractional seconds.
        if hour > 24 || minute > 59 || second > 60 { return false; }
        let fraction = &body[8..];
        let zero = if fraction.is_empty() { true } else {
            if fraction.len() < 2 || fraction[0] != b'.' { return false; }
            if !fraction[1..].iter().all(u8::is_ascii_digit) { return false; }
            fraction[1..].iter().all(|digit| *digit == b'0')
        };
        hour != 24 || (minute == 0 && second == 0 && zero)
    }
    fn two(bytes: &[u8]) -> Option<u8> {
        bytes.iter().all(u8::is_ascii_digit)
            .then(|| (bytes[0] - b'0') * 10 + bytes[1] - b'0')
    }
}
"#;
