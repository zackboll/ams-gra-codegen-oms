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

/// The repository-root path of the shared corpus.
pub fn corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/temporal/datetime-zulu.txt")
}

/// Parse the shared corpus.
///
/// # Panics
///
/// Panics when the corpus is missing or a line is malformed: a silently
/// skipped case would weaken every backend's evidence at once.
pub fn load_cases() -> Vec<TemporalCase> {
    let path = corpus_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("shared temporal corpus {}: {error}", path.display()));
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
                input: unescape(input),
                expected: Some(unescape(expected)),
            });
        } else if line == "INVALID" {
            // A payload-free INVALID line is the empty string case.
            cases.push(TemporalCase {
                input: String::new(),
                expected: None,
            });
        } else if let Some(rest) = line.strip_prefix("INVALID ") {
            cases.push(TemporalCase {
                input: unescape(rest),
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

/// Expand the corpus's small escape vocabulary.
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
            other => panic!("unsupported corpus escape \\{other:?}"),
        }
    }
    out
}

/// Render a string as a source-level literal body for a generated probe.
///
/// Shared by all three backends so one escaping bug cannot make one language
/// silently test a different string than the others.
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
