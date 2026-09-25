//! Task 041: the shared corpus loader's empty-field encoding.
//!
//! The loader is read by all three backends' generated-code probes, so a parsing
//! change must be proven backward compatible rather than assumed. These tests
//! exercise the loader directly, through a temporary corpus file, so they do not
//! depend on any particular backend.

mod common;

use std::io::Write as _;

/// Write a temporary corpus and load it.
fn load(body: &str, label: &str) -> Vec<common::TemporalCase> {
    let path = std::env::temp_dir().join(format!("ams-gra-oms-task041-corpus-{label}.txt"));
    let mut file = std::fs::File::create(&path).expect("create temporary corpus");
    file.write_all(body.as_bytes()).expect("write corpus");
    drop(file);
    let cases = common::load_corpus(&path);
    std::fs::remove_file(&path).expect("remove temporary corpus");
    cases
}

/// `<empty>` expresses all four empty-value situations the feature needs.
///
/// `Some("")` is successful construction of an empty String value and `None` is
/// rejection; the token only changes how an empty FIELD is written, never what
/// the outcome means.
#[test]
fn the_empty_field_token_expresses_every_empty_value_situation() {
    let cases = load(
        "# comment\n\
         VALID <empty> => <empty>\n\
         VALID \\s\\s\\s => <empty>\n\
         VALID <empty> => x\n\
         INVALID <empty>\n",
        "empty-vocabulary",
    );
    assert_eq!(cases.len(), 4);

    // 1. valid empty INPUT with a valid empty stored OUTPUT.
    assert_eq!(cases[0].input, "");
    assert_eq!(cases[0].expected.as_deref(), Some(""));

    // 2. whitespace-only input NORMALIZING to empty: a non-empty input whose
    //    expected stored value is empty. This is the collapse profile's case.
    assert_eq!(cases[1].input, "   ");
    assert_eq!(cases[1].expected.as_deref(), Some(""));

    // 3. an empty input accepted with a NON-empty stored value, so the token is
    //    independently usable on either side of the delimiter.
    assert_eq!(cases[2].input, "");
    assert_eq!(cases[2].expected.as_deref(), Some("x"));

    // 4. invalid empty input, which is what a minLength = 1 profile requires.
    assert_eq!(cases[3].input, "");
    assert_eq!(cases[3].expected, None);
}

/// `Some("")` and `None` remain distinguishable, and neither is a NUL.
///
/// An empty stored value is not missing data and not a NUL character. Confusing
/// them would make the collapse corpus silently agree with a validator that
/// rejected everything.
#[test]
fn an_empty_value_is_neither_a_rejection_nor_a_nul() {
    let cases = load(
        "VALID <empty> => <empty>\n\
         INVALID <empty>\n\
         INVALID \\u{0}\n",
        "empty-vs-nul",
    );
    assert_eq!(cases.len(), 3);
    // Accepted-empty and rejected-empty have the SAME input and DIFFERENT
    // outcomes, which is exactly the distinction a trailing-space encoding lost.
    assert_eq!(cases[0].input, cases[1].input);
    assert_ne!(cases[0].expected, cases[1].expected);
    // A NUL input is one character long, not zero.
    assert_eq!(cases[2].input, "\u{0}");
    assert_eq!(cases[2].input.len(), 1);
    assert!(!cases[2].input.is_empty());
}

/// The token is recognized only as a WHOLE field.
///
/// `<empty>` inside a longer field is ordinary text, so a corpus can still test
/// those literal characters.
#[test]
fn the_empty_field_token_is_only_recognized_as_a_whole_field() {
    // The loader requires both a valid and an invalid case, so one of each.
    let cases = load(
        "VALID a<empty>b => a<empty>b\n\
         VALID <empty>x => <empty>x\n\
         INVALID <empty>y\n",
        "token-scope",
    );
    assert_eq!(cases.len(), 3);
    // The INVALID payload is likewise ordinary text, not the empty input.
    assert_eq!(cases[2].input, "<empty>y");
    assert_eq!(cases[2].expected, None);
    assert_eq!(cases[0].input, "a<empty>b");
    assert_eq!(cases[0].expected.as_deref(), Some("a<empty>b"));
    assert_eq!(cases[1].input, "<empty>x");
    assert_eq!(cases[1].expected.as_deref(), Some("<empty>x"));
}

/// Every EXISTING escape keeps its exact meaning.
///
/// The change is additive: the older corpora must parse to precisely what they
/// parsed to before, so this pins the whole pre-existing vocabulary.
#[test]
fn the_existing_escape_vocabulary_is_unchanged() {
    let cases = load(
        "VALID \\s => \\s\n\
         VALID \\t => \\t\n\
         VALID \\n => \\n\n\
         VALID \\r => \\r\n\
         VALID \\\\ => \\\\\n\
         VALID \\u{7f} => \\u{7f}\n\
         INVALID\n\
         INVALID x\n",
        "legacy-escapes",
    );
    assert_eq!(cases.len(), 8);
    for (index, expected) in [" ", "\t", "\n", "\r", "\\", "\u{7f}"].iter().enumerate() {
        assert_eq!(cases[index].input, *expected, "case {index} input");
        assert_eq!(
            cases[index].expected.as_deref(),
            Some(*expected),
            "case {index} stored"
        );
    }
    // A payload-free `INVALID` line still means the empty string, which is the
    // pre-existing spelling and is NOT replaced by `<empty>`.
    assert_eq!(cases[6].input, "");
    assert_eq!(cases[6].expected, None);
    assert_eq!(cases[7].input, "x");
    assert_eq!(cases[7].expected, None);
}

/// Malformed cases still FAIL rather than being skipped.
///
/// A silently skipped case would weaken every backend's evidence at once, so the
/// loader panics. `<empty>` must not have created a new way to slip past that:
/// a `VALID` line with no delimiter is still malformed even when its single field
/// is the token.
#[test]
fn malformed_corpus_lines_still_panic() {
    for (label, body) in [
        ("VALID without a delimiter", "VALID abc\n"),
        ("VALID token without a delimiter", "VALID <empty>\n"),
        ("an unknown keyword", "MAYBE abc => abc\n"),
        ("a bare payload", "abc => abc\n"),
        ("an unsupported escape", "VALID \\q => \\q\n"),
        ("an unterminated \\u escape", "VALID \\u{41 => A\n"),
        ("a \\u escape with no brace", "VALID \\u41 => A\n"),
        // A corpus with no valid cases, and one with no invalid cases, are both
        // rejected: each would silently halve a backend's evidence.
        ("no valid cases", "INVALID x\n"),
        ("no invalid cases", "VALID x => x\n"),
    ] {
        let outcome = std::panic::catch_unwind(|| load(body, "malformed"));
        assert!(outcome.is_err(), "{label} must be rejected, not skipped");
    }
}

/// Every shipped corpus still loads, and the new ones exercise the new encoding.
///
/// Reading the real files keeps this honest: a loader change that broke an
/// existing corpus would fail here rather than in one backend only.
#[test]
fn every_shipped_corpus_still_loads() {
    for (label, path) in [
        ("temporal", common::corpus_path()),
        ("schema version", common::string_corpus_path()),
        ("uuid", common::uuid_corpus_path()),
        ("visible ascii", common::visible_ascii_corpus_path()),
        (
            "whitespace-visible collapse",
            common::whitespace_visible_collapse_corpus_path(),
        ),
        (
            "whitespace-visible preserve",
            common::whitespace_visible_preserve_corpus_path(),
        ),
    ] {
        let cases = common::load_corpus(&path);
        assert!(!cases.is_empty(), "{label} corpus must load cases");
        assert!(
            cases.iter().any(|case| case.expected.is_some()),
            "{label} needs valid cases"
        );
        assert!(
            cases.iter().any(|case| case.expected.is_none()),
            "{label} needs invalid cases"
        );
    }

    // The collapse corpus must actually EXERCISE the new encoding, otherwise the
    // token would be untested in practice.
    let collapse = common::load_corpus(&common::whitespace_visible_collapse_corpus_path());
    assert!(
        collapse
            .iter()
            .any(|case| case.input.is_empty() && case.expected.as_deref() == Some("")),
        "the collapse corpus must contain a valid empty input"
    );
    assert!(
        collapse
            .iter()
            .any(|case| !case.input.is_empty() && case.expected.as_deref() == Some("")),
        "the collapse corpus must contain whitespace-only input normalizing to empty"
    );
    // And the preserve corpus must contain the empty input as a REJECTION, which
    // is the same written input with the opposite outcome.
    let preserve = common::load_corpus(&common::whitespace_visible_preserve_corpus_path());
    assert!(
        preserve
            .iter()
            .any(|case| case.input.is_empty() && case.expected.is_none()),
        "the preserve corpus must reject the empty input"
    );
}
