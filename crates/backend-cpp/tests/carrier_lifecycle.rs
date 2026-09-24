//! Task 040: the generated C++ validated lexical carriers' special-member
//! lifecycle, compiled and executed.
//!
//! # The defect this file exists to prevent
//!
//! Before Task 040 the generated carriers declared no special members at all,
//! so the compiler implicitly declared a move constructor and a move
//! assignment operator. Both transferred `value_`'s buffer and left the
//! **still-live source** holding an unspecified `std::string`. Every
//! implementation observed leaves it empty, and an empty string is rejected by
//! each of these carriers' own `create`. The source could then be copied,
//! propagating that invalid representation into a fresh object.
//!
//! # What is asserted here
//!
//! The permanent assertion is the lifecycle *contract*, not the observed
//! moved-from behavior of any one standard library: after every applicable
//! operation, each inspectable carrier must still hold a representation its
//! own `create` accepts, and under the selected copy-preserving policy the
//! source must retain its exact original spelling. That is true regardless of
//! what a given `std::string` implementation would have done to a moved-from
//! buffer.
//!
//! All four existing lexical-carrier families are covered: the UCI
//! schema-version profile, the UUID profile, the visible-ASCII profiles
//! (including the 2..4 bound, whose minimum is above 1 so no accidental
//! valid-default assumption can hide there), and the named Zulu DateTime
//! profile.

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn fixture(name: &str) -> ams_gra_oms_ir::SchemaIr {
    ams_gra_oms_xsd_frontend::load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../xsd-frontend/tests/fixtures/{name}")),
    )
    .unwrap_or_else(|error| panic!("{name} fixture should parse: {error:?}"))
}

/// The header carrying the schema-version, UUID, and visible-ASCII families.
fn string_source() -> String {
    generate(&fixture("backend-string-visible-ascii.xsd"), CLOSED)
        .expect("visible-ASCII fixture must generate")
}

/// The header carrying the named Zulu DateTime family.
fn temporal_source() -> String {
    generate(&fixture("backend-temporal-datetime.xsd"), CLOSED)
        .expect("temporal fixture must generate")
}

/// Compile and run one probe against one generated header.
///
/// Built under exactly the strict flags the repository requires, so a policy
/// that only compiles with warnings suppressed cannot pass.
fn run_probe(label: &str, header: &str, probe: &str) {
    let directory = std::env::temp_dir().join(format!("ams-gra-oms-task040-cpp-{label}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create C++ probe directory");
    std::fs::write(directory.join("generated.hpp"), header).expect("write generated header");
    std::fs::write(directory.join("probe.cpp"), probe).expect("write probe");

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
        "{label}: generated header must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new(directory.join("probe"))
        .output()
        .expect("compiled probe must run");
    assert!(
        run.status.success(),
        "{label}: carrier lifecycle contract violated:\n{}",
        String::from_utf8_lossy(&run.stdout)
    );
    std::fs::remove_dir_all(&directory).expect("remove C++ probe directory");
}

/// The emitted special-member policy, asserted as text before anything runs.
///
/// Every affected carrier must declare the copy operations explicitly. That
/// declaration is what suppresses the implicit move operations; asserting it
/// here makes an accidental omission in one template visible even if some
/// future runtime probe happened not to exercise that carrier.

#[test]
fn every_validated_carrier_declares_the_copy_special_members() {
    let string_source = string_source();
    let temporal_source = temporal_source();

    // All four families, spanning both generated headers.
    for (source, carriers) in [
        (
            &string_source,
            // Visible-ASCII at several different bounds, plus schema-version
            // and UUID in the same header.
            vec![
                "Callsign",
                "ShortLabel",
                "CountryCode",
                "DerivedLabel",
                "SchemaVersion",
                "Uuid",
            ],
        ),
        (&temporal_source, vec!["Instant", "Deadline"]),
    ] {
        for carrier in carriers {
            assert!(
                source.contains(&format!("{carrier}(const {carrier}&) = default;")),
                "{carrier} must declare its copy constructor"
            );
            assert!(
                source.contains(&format!(
                    "{carrier}& operator=(const {carrier}&) = default;"
                )),
                "{carrier} must declare its copy assignment operator"
            );
            // No deleted or user-declared move overload: deleting them would
            // break rvalue construction, std::optional, and composition.
            assert!(
                !source.contains(&format!("{carrier}({carrier}&&)")),
                "{carrier} must not declare a move constructor"
            );
            assert!(
                !source.contains(&format!("operator=({carrier}&&)")),
                "{carrier} must not declare a move assignment operator"
            );
        }
    }

    // The existing carrier interface is untouched: private storage, a private
    // unchecked constructor, and a read-only accessor.
    assert!(string_source.contains("explicit Uuid(std::string validated)"));
    assert!(string_source.contains("const std::string& value() const noexcept"));
    assert!(string_source.contains("std::string value_;"));
    // Existing DateTime normalization is preserved.

    assert!(temporal_source.contains("explicit Instant(std::string normalized)"));
}

const PROBE_PROLOGUE: &str = r##"#include "generated.hpp"
#include <cstdio>
#include <optional>
#include <string>
#include <utility>
#include <vector>

static int failures = 0;

// The permanent contract: whatever the operation was, the carrier's own
// validator must still accept what the carrier now holds. This is deliberately
// NOT a check that some standard library emptied a moved-from std::string.
template <typename T>
static void still_valid(const T& item, const char* label) {
    if (!T::create(item.value()).has_value()) {
        std::printf("invalid representation after %s\n", label);
        ++failures;
    }
}

// Under the selected copy-preserving policy the exact spelling survives too.
template <typename T>
static void spelled(const T& item, const char* expected, const char* label) {
    still_valid(item, label);
    if (item.value() != expected) {
        std::printf("wrong spelling after %s: [%s]\n", label, item.value().c_str());
        ++failures;
    }
}

int main() {
"##;

/// The probe epilogue.
const PROBE_EPILOGUE: &str = r##"    if (failures != 0) {
        std::printf("%d failure(s)\n", failures);
        return 1;
    }
    std::puts("ok");
    return 0;
}
"##;

/// One carrier's full lifecycle control matrix, rendered into a probe block.
///
/// `first` and `second` are two distinct values the carrier's own factory
/// accepts. Cases are reported by carrier name and a fixed operation label,
/// never by interpolating arbitrary text into a diagnostic.
fn lifecycle_block(namespace: &str, carrier: &str, first: &str, second: &str) -> String {
    let mut block = String::new();
    writeln!(
        block,
        r##"    {{
        using C = {namespace}::{carrier};
        const char* kFirst = "{first}";
        const char* kSecond = "{second}";

        // Positive control: the factory still accepts and stores exactly.
        auto made = C::create(kFirst);
        if (!made) {{
            std::puts("{carrier}: factory must accept its own valid value");
            ++failures;
        }} else {{
            C origin = *made;
            spelled(origin, kFirst, "{carrier} create");

            // Copy construction: destination correct, source preserved.
            C copied = origin;
            spelled(copied, kFirst, "{carrier} copy-construct dst");
            spelled(origin, kFirst, "{carrier} copy-construct src");

            // Copy assignment.
            C target = *C::create(kSecond);
            target = origin;
            spelled(target, kFirst, "{carrier} copy-assign dst");
            spelled(origin, kFirst, "{carrier} copy-assign src");

            // Construction from an rvalue. Under this policy the copy
            // constructor is selected, so the source is preserved.
            C from_rvalue = std::move(origin);
            spelled(from_rvalue, kFirst, "{carrier} rvalue-construct dst");
            spelled(origin, kFirst, "{carrier} rvalue-construct src");

            // Assignment from an rvalue.
            C rvalue_target = *C::create(kSecond);
            rvalue_target = std::move(origin);
            spelled(rvalue_target, kFirst, "{carrier} rvalue-assign dst");
            spelled(origin, kFirst, "{carrier} rvalue-assign src");

            // Copying the source AFTER an rvalue operation: exactly the
            // propagation path the old implicit move made unsound.
            C after = origin;
            spelled(after, kFirst, "{carrier} copy-after-rvalue");

            // Self copy assignment and self rvalue assignment.
            C self = *C::create(kFirst);
            const C& alias = self;
            self = alias;
            spelled(self, kFirst, "{carrier} self-copy-assign");
            self = std::move(self);
            spelled(self, kFirst, "{carrier} self-rvalue-assign");

            // std::swap exchanges the values and keeps both valid.
            C left = *C::create(kFirst);
            C right = *C::create(kSecond);
            std::swap(left, right);
            spelled(left, kSecond, "{carrier} swap left");
            spelled(right, kFirst, "{carrier} swap right");

            // std::optional composition, including an rvalue transfer. A
            // moved-from std::optional is still engaged, so the source
            // carrier remains inspectable.
            std::optional<C> boxed = C::create(kFirst);
            if (!boxed) {{
                std::puts("{carrier}: optional must be engaged");
                ++failures;
            }} else {{
                std::optional<C> moved_box = std::move(boxed);
                spelled(*moved_box, kFirst, "{carrier} optional dst");
                spelled(*boxed, kFirst, "{carrier} optional src");
            }}

            // Container composition, including reallocating growth, which is
            // where a container would use a move operation if one existed.
            std::vector<C> items;
            items.push_back(*C::create(kFirst));
            items.push_back(*C::create(kSecond));
            for (int index = 0; index < 64; ++index) {{
                items.push_back(*C::create(kFirst));
            }}
            spelled(items[0], kFirst, "{carrier} vector[0]");
            spelled(items[1], kSecond, "{carrier} vector[1]");
        }}
    }}"##
    )
    .expect("writing to String cannot fail");
    block
}

/// The full runtime control matrix over the three String-profile families.
#[test]
fn string_carriers_keep_a_valid_value_through_every_lifecycle_operation() {
    // Each entry is (carrier, a valid literal, a different valid literal).
    // `CountryCode` is the 2..4 profile: its minimum is above 1, so neither an
    // empty nor a single-character representation is valid for it either.
    let carriers: [(&str, &str, &str); 6] = [
        ("Callsign", "ALPHA-1", "BRAVO-2"),
        ("ShortLabel", "short", "other"),
        ("CountryCode", "US", "NATO"),
        ("DerivedLabel", "derived", "second"),
        ("SchemaVersion", "002.5.0", "999.99.99"),
        (
            "Uuid",
            "123e4567-e89b-12d3-a456-426614174000",
            "00000000-0000-0000-0000-000000000000",
        ),
    ];

    let mut probe = String::from(PROBE_PROLOGUE);
    for (carrier, first, second) in carriers {
        probe.push_str(&lifecycle_block("test::callsign", carrier, first, second));
        probe.push('\n');
    }

    // Representative generated record composition: a real generated struct
    // holding required, optional, and ordinary-String members, transferred
    // with std::move as a whole. Every carrier inside BOTH the destination and
    // the still-live source record must remain valid and exact.
    probe.push_str(
        r##"    {
        using namespace test::callsign;
        Payload payload{*Callsign::create("ALPHA-1"),
                        Callsign::create("BRAVO-2"),
                        ShortLabel::create("short"),
                        *CountryCode::create("US"),
                        *DerivedLabel::create("derived"),
                        *SchemaVersion::create("002.5.0"),
                        *Uuid::create("123e4567-e89b-12d3-a456-426614174000"),
                        std::string("notes")};
        Payload transferred = std::move(payload);
        spelled(transferred.primary, "ALPHA-1", "record dst primary");
        spelled(transferred.country, "US", "record dst country");
        spelled(*transferred.alternate, "BRAVO-2", "record dst alternate");
        spelled(transferred.version, "002.5.0", "record dst version");
        spelled(transferred.identifier,
                "123e4567-e89b-12d3-a456-426614174000", "record dst identifier");

        spelled(payload.primary, "ALPHA-1", "record src primary");
        spelled(payload.country, "US", "record src country");
        spelled(*payload.alternate, "BRAVO-2", "record src alternate");
        spelled(*payload.label, "short", "record src label");
        spelled(payload.derived, "derived", "record src derived");
        spelled(payload.version, "002.5.0", "record src version");
        spelled(payload.identifier,
                "123e4567-e89b-12d3-a456-426614174000", "record src identifier");
    }
"##,
    );
    probe.push_str(PROBE_EPILOGUE);

    run_probe("string", &string_source(), &probe);
}

/// The same control matrix for the named Zulu DateTime carrier.
///
/// This family is covered separately because its storage is the **normalized**
/// lexical form, so the preserved spelling is the normalization result rather
/// than the constructor input.

#[test]
fn datetime_carriers_keep_a_valid_value_through_every_lifecycle_operation() {
    // A deliberately un-normalized input, so the probe also proves the existing
    // `collapse` normalization is still applied and is what survives every
    // later lifecycle operation.
    let raw = "  2024-01-15T12:30:00Z  ";
    let normalized = "2024-01-15T12:30:00Z";
    let other = "2025-06-30T23:59:59Z";

    let mut probe = String::from(PROBE_PROLOGUE);
    writeln!(
        probe,
        r##"    {{
        using test::temporal::Instant;
        // Existing normalization is unchanged: collapse is still applied.
        auto made = Instant::create("{raw}");
        if (!made) {{
            std::puts("Instant: factory must accept a collapsible value");
            ++failures;
        }} else {{
            spelled(*made, "{normalized}", "Instant create normalizes");
        }}
    }}"##
    )
    .expect("writing to String cannot fail");

    // Both DateTime carriers in the header get the full matrix, driven from the
    // normalized spelling the carrier actually stores.
    for carrier in ["Instant", "Deadline"] {
        probe.push_str(&lifecycle_block(
            "test::temporal",
            carrier,
            normalized,
            other,
        ));
        probe.push('\n');
    }

    // Representative generated record composition for the temporal family.
    probe.push_str(
        r##"    {
        using namespace test::temporal;
        Payload payload{*Instant::create("2024-01-15T12:30:00Z"),
                        Instant::create("2025-06-30T23:59:59Z")};
        Payload transferred = std::move(payload);
        spelled(transferred.observed, "2024-01-15T12:30:00Z", "record dst observed");
        spelled(*transferred.timestamp, "2025-06-30T23:59:59Z", "record dst timestamp");
        spelled(payload.observed, "2024-01-15T12:30:00Z", "record src observed");
        spelled(*payload.timestamp, "2025-06-30T23:59:59Z", "record src timestamp");
    }
"##,
    );
    probe.push_str(PROBE_EPILOGUE);

    run_probe("temporal", &temporal_source(), &probe);
}

/// The Task 038 privacy guarantees are preserved by the new policy.
///
/// Declaring the copy operations must not accidentally expose the private
/// unchecked constructor or the private storage. Each probe is additionally
/// required to fail *for privacy*, so an unrelated syntax error cannot be
/// mistaken for enforcement.

#[test]
fn the_copy_policy_does_not_expose_private_construction_or_storage() {
    let source = string_source();

    let directory = std::env::temp_dir().join("ams-gra-oms-task040-cpp-privacy");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create C++ probe directory");
    std::fs::write(directory.join("generated.hpp"), &source).expect("write generated header");

    for (label, body) in [
        (
            "direct construction",
            "    test::callsign::Uuid bad(std::string(\"nope\"));\n    (void) bad;",
        ),
        (
            "member read",
            "    auto made = test::callsign::Uuid::create(\"00000000-0000-0000-0000-000000000000\");\n\
             \x20   (void) made->value_;",
        ),
        (
            // A carrier obtained through an rvalue operation still cannot have
            // its raw storage reached.
            "member read after an rvalue operation",
            "    auto made = test::callsign::CountryCode::create(\"US\");\n\
             \x20   test::callsign::CountryCode taken = std::move(*made);\n\
             \x20   (void) taken.value_;",
        ),
    ] {
        let probe = format!(
            "#include \"generated.hpp\"\n#include <string>\n#include <utility>\n\n\
             int main() {{\n{body}\n    return 0;\n}}\n"
        );
        std::fs::write(directory.join("probe.cpp"), &probe).expect("write probe");
        let output = Command::new("c++")
            .current_dir(&directory)
            .args(["-std=c++17", "-fsyntax-only", "probe.cpp"])
            .output()
            .expect("a C++ compiler should be available");
        assert!(
            !output.status.success(),
            "the {label} bypass must not compile"
        );
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostics.contains("private"),
            "the {label} bypass must be rejected for privacy, got:\n{diagnostics}"
        );
    }
    std::fs::remove_dir_all(&directory).expect("remove C++ probe directory");
}
