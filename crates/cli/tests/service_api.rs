//! Task 047 CLI integration tests: generated typed service API wrappers.
//!
//! Fixtures live under `tests/fixtures/service-generate` and select against
//! the synthetic `root.xsd` (MessageA -> PayloadA, MessageB -> PayloadB).
//! Every compile probe here uses a real toolchain: `rustc`, a strict C++17
//! `c++`, and GNAT. The GNAT probes honour `AMS_GRA_REQUIRE_GNAT`, so CI can
//! never pass them by silently skipping.

use std::ffi::OsString;
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

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
}

/// A unique temporary output root per test, removed first so repeated local
/// runs cannot observe a previous run's files.
fn output_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ams-gra-oms-task047-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn run(command: &str, contract: &str, language: &str, output: Option<&Path>) -> Output {
    let mut invocation = cli();
    invocation
        .arg(command)
        .arg("--schema")
        .arg(fixture("root.xsd"))
        .arg("--contract")
        .arg(fixture(contract))
        .args(["--language", language, "--world", "closed-schema"]);
    if let Some(output) = output {
        invocation.args([OsString::from("--output"), OsString::from(output)]);
    }
    invocation.output().expect("CLI should run")
}

fn generate(contract: &str, language: &str, label: &str) -> PathBuf {
    let root = output_dir(&format!("{label}-{language}"));
    let output = run("service-generate", contract, language, Some(&root));
    assert_eq!(
        output.status.code(),
        Some(0),
        "{language} {contract}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    root
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("report should be UTF-8")
}

/// Every generated file (relative path, contents), sorted by path.
fn files(root: &Path) -> Vec<(String, String)> {
    let mut files = std::fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| {
                    (
                        entry.file_name().to_string_lossy().into_owned(),
                        std::fs::read_to_string(entry.path()).expect("UTF-8"),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn wrapper(root: &Path, file: &str) -> String {
    std::fs::read_to_string(root.join(file)).expect("service API file should exist")
}

fn model_files(root: &Path) -> Vec<(String, String)> {
    files(root)
        .into_iter()
        .filter(|(path, _)| !path.starts_with("service_api."))
        .collect()
}

/// Assert `needles` occur in `source` in exactly the given order, each found
/// strictly after the previous one, and that no needle occurs more often than
/// it is listed. A needle may legitimately repeat (the same exchange ID in two
/// functions), in which case it is listed once per occurrence.
fn assert_in_order(source: &str, needles: &[String], context: &str) {
    let mut from = 0;
    for needle in needles {
        let at = source[from..]
            .find(needle.as_str())
            .unwrap_or_else(|| panic!("{context}: {needle:?} missing or out of contract order"));
        from += at + needle.len();
    }
    for needle in needles {
        let listed = needles.iter().filter(|other| *other == needle).count();
        assert_eq!(
            source.matches(needle.as_str()).count(),
            listed,
            "{context}: unexpected occurrence count for {needle:?}"
        );
    }
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

fn assert_success(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------
// Structure: all five kinds, contract order, repeated messages
// ---------------------------------------------------------------------

/// Scope names in the order the all-kinds contract declares them.
fn all_kinds_scopes(language: &str) -> Vec<String> {
    let (function, exchange) = match language {
        "ada" => ("package Function_", "package Exchange_"),
        "rust" => ("pub mod function_", "pub mod exchange_"),
        _ => ("namespace function_", "namespace exchange_"),
    };
    let ada = language == "ada";
    let name = |snake: &str, title: &str| {
        if ada {
            title.to_owned()
        } else {
            snake.to_owned()
        }
    };
    [
        format!("{function}{} ", name("mission_data", "Mission_Data")),
        format!("{exchange}{} ", name("a_input", "A_Input")),
        format!("{exchange}{} ", name("bulk_imagery", "Bulk_Imagery")),
        format!("{exchange}{} ", name("sync_pulse", "Sync_Pulse")),
        format!("{exchange}{} ", name("key_exchange", "Key_Exchange")),
        format!("{exchange}{} ", name("legacy_status", "Legacy_Status")),
        // 'status-output' (OMS) inside mission-data ...
        format!("{exchange}{} ", name("status_output", "Status_Output")),
        format!("{function}{} ", name("mission_backup", "Mission_Backup")),
        format!("{exchange}{} ", name("a_repeat", "A_Repeat")),
        // ... and the SAME exchange id (non-OMS) inside mission_backup: legal,
        // because each exchange scope nests under its own function scope.
        format!("{exchange}{} ", name("status_output", "Status_Output")),
    ]
    .into_iter()
    .collect()
}

/// All five exchange kinds appear, one scope per occurrence, in contract
/// order. OMS scopes carry TOPIC and Payload; the other four carry only the
/// common ID/KIND/DIRECTION/MANDATE metadata.
#[test]
fn task047_all_five_exchange_kinds_are_represented_in_contract_order() {
    for (language, file) in LANGUAGES {
        let root = generate("api-all-kinds.yaml", language, "all-kinds");
        let source = wrapper(&root, file);

        assert_in_order(&source, &all_kinds_scopes(language), language);
        for kind in [
            "\"oms_message\"",
            "\"data_transfer\"",
            "\"special_signal\"",
            "\"security_exchange\"",
            "\"non_oms_message\"",
        ] {
            assert!(source.contains(kind), "{language} missing {kind}");
        }
        // Three OMS occurrences (a-input, status-output, a-repeat): exactly
        // three TOPICs and three Payloads, however many non-OMS scopes exist.
        let payload = match language {
            "ada" => "subtype Payload is",
            "rust" => "pub type Payload =",
            _ => "using Payload =",
        };
        assert_eq!(source.matches(payload).count(), 3, "{language}");
        let topic = match language {
            "ada" => "Topic     : constant",
            "rust" => "pub const TOPIC:",
            _ => "std::string_view topic =",
        };
        assert_eq!(source.matches(topic).count(), 3, "{language}");
        for value in ["\"mission.a\"", "\"mission.b\"", "\"backup.a\""] {
            assert!(source.contains(value), "{language} missing topic {value}");
        }

        // Each non-OMS scope ends before any Payload/Topic could appear.
        let non_oms_block = source
            .split(match language {
                "ada" => "package Exchange_Bulk_Imagery ",
                "rust" => "pub mod exchange_bulk_imagery ",
                _ => "namespace exchange_bulk_imagery ",
            })
            .nth(1)
            .and_then(|rest| {
                rest.split(match language {
                    "ada" => "end Exchange_Bulk_Imagery;",
                    "rust" => "}",
                    _ => "}  // namespace exchange_bulk_imagery",
                })
                .next()
            })
            .expect("data_transfer scope");
        assert!(non_oms_block.contains("data_transfer"), "{language}");
        assert!(!non_oms_block.contains("Payload"), "{language}");
        assert!(
            !non_oms_block.to_ascii_lowercase().contains("topic"),
            "{language}"
        );
    }
}

/// MessageA is selected by two occurrences in two functions. The model emits
/// PayloadA once; the wrapper emits two endpoints with two topics whose
/// Payload aliases name the same generated type.
#[test]
fn task047_repeated_message_selection_is_two_endpoints_one_type() {
    for (language, file) in LANGUAGES {
        let root = generate("api-all-kinds.yaml", language, "repeated");
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
        assert_eq!(source.matches(alias).count(), 2, "{language}:\n{source}");
        assert!(source.contains("\"mission.a\"") && source.contains("\"backup.a\""));
        // The Payload binds the resolved PAYLOAD, not the message's name.
        assert!(!source.contains("MessageA"), "{language}");
    }
}

/// Scope spellings for one language: (function prefix, exchange prefix,
/// whether names are Ada title case).
fn scope_prefixes(language: &str) -> (&'static str, &'static str, bool) {
    match language {
        "ada" => ("package Function_", "package Exchange_", true),
        "rust" => ("pub mod function_", "pub mod exchange_", false),
        _ => ("namespace function_", "namespace exchange_", false),
    }
}

/// Task 032 proved reordering exchanges while selecting the same messages
/// leaves TYPE output byte-identical. That still holds for the model files.
/// The wrapper is contract order made observable, so it must differ -- and
/// must follow the NEW order.
#[test]
fn task047_contract_order_changes_only_the_service_api_file() {
    for (language, file) in LANGUAGES {
        let forward = generate("api-all-kinds.yaml", language, "order-forward");
        let reordered = generate("api-all-kinds-reordered.yaml", language, "order-reordered");

        assert!(!model_files(&forward).is_empty(), "{language}");
        assert_eq!(
            model_files(&forward),
            model_files(&reordered),
            "{language}: model type files must be byte-identical"
        );
        let forward_api = wrapper(&forward, file);
        let reordered_api = wrapper(&reordered, file);
        assert_ne!(forward_api, reordered_api, "{language}");

        // mission_backup first, then mission-data's exchanges reversed.
        let (function, exchange, ada) = scope_prefixes(language);
        let pick = |snake: &str, title: &str| {
            if ada {
                title.to_owned()
            } else {
                snake.to_owned()
            }
        };
        let expected = [
            format!("{function}{} ", pick("mission_backup", "Mission_Backup")),
            format!("{exchange}{} ", pick("a_repeat", "A_Repeat")),
            format!("{exchange}{} ", pick("status_output", "Status_Output")),
            format!("{function}{} ", pick("mission_data", "Mission_Data")),
            format!("{exchange}{} ", pick("status_output", "Status_Output")),
            format!("{exchange}{} ", pick("legacy_status", "Legacy_Status")),
            format!("{exchange}{} ", pick("key_exchange", "Key_Exchange")),
            format!("{exchange}{} ", pick("sync_pulse", "Sync_Pulse")),
            format!("{exchange}{} ", pick("bulk_imagery", "Bulk_Imagery")),
            format!("{exchange}{} ", pick("a_input", "A_Input")),
        ];
        assert_in_order(&reordered_api, &expected, language);
    }
}

// ---------------------------------------------------------------------
// Naming
// ---------------------------------------------------------------------

/// Reserved-looking portable IDs (`type`, `match`, `range`, `operator`,
/// `delete`, `package`) generate and compile in every language, because the
/// fixed prefix keeps the raw ID from ever being a host identifier.
#[test]
fn task047_reserved_looking_ids_generate_safely() {
    for (language, file) in LANGUAGES {
        let root = generate("api-reserved-ids.yaml", language, "reserved");
        let source = wrapper(&root, file);
        let (function, exchange, ada) = scope_prefixes(language);
        let pick = |snake: &str, title: &str| {
            if ada {
                title.to_owned()
            } else {
                snake.to_owned()
            }
        };
        let expected = [
            format!("{function}{} ", pick("type", "Type")),
            format!("{exchange}{} ", pick("match", "Match")),
            format!("{exchange}{} ", pick("range", "Range")),
            format!("{function}{} ", pick("operator", "Operator")),
            format!("{exchange}{} ", pick("delete", "Delete")),
            format!("{exchange}{} ", pick("package", "Package")),
        ];
        assert_in_order(&source, &expected, language);
        // The raw ID is still available verbatim as the ID constant.
        for raw in [
            "\"type\"",
            "\"match\"",
            "\"range\"",
            "\"operator\"",
            "\"delete\"",
        ] {
            assert!(source.contains(raw), "{language} missing {raw}");
        }
        // Task 044's enum escape is not involved.
        assert!(!source.contains("Value_"), "{language}");
        compile_wrapper(language, &root, None);
    }
}

/// Human service/function names with spaces, quotes, and punctuation appear
/// only as escaped string constants. They never contribute to a symbol.
#[test]
fn task047_human_names_are_metadata_only() {
    for (language, file) in LANGUAGES {
        let root = generate("api-all-kinds.yaml", language, "human-names");
        let source = wrapper(&root, file);
        let service = if language == "ada" {
            "\"Wrapper Demo: All \"\"Kinds\"\" (v1)\""
        } else {
            "\"Wrapper Demo: All \\\"Kinds\\\" (v1)\""
        };
        for literal in [
            service,
            "\"Mission Data & Status, Primary\"",
            "\"Mission Backup (secondary) -- 2nd\"",
        ] {
            assert!(source.contains(literal), "{language} missing {literal}");
        }
        compile_wrapper(language, &root, None);
    }
}

/// Colliding function IDs, and colliding exchange IDs within one function:
/// the contract is portable-valid, service-check is NOT READY with a
/// `service api boundary:` line and unchanged counts, and service-generate
/// writes NOTHING -- not the model, not the wrapper, not the directory.
#[test]
fn task047_normalization_collisions_fail_closed_and_write_nothing() {
    for (contract, selected, types, needle) in [
        (
            "api-function-collision.yaml",
            2,
            5,
            "function id 'foo-bar' and function id 'foo_bar'",
        ),
        (
            "api-exchange-collision.yaml",
            1,
            3,
            "function 'mission-data' exchange id 'foo-bar' and function 'mission-data' \
             exchange id 'foo_bar'",
        ),
    ] {
        // The contract itself is valid: service-plan accepts it.
        let plan = cli()
            .arg("service-plan")
            .arg("--schema")
            .arg(fixture("root.xsd"))
            .arg("--contract")
            .arg(fixture(contract))
            .output()
            .expect("CLI should run");
        assert_eq!(plan.status.code(), Some(0), "{contract} must stay valid");

        for (language, _) in LANGUAGES {
            let check = run("service-check", contract, language, None);
            assert_eq!(check.status.code(), Some(1), "{language} {contract}");
            let report = stdout_of(&check);
            assert!(report.contains("status: NOT READY"), "{report}");
            for line in [
                format!("selected oms messages: {selected}\n"),
                format!("renderable selected oms messages: {selected}\n"),
                format!("selected type closure: {types}\n"),
                format!("renderable selected types: {types}\n"),
            ] {
                assert!(report.contains(&line), "{language}: {line:?} in\n{report}");
            }
            let boundary = report
                .lines()
                .find(|line| line.starts_with("service api boundary: "))
                .unwrap_or_else(|| panic!("{language}: no boundary line in\n{report}"));
            assert!(boundary.contains(needle), "{boundary}");
            assert!(boundary.contains("normalization collisions fail closed"));
            // No fake UCI blocker is invented for a wrapper-name problem.
            assert!(!report.contains("unsupported selected types:"), "{report}");
            assert!(!report.contains("blocked selected messages:"), "{report}");

            let root = output_dir(&format!("collision-{language}"));
            let generated = run("service-generate", contract, language, Some(&root));
            assert_eq!(generated.status.code(), Some(1), "{language} {contract}");
            assert!(
                !root.exists(),
                "{language} {contract}: nothing may be written"
            );
            // Exactly the report service-check printed, from one formatter.
            assert_eq!(stdout_of(&generated), report);
        }
    }
}

/// The UCI model `service-generate` writes is byte-for-byte what the ordinary
/// backend type generator produces for the same projected schema. The model
/// files are computed here independently through the library API, so the
/// wrapper path is proven not to perturb type generation.
#[test]
fn task047_model_files_are_exactly_the_ordinary_backend_output() {
    use ams_gra_oms_codegen_core::{
        Backend, GenerationWorld, project_service_generation_schema, resolve_service_plan,
    };
    let schema = ams_gra_oms_xsd_frontend::load_schema_set_with_overlays(&fixture("root.xsd"), &[])
        .expect("schema");
    let contract = ams_gra_oms_service_contract::load_contract(&fixture("api-all-kinds.yaml"))
        .expect("contract");
    let plan = resolve_service_plan(&contract, &schema).expect("plan");
    let world = GenerationWorld::ClosedSchemaSet;
    let projection = project_service_generation_schema(&plan, &schema, world).expect("project");
    let backends: [(&str, Box<dyn Backend>); 3] = [
        ("rust", Box::new(ams_gra_oms_backend_rust::RustBackend)),
        ("cpp", Box::new(ams_gra_oms_backend_cpp::CppBackend)),
        ("ada", Box::new(ams_gra_oms_backend_ada::AdaBackend)),
    ];
    for (language, backend) in backends {
        let mut expected = backend
            .generate(projection.schema(), world)
            .expect("ordinary type generation")
            .into_iter()
            .map(|file| (file.relative_path.display().to_string(), file.contents))
            .collect::<Vec<_>>();
        expected.sort();
        let root = generate("api-all-kinds.yaml", language, "model-identity");
        assert_eq!(model_files(&root), expected, "{language}");
    }
}

/// Readiness for a safe service keeps the pre-Task-047 report byte-for-byte:
/// the wrapper adds no line to a READY report.
#[test]
fn task047_safe_service_check_report_is_unchanged() {
    for (language, _) in LANGUAGES {
        let check = run("service-check", "both.yaml", language, None);
        assert_eq!(check.status.code(), Some(0), "{language}");
        assert_eq!(
            stdout_of(&check),
            format!(
                "service contract valid\nservice: Selected Generation Both\nlanguage: {language}\n\
                 generation world: closed-schema\n\nselected oms messages: 2\n\
                 renderable selected oms messages: 2\nselected type closure: 5\n\
                 renderable selected types: 5\nstatus: READY\n"
            ),
            "{language}"
        );
    }
}

/// Ordinary `generate` has no contract and therefore no service API: it
/// never emits a `service_api.*` file, and its output is unchanged.
#[test]
fn task047_ordinary_generate_never_emits_a_service_api() {
    for (language, _) in LANGUAGES {
        let root = output_dir(&format!("ordinary-{language}"));
        let output = cli()
            .arg("generate")
            .arg("--schema")
            .arg(fixture("abstract.xsd"))
            .args(["--language", language, "--world", "closed-schema"])
            .arg("--output")
            .arg(&root)
            .output()
            .expect("CLI should run");
        assert_success(&output, "ordinary generate");
        let generated = files(&root);
        assert!(!generated.is_empty(), "{language}");
        assert!(
            generated
                .iter()
                .all(|(path, _)| !path.starts_with("service_api")),
            "{language}: {generated:?}"
        );
        assert!(!stdout_of(&output).contains("service api"), "{language}");
    }
}

// ---------------------------------------------------------------------
// Compile and consume the generated wrappers
// ---------------------------------------------------------------------

fn run_binary(root: &Path, what: &str) {
    assert_success(
        &Command::new(root.join("client")).output().expect("run"),
        what,
    );
}

fn compile_rust(root: &Path, probe: Option<&str>) {
    let library = Command::new("rustc")
        .current_dir(root)
        .args(["--edition", "2021", "--crate-type", "lib"])
        .args(["--crate-name", "service_api", "-D", "warnings"])
        .args(["service_api.rs", "-o", "libservice_api.rlib"])
        .output()
        .expect("rustc should be available in a Rust workspace");
    assert_success(&library, "Rust wrapper compile");
    if let Some(probe) = probe {
        std::fs::write(root.join("client.rs"), probe).expect("write Rust probe");
        let client = Command::new("rustc")
            .current_dir(root)
            .args(["--edition", "2021", "-D", "warnings"])
            .args(["--extern", "service_api=libservice_api.rlib", "-L", "."])
            .args(["client.rs", "-o", "client"])
            .output()
            .expect("rustc");
        assert_success(&client, "Rust consumer compile");
        run_binary(root, "Rust consumer run");
    }
}

fn compile_cpp(root: &Path, probe: Option<&str>) {
    let source = probe.map_or_else(
        || "#include \"service_api.hpp\"\nint main() { return 0; }\n".to_owned(),
        str::to_owned,
    );
    std::fs::write(root.join("client.cpp"), source).expect("write C++ probe");
    let client = Command::new("c++")
        .current_dir(root)
        .args([
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic-errors",
        ])
        .args(["client.cpp", "-o", "client"])
        .output()
        .expect("c++ should be available");
    assert_success(&client, "strict C++17 consumer compile");
    run_binary(root, "C++ consumer run");
}

/// Returns `false` only when GNAT is absent and not required.
fn compile_ada(root: &Path, probe: Option<&str>) -> bool {
    if !gnat_available() {
        return false;
    }
    // The wrapper alone: a spec needing no body, with warnings as errors.
    let spec = Command::new("gnatmake")
        .current_dir(root)
        .args(["-q", "-gnatwa", "-gnatwe", "-gnatc", "service_api.ads"])
        .output()
        .expect("gnatmake");
    assert_success(&spec, "GNAT wrapper compile");
    if let Some(probe) = probe {
        std::fs::write(root.join("client.adb"), probe).expect("write Ada probe");
        let client = Command::new("gnatmake")
            .current_dir(root)
            .args(["-q", "-gnata", "client.adb"])
            .output()
            .expect("gnatmake");
        assert_success(&client, "GNAT consumer compile");
        run_binary(root, "Ada consumer run");
    }
    true
}

/// Compile the wrapper in `root` (plus an optional consumer probe) with the
/// real toolchain for `language`.
fn compile_wrapper(language: &str, root: &Path, probe: Option<&str>) {
    match language {
        "rust" => compile_rust(root, probe),
        "cpp" => compile_cpp(root, probe),
        _ => {
            compile_ada(root, probe);
        }
    }
}

/// A zero-OMS service's wrapper stands alone: no model import and no model
/// files, yet it compiles in every language.
#[test]
fn task047_zero_oms_wrapper_compiles_standalone() {
    for (language, _) in LANGUAGES {
        let root = generate("non-uci.yaml", language, "zero-oms");
        assert!(model_files(&root).is_empty(), "{language}");
        compile_wrapper(language, &root, None);
    }
}

// ---------------------------------------------------------------------
// The verbatim upstream contract
// ---------------------------------------------------------------------

/// Per-language expectations for the upstream PositionReport wrapper:
/// authored facts, the exact Payload binding, and a consumer probe.
fn upstream_expectations(language: &str) -> (&'static [&'static str], &'static str, &'static str) {
    match language {
        "rust" => (
            &[
                "#[path = \"oam.rs\"]",
                "pub const SERVICE_NAME: &str = \"example-service\";",
                "pub const SERVICE_VERSION: &str = \"0.1.0\";",
                "pub const SERVICE_KIND: &str = \"service\";",
                "pub mod function_position_input_example {",
                "pub const ID: &str = \"position-input-example\";",
                "pub const NAME: &str = \"Position Input Example\";",
                "pub mod exchange_position_report_input {",
                "pub const ID: &str = \"position-report-input\";",
                "pub const KIND: &str = \"oms_message\";",
                "pub const DIRECTION: &str = \"input\";",
                "pub const MANDATE: &str = \"mandatory\";",
                "pub const TOPIC: &str = \"PositionReport\";",
            ],
            "pub type Payload = super::super::super::model::PositionReportMT;",
            concat!(
                "use service_api::service_api::function_position_input_example::",
                "exchange_position_report_input as position;\n\n",
                "fn accepts_payload(_: position::Payload) {}\n",
                "fn same(value: position::Payload) -> service_api::model::PositionReportMT { value }\n\n",
                "fn main() {\n",
                "    assert_eq!(position::TOPIC, \"PositionReport\");\n",
                "    assert_eq!(position::DIRECTION, \"input\");\n",
                "    let _ = accepts_payload as fn(position::Payload);\n",
                "    let _ = same as fn(position::Payload) -> service_api::model::PositionReportMT;\n",
                "}\n",
            ),
        ),
        "cpp" => (
            &[
                "#include \"oam.hpp\"",
                "inline constexpr std::string_view service_name = \"example-service\";",
                "namespace function_position_input_example {",
                "inline constexpr std::string_view id = \"position-input-example\";",
                "namespace exchange_position_report_input {",
                "inline constexpr std::string_view kind = \"oms_message\";",
                "inline constexpr std::string_view direction = \"input\";",
                "inline constexpr std::string_view mandate = \"mandatory\";",
                "inline constexpr std::string_view topic = \"PositionReport\";",
            ],
            "using Payload = ::programs::oam::PositionReportMT;",
            concat!(
                "#include \"service_api.hpp\"\n#include <type_traits>\n",
                "namespace endpoint =\n    service_api::function_position_input_example::\n",
                "        exchange_position_report_input;\n",
                "static_assert(endpoint::topic == \"PositionReport\");\n",
                "static_assert(endpoint::direction == \"input\");\n",
                "using Payload = endpoint::Payload;\n",
                "static_assert(std::is_same_v<Payload, ::programs::oam::PositionReportMT>);\n",
                "int main() { return 0; }\n",
            ),
        ),
        _ => (
            &[
                "with Programs.Oam;",
                "Service_Name    : constant String := \"example-service\";",
                "package Function_Position_Input_Example is",
                "Id   : constant String := \"position-input-example\";",
                "package Exchange_Position_Report_Input is",
                "Kind      : constant String := \"oms_message\";",
                "Direction : constant String := \"input\";",
                "Mandate   : constant String := \"mandatory\";",
                "Topic     : constant String := \"PositionReport\";",
            ],
            "subtype Payload is Programs.Oam.PositionReportMT;",
            concat!(
                "with Programs.Oam;\nwith Service_API;\nprocedure Client is\n",
                "   package Position renames\n",
                "     Service_API.Function_Position_Input_Example.Exchange_Position_Report_Input;\n",
                "   procedure Accepts_Model (Value : Programs.Oam.PositionReportMT) is null;\n",
                "   procedure Pass_Through (Value : Position.Payload) is\n",
                "   begin\n      Accepts_Model (Value);\n   end Pass_Through;\n",
                "   pragma Unreferenced (Pass_Through);\n",
                "begin\n",
                "   if Position.Topic /= \"PositionReport\"\n",
                "     or else Position.Direction /= \"input\"\n   then\n",
                "      raise Program_Error;\n   end if;\nend Client;\n",
            ),
        ),
    }
}

/// The upstream `upstream-minimal.yaml`, unmodified, against a UCI-shaped
/// synthetic schema. The wrapper states exactly the authored interface facts,
/// binds `Payload` to the resolved payload `PositionReportMT` (not the
/// message name), and a consumer compiles in every language.
///
/// The same contract against the real pinned UCI 2.5 release is recorded in
/// `docs/task-047-service-api-wrappers.md`; that release is not in CI.
#[test]
fn task047_upstream_contract_wrapper_states_the_authored_facts() {
    let contract =
        workspace_root().join("crates/service-contract/tests/fixtures/upstream-minimal.yaml");
    for (language, file) in LANGUAGES {
        let root = output_dir(&format!("upstream-{language}"));
        let output = cli()
            .arg("service-generate")
            .arg("--schema")
            .arg(fixture("position-report.xsd"))
            .arg("--contract")
            .arg(&contract)
            .args(["--language", language, "--world", "closed-schema"])
            .arg("--output")
            .arg(&root)
            .output()
            .expect("CLI should run");
        assert_success(&output, "upstream service-generate");
        let source = wrapper(&root, file);
        let (facts, payload, probe) = upstream_expectations(language);
        for fact in facts {
            assert!(
                source.contains(fact),
                "{language} missing {fact:?}:\n{source}"
            );
        }
        assert_eq!(source.matches(payload).count(), 1, "{language}:\n{source}");
        // Exactly one function scope and one exchange scope: nothing
        // invented. Counted at line start so C++ closing comments
        // (`}  // namespace function_...`) are not mistaken for scopes.
        let (function, exchange, _) = scope_prefixes(language);
        let opened = |prefix: &str| {
            source
                .lines()
                .filter(|line| line.trim_start().starts_with(prefix))
                .count()
        };
        assert_eq!(opened(function), 1, "{language}");
        assert_eq!(opened(exchange), 1, "{language}");
        compile_wrapper(language, &root, Some(probe));
    }
}

/// The Rust wrapper compiles as a crate root under `-D warnings`, and a
/// separate client crate consumes endpoints: constants, and `Payload` aliases
/// type-checked against the generated model type and each other.
#[test]
fn task047_rust_wrapper_compiles_and_is_consumable() {
    let root = generate("api-all-kinds.yaml", "rust", "rust-consumer");
    compile_rust(
        &root,
        Some(concat!(
            "use service_api::service_api::function_mission_backup::exchange_a_repeat as a_repeat;\n",
            "use service_api::service_api::function_mission_data::exchange_a_input as a_input;\n",
            "use service_api::service_api::function_mission_data::exchange_bulk_imagery as bulk;\n\n",
            "fn accepts_payload(_: a_input::Payload) {}\n",
            "fn same_type(value: a_repeat::Payload) -> service_api::model::PayloadA { value }\n",
            "fn interchangeable(value: a_input::Payload) -> a_repeat::Payload { value }\n\n",
            "fn main() {\n",
            "    assert_eq!(service_api::service_api::SERVICE_KIND, \"subsystem\");\n",
            "    assert_eq!(a_input::TOPIC, \"mission.a\");\n",
            "    assert_eq!(a_input::DIRECTION, \"input\");\n",
            "    assert_eq!(a_repeat::TOPIC, \"backup.a\");\n",
            "    assert_eq!(a_repeat::MANDATE, \"optional\");\n",
            "    assert_eq!(bulk::KIND, \"data_transfer\");\n",
            "    let _ = accepts_payload as fn(a_input::Payload);\n",
            "    let _ = same_type as fn(a_repeat::Payload) -> service_api::model::PayloadA;\n",
            "    let _ = interchangeable as fn(a_input::Payload) -> a_repeat::Payload;\n",
            "}\n",
        )),
    );
}

/// The C++ wrapper compiles under strict C++17 with `-Werror`, and a consumer
/// checks constants and `Payload` at compile time with `static_assert`.
#[test]
fn task047_cpp_wrapper_compiles_under_strict_cpp17_and_is_consumable() {
    let root = generate("api-all-kinds.yaml", "cpp", "cpp-consumer");
    compile_cpp(
        &root,
        Some(concat!(
            "#include \"service_api.hpp\"\n",
            "#include <type_traits>\n\n",
            "namespace a_input = service_api::function_mission_data::exchange_a_input;\n",
            "namespace a_repeat = service_api::function_mission_backup::exchange_a_repeat;\n",
            "namespace bulk = service_api::function_mission_data::exchange_bulk_imagery;\n\n",
            "static_assert(service_api::service_kind == \"subsystem\");\n",
            "static_assert(a_input::topic == \"mission.a\");\n",
            "static_assert(a_input::direction == \"input\");\n",
            "static_assert(a_repeat::topic == \"backup.a\");\n",
            "static_assert(a_repeat::mandate == \"optional\");\n",
            "static_assert(bulk::kind == \"data_transfer\");\n",
            "static_assert(std::is_same_v<a_input::Payload, ::urn::test::PayloadA>);\n",
            "static_assert(std::is_same_v<a_input::Payload, a_repeat::Payload>);\n\n",
            "int main() { return 0; }\n",
        )),
    );
}

/// The Ada wrapper compiles under GNAT as a body-less spec with warnings as
/// errors, and a consumer checks constants and uses each `Payload` subtype
/// interchangeably with the generated model type. Gated in CI with
/// `require_one_test`, so it can never pass by skipping.
#[test]
fn task047_ada_wrapper_compiles_under_gnat_and_is_consumable() {
    let root = generate("api-all-kinds.yaml", "ada", "ada-consumer");
    assert!(
        !root.join("service_api.adb").exists(),
        "a metadata-only wrapper needs no package body"
    );
    compile_ada(
        &root,
        Some(concat!(
            "with Urn.Test;\n",
            "with Service_API;\n",
            "procedure Client is\n",
            "   package A_Input renames Service_API.Function_Mission_Data.Exchange_A_Input;\n",
            "   package A_Repeat renames Service_API.Function_Mission_Backup.Exchange_A_Repeat;\n",
            "   package Bulk renames Service_API.Function_Mission_Data.Exchange_Bulk_Imagery;\n",
            "   procedure Accepts_Model (Value : Urn.Test.PayloadA) is null;\n",
            "   procedure Accepts_Repeat (Value : A_Repeat.Payload) is null;\n",
            "   procedure Pass_Through (Value : A_Input.Payload) is\n",
            "   begin\n",
            "      Accepts_Model (Value);\n",
            "      Accepts_Repeat (Value);\n",
            "   end Pass_Through;\n",
            "   pragma Unreferenced (Pass_Through);\n",
            "begin\n",
            "   if Service_API.Service_Kind /= \"subsystem\"\n",
            "     or else A_Input.Topic /= \"mission.a\"\n",
            "     or else A_Input.Direction /= \"input\"\n",
            "     or else A_Repeat.Topic /= \"backup.a\"\n",
            "     or else A_Repeat.Mandate /= \"optional\"\n",
            "     or else Bulk.Kind /= \"data_transfer\"\n",
            "   then\n",
            "      raise Program_Error;\n",
            "   end if;\n",
            "end Client;\n",
        )),
    );
}

// ---------------------------------------------------------------------
// Task 047 corrective: the model and the wrapper as one artifact set
// ---------------------------------------------------------------------

/// `service-check` / `service-generate` against one `artifact-*.xsd` schema
/// and the shared single-exchange `artifact-boundary.yaml` contract.
fn run_artifact(command: &str, schema: &str, language: &str, output: Option<&Path>) -> Output {
    let mut invocation = cli();
    invocation
        .arg(command)
        .arg("--schema")
        .arg(fixture(schema))
        .arg("--contract")
        .arg(fixture("artifact-boundary.yaml"))
        .args(["--language", language, "--world", "closed-schema"]);
    if let Some(output) = output {
        invocation.args([OsString::from("--output"), OsString::from(output)]);
    }
    invocation.output().expect("CLI should run")
}

/// The corrective brief's two predicted collisions, `urn:test:serviceApi`
/// (model file `service_api.*`?) and `urn:serviceApi:serviceName` (C++
/// namespace `service_api::service_name`?), do not occur: the shared layout
/// rule splits the URI on every non-alphanumeric and lowercases, giving
/// `serviceapi.*` and `serviceapi::servicename`. Both are READY, generate
/// distinct files, and compile in every language -- READY is truthful and
/// no false positive is introduced.
#[test]
fn task047c_wrapper_look_alike_namespaces_are_ready_and_compile() {
    for (schema, expected) in [
        (
            "artifact-service-api-stem.xsd",
            [
                ("rust", &["service_api.rs", "serviceapi.rs"][..]),
                ("cpp", &["service_api.hpp", "serviceapi.hpp"][..]),
                (
                    "ada",
                    &["service_api.ads", "test-serviceapi.ads", "test.ads"][..],
                ),
            ],
        ),
        (
            "artifact-service-api-namespace.xsd",
            [
                ("rust", &["service_api.rs", "servicename.rs"][..]),
                ("cpp", &["service_api.hpp", "servicename.hpp"][..]),
                (
                    "ada",
                    &[
                        "service_api.ads",
                        "serviceapi-servicename.ads",
                        "serviceapi.ads",
                    ][..],
                ),
            ],
        ),
    ] {
        for (language, files_expected) in expected {
            let check = run_artifact("service-check", schema, language, None);
            assert_eq!(check.status.code(), Some(0), "{language} {schema}");
            let root = output_dir(&format!("lookalike-{schema}-{language}"));
            let generated = run_artifact("service-generate", schema, language, Some(&root));
            assert_success(&generated, "look-alike service-generate");
            let mut written = files(&root)
                .into_iter()
                .map(|(path, _)| path)
                .collect::<Vec<_>>();
            written.sort();
            let mut files_expected = files_expected.to_vec();
            files_expected.sort_unstable();
            assert_eq!(written, files_expected, "{language} {schema}");
            if language == "cpp" && schema == "artifact-service-api-namespace.xsd" {
                // The model namespace is `serviceapi::servicename`, not
                // `service_api::service_name`, so nothing is redeclared.
                assert!(
                    wrapper(&root, "servicename.hpp")
                        .contains("namespace serviceapi::servicename {")
                );
                compile_cpp(
                    &root,
                    Some(concat!(
                        "#include \"service_api.hpp\"\n#include <type_traits>\n",
                        "static_assert(service_api::service_name == \"artifact-boundary\");\n",
                        "static_assert(std::is_same_v<\n",
                        "    service_api::function_mission_data::exchange_a_input::Payload,\n",
                        "    ::serviceapi::servicename::PayloadA>);\n",
                        "int main() { return 0; }\n",
                    )),
                );
            } else {
                compile_wrapper(language, &root, None);
            }
        }
    }
}

/// The real gap the corrective closes. An Ada model whose parent package is
/// spelled like a wrapper identifier visible at a `Payload` subtype is
/// hidden there, so `subtype Payload is Name.Model.PayloadA;` does not
/// compile. On the reviewed Task 047 head these were READY, generated, and
/// failed under GNAT. Now: NOT READY with a typed `service api boundary:`
/// line, ordinary counts, and nothing written. Rust and C++ are unaffected
/// (their model is not named from inside the wrapper by that identifier).
#[test]
fn task047c_ada_model_package_hidden_by_wrapper_name_is_not_ready() {
    for (schema, parent, hider, region) in [
        (
            "artifact-ada-name-parent.xsd",
            "Name",
            "fixed name 'Name'",
            "scope of function 'mission-data'",
        ),
        (
            "artifact-ada-payload-parent.xsd",
            "Payload",
            "fixed name 'Payload'",
            "scope of function 'mission-data' exchange 'a-input'",
        ),
    ] {
        let check = run_artifact("service-check", schema, "ada", None);
        assert_eq!(check.status.code(), Some(1), "{schema}");
        let report = stdout_of(&check);
        for line in [
            "selected oms messages: 1\n",
            "renderable selected oms messages: 1\n",
            "selected type closure: 1\n",
            "renderable selected types: 1\n",
            "status: NOT READY\n",
        ] {
            assert!(report.contains(line), "{line:?} in\n{report}");
        }
        let boundary = report
            .lines()
            .find(|line| line.starts_with("service api boundary: "))
            .unwrap_or_else(|| panic!("no boundary line in\n{report}"));
        assert_eq!(
            boundary,
            format!(
                "service api boundary: Ada service API {hider} declares '{parent}' (in the \
                 {region}), which hides '{parent}' of the model package '{parent}.Model' where \
                 the wrapper names it in Service_API.Function_Mission_Data.Exchange_A_Input"
            )
        );
        assert!(!report.contains("unsupported selected types:"), "{report}");
        assert!(!report.contains("blocked selected messages:"), "{report}");

        let root = output_dir(&format!("ada-hidden-{parent}"));
        let generated = run_artifact("service-generate", schema, "ada", Some(&root));
        assert_eq!(generated.status.code(), Some(1), "{schema}");
        assert!(!root.exists(), "{schema}: nothing may be written");
        assert_eq!(stdout_of(&generated), report);
        // Readiness stopped it: the late writer diagnostic never appears.
        assert!(
            !String::from_utf8_lossy(&generated.stderr).contains("duplicate generated path"),
            "{schema}"
        );

        for language in ["rust", "cpp"] {
            assert_eq!(
                run_artifact("service-check", schema, language, None)
                    .status
                    .code(),
                Some(0),
                "{language} {schema}"
            );
            let root = output_dir(&format!("ada-hidden-{parent}-{language}"));
            assert_success(
                &run_artifact("service-generate", schema, language, Some(&root)),
                "non-Ada service-generate",
            );
            compile_wrapper(language, &root, None);
        }
    }
}

/// Defence in depth: a caller that skips readiness and invokes the backend
/// wrapper renderer directly still fails closed with the same typed
/// diagnostic, instead of emitting source known not to compile beside its
/// model. The lowered model itself is valid: payload binding succeeds.
#[test]
fn task047c_direct_backend_call_fails_closed_on_the_artifact_boundary() {
    use ams_gra_oms_codegen_core::{
        Backend, BackendLanguage, GenerationWorld, ServiceApiError, build_service_api_model,
        project_service_generation_schema, resolve_service_plan, service_api_preflight,
    };
    let world = GenerationWorld::ClosedSchemaSet;
    let contract = ams_gra_oms_service_contract::load_contract(&fixture("artifact-boundary.yaml"))
        .expect("contract");
    let schema = ams_gra_oms_xsd_frontend::load_schema_set_with_overlays(
        &fixture("artifact-ada-name-parent.xsd"),
        &[],
    )
    .expect("schema");
    let plan = resolve_service_plan(&contract, &schema).expect("plan");
    let projection = project_service_generation_schema(&plan, &schema, world).expect("project");
    let model = build_service_api_model(&plan, projection.schema(), world).expect("lowering");

    let preflight = service_api_preflight(&plan, projection.schema(), BackendLanguage::Ada, world)
        .expect_err("the shared preflight rejects the Ada artifact set");
    assert!(
        matches!(preflight, ServiceApiError::ModelWrapperNameCollision(_)),
        "{preflight:?}"
    );
    let direct = ams_gra_oms_backend_ada::AdaBackend
        .generate_service_api(&model, projection.schema())
        .expect_err("the backend must not render a wrapper hidden from its model");
    assert_eq!(direct.message, preflight.to_string());

    // The same model is fine for Rust and C++, whose wrappers never name the
    // model through the Ada parent package.
    for backend in [
        Box::new(ams_gra_oms_backend_rust::RustBackend) as Box<dyn Backend>,
        Box::new(ams_gra_oms_backend_cpp::CppBackend),
    ] {
        assert!(
            backend
                .generate_service_api(&model, projection.schema())
                .is_ok(),
            "{}",
            backend.name()
        );
    }
}
