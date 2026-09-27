//! Task 053: generated C++ constrained named Binary carriers, compiled under
//! the strict flags and EXECUTED.
//!
//! * `create` at both boundaries, `std::nullopt` one octet either side;
//! * `value()` returns a const reference to the stored octets;
//! * Task 040 lifecycle: copy, rvalue construction and rvalue assignment all
//!   COPY, so a moved-from carrier still holds a schema-valid octet sequence
//!   (a synthesized move would empty the still-live `std::vector`);
//! * `ZeroBlob` (`length = 0`): `create({})` succeeds, yet default
//!   construction is still unavailable;
//! * composition through `std::optional`, `BoundedVector`, `UnboundedVector`
//!   and Choice `std::variant`;
//! * negative compile probes: no public vector constructor, no default
//!   constructor, no access to `value_`, each rejected for the stated reason.

use ams_gra_oms_backend_cpp::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use std::path::{Path, PathBuf};
use std::process::Command;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;
const STRICT: [&str; 5] = [
    "-std=c++17",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-pedantic-errors",
];

fn header() -> String {
    let schema = ams_gra_oms_xsd_frontend::load_schema_document(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/service-generate/constrained-binary.xsd"),
    )
    .expect("constrained-binary fixture parses");
    generate(&schema, CLOSED).expect("constrained Binary header generates")
}

fn directory(label: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "ams-gra-oms-task053-cpp-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create probe directory");
    std::fs::write(directory.join("generated.hpp"), header()).expect("write header");
    directory
}

#[test]
fn task053_cpp_carrier_shape_is_checked_and_copy_only() {
    let source = header();
    for carrier in [
        "Exact4",
        "Min2",
        "Max6",
        "Between2And6",
        "DerivedExact4",
        "ZeroBlob",
        "BlobBase",
        "BlobMiddle",
        "BlobExact",
    ] {
        for required in [
            format!("static std::optional<{carrier}> create(std::vector<std::uint8_t> value) {{"),
            format!("{carrier}(const {carrier}&) = default;"),
            format!("{carrier}& operator=(const {carrier}&) = default;"),
            format!("private:\n    explicit {carrier}(std::vector<std::uint8_t> validated)"),
        ] {
            assert!(
                source.contains(&required),
                "{carrier}: missing {required:?}"
            );
        }
        assert!(!source.contains(&format!("{carrier}({carrier}&&)")));
        assert!(!source.contains(&format!("operator=({carrier}&&)")));
        assert!(!source.contains(&format!("{carrier}() ")));
    }
    // The unconstrained control keeps the Task 025 class byte-for-byte.
    assert!(source.contains(
        "class PlainBytes {\npublic:\n    explicit PlainBytes(std::vector<std::uint8_t> value) : value_(std::move(value)) {}\n    const std::vector<std::uint8_t>& value() const noexcept { return value_; }\nprivate:\n    std::vector<std::uint8_t> value_;\n};\n"
    ));
}

const RUNTIME_PROBE: &str = r##"#include "generated.hpp"
#include <cstdio>
#include <optional>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace o = programs::oam;
static int failures = 0;
#define CHECK(condition, label) do { if (!(condition)) { std::printf("FAIL: %s\n", label); ++failures; } } while (0)

static std::vector<std::uint8_t> octets(std::size_t count) {
    std::vector<std::uint8_t> bytes;
    for (std::size_t index = 0; index < count; ++index) {
        bytes.push_back(static_cast<std::uint8_t>(index ^ 0xA5u));
    }
    return bytes;
}

template <typename T>
static void bounds(const char* label, std::size_t min, std::size_t max, bool bounded) {
    if (min > 0) CHECK(!T::create(octets(min - 1)).has_value(), label);
    CHECK(T::create(octets(min)).has_value(), label);
    if (bounded) {
        CHECK(T::create(octets(max)).has_value(), label);
        CHECK(!T::create(octets(max + 1)).has_value(), label);
    } else {
        CHECK(T::create(octets(4096)).has_value(), label);
    }
}

// The permanent contract: whatever the operation, the carrier's own factory
// still accepts what it holds, and the octets are exactly the original ones.
template <typename T>
static void holds(const T& item, const std::vector<std::uint8_t>& expected, const char* label) {
    CHECK(T::create(item.value()).has_value(), label);
    CHECK(item.value() == expected, label);
}

template <typename T>
static void lifecycle(const char* label, const std::vector<std::uint8_t>& first,
                      const std::vector<std::uint8_t>& second) {
    static_assert(!std::is_default_constructible<T>::value, "no default constructor");
    static_assert(!std::is_constructible<T, std::vector<std::uint8_t>>::value,
                  "no public unchecked constructor");
    static_assert(std::is_copy_constructible<T>::value, "copyable");
    static_assert(std::is_same<decltype(std::declval<const T&>().value()),
                               const std::vector<std::uint8_t>&>::value,
                  "value() returns a const reference");
    T a = T::create(first).value();
    T b = std::move(a);                 // copy fallback: source preserved
    holds(a, first, label);
    holds(b, first, label);
    T c = T::create(second).value();
    c = std::move(b);                   // rvalue assignment copies too
    holds(b, first, label);
    holds(c, first, label);
    T d = a;                            // copy after an rvalue operation
    holds(d, first, label);
    std::optional<T> box = T::create(second);
    std::optional<T> moved = std::move(box);
    holds(*box, second, label);
    holds(*moved, second, label);
    std::vector<T> items;
    for (int index = 0; index < 64; ++index) items.push_back(T::create(first).value());
    holds(items.front(), first, label);
    holds(items.back(), first, label);
}

int main() {
    bounds<o::Exact4>("Exact4", 4, 4, true);
    bounds<o::Min2>("Min2", 2, 0, false);
    bounds<o::Max6>("Max6", 0, 6, true);
    bounds<o::Between2And6>("Between2And6", 2, 6, true);
    bounds<o::DerivedExact4>("DerivedExact4", 4, 4, true);
    bounds<o::ZeroBlob>("ZeroBlob", 0, 0, true);
    bounds<o::BlobBase>("BlobBase", 4, 0, false);
    bounds<o::BlobMiddle>("BlobMiddle", 4, 16, true);
    bounds<o::BlobExact>("BlobExact", 8, 8, true);
    CHECK(o::BlobExact::min_octets == 8u && o::BlobExact::max_octets == 8u, "BlobExact consts");

    // Positive minimum: a destructive move would empty a live Exact4.
    lifecycle<o::Exact4>("Exact4 lifecycle", octets(4), {1, 2, 3, 4});
    lifecycle<o::Between2And6>("Between2And6 lifecycle", octets(6), octets(2));
    lifecycle<o::BlobBase>("BlobBase lifecycle", octets(40), octets(4));

    // Exact length zero: the empty VALUE is valid...
    auto zero = o::ZeroBlob::create({});
    CHECK(zero.has_value() && zero->value().empty(), "ZeroBlob create({})");
    CHECK(!o::ZeroBlob::create({0}).has_value(), "ZeroBlob rejects 1 octet");
    // ...but unchecked default construction is still unavailable.
    static_assert(!std::is_default_constructible<o::ZeroBlob>::value, "ZeroBlob default");

    // Structural composition.
    auto exact = o::Exact4::create({0x00, 0x0A, 0xEE, 0xFF}).value();
    auto bounded = o::BoundedVector<o::Max6, 0, 3>::create({o::Max6::create({}).value(),
                                                            o::Max6::create(octets(6)).value()});
    auto required = o::BoundedVector<o::Between2And6, 1, 3>::create(
        {o::Between2And6::create(octets(2)).value()});
    auto stream = o::UnboundedVector<o::DerivedExact4, 0>::create({});
    auto stream_required = o::UnboundedVector<o::Exact4, 1>::create({exact});
    CHECK(bounded && required && stream && stream_required, "sequence composition");
    CHECK(!(o::BoundedVector<o::Between2And6, 1, 3>::create({}).has_value()), "1..3 empty");
    o::ConstrainedBlobPayload payload{
        o::BoundedInteger<std::int64_t, -2147483648, 2147483647>::create(53).value(),
        exact,
        std::nullopt,
        *bounded,
        *required,
        *stream,
        *stream_required,
        o::ZeroBlob::create({}).value(),
        o::BlobBase::create(octets(4)),
        std::nullopt,
        o::BlobExact::create(octets(8)).value(),
        o::PlainBytes(std::vector<std::uint8_t>{0x0F}),
        o::BlobChoice{o::BlobChoice::Ranged{o::Between2And6::create({0xAB, 0xCD}).value()}},
    };
    CHECK(!payload.maybemin.has_value(), "absent optional carrier");
    CHECK(payload.base->value().size() == 4u, "present optional carrier");
    CHECK(payload.bounded.values().size() == 2u, "bounded values");
    CHECK(payload.streamrequired.values().front().value() == exact.value(), "unbounded element");
    CHECK(std::holds_alternative<o::BlobChoice::Ranged>(payload.pick.value), "choice");
    o::ConstrainedBlobPayload taken = std::move(payload);
    holds(payload.exact, exact.value(), "payload move source");
    holds(taken.exact, exact.value(), "payload move destination");

    if (failures != 0) {
        std::printf("%d failure(s)\n", failures);
        return 1;
    }
    std::puts("CPP CONSTRAINED BINARY RUNTIME: PASSED");
    return 0;
}
"##;

#[test]
fn task053_cpp_constrained_binary_runtime_and_lifecycle() {
    let directory = directory("runtime");
    std::fs::write(directory.join("probe.cpp"), RUNTIME_PROBE).expect("write probe");
    let built = Command::new("c++")
        .current_dir(&directory)
        .args(STRICT)
        .args(["-o", "probe", "probe.cpp"])
        .output()
        .expect("a C++ compiler should be available");
    assert!(
        built.status.success(),
        "strict C++17 build failed:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new(directory.join("probe"))
        .output()
        .expect("probe runs");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(), "{stdout}");
    assert!(stdout.contains("CPP CONSTRAINED BINARY RUNTIME: PASSED"));
    println!("{stdout}");
    std::fs::remove_dir_all(&directory).expect("remove probe directory");
}

/// Every bypass must FAIL to compile under the strict flags, and for the
/// stated reason, so an unrelated error cannot pass for enforcement.
#[test]
fn task053_cpp_negative_api_probes() {
    let directory = directory("negative");
    for (label, body, reason) in [
        (
            "public construction from a vector",
            "programs::oam::Exact4 bad(std::vector<std::uint8_t>{1, 2, 3, 4}); (void) bad;",
            "private",
        ),
        (
            "default construction",
            "programs::oam::Exact4 bad; (void) bad;",
            "no matching function",
        ),
        (
            "zero-length default construction",
            "programs::oam::ZeroBlob bad; (void) bad;",
            "no matching function",
        ),
        (
            "raw value_ access",
            "auto made = programs::oam::Exact4::create({1, 2, 3, 4}); (void) made->value_;",
            "private",
        ),
    ] {
        let probe = format!(
            "#include \"generated.hpp\"\n#include <vector>\n\nint main() {{\n    {body}\n    return 0;\n}}\n"
        );
        std::fs::write(directory.join("probe.cpp"), &probe).expect("write probe");
        let output = Command::new("c++")
            .current_dir(&directory)
            .args(STRICT)
            .args(["-fsyntax-only", "probe.cpp"])
            .output()
            .expect("a C++ compiler should be available");
        assert!(
            !output.status.success(),
            "the {label} bypass must not compile"
        );
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostics.contains(reason),
            "the {label} bypass must be rejected ({reason}), got:\n{diagnostics}"
        );
    }
    std::fs::remove_dir_all(&directory).expect("remove probe directory");
}
