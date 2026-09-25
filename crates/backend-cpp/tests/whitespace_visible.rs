//! Task 041: the generated C++17 whitespace-visible carriers, compiled and run.
//!
//! Every probe is built under `-std=c++17 -Wall -Wextra -pedantic-errors`, and
//! every corpus case is passed with an EXPLICIT length so an embedded NUL case
//! cannot be silently truncated into a shorter, accidentally-valid input.

mod common;

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

fn whitespace_visible_schema() -> ams_gra_oms_ir::SchemaIr {
    load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../xsd-frontend/tests/fixtures/backend-string-whitespace-visible.xsd"),
    )
    .expect("whitespace-visible fixture should parse")
}

/// The generated API shape, asserted before anything is run.
#[test]
fn generated_whitespace_visible_api_is_a_checked_factory_carrier() {
    let source = generate(
        &whitespace_visible_schema(),
        GenerationWorld::ClosedSchemaSet,
    )
    .expect("fixture must generate");

    // A checked factory plus read-only access, with private storage.
    for name in [
        "CollapsedRemarks",
        "CollapsedNarrative",
        "PreservedRemarks",
        "PreservedNarrative",
        "PreservedQuery",
    ] {
        assert!(
            source.contains(&format!(
                "static std::optional<{name}> create(std::string_view value)"
            )),
            "{name} must expose a checked factory"
        );
        // Task 040 special-member policy: the copy operations are DECLARED,
        // which suppresses the implicit destructive move operations.
        assert!(
            source.contains(&format!("{name}(const {name}&) = default;")),
            "{name} must declare its copy constructor"
        );
        assert!(
            source.contains(&format!("{name}& operator=(const {name}&) = default;")),
            "{name} must declare its copy assignment"
        );
        // ...and no move operation is reintroduced.
        assert!(
            !source.contains(&format!("{name}({name}&&)")),
            "{name} must not declare a move constructor"
        );
    }
    assert!(source.contains("const std::string& value() const noexcept"));
    assert!(source.contains("std::string value_;"));

    // Per-declaration bounds, over whole evidenced triples.
    assert!(source.contains("static constexpr std::size_t kMinLength = 0;"));
    assert!(source.contains("static constexpr std::size_t kMinLength = 1;"));
    assert!(source.contains("static constexpr std::size_t kMaxLength = 1024;"));
    assert!(source.contains("static constexpr std::size_t kMaxLength = 4096;"));

    // The class is the ordinal interval PLUS the two escaped characters.
    assert!(source.contains("static constexpr unsigned char kMinCodePoint = 0x20;"));
    assert!(source.contains("static constexpr unsigned char kMaxCodePoint = 0x7E;"));
    assert!(source.contains("static constexpr unsigned char kLineFeed = 0x0A;"));
    assert!(source.contains("static constexpr unsigned char kCarriageReturn = 0x0D;"));

    // High-bit bytes: the comparison is made on `unsigned char`, never on a
    // possibly-signed plain `char`.
    assert!(source.contains("static_cast<unsigned char>(character)"));

    // The collapse half gets the normalization helpers; the fixture has exactly
    // three collapse carriers. A fourth would mean a preserve carrier acquired
    // them, which would also be an unused static function under -Wall -Wextra.
    assert_eq!(
        source
            .matches("static std::string collapse(std::string_view text)")
            .count(),
        3
    );

    // Documentation distinguishes the two policies.
    assert!(source.contains("the collapse-normalized form of the input, not the input itself"));
    assert!(source.contains("the caller's text preserved unchanged"));

    // No regex engine, and nothing locale-sensitive. `std::isspace` is
    // specifically forbidden: it is locale-dependent, it reports U+000B and
    // U+000C, and passing a negative `char` to it is undefined.
    for forbidden in [
        "<regex>",
        "std::regex",
        // As CALLS: the bare words appear in the carrier's own documentation,
        // which records that these classifiers are deliberately NOT used.
        "isspace(",
        "isprint(",
        "isgraph(",
        "std::locale",
        // The header itself must not be included. Checked as a directive so the
        // carrier's own documentation may still name it as a non-dependency.
        "#include <cctype>",
    ] {
        assert!(
            !source.contains(forbidden),
            "generated C++ must not reference {forbidden}"
        );
    }
}

/// Both shared corpora, executed against a probe built under strict C++17.
#[test]
fn generated_whitespace_visible_validators_match_the_shared_corpora() {
    let source = generate(
        &whitespace_visible_schema(),
        GenerationWorld::ClosedSchemaSet,
    )
    .expect("fixture must generate");

    let mut probe = String::from(
        "#include \"generated.hpp\"\n#include <cstdio>\n#include <string>\n\
         #include <utility>\n\n\
         using test::remarks::CollapsedNarrative;\n\
         using test::remarks::CollapsedRemarks;\n\
         using test::remarks::DerivedCollapsedRemarks;\n\
         using test::remarks::PreservedNarrative;\n\
         using test::remarks::PreservedQuery;\n\
         using test::remarks::PreservedRemarks;\n\nint main() {\n",
    );
    for (carrier, path) in [
        (
            "CollapsedRemarks",
            common::whitespace_visible_collapse_corpus_path(),
        ),
        (
            "PreservedRemarks",
            common::whitespace_visible_preserve_corpus_path(),
        ),
    ] {
        let cases = common::load_corpus(&path);
        assert!(!cases.is_empty(), "{carrier} corpus must not be empty");
        for (index, case) in cases.iter().enumerate() {
            // Cases are reported by carrier plus corpus index, never by
            // interpolating their own text into a diagnostic, so a braced case
            // cannot disturb the probe.
            let input = common::escape_for_cpp_source(&case.input);
            // The length is passed explicitly: a corpus case may contain an
            // embedded NUL, which a bare string literal would truncate into a
            // shorter and possibly VALID input.
            let bytes = case.input.len();
            match &case.expected {
                Some(stored) => {
                    let stored_escaped = common::escape_for_cpp_source(stored);
                    let stored_bytes = stored.len();
                    writeln!(
                        probe,
                        "    {{\n\
                         \x20       const std::string input(\"{input}\", {bytes});\n\
                         \x20       const std::string want(\"{stored_escaped}\", {stored_bytes});\n\
                         \x20       auto made = {carrier}::create(input);\n\
                         \x20       if (!made) {{ std::puts(\"{carrier} must accept case {index}\"); return 1; }}\n\
                         \x20       if (made->value() != want) {{ std::puts(\"{carrier} bad storage case {index}\"); return 1; }}\n\
                         \x20       auto again = {carrier}::create(want);\n\
                         \x20       if (!again || again->value() != want) {{ std::puts(\"{carrier} not idempotent case {index}\"); return 1; }}\n\
                         \x20   }}"
                    )
                    .expect("writing to String cannot fail");
                }
                None => {
                    writeln!(
                        probe,
                        "    if ({carrier}::create(std::string(\"{input}\", {bytes}))) {{ std::puts(\"{carrier} must reject case {index}\"); return 1; }}"
                    )
                    .expect("writing to String cannot fail");
                }
            }
        }
    }
    probe.push_str(&semantics_probe());
    probe.push_str("    std::puts(\"ok\");\n    return 0;\n}\n");

    let directory = std::env::temp_dir().join("ams-gra-oms-task041-cpp-whitespace-visible");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create C++ probe directory");
    std::fs::write(directory.join("generated.hpp"), &source).expect("write generated header");
    std::fs::write(directory.join("probe.cpp"), &probe).expect("write probe");

    let output = Command::new("c++")
        .current_dir(&directory)
        .args([
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-pedantic-errors",
            "-o",
            "probe",
            "probe.cpp",
        ])
        .output()
        .expect("a C++ compiler should be available");
    assert!(
        output.status.success(),
        "generated header must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new(directory.join("probe"))
        .output()
        .expect("compiled probe must run");
    assert!(
        run.status.success(),
        "generated validators must agree with the shared corpora:\n{}",
        String::from_utf8_lossy(&run.stdout)
    );
    std::fs::remove_dir_all(&directory).expect("remove C++ probe directory");
}

/// The assertions the line-oriented corpora cannot express.
///
/// Long inputs, the two differing maxima, the raw-versus-normalized length rule,
/// and the Task 040 copy/rvalue lifecycle guarantees.
fn semantics_probe() -> String {
    let mut probe = String::new();
    // The raw/normalized length distinction: a RAW input far longer than
    // kMaxLength is ACCEPTED when its NORMALIZED form fits.
    probe.push_str(
        "    {\n\
         \x20       const std::string raw = std::string(\"a\") + std::string(4000, ' ') + \"b\";\n\
         \x20       if (raw.size() <= 1024) { std::puts(\"raw must exceed maxLength\"); return 1; }\n\
         \x20       auto made = CollapsedRemarks::create(raw);\n\
         \x20       if (!made || made->value() != \"a b\") { std::puts(\"raw/normalized rule\"); return 1; }\n\
         \x20   }\n",
    );
    // Normalized length exactly N and N+1, for both maxima, and a length
    // BETWEEN them proving the two bounds really differ.
    probe.push_str(
        "    if (!CollapsedRemarks::create(std::string(1024, 'x'))) { std::puts(\"1024 fits 1024\"); return 1; }\n\
         \x20   if (CollapsedRemarks::create(std::string(1025, 'x'))) { std::puts(\"1025 exceeds 1024\"); return 1; }\n\
         \x20   if (!PreservedRemarks::create(std::string(1024, 'x'))) { std::puts(\"preserve 1024\"); return 1; }\n\
         \x20   if (PreservedRemarks::create(std::string(1025, 'x'))) { std::puts(\"preserve 1025\"); return 1; }\n\
         \x20   if (!CollapsedNarrative::create(std::string(4096, 'x'))) { std::puts(\"4096 fits 4096\"); return 1; }\n\
         \x20   if (CollapsedNarrative::create(std::string(4097, 'x'))) { std::puts(\"4097 exceeds 4096\"); return 1; }\n\
         \x20   if (!PreservedNarrative::create(std::string(4096, 'x'))) { std::puts(\"preserve 4096\"); return 1; }\n\
         \x20   if (PreservedNarrative::create(std::string(4097, 'x'))) { std::puts(\"preserve 4097\"); return 1; }\n\
         \x20   if (CollapsedRemarks::create(std::string(2000, 'x'))) { std::puts(\"2000 exceeds 1024\"); return 1; }\n\
         \x20   if (!CollapsedNarrative::create(std::string(2000, 'x'))) { std::puts(\"2000 fits 4096\"); return 1; }\n",
    );
    // The two minima differ, for the SAME empty input.
    probe.push_str(
        "    {\n\
         \x20       auto empty = CollapsedRemarks::create(\"\");\n\
         \x20       if (!empty || !empty->value().empty()) { std::puts(\"minLength 0 accepts empty\"); return 1; }\n\
         \x20   }\n\
         \x20   if (PreservedRemarks::create(\"\")) { std::puts(\"minLength 1 rejects empty\"); return 1; }\n\
         \x20   {\n\
         \x20       auto query = PreservedQuery::create(\"  \");\n\
         \x20       if (!query || query->value() != \"  \") { std::puts(\"preserve does not trim\"); return 1; }\n\
         \x20   }\n",
    );
    // TAB: rejected under preserve, normalized to SPACE under collapse.
    probe.push_str(
        "    if (PreservedRemarks::create(\"a\\tb\")) { std::puts(\"preserved class excludes TAB\"); return 1; }\n\
         \x20   {\n\
         \x20       auto tabbed = CollapsedRemarks::create(\"a\\tb\");\n\
         \x20       if (!tabbed || tabbed->value() != \"a b\") { std::puts(\"collapse maps TAB to SPACE\"); return 1; }\n\
         \x20   }\n",
    );
    // LF and CR are PRESERVED under preserve, not normalized away.
    probe.push_str(
        "    {\n\
         \x20       auto lf = PreservedRemarks::create(\"a\\nb\");\n\
         \x20       if (!lf || lf->value() != \"a\\nb\") { std::puts(\"LF must be preserved\"); return 1; }\n\
         \x20       auto cr = PreservedRemarks::create(\"a\\rb\");\n\
         \x20       if (!cr || cr->value() != \"a\\rb\") { std::puts(\"CR must be preserved\"); return 1; }\n\
         \x20   }\n",
    );
    // High-bit bytes: U+00A0 is 0xC2 0xA0, both NEGATIVE as a signed `char`.
    // This is the case that would misbehave without the unsigned-char cast.
    probe.push_str(
        "    if (CollapsedRemarks::create(std::string(\"a\\302\\240b\", 4))) { std::puts(\"NBSP must be rejected\"); return 1; }\n\
         \x20   if (PreservedRemarks::create(std::string(\"caf\\303\\251\", 5))) { std::puts(\"non-ASCII letter\"); return 1; }\n",
    );
    // Task 040 lifecycle: after copying AND after an rvalue operation, BOTH the
    // source and the destination still hold their validated values. A
    // destructive implicit move would leave `source` holding an empty string.
    probe.push_str(
        "    {\n\
         \x20       auto source = *CollapsedRemarks::create(\" keep  me \");\n\
         \x20       auto dest = source;\n\
         \x20       if (source.value() != \"keep me\") { std::puts(\"copy source damaged\"); return 1; }\n\
         \x20       if (dest.value() != \"keep me\") { std::puts(\"copy dest wrong\"); return 1; }\n\
         \x20       auto moved = std::move(source);\n\
         \x20       if (source.value() != \"keep me\") { std::puts(\"rvalue ctor damaged source\"); return 1; }\n\
         \x20       if (moved.value() != \"keep me\") { std::puts(\"rvalue ctor dest wrong\"); return 1; }\n\
         \x20       dest = std::move(moved);\n\
         \x20       if (moved.value() != \"keep me\") { std::puts(\"rvalue assign damaged source\"); return 1; }\n\
         \x20       if (dest.value() != \"keep me\") { std::puts(\"rvalue assign dest wrong\"); return 1; }\n\
         \x20   }\n",
    );
    // The transitive chain member keeps the collapse profile it inherits.
    probe.push_str(
        "    {\n\
         \x20       auto derived = DerivedCollapsedRemarks::create(\" a  b \");\n\
         \x20       if (!derived || derived->value() != \"a b\") { std::puts(\"derived chain member\"); return 1; }\n\
         \x20   }\n",
    );
    probe
}
