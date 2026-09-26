//! Task 048 CLI integration tests: the generated typed publish/subscribe
//! façade.
//!
//! Every behavioural claim here is checked with a real toolchain -- `rustc`
//! (`-D warnings`), a strict C++17 `c++` (`-Wall -Wextra -Werror
//! -pedantic-errors`), and GNAT (`-gnatwa -gnatwe`) -- against FAKE adapters
//! written in each test. No network runtime exists; nothing here connects,
//! encodes, or dispatches. The GNAT probes honour `AMS_GRA_REQUIRE_GNAT`.
//!
//! Every negative compile is paired with a positive compile of the same
//! consumer (only one `cfg`/macro/line differs), so a failure can never be
//! caused by a broken fixture or import, and each failure is matched on a
//! diagnostic substring stable across compiler versions.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LANGUAGES: [(&str, &str); 3] = [
    ("rust", "service_api.rs"),
    ("cpp", "service_api.hpp"),
    ("ada", "service_api.ads"),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/cli has a workspace root")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures/service-generate")
        .join(name)
}

fn contract_fixture(name: &str) -> PathBuf {
    workspace_root()
        .join("crates/service-contract/tests/fixtures")
        .join(name)
}

fn output_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ams-gra-oms-task048-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
}

fn run(
    command: &str,
    schema: &Path,
    contract: &Path,
    language: &str,
    out: Option<&Path>,
) -> Output {
    let mut invocation = cli();
    invocation
        .arg(command)
        .arg("--schema")
        .arg(schema)
        .arg("--contract")
        .arg(contract)
        .args(["--language", language, "--world", "closed-schema"]);
    if let Some(out) = out {
        invocation.arg("--output").arg(out);
    }
    invocation.output().expect("CLI should run")
}

fn assert_success(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `service-generate` into a fresh directory.
fn generate_with(schema: &Path, contract: &Path, language: &str, label: &str) -> PathBuf {
    let root = output_dir(&format!("{label}-{language}"));
    let output = run("service-generate", schema, contract, language, Some(&root));
    assert_success(&output, &format!("{label} {language} service-generate"));
    root
}

fn generate(contract: &str, language: &str, label: &str) -> PathBuf {
    generate_with(&fixture("root.xsd"), &fixture(contract), language, label)
}

fn wrapper(root: &Path, file: &str) -> String {
    std::fs::read_to_string(root.join(file)).expect("service API file should exist")
}

/// Every generated file other than the wrapper, sorted by path. Call it
/// straight after generation, before any compiler writes into `root`.
fn model_files(root: &Path) -> Vec<(String, String)> {
    let mut files = std::fs::read_dir(root)
        .expect("output directory")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with("service_api."))
        .map(|name| {
            let contents = std::fs::read_to_string(root.join(&name)).expect("UTF-8");
            (name, contents)
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn gnat_available() -> bool {
    if Command::new("gnatmake").arg("--version").output().is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
        "GNAT is required (AMS_GRA_REQUIRE_GNAT is set) but gnatmake is unavailable"
    );
    false
}

/// Assert `needles` occur in `source` in exactly this order.
fn assert_in_order(source: &str, needles: &[&str], context: &str) {
    let mut from = 0;
    for needle in needles {
        let at = source[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("{context}: {needle:?} missing or out of order"));
        from += at + needle.len();
    }
}

/// The text of one exchange scope, from its opening line to its close.
fn exchange_block<'a>(source: &'a str, language: &str, snake: &str, title: &str) -> &'a str {
    let (open, close) = match language {
        "rust" => (
            format!("pub mod exchange_{snake} {{"),
            "\n        }\n".to_owned(),
        ),
        "cpp" => (
            format!("namespace exchange_{snake} {{"),
            format!("}}  // namespace exchange_{snake}"),
        ),
        _ => (
            format!("package Exchange_{title} is"),
            format!("end Exchange_{title};"),
        ),
    };
    let start = source
        .find(&open)
        .unwrap_or_else(|| panic!("{language}: no scope {open:?}"));
    let rest = &source[start..];
    let end = rest
        .find(&close)
        .unwrap_or_else(|| panic!("{language}: scope {open:?} never closes"));
    &rest[..end]
}

/// The operation-bearing declaration each language emits for Publish.
fn publish_marker(language: &str) -> &'static str {
    match language {
        "rust" => "pub fn publish<",
        "cpp" => "decltype(auto) publish(",
        _ => "package Publisher is",
    }
}

/// The operation-bearing declaration each language emits for Subscribe.
fn subscribe_marker(language: &str) -> &'static str {
    match language {
        "rust" => "pub fn subscribe<",
        "cpp" => "decltype(auto) subscribe(",
        _ => "package Subscriber is",
    }
}

// ---------------------------------------------------------------------
// Real-toolchain helpers
// ---------------------------------------------------------------------

fn run_binary(root: &Path, binary: &str, what: &str) {
    assert_success(
        &Command::new(root.join(binary)).output().expect("run"),
        what,
    );
}

fn stderr_of(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Compile `service_api.rs` as a library crate under `-D warnings`.
fn rust_library(root: &Path) {
    let library = Command::new("rustc")
        .current_dir(root)
        .args(["--edition", "2021", "--crate-type", "lib"])
        .args(["--crate-name", "service_api", "-D", "warnings"])
        .args(["service_api.rs", "-o", "libservice_api.rlib"])
        .output()
        .expect("rustc should be available in a Rust workspace");
    assert_success(&library, "Rust wrapper compile");
}

/// Compile a Rust consumer, optionally with one `--cfg`.
fn rust_consumer(root: &Path, name: &str, source: &str, cfg: Option<&str>) -> Output {
    std::fs::write(root.join(format!("{name}.rs")), source).expect("write Rust consumer");
    let mut command = Command::new("rustc");
    command
        .current_dir(root)
        .args(["--edition", "2021", "-D", "warnings"])
        .args(["--extern", "service_api=libservice_api.rlib", "-L", "."]);
    if let Some(cfg) = cfg {
        command.args(["--cfg", cfg]);
    }
    command
        .arg(format!("{name}.rs"))
        .args(["-o", name])
        .output()
        .expect("rustc")
}

/// Positive compile and run of `source`, then each `(cfg, diagnostic)`
/// negative compile must fail with that diagnostic.
fn rust_positive_and_negatives(root: &Path, source: &str, negatives: &[(&str, &str)]) {
    rust_library(root);
    let positive = rust_consumer(root, "client", source, None);
    assert_success(&positive, "Rust consumer compile (positive control)");
    run_binary(root, "client", "Rust consumer run");
    for (cfg, diagnostic) in negatives {
        let negative = rust_consumer(root, "negative", source, Some(cfg));
        assert!(!negative.status.success(), "{cfg}: must not compile");
        let text = stderr_of(&negative);
        assert!(
            text.contains(diagnostic),
            "{cfg}: expected {diagnostic:?} in\n{text}"
        );
    }
}

fn cpp_compile(root: &Path, name: &str, source: &str, define: Option<&str>) -> Output {
    std::fs::write(root.join(format!("{name}.cpp")), source).expect("write C++ consumer");
    let mut command = Command::new("c++");
    // ASCII diagnostics: GCC's quote characters otherwise follow the locale.
    command.env("LC_ALL", "C").current_dir(root).args([
        "-std=c++17",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic-errors",
    ]);
    if let Some(define) = define {
        command.arg(format!("-D{define}"));
    }
    command
        .arg(format!("{name}.cpp"))
        .args(["-o", name])
        .output()
        .expect("c++ should be available")
}

/// C++ analogue of [`rust_positive_and_negatives`], keyed by `-D` macro.
fn cpp_positive_and_negatives(root: &Path, source: &str, negatives: &[(&str, &str)]) {
    let positive = cpp_compile(root, "client", source, None);
    assert_success(
        &positive,
        "strict C++17 consumer compile (positive control)",
    );
    run_binary(root, "client", "C++ consumer run");
    for (define, diagnostic) in negatives {
        let negative = cpp_compile(root, "negative", source, Some(define));
        assert!(!negative.status.success(), "{define}: must not compile");
        let text = stderr_of(&negative);
        assert!(
            text.contains(diagnostic),
            "{define}: expected {diagnostic:?} in\n{text}"
        );
    }
}

/// Compile the Ada wrapper spec alone, bodyless, warnings as errors.
fn ada_spec(root: &Path) {
    let spec = Command::new("gnatmake")
        .current_dir(root)
        .args(["-q", "-gnatwa", "-gnatwe", "-gnatc", "service_api.ads"])
        .output()
        .expect("gnatmake");
    assert_success(&spec, "GNAT wrapper compile");
}

/// Build `main` (with its extra units already written into `root`) with
/// assertions and warnings as errors. `-f` forces recompilation: the
/// negative controls rewrite `main` within the same second, which GNAT's
/// timestamp check would otherwise treat as up to date.
fn ada_build(root: &Path, main: &str) -> Output {
    Command::new("gnatmake")
        .current_dir(root)
        .args(["-f", "-q", "-gnata", "-gnatwa", "-gnatwe", main])
        .output()
        .expect("gnatmake")
}

// ---------------------------------------------------------------------
// Structure: direction is an API capability
// ---------------------------------------------------------------------

/// `(snake, Title, expected operation is Subscribe)` for the bidirectional
/// fixture, in contract order.
const BIDIRECTIONAL: [(&str, &str, bool); 4] = [
    ("in_mandatory", "In_Mandatory", true),
    ("in_optional", "In_Optional", true),
    ("out_mandatory", "Out_Mandatory", false),
    ("out_optional", "Out_Optional", false),
];

/// Same payload, both directions, both mandates: each input exposes
/// Subscribe only, each output Publish only, whatever its mandate or timing.
/// All four Payload aliases name the one PayloadA, declared once.
#[test]
fn task048_direction_alone_selects_the_operation() {
    for (language, file) in LANGUAGES {
        let root = generate("facade-bidirectional.yaml", language, "bidirectional");
        let model = model_files(&root)
            .into_iter()
            .map(|(_, source)| source)
            .collect::<String>();
        let declaration = match language {
            "ada" => "type PayloadA is record",
            "rust" => "pub struct PayloadA {",
            _ => "struct PayloadA {",
        };
        assert_eq!(model.matches(declaration).count(), 1, "{language}");

        let source = wrapper(&root, file);
        let alias = match language {
            "ada" => "subtype Payload is Urn.Test.PayloadA;",
            "rust" => "pub type Payload = super::super::super::model::PayloadA;",
            _ => "using Payload = ::urn::test::PayloadA;",
        };
        assert_eq!(source.matches(alias).count(), 4, "{language}");
        for (snake, title, subscribes) in BIDIRECTIONAL {
            let block = exchange_block(&source, language, snake, title);
            assert_eq!(
                block.contains(subscribe_marker(language)),
                subscribes,
                "{language} {snake}"
            );
            assert_eq!(
                block.contains(publish_marker(language)),
                !subscribes,
                "{language} {snake}"
            );
        }
        assert_eq!(source.matches(subscribe_marker(language)).count(), 2);
        assert_eq!(source.matches(publish_marker(language)).count(), 2);
        // Timing schedules nothing; no async machinery is emitted.
        for forbidden in [
            "async ",
            "Future",
            "tokio",
            "std::future",
            "co_await",
            "thread",
            "task type",
            "delay ",
            "sleep",
            "timer",
            "std::function",
            "new ",
        ] {
            assert!(!source.contains(forbidden), "{language}: {forbidden:?}");
        }
    }
}

/// The subscription group reaches only the Subscribe façade, verbatim when
/// authored and absent (never fabricated) otherwise.
#[test]
fn task048_subscription_group_is_verbatim_or_absent() {
    for (language, file) in LANGUAGES {
        let root = generate("facade-bidirectional.yaml", language, "group");
        let source = wrapper(&root, file);
        let (absent, present) = match language {
            "rust" => (
                "pub const SUBSCRIPTION_GROUP: Option<&str> = None;".to_owned(),
                "pub const SUBSCRIPTION_GROUP: Option<&str> = Some(\"pool 7: \\\"hot\\\"\");"
                    .to_owned(),
            ),
            "cpp" => (
                "subscription_group = std::nullopt;".to_owned(),
                "subscription_group = std::string_view(\"pool 7: \\\"hot\\\"\");".to_owned(),
            ),
            _ => (
                "Has_Subscription_Group : constant Boolean := False;".to_owned(),
                "Has_Subscription_Group : constant Boolean := True;".to_owned(),
            ),
        };
        let mandatory = exchange_block(&source, language, "in_mandatory", "In_Mandatory");
        let optional = exchange_block(&source, language, "in_optional", "In_Optional");
        assert!(mandatory.contains(&absent), "{language}:\n{mandatory}");
        assert!(optional.contains(&present), "{language}:\n{optional}");
        if language == "ada" {
            assert!(
                optional.contains(
                    "Subscription_Group     : constant String := \"pool 7: \"\"hot\"\"\";"
                )
            );
            assert!(mandatory.contains("Subscription_Group     : constant String := \"\";"));
        }
        for (snake, title, _) in &BIDIRECTIONAL[2..] {
            let block = exchange_block(&source, language, snake, title);
            assert!(
                !block.to_ascii_lowercase().contains("subscription_group"),
                "{language} {snake}"
            );
        }
    }
}

/// Data Transfer, Special Signal, Security Exchange, and Non-OMS Message
/// endpoints keep only their Task 047 metadata, whatever their direction.
#[test]
fn task048_non_oms_kinds_gain_no_operation() {
    for (language, file) in LANGUAGES {
        let root = generate("api-all-kinds.yaml", language, "non-oms");
        let source = wrapper(&root, file);
        for (snake, title) in [
            ("bulk_imagery", "Bulk_Imagery"),
            ("sync_pulse", "Sync_Pulse"),
            ("key_exchange", "Key_Exchange"),
            ("legacy_status", "Legacy_Status"),
        ] {
            let block = exchange_block(&source, language, snake, title).to_ascii_lowercase();
            for forbidden in ["publish", "subscribe", "handler", "message_name", "payload"] {
                assert!(
                    !block.contains(forbidden),
                    "{language} {snake}: {forbidden}"
                );
            }
        }
        // The three OMS occurrences each get exactly one operation.
        assert_eq!(source.matches(subscribe_marker(language)).count(), 1);
        assert_eq!(source.matches(publish_marker(language)).count(), 2);
    }
}

// ---------------------------------------------------------------------
// Compatibility with the Task 047 surface
// ---------------------------------------------------------------------

/// A zero-OMS service needs no façade. Its wrapper is still the exact Task
/// 047 descriptor file: no adapter contract, no extra include, no new
/// comment.
#[test]
fn task048_zero_oms_wrapper_is_the_task047_file() {
    for (language, file) in LANGUAGES {
        let root = generate("non-uci.yaml", language, "zero-oms");
        assert!(model_files(&root).is_empty(), "{language}");
        let source = wrapper(&root, file);
        assert!(source.contains("(Task 047)."), "{language}");
        assert!(!source.contains("Task 048"), "{language}");
        // Declarations, not words: the unchanged Task 047 header comment
        // itself says it "subscribes, publishes ... nothing".
        for forbidden in [
            publish_marker(language),
            subscribe_marker(language),
            "PublishAdapter",
            "SubscribeAdapter",
            "Handler",
            "Subscription_Group",
            "SUBSCRIPTION_GROUP",
            "subscription_group",
            "<optional>",
            "<utility>",
            "<type_traits>",
        ] {
            assert!(!source.contains(forbidden), "{language}: {forbidden:?}");
        }
    }
}

/// Every Task 047 metadata spelling survives unchanged for a representative
/// OMS wrapper with input and output endpoints.
#[test]
fn task048_preserves_every_task047_metadata_spelling() {
    let expectations: [(&str, &str, &[&str]); 3] = [
        (
            "rust",
            "service_api.rs",
            &[
                "#[path = \"test.rs\"]\npub mod model;",
                "pub const SERVICE_VERSION: &str = \"1.2.3\";",
                "pub const SERVICE_KIND: &str = \"subsystem\";",
                "pub const NAME: &str = \"Mission Data & Status, Primary\";",
                "pub const ID: &str = \"a-input\";",
                "pub const KIND: &str = \"oms_message\";",
                "pub const DIRECTION: &str = \"input\";",
                "pub const MANDATE: &str = \"mandatory\";",
                "pub const TOPIC: &str = \"mission.a\";",
                "pub type Payload = super::super::super::model::PayloadA;",
                "pub const DIRECTION: &str = \"output\";",
                "pub type Payload = super::super::super::model::PayloadB;",
            ],
        ),
        (
            "cpp",
            "service_api.hpp",
            &[
                "#include \"test.hpp\"",
                "inline constexpr std::string_view service_kind = \"subsystem\";",
                "inline constexpr std::string_view id = \"a-input\";",
                "inline constexpr std::string_view kind = \"oms_message\";",
                "inline constexpr std::string_view direction = \"input\";",
                "inline constexpr std::string_view mandate = \"mandatory\";",
                "inline constexpr std::string_view topic = \"mission.a\";",
                "using Payload = ::urn::test::PayloadA;",
                "using Payload = ::urn::test::PayloadB;",
            ],
        ),
        (
            "ada",
            "service_api.ads",
            &[
                "with Urn.Test;",
                "Service_Kind    : constant String := \"subsystem\";",
                "Id        : constant String := \"a-input\";",
                "Kind      : constant String := \"oms_message\";",
                "Direction : constant String := \"input\";",
                "Mandate   : constant String := \"mandatory\";",
                "Topic     : constant String := \"mission.a\";",
                "subtype Payload is Urn.Test.PayloadA;",
                "subtype Payload is Urn.Test.PayloadB;",
            ],
        ),
    ];
    for (language, file, spellings) in expectations {
        let root = generate("api-all-kinds.yaml", language, "task047-spellings");
        let source = wrapper(&root, file);
        for spelling in spellings {
            assert!(source.contains(spelling), "{language} missing {spelling:?}");
        }
    }
}

/// One message, two occurrences, opposite directions: one model type, one
/// Subscribe (topic A) and one Publish (topic B), never deduplicated.
/// Reversing contract order keeps the model byte-identical, moves each
/// operation with its exchange, and never regroups endpoints by kind.
#[test]
fn task048_repeated_message_and_contract_order() {
    for (language, file) in LANGUAGES {
        let forward = generate("facade-repeated.yaml", language, "order-forward");
        let reversed = generate("facade-repeated-reversed.yaml", language, "order-reversed");
        let model = model_files(&forward);
        assert!(!model.is_empty(), "{language}");
        assert_eq!(model, model_files(&reversed), "{language}");
        let declaration = match language {
            "ada" => "type PayloadA is record",
            "rust" => "pub struct PayloadA {",
            _ => "struct PayloadA {",
        };
        let joined = model
            .iter()
            .map(|(_, source)| source.as_str())
            .collect::<String>();
        assert_eq!(joined.matches(declaration).count(), 1, "{language}");

        let forward_api = wrapper(&forward, file);
        let reversed_api = wrapper(&reversed, file);
        assert_ne!(forward_api, reversed_api, "{language}");
        let (a, b) = match language {
            "ada" => (
                "package Function_Function_A is",
                "package Function_Function_B is",
            ),
            "rust" => (
                "pub mod function_function_a {",
                "pub mod function_function_b {",
            ),
            _ => (
                "namespace function_function_a {",
                "namespace function_function_b {",
            ),
        };
        let (subscribe, publish) = (subscribe_marker(language), publish_marker(language));
        assert_in_order(&forward_api, &[a, subscribe, b, publish], language);
        assert_in_order(&reversed_api, &[b, publish, a, subscribe], language);
        for source in [&forward_api, &reversed_api] {
            assert_eq!(source.matches(subscribe).count(), 1, "{language}");
            assert_eq!(source.matches(publish).count(), 1, "{language}");
            let input = exchange_block(source, language, "report_in", "Report_In");
            let output = exchange_block(source, language, "report_out", "Report_Out");
            assert!(
                input.contains(subscribe) && !input.contains(publish),
                "{language}"
            );
            assert!(
                output.contains(publish) && !output.contains(subscribe),
                "{language}"
            );
            assert!(input.contains("\"position.a\"") && output.contains("\"position.b\""));
        }
    }
}

// ---------------------------------------------------------------------
// Fake-adapter consumers and negative compile controls
// ---------------------------------------------------------------------

/// The Rust fake adapter: records every hook call, returns a custom
/// `Result` for publish and a custom subscription token for subscribe, and
/// delivers a real typed `PayloadA` to the subscriber handler.
const RUST_BIDIRECTIONAL: &str = r#"
use service_api::model::{LabelType, PayloadA, SharedType};
use service_api::service_api::function_track_io as track;
use service_api::service_api::{PublishAdapter, SubscribeAdapter};

#[derive(Debug, PartialEq)]
enum Sent { Ok }
#[derive(Debug, PartialEq)]
struct Token(u32);

#[derive(Default)]
struct Fake {
    published: Vec<(&'static str, i64)>,
    routes: Vec<(&'static str, &'static str, &'static str, Option<&'static str>)>,
}

impl PublishAdapter<PayloadA> for Fake {
    type Output = Result<Sent, String>;
    fn publish(&mut self, topic: &'static str, value: &PayloadA) -> Self::Output {
        self.published.push((topic, value.shared.count.get()));
        Ok(Sent::Ok)
    }
}

impl<H: FnMut(&PayloadA)> SubscribeAdapter<PayloadA, H> for Fake {
    type Output = Token;
    fn subscribe(&mut self, namespace: &'static str, name: &'static str, topic: &'static str,
                 group: Option<&'static str>, mut handler: H) -> Token {
        self.routes.push((namespace, name, topic, group));
        handler(&sample(41));
        Token(self.routes.len() as u32)
    }
}

fn sample(count: i64) -> PayloadA {
    PayloadA {
        shared: SharedType {
            label: LabelType::Secondary,
            count: service_api::model::BoundedI64::new(count).expect("in range"),
        },
    }
}

fn main() {
    let mut fake = Fake::default();
    let mut seen = Vec::new();
    // No topic, message name, namespace, or group is supplied here.
    let first = track::exchange_in_mandatory::subscribe(&mut fake, |m: &track::exchange_in_mandatory::Payload| {
        seen.push(m.shared.count.get());
    });
    let second = track::exchange_in_optional::subscribe(&mut fake, |m| seen.push(m.shared.count.get() + 1));
    assert_eq!((first, second), (Token(1), Token(2)));
    assert_eq!(seen, vec![41, 42]);
    assert_eq!(fake.routes, vec![
        ("urn:test", "MessageA", "track.in.mandatory", None),
        ("urn:test", "MessageA", "track.in.optional", Some("pool 7: \"hot\"")),
    ]);
    assert_eq!(track::exchange_out_mandatory::publish(&mut fake, &sample(5)), Ok(Sent::Ok));
    assert_eq!(track::exchange_out_optional::publish(&mut fake, &sample(6)), Ok(Sent::Ok));
    assert_eq!(fake.published, vec![("track.out.mandatory", 5), ("track.out.optional", 6)]);
    // Task 047 metadata is still there, unchanged.
    assert_eq!(track::exchange_in_optional::MANDATE, "optional");
    assert_eq!(track::exchange_out_mandatory::DIRECTION, "output");

    #[cfg(neg_publish_on_input)]
    track::exchange_in_mandatory::publish(&mut fake, &sample(1));
    #[cfg(neg_subscribe_on_output)]
    track::exchange_out_mandatory::subscribe(&mut fake, |_m: &PayloadA| {});
    #[cfg(neg_wrong_payload)]
    track::exchange_out_mandatory::publish(&mut fake, &sample(1).shared);
    #[cfg(neg_wrong_handler)]
    track::exchange_in_mandatory::subscribe(&mut fake, |_m: &SharedType| {});
}
"#;

/// Publish and Subscribe through a Rust fake adapter, and the four wrong
/// uses fail to compile for the intended reason.
#[test]
fn task048_rust_facade_round_trips_through_a_fake_adapter() {
    let root = generate("facade-bidirectional.yaml", "rust", "rust-fake");
    rust_positive_and_negatives(
        &root,
        RUST_BIDIRECTIONAL,
        &[
            // Input exchanges have no `publish`; output ones no `subscribe`.
            ("neg_publish_on_input", "cannot find function `publish`"),
            (
                "neg_subscribe_on_output",
                "cannot find function `subscribe`",
            ),
            // Another type where the endpoint Payload is required.
            ("neg_wrong_payload", "mismatched types"),
            // A handler over the wrong payload type.
            ("neg_wrong_handler", "type mismatch in closure arguments"),
        ],
    );
}

/// The C++ fake adapter: a plain struct, duck-typed, no virtual base and no
/// `std::function`. `publish` returns an `int` status and `subscribe` a
/// custom token; the handler receives a real typed `PayloadA`.
const CPP_BIDIRECTIONAL: &str = r#"
#include "service_api.hpp"
#include <cassert>
#include <optional>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

namespace track = service_api::function_track_io;
using ::urn::test::PayloadA;

struct Token { int id; };

::urn::test::PayloadA sample(std::int64_t count) {
    using Count = decltype(std::declval<PayloadA&>().shared.count);
    auto bounded = Count::create(count);
    return PayloadA{::urn::test::SharedType{::urn::test::LabelType::Secondary, *bounded}};
}

struct Fake {
    std::vector<std::string> log;
    int publish(std::string_view topic, const PayloadA& value) {
        log.push_back("PUB " + std::string(topic) + " " + std::to_string(value.shared.count.value()));
        return 200;
    }
    template <typename Payload, typename Handler>
    Token subscribe(std::string_view ns, std::string_view name, std::string_view topic,
                    std::optional<std::string_view> group, Handler&& handler) {
        static_assert(std::is_same_v<Payload, PayloadA>);
        log.push_back("SUB " + std::string(ns) + " " + std::string(name) + " " +
                      std::string(topic) + " " + std::string(group.value_or("<none>")));
        handler(sample(41));
        return Token{static_cast<int>(log.size())};
    }
};

static_assert(std::is_same_v<track::exchange_in_mandatory::Payload, PayloadA>);
static_assert(std::is_same_v<track::exchange_out_optional::Payload, PayloadA>);
static_assert(track::exchange_in_optional::subscription_group.has_value());
static_assert(!track::exchange_in_mandatory::subscription_group.has_value());

int main() {
    Fake fake;
    std::vector<std::int64_t> seen;
    // No topic, message name, namespace, or group is supplied here.
    Token first = track::exchange_in_mandatory::subscribe(
        fake, [&](const track::exchange_in_mandatory::Payload& m) { seen.push_back(m.shared.count.value()); });
    Token second = track::exchange_in_optional::subscribe(
        fake, [&](const PayloadA& m) { seen.push_back(m.shared.count.value() + 1); });
    assert(first.id == 1 && second.id == 2);
    assert(seen.size() == 2 && seen[0] == 41 && seen[1] == 42);
    assert(fake.log[0] == "SUB urn:test MessageA track.in.mandatory <none>");
    assert(fake.log[1] == "SUB urn:test MessageA track.in.optional pool 7: \"hot\"");
    assert(track::exchange_out_mandatory::publish(fake, sample(5)) == 200);
    assert(track::exchange_out_optional::publish(fake, sample(6)) == 200);
    assert(fake.log[2] == "PUB track.out.mandatory 5");
    assert(fake.log[3] == "PUB track.out.optional 6");
    static_assert(track::exchange_out_mandatory::direction == "output");
#if defined(NEG_PUBLISH_ON_INPUT)
    track::exchange_in_mandatory::publish(fake, sample(1));
#elif defined(NEG_SUBSCRIBE_ON_OUTPUT)
    track::exchange_out_mandatory::subscribe(fake, [](const PayloadA&) {});
#elif defined(NEG_WRONG_PAYLOAD)
    track::exchange_out_mandatory::publish(fake, sample(1).shared);
#elif defined(NEG_WRONG_HANDLER)
    track::exchange_in_mandatory::subscribe(fake, [](const ::urn::test::SharedType&) {});
#endif
    return 0;
}
"#;

/// Publish and Subscribe through a C++ fake adapter under strict C++17, and
/// the four wrong uses fail to compile for the intended reason.
#[test]
fn task048_cpp_facade_round_trips_through_a_fake_adapter() {
    let root = generate("facade-bidirectional.yaml", "cpp", "cpp-fake");
    cpp_positive_and_negatives(
        &root,
        CPP_BIDIRECTIONAL,
        &[
            ("NEG_PUBLISH_ON_INPUT", "'publish' is not a member of"),
            ("NEG_SUBSCRIBE_ON_OUTPUT", "'subscribe' is not a member of"),
            ("NEG_WRONG_PAYLOAD", "no matching function for call to"),
            (
                "NEG_WRONG_HANDLER",
                "subscriber handler must accept const Payload&",
            ),
        ],
    );
}

/// The Ada fake runtime hooks: ordinary library-level subprograms, no
/// tasks, no heap. The publish hook returns a status enumeration and the
/// subscribe hook a record token; both are chosen here, not by the wrapper.
const ADA_FAKE_SPEC: &str = r#"with Service_API;
package Fake_Hooks is
   package Track renames Service_API.Function_Track_Io;
   package In_M renames Track.Exchange_In_Mandatory;
   package In_O renames Track.Exchange_In_Optional;

   type Status is (Accepted, Rejected);
   type Token is record
      Id : Natural := 0;
   end record;

   Log_Line : array (1 .. 4) of String (1 .. 64) := (others => (others => ' '));
   Log_Size : Natural := 0;

   function Publish_To (Topic : String; Value : Track.Exchange_Out_Mandatory.Payload)
     return Status;
   function Subscribe_M
     (Message_Namespace      : String;
      Message_Name           : String;
      Topic                  : String;
      Has_Subscription_Group : Boolean;
      Subscription_Group     : String;
      Receiver               : not null access In_M.Handler'Class) return Token;
   function Subscribe_O
     (Message_Namespace      : String;
      Message_Name           : String;
      Topic                  : String;
      Has_Subscription_Group : Boolean;
      Subscription_Group     : String;
      Receiver               : not null access In_O.Handler'Class) return Token;

   type Recorder is new In_M.Handler and In_O.Handler with record
      Seen : Long_Long_Integer := 0;
   end record;
   overriding procedure Handle (Self : in out Recorder; Message : In_M.Payload);
end Fake_Hooks;
"#;

const ADA_FAKE_BODY: &str = r#"with Urn.Test;
package body Fake_Hooks is
   procedure Record_Line (Text : String) is
   begin
      Log_Size := Log_Size + 1;
      Log_Line (Log_Size) (1 .. Text'Length) := Text;
   end Record_Line;

   function Route (Namespace, Name, Topic : String; Has_Group : Boolean; Group : String)
     return String is
     (Namespace & "|" & Name & "|" & Topic & "|" & (if Has_Group then Group else "<none>"));

   function Publish_To (Topic : String; Value : Track.Exchange_Out_Mandatory.Payload)
     return Status is
   begin
      Record_Line ("PUB|" & Topic & "|" & Long_Long_Integer'Image (Value.Shared.Count));
      return Accepted;
   end Publish_To;

   function Subscribe_M
     (Message_Namespace      : String;
      Message_Name           : String;
      Topic                  : String;
      Has_Subscription_Group : Boolean;
      Subscription_Group     : String;
      Receiver               : not null access In_M.Handler'Class) return Token is
   begin
      Record_Line (Route (Message_Namespace, Message_Name, Topic,
                          Has_Subscription_Group, Subscription_Group));
      Receiver.Handle ((Shared => (Label => Urn.Test.Secondary, Count => 41)));
      return (Id => Log_Size);
   end Subscribe_M;

   function Subscribe_O
     (Message_Namespace      : String;
      Message_Name           : String;
      Topic                  : String;
      Has_Subscription_Group : Boolean;
      Subscription_Group     : String;
      Receiver               : not null access In_O.Handler'Class) return Token is
   begin
      Record_Line (Route (Message_Namespace, Message_Name, Topic,
                          Has_Subscription_Group, Subscription_Group));
      Receiver.Handle ((Shared => (Label => Urn.Test.Primary, Count => 42)));
      return (Id => Log_Size);
   end Subscribe_O;

   overriding procedure Handle (Self : in out Recorder; Message : In_M.Payload) is
   begin
      Self.Seen := Self.Seen + Message.Shared.Count;
   end Handle;
end Fake_Hooks;
"#;

/// The Ada consumer. `Neg` selects one wrong use; `None` is the positive
/// control. Only one declaration differs between the variants.
fn ada_client(negative: Option<&str>) -> String {
    let extra = match negative {
        None => "",
        // An input exchange has no Publisher.
        Some("publish_on_input") => "   package Bad is new In_M.Publisher (Status, Publish_To);\n",
        // An output exchange has no Subscriber.
        Some("subscribe_on_output") => {
            "   package Bad is new Track.Exchange_Out_Mandatory.Subscriber (Token, Subscribe_M);\n"
        }
        // Another type where the endpoint Payload is required.
        Some("wrong_payload") => "   Bad : constant Status := Pub_M.Publish (Sample.Shared);\n",
        // A handler over the wrong payload type cannot implement Handler.
        Some("wrong_handler") => concat!(
            "   package Wrong is\n",
            "      type Bad is new In_M.Handler with null record;\n",
            "      overriding procedure Handle (Self : in out Bad; Message : Urn.Test.SharedType);\n",
            "   end Wrong;\n",
        ),
        Some(other) => panic!("unknown Ada negative {other}"),
    };
    format!(
        "with Urn.Test;\n\
         with Fake_Hooks; use Fake_Hooks;\n\
         procedure Client is\n\
         \x20  package Pub_M is new Track.Exchange_Out_Mandatory.Publisher (Status, Publish_To);\n\
         \x20  package Pub_O is new Track.Exchange_Out_Optional.Publisher (Status, Publish_To);\n\
         \x20  package Sub_M is new In_M.Subscriber (Token, Subscribe_M);\n\
         \x20  package Sub_O is new In_O.Subscriber (Token, Subscribe_O);\n\
         \x20  Sample : constant Track.Exchange_Out_Mandatory.Payload :=\n\
         \x20    (Shared => (Label => Urn.Test.Primary, Count => 5));\n\
         {extra}\
         \x20  function Logged (Index : Positive; Text : String) return Boolean is\n\
         \x20    (Log_Line (Index) (1 .. Text'Length) = Text);\n\
         \x20  R : aliased Recorder;\n\
         \x20  First  : constant Token := Sub_M.Subscribe (R'Access);\n\
         \x20  Second : constant Token := Sub_O.Subscribe (R'Access);\n\
         begin\n\
         \x20  if First.Id /= 1 or else Second.Id /= 2 or else R.Seen /= 83\n\
         \x20    or else not Logged (1, \"urn:test|MessageA|track.in.mandatory|<none>\")\n\
         \x20    or else not Logged (2, \"urn:test|MessageA|track.in.optional|pool 7: \"\"hot\"\"\")\n\
         \x20    or else Pub_M.Publish (Sample) /= Accepted\n\
         \x20    or else Pub_O.Publish (Sample) /= Accepted\n\
         \x20    or else not Logged (3, \"PUB|track.out.mandatory| 5\")\n\
         \x20    or else not Logged (4, \"PUB|track.out.optional| 5\")\n\
         \x20  then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         end Client;\n"
    )
}

/// Publish and Subscribe through Ada fake hooks under GNAT with warnings
/// as errors: the spec needs no body, hooks pick their own result types,
/// one library-level object serves two endpoints' `Handler` interfaces, and
/// each wrong use is a compile error matched on a GNAT 13/14-stable
/// substring. Gated in CI with `require_one_test`.
#[test]
fn task048_ada_facade_round_trips_through_fake_hooks() {
    if !gnat_available() {
        return;
    }
    let root = generate("facade-bidirectional.yaml", "ada", "ada-fake");
    assert!(
        !root.join("service_api.adb").exists(),
        "the Ada facade must need no package body"
    );
    ada_spec(&root);
    std::fs::write(root.join("fake_hooks.ads"), ADA_FAKE_SPEC).expect("write");
    std::fs::write(root.join("fake_hooks.adb"), ADA_FAKE_BODY).expect("write");
    std::fs::write(root.join("client.adb"), ada_client(None)).expect("write");
    assert_success(
        &ada_build(&root, "client.adb"),
        "GNAT consumer (positive control)",
    );
    run_binary(&root, "client", "Ada consumer run");

    for (negative, diagnostic) in [
        (
            "publish_on_input",
            "\"Publisher\" not declared in \"Exchange_In_Mandatory\"",
        ),
        (
            "subscribe_on_output",
            "\"Subscriber\" not declared in \"Exchange_Out_Mandatory\"",
        ),
        ("wrong_payload", "expected type \"Payload\""),
        ("wrong_handler", "not overriding"),
    ] {
        std::fs::write(root.join("client.adb"), ada_client(Some(negative))).expect("write");
        let failed = ada_build(&root, "client.adb");
        assert!(!failed.status.success(), "{negative}: must not compile");
        let text = stderr_of(&failed);
        assert!(
            text.contains(diagnostic),
            "{negative}: expected {diagnostic:?} in\n{text}"
        );
    }
}

// ---------------------------------------------------------------------
// Upstream-authored contracts
// ---------------------------------------------------------------------

/// The verbatim upstream `upstream-minimal.yaml` exchange is an INPUT, so it
/// gets Subscribe and no Publish. The fake adapter sees the resolved
/// PositionReport identity, topic `PositionReport`, and no group; the
/// handler receives a real `PositionReportMT`. (Real UCI 2.5 evidence is in
/// `docs/task-048-publish-subscribe-facade.md`.)
#[test]
fn task048_upstream_position_report_is_subscribe_only() {
    let contract = contract_fixture("upstream-minimal.yaml");
    let schema = fixture("position-report.xsd");
    for (language, file) in LANGUAGES {
        let root = generate_with(&schema, &contract, language, "position");
        let source = wrapper(&root, file);
        assert_eq!(source.matches(subscribe_marker(language)).count(), 1);
        assert_eq!(source.matches(publish_marker(language)).count(), 0);
        match language {
            "rust" => {
                rust_library(&root);
                let probe = rust_consumer(&root, "client", RUST_POSITION, None);
                assert_success(&probe, "Rust PositionReport consumer");
                run_binary(&root, "client", "Rust PositionReport consumer run");
            }
            "cpp" => {
                let probe = cpp_compile(&root, "client", CPP_POSITION, None);
                assert_success(&probe, "C++ PositionReport consumer");
                run_binary(&root, "client", "C++ PositionReport consumer run");
            }
            _ => {
                if !gnat_available() {
                    continue;
                }
                ada_spec(&root);
                std::fs::write(root.join("client.adb"), ADA_POSITION).expect("write");
                assert_success(
                    &ada_build(&root, "client.adb"),
                    "Ada PositionReport consumer",
                );
                run_binary(&root, "client", "Ada PositionReport consumer run");
            }
        }
    }
}

const RUST_POSITION: &str = r#"
use service_api::model::{PositionReportMDT, PositionReportMT};
use service_api::service_api::function_position_input_example::exchange_position_report_input as position;
use service_api::service_api::SubscribeAdapter;

struct Fake(Vec<(&'static str, &'static str, &'static str, Option<&'static str>)>);
impl<H: FnMut(&PositionReportMT)> SubscribeAdapter<PositionReportMT, H> for Fake {
    type Output = usize;
    fn subscribe(&mut self, ns: &'static str, name: &'static str, topic: &'static str,
                 group: Option<&'static str>, mut handler: H) -> usize {
        self.0.push((ns, name, topic, group));
        handler(&PositionReportMT { messagedata: PositionReportMDT { latitude: 1.5, longitude: -2.5 } });
        self.0.len()
    }
}
fn main() {
    let mut fake = Fake(Vec::new());
    let mut latitude = 0.0;
    let id = position::subscribe(&mut fake, |report: &position::Payload| latitude = report.messagedata.latitude);
    assert_eq!(id, 1);
    assert_eq!(latitude, 1.5);
    assert_eq!(fake.0, vec![("https://www.vdl.afrl.af.mil/programs/oam", "PositionReport", "PositionReport", None)]);
    assert_eq!(position::DIRECTION, "input");
    assert_eq!(position::TOPIC, "PositionReport");
}
"#;

const CPP_POSITION: &str = r#"
#include "service_api.hpp"
#include <cassert>
#include <string>
namespace position = service_api::function_position_input_example::exchange_position_report_input;
static_assert(std::is_same_v<position::Payload, ::programs::oam::PositionReportMT>);
struct Fake {
    std::string route;
    template <typename Payload, typename Handler>
    int subscribe(std::string_view ns, std::string_view name, std::string_view topic,
                  std::optional<std::string_view> group, Handler&& handler) {
        route = std::string(ns) + "|" + std::string(name) + "|" + std::string(topic) + "|" +
                std::string(group.value_or("<none>"));
        handler(Payload{::programs::oam::PositionReportMDT{1.5, -2.5}});
        return 1;
    }
};
int main() {
    Fake fake;
    double latitude = 0.0;
    int id = position::subscribe(fake, [&](const position::Payload& r) { latitude = r.messagedata.latitude; });
    assert(id == 1 && latitude == 1.5);
    assert(fake.route == "https://www.vdl.afrl.af.mil/programs/oam|PositionReport|PositionReport|<none>");
    static_assert(position::direction == "input");
    return 0;
}
"#;

const ADA_POSITION: &str = r#"with Interfaces;
with Service_API;
procedure Client is
   package Position renames
     Service_API.Function_Position_Input_Example.Exchange_Position_Report_Input;
   use type Interfaces.IEEE_Float_64;

   Route : String (1 .. 96) := (others => ' ');

   package Handlers is
      type Recorder is new Position.Handler with record
         Latitude : Interfaces.IEEE_Float_64 := 0.0;
      end record;
      overriding procedure Handle (Self : in out Recorder; Message : Position.Payload);
   end Handlers;
   package body Handlers is
      overriding procedure Handle (Self : in out Recorder; Message : Position.Payload) is
      begin
         Self.Latitude := Message.MessageData.Latitude;
      end Handle;
   end Handlers;

   function Subscribe_To
     (Message_Namespace      : String;
      Message_Name           : String;
      Topic                  : String;
      Has_Subscription_Group : Boolean;
      Subscription_Group     : String;
      Receiver               : not null access Position.Handler'Class) return Natural
   is
      Text : constant String := Message_Namespace & "|" & Message_Name & "|" & Topic & "|"
        & (if Has_Subscription_Group then Subscription_Group else "<none>");
   begin
      Route (1 .. Text'Length) := Text;
      Receiver.Handle ((MessageData => (Latitude => 1.5, Longitude => -2.5)));
      return 1;
   end Subscribe_To;

   package Sub is new Position.Subscriber (Natural, Subscribe_To);
   Expected : constant String :=
     "https://www.vdl.afrl.af.mil/programs/oam|PositionReport|PositionReport|<none>";
   R  : aliased Handlers.Recorder;
   Id : constant Natural := Sub.Subscribe (R'Access);
begin
   if Id /= 1 or else R.Latitude /= 1.5 or else Route (1 .. Expected'Length) /= Expected then
      raise Program_Error;
   end if;
end Client;
"#;

/// The verbatim upstream Service Status example: operations follow contract
/// order exactly as Publish, Subscribe, Publish, decided by each exchange's
/// direction (output, input, output).
#[test]
fn task048_upstream_service_status_is_publish_subscribe_publish() {
    let contract = contract_fixture("upstream-service-status.yaml");
    let schema = fixture("service-status.xsd");
    for (language, file) in LANGUAGES {
        let check = run("service-check", &schema, &contract, language, None);
        assert_success(&check, "service-status service-check");
        let root = generate_with(&schema, &contract, language, "service-status");
        let source = wrapper(&root, file);
        let (publish, subscribe) = (publish_marker(language), subscribe_marker(language));
        let blocks = [
            (
                "service_status_output",
                "Service_Status_Output",
                publish,
                subscribe,
            ),
            (
                "service_status_data_request_input",
                "Service_Status_Data_Request_Input",
                subscribe,
                publish,
            ),
            (
                "service_status_data_request_status_output",
                "Service_Status_Data_Request_Status_Output",
                publish,
                subscribe,
            ),
        ];
        let mut in_order = Vec::new();
        for (snake, title, present, absent) in blocks {
            let block = exchange_block(&source, language, snake, title);
            assert!(block.contains(present), "{language} {snake}");
            assert!(!block.contains(absent), "{language} {snake}");
            in_order.push(present);
        }
        assert_in_order(&source, &in_order, language);
        assert_eq!(source.matches(publish).count(), 2, "{language}");
        assert_eq!(source.matches(subscribe).count(), 1, "{language}");
        match language {
            "rust" => rust_library(&root),
            "cpp" => {
                let probe = cpp_compile(
                    &root,
                    "client",
                    "#include \"service_api.hpp\"\nint main() { return 0; }\n",
                    None,
                );
                assert_success(&probe, "C++ service-status compile");
            }
            _ => {
                if gnat_available() {
                    ada_spec(&root);
                }
            }
        }
    }
}

/// `service-check` stays read-only and READY, and ordinary `generate` still
/// emits no service API or façade.
#[test]
fn task048_readiness_and_ordinary_generate_are_unchanged() {
    for (language, _) in LANGUAGES {
        for contract in ["facade-bidirectional.yaml", "facade-repeated.yaml"] {
            let check = run(
                "service-check",
                &fixture("root.xsd"),
                &fixture(contract),
                language,
                None,
            );
            assert_success(&check, contract);
            let report = String::from_utf8_lossy(&check.stdout);
            assert!(
                report.ends_with("status: READY\n"),
                "{language} {contract}:\n{report}"
            );
            assert!(!report.contains("service api boundary"), "{report}");
        }
        let root = output_dir(&format!("ordinary-{language}"));
        let ordinary = cli()
            .arg("generate")
            .arg("--schema")
            .arg(fixture("abstract.xsd"))
            .args(["--language", language, "--world", "closed-schema"])
            .arg("--output")
            .arg(&root)
            .output()
            .expect("CLI should run");
        assert_success(&ordinary, "ordinary generate");
        for (name, source) in model_files(&root) {
            assert!(!name.starts_with("service_api"), "{language}: {name}");
            assert!(!source.contains("Publisher") && !source.contains("PublishAdapter"));
        }
        assert!(
            !root
                .join(format!(
                    "service_api.{}",
                    match language {
                        "rust" => "rs",
                        "cpp" => "hpp",
                        _ => "ads",
                    }
                ))
                .exists()
        );
    }
}
