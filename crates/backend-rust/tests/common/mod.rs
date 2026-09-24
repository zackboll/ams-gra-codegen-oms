//! Shared Task 036 DateTime Zulu conformance corpus loader.
//!
//! ONE corpus file drives the Rust, C++, and Ada generated validators. Each
//! backend's runtime test reads these same cases and emits them into its probe
//! program, so the three languages are proven to agree rather than each being
//! checked against its own curated subset.

use std::path::{Path, PathBuf};

/// One corpus case: an input and its expected outcome.
pub struct TemporalCase {
    /// The raw constructor input, after un-escaping the corpus file's escapes.
    pub input: String,
    /// `Some(normalized)` when the value is valid; `None` when it must be
    /// rejected.
    pub expected: Option<String>,
}

/// The repository-root path of the shared Task 036 temporal corpus.
#[allow(dead_code)]
pub fn corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/temporal/datetime-zulu.txt")
}

/// The repository-root path of the shared Task 037 String-profile corpus.
#[allow(dead_code)]
pub fn string_corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/string/schema-version.txt")
}

/// The repository-root path of the shared Task 038 UUID corpus.
#[allow(dead_code)]
pub fn uuid_corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/string/uuid.txt")
}

/// The repository-root path of the shared Task 039 visible-ASCII corpus.
#[allow(dead_code)]
pub fn visible_ascii_corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/string/visible-ascii.txt")
}

/// The repository-root path of the shared Task 041 collapse corpus.
#[allow(dead_code)]
pub fn whitespace_visible_collapse_corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/string/whitespace-visible-collapse.txt")
}

/// The repository-root path of the shared Task 041 preserve corpus.
///
/// Separate from the collapse corpus on purpose: the two policies have
/// different expected stored values for the same inputs.
#[allow(dead_code)]
pub fn whitespace_visible_preserve_corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/string/whitespace-visible-preserve.txt")
}

/// Parse the shared corpus.
///
/// # Panics
///
/// Panics when the corpus is missing or a line is malformed: a silently
/// skipped case would weaken every backend's evidence at once.
#[allow(dead_code)]
pub fn load_cases() -> Vec<TemporalCase> {
    load_corpus(&corpus_path())
}

/// Parse any corpus file sharing the Task 036 format.
///
/// Task 037 reuses the exact same format and loader, so one escaping or
/// parsing bug cannot make the String corpus disagree with the temporal one.
///
/// # Panics
///
/// Panics when the corpus is missing or a line is malformed.
#[allow(dead_code)]
pub fn load_corpus(path: &Path) -> Vec<TemporalCase> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("shared corpus {}: {error}", path.display()));
    let mut cases = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("VALID ") {
            let (input, expected) = rest
                .split_once(" => ")
                .unwrap_or_else(|| panic!("VALID case needs ' => ': {line}"));
            cases.push(TemporalCase {
                input: unescape_field(input),
                expected: Some(unescape_field(expected)),
            });
        } else if line == "INVALID" {
            // A payload-free INVALID line is the empty string case.
            cases.push(TemporalCase {
                input: String::new(),
                expected: None,
            });
        } else if let Some(rest) = line.strip_prefix("INVALID ") {
            cases.push(TemporalCase {
                input: unescape_field(rest),
                expected: None,
            });
        } else {
            panic!("malformed corpus line: {line}");
        }
    }
    assert!(
        cases.iter().any(|case| case.expected.is_some()),
        "corpus must contain valid cases"
    );
    assert!(
        cases.iter().any(|case| case.expected.is_none()),
        "corpus must contain invalid cases"
    );
    cases
}

/// The explicit empty-field token.
///
/// # Why an explicit token is required
///
/// `load_corpus` calls `line.trim_end()` before splitting a `VALID` case on
/// `" => "`. That is deliberate text hygiene -- it keeps an editor's trailing
/// whitespace out of every case -- but it makes an *empty* field on either side
/// unrepresentable by ordinary means:
///
/// * `VALID  => ` would have its trailing space trimmed, leaving `VALID  =>`,
///   whose `" => "` delimiter is now incomplete, so the case fails to parse;
/// * padding with trailing spaces to compensate is exactly what `trim_end` and
///   any repository text-hygiene check will remove again.
///
/// Task 041's whitespace-visible collapse profiles make empty fields
/// load-bearing rather than hypothetical: `minLength = 0` accepts the empty
/// string, and whitespace-only input *normalizes to* the empty string, so both
/// an empty input and an empty expected output must be expressible.
///
/// # Compatibility
///
/// This token is a new spelling, not a changed one. No existing corpus line
/// contains it, so every existing case parses to exactly the value it parsed to
/// before. `Some("")` still means "construction succeeded and the stored value
/// is the empty String", and `None` still means "rejected" -- the token affects
/// only how an empty field is *written*, never what it means.
///
/// It is deliberately not `""`: a literal pair of quotation marks is itself a
/// valid two-character input for these profiles, and would be ambiguous.
const EMPTY_FIELD: &str = "<empty>";

/// Expand one corpus field, honouring the explicit empty-field token.
///
/// The token is recognized only as the *entire* field. `<empty>` appearing
/// inside a longer field is ordinary text, so a case can still test the literal
/// characters `<empty>`.
fn unescape_field(field: &str) -> String {
    if field == EMPTY_FIELD {
        return String::new();
    }
    unescape(field)
}

/// Expand the corpus's small escape vocabulary.
///
/// `\u{HEX}` is the general form, added for Task 039: that profile's corpus
/// must pin arbitrary code points either side of the `[ -~]` interval --
/// U+0000, U+001F, U+007F, and a range of non-ASCII characters -- none of
/// which can appear literally in a text file case without ambiguity.
fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some('u') => {
                assert_eq!(chars.next(), Some('{'), "\\u escape needs a brace: {text}");
                let mut digits = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(digit) => digits.push(digit),
                        None => panic!("unterminated \\u escape: {text}"),
                    }
                }
                let code = u32::from_str_radix(&digits, 16)
                    .unwrap_or_else(|error| panic!("bad \\u{{{digits}}}: {error}"));
                out.push(
                    char::from_u32(code)
                        .unwrap_or_else(|| panic!("\\u{{{digits}}} is not a code point")),
                );
            }
            other => panic!("unsupported corpus escape \\{other:?}"),
        }
    }
    out
}

/// Render a string as a source-level literal body for a generated probe.
///
/// Shared by all three backends so one escaping bug cannot make one language
/// silently test a different string than the others.
#[allow(dead_code)]
pub fn escape_for_source(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

/// Render a string as a **Rust** string literal body, controls included.
///
/// [`escape_for_source`] covers the vocabulary Tasks 036--038 need and is left
/// exactly as those tasks proved it. Task 039 additionally pins NUL, U+001F,
/// and U+007F, none of which that function escapes, so this superset is used
/// by the Rust visible-ASCII probe. `\u{...}` is a Rust-only spelling.
#[allow(dead_code)]
pub fn escape_for_rust_source(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other if is_visible_ascii(other) => out.push(other),
            other => out.push_str(&format!("\\u{{{:x}}}", other as u32)),
        }
    }
    out
}

/// Render a string as a **C++** narrow string literal body, controls included.
///
/// C++ has no `\u{...}` form, and a `\uXXXX` naming a basic-source character is
/// ill-formed, so every character outside the printable-ASCII range is emitted
/// as octal escapes of its individual UTF-8 bytes. Octal escapes consume at
/// most three digits, so an escape can never absorb a following digit of the
/// string under test -- the failure mode a hexadecimal spelling would have.
#[allow(dead_code)]
pub fn escape_for_cpp_source(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            other if is_visible_ascii(other) => out.push(other),
            other => {
                let mut buffer = [0_u8; 4];
                for byte in other.encode_utf8(&mut buffer).as_bytes() {
                    out.push_str(&format!("\\{byte:03o}"));
                }
            }
        }
    }
    out
}

/// The `[ -~]` interval, used only to decide what a probe may emit literally.
///
/// This is deliberately *not* the validator under test: it decides source
/// encoding, and every corpus case is still checked against the generated
/// carrier's own answer.
#[allow(dead_code)]
fn is_visible_ascii(character: char) -> bool {
    matches!(character, ' '..='~')
}

/// Render a string as an Ada `String` expression.
///
/// Ada string literals cannot carry control characters, and the generated
/// carriers take `String`, whose component is Latin-1 `Character`. Controls and
/// Latin-1 characters are spliced in as `Character'Val` constants; any code
/// point above U+00FF is emitted as its individual UTF-8 bytes, which is
/// exactly the byte sequence the Rust and C++ probes receive.
#[allow(dead_code)]
pub fn ada_literal(text: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut literal = String::new();
    for character in text.chars() {
        match character {
            // Printable ASCII, minus the quotation mark Ada doubles.
            '"' => literal.push_str("\"\""),
            other if other.is_ascii() && (other as u32) >= 0x20 && (other as u32) != 0x7F => {
                literal.push(other);
            }
            other => {
                if !literal.is_empty() {
                    parts.push(format!("\"{literal}\""));
                    literal.clear();
                }
                // `(1 => X)` is a positional String aggregate, so every part
                // of the concatenation is String-typed. A bare Character'Val
                // would be a Character and would not match `Create`'s profile.
                if (other as u32) <= 0xFF {
                    parts.push(format!("(1 => Character'Val ({}))", other as u32));
                } else {
                    let mut buffer = [0_u8; 4];
                    for byte in other.encode_utf8(&mut buffer).as_bytes() {
                        parts.push(format!("(1 => Character'Val ({byte}))"));
                    }
                }
            }
        }
    }
    if !literal.is_empty() || parts.is_empty() {
        parts.push(format!("\"{literal}\""));
    }
    parts.join(" & ")
}
