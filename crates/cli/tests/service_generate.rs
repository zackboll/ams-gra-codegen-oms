//! CLI `service-generate` integration tests.
//!
//! Fixtures live under `tests/fixtures/service-generate` at the workspace
//! root. `root.xsd` deliberately contains unrenderable and helper-provoking
//! declarations that no contract here selects, so these tests pin the Task 032
//! value proposition: full-schema `generate` FAILS on that schema set while
//! `service-check` reports READY and `service-generate` succeeds, and the
//! selected output carries no helper that only an unselected declaration
//! would have needed.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
    let path = std::env::temp_dir().join(format!("ams-gra-oms-task032-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn generate(schema: &str, contract: &str, language: &str, world: &str, output: &Path) -> Output {
    cli()
        .arg("service-generate")
        .arg("--schema")
        .arg(fixture(schema))
        .arg("--contract")
        .arg(fixture(contract))
        .args([
            OsString::from("--language"),
            OsString::from(language),
            OsString::from("--world"),
            OsString::from(world),
            OsString::from("--output"),
            OsString::from(output),
        ])
        .output()
        .expect("CLI should run")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("report should be UTF-8")
}

/// Every generated file's relative path and contents, sorted for comparison.
fn generated_files(root: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort();
    files
}

fn collect(root: &Path, directory: &Path, files: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else {
            files.push((
                path.strip_prefix(root)
                    .expect("generated path is under the output root")
                    .display()
                    .to_string(),
                std::fs::read_to_string(&path).expect("generated file should be UTF-8"),
            ));
        }
    }
}

fn single_source(root: &Path) -> String {
    let files = generated_files(root);
    assert_eq!(files.len(), 1, "expected exactly one generated file");
    files[0].1.clone()
}

// ---------------------------------------------------------------------
// Sections 33/57 -- the central value demonstration
// ---------------------------------------------------------------------

/// Section 33: full-schema `generate` FAILS on this schema set, yet the same
/// set's selected model generates. This is the whole point of the command.
#[test]
fn full_schema_generate_fails_while_selected_generation_succeeds() {
    let full = output_dir("full-schema");
    let failure = cli()
        .arg("generate")
        .arg("--schema")
        .arg(fixture("root.xsd"))
        .args(["--language", "rust", "--world", "closed-schema"])
        .arg("--output")
        .arg(&full)
        .output()
        .expect("CLI should run");
    assert_eq!(failure.status.code(), Some(1));

    let selected = output_dir("value-demo");
    let output = generate("root.xsd", "both.yaml", "rust", "closed-schema", &selected);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let source = single_source(&selected);
    // The selected closure is present...
    for present in [
        "PayloadA",
        "PayloadB",
        "SharedType",
        "OnlyBType",
        "LabelType",
    ] {
        assert!(source.contains(present), "missing selected {present}");
    }
    // ...and the unrelated declarations that break full-schema generation are
    // not, so selection is a genuine narrowing rather than a retry.
    for absent in [
        "UnrelatedDuration",
        "UnrelatedUnboundedType",
        "UnrelatedBinaryType",
        "UnrelatedFloatType",
    ] {
        assert!(!source.contains(absent), "leaked unselected {absent}");
    }
}

/// Section 57: the success report names every required field, and reports
/// contract-selected and generated-support counts SEPARATELY so support types
/// are never presented as contract selections.
#[test]
fn ready_report_states_selection_and_support_separately() {
    let output_root = output_dir("ready-report");
    let output = generate(
        "root.xsd",
        "both.yaml",
        "rust",
        "closed-schema",
        &output_root,
    );
    assert_eq!(output.status.code(), Some(0));
    let stdout = stdout_of(&output);

    assert!(stdout.contains("service contract valid"));
    assert!(stdout.contains("service: Selected Generation Both"));
    assert!(stdout.contains("language: rust"));
    assert!(stdout.contains("generation world: closed-schema"));
    assert!(stdout.contains("selected oms messages: 2"));
    assert!(stdout.contains("contract-selected types: 5"));
    assert!(stdout.contains("generated support types: 0"));
    assert!(stdout.contains("projected schema types: 5"));
    assert!(stdout.contains("generated 1 file(s)"));
    assert!(stdout.contains(&format!("output: {}", output_root.display())));
}

// ---------------------------------------------------------------------
// Sections 32/38/39/40/60 -- what the selected model contains
// ---------------------------------------------------------------------

/// Sections 32/60: helper emission follows the PROJECTED schema. The only
/// unbounded occurrence, the only binary value, and the only floating value
/// in this schema set all live in unselected declarations, so no unbounded
/// sequence helper may appear in the selected Rust output.
#[test]
fn unselected_declarations_contribute_no_helpers() {
    let output_root = output_dir("helpers");
    assert_eq!(
        generate(
            "root.xsd",
            "both.yaml",
            "rust",
            "closed-schema",
            &output_root
        )
        .status
        .code(),
        Some(0)
    );
    let source = single_source(&output_root);

    assert!(
        !source.contains("UnboundedVec"),
        "an unselected unbounded field must not emit the unbounded helper"
    );
    assert!(
        !source.contains("Vec<u8>"),
        "no selected Binary value exists"
    );
    assert!(!source.contains("f64"), "no selected floating value exists");
}

/// Section 38: narrowing the contract narrows the generated model. Everything
/// reachable only from MessageB disappears rather than surviving because it
/// exists in the root schema.
#[test]
fn contract_narrowing_removes_declarations_reachable_only_from_the_dropped_message() {
    let both = output_dir("narrow-both");
    let only_a = output_dir("narrow-only-a");
    assert_eq!(
        generate("root.xsd", "both.yaml", "rust", "closed-schema", &both)
            .status
            .code(),
        Some(0)
    );
    assert_eq!(
        generate("root.xsd", "only-a.yaml", "rust", "closed-schema", &only_a)
            .status
            .code(),
        Some(0)
    );

    let wide = single_source(&both);
    let narrow = single_source(&only_a);
    assert!(wide.contains("OnlyBType") && wide.contains("PayloadB"));
    assert!(!narrow.contains("OnlyBType"));
    assert!(!narrow.contains("PayloadB"));
    // The shared dependency survives in both.
    assert!(narrow.contains("SharedType"));
}

/// Section 39: MessageA is selected from two exchanges in `both.yaml`, so a
/// naive per-exchange projection would emit its types twice.
#[test]
fn repeated_message_selection_emits_each_type_once() {
    let output_root = output_dir("dedupe");
    assert_eq!(
        generate(
            "root.xsd",
            "both.yaml",
            "rust",
            "closed-schema",
            &output_root
        )
        .status
        .code(),
        Some(0)
    );
    let source = single_source(&output_root);
    assert_eq!(source.matches("pub struct PayloadA {").count(), 1);
    assert_eq!(source.matches("pub struct SharedType {").count(), 1);
}

/// Section 40: reversing contract exchange order while selecting the SAME
/// message set leaves generated output byte-identical, because type order
/// follows SchemaIr rather than contract presentation.
#[test]
fn reversed_exchange_order_generates_identical_output() {
    let forward = output_dir("order-forward");
    let reverse = output_dir("order-reverse");
    assert_eq!(
        generate("root.xsd", "both.yaml", "rust", "closed-schema", &forward)
            .status
            .code(),
        Some(0)
    );
    assert_eq!(
        generate(
            "root.xsd",
            "both-reversed.yaml",
            "rust",
            "closed-schema",
            &reverse
        )
        .status
        .code(),
        Some(0)
    );
    assert_eq!(generated_files(&forward), generated_files(&reverse));
}

/// Sections 37/77: two runs on identical inputs produce byte-identical files,
/// and identical stdout apart from the reported output path.
#[test]
fn generated_output_is_byte_deterministic() {
    let first = output_dir("determinism-1");
    let second = output_dir("determinism-2");
    let a = generate("root.xsd", "both.yaml", "rust", "closed-schema", &first);
    let b = generate("root.xsd", "both.yaml", "rust", "closed-schema", &second);
    assert_eq!(a.status.code(), Some(0));
    assert_eq!(b.status.code(), Some(0));

    assert_eq!(generated_files(&first), generated_files(&second));
    assert_eq!(
        stdout_of(&a).replace(&first.display().to_string(), "OUT"),
        stdout_of(&b).replace(&second.display().to_string(), "OUT")
    );
}

/// Sections 17/89: with no abstract value in the selection, the two worlds
/// produce byte-identical selected output.
#[test]
fn concrete_only_selection_generates_identically_in_both_worlds() {
    let closed = output_dir("world-closed");
    let open = output_dir("world-open");
    assert_eq!(
        generate("root.xsd", "both.yaml", "rust", "closed-schema", &closed)
            .status
            .code(),
        Some(0)
    );
    assert_eq!(
        generate("root.xsd", "both.yaml", "rust", "open-extensions", &open)
            .status
            .code(),
        Some(0)
    );
    assert_eq!(generated_files(&closed), generated_files(&open));
}

// ---------------------------------------------------------------------
// Sections 27/28/45/46/55/56/58 -- readiness gate and world behavior
// ---------------------------------------------------------------------

/// Sections 27/28/56: a NOT READY selection writes the SAME readiness report
/// `service-check` writes, emits one concise stderr diagnostic, exits 1, and
/// leaves no file -- not even the output directory.
#[test]
fn not_ready_selection_writes_nothing() {
    let output_root = output_dir("not-ready");
    let output = generate(
        "abstract.xsd",
        "abstract.yaml",
        "rust",
        "open-extensions",
        &output_root,
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(!output_root.exists(), "no write phase may have begun");

    let stdout = stdout_of(&output);
    assert!(stdout.contains("status: NOT READY"));
    assert!(stdout.contains("blocked selected messages:"));

    // Byte-identical to the 'service-check' report for the same inputs.
    let check = cli()
        .arg("service-check")
        .arg("--schema")
        .arg(fixture("abstract.xsd"))
        .arg("--contract")
        .arg(fixture("abstract.yaml"))
        .args(["--language", "rust", "--world", "open-extensions"])
        .output()
        .expect("CLI should run");
    assert_eq!(stdout, stdout_of(&check));

    let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
    assert!(stderr.contains("no files were generated"));
    assert_eq!(stderr.lines().count(), 1, "one concise diagnostic only");
}

/// Sections 42/43/44/89: closed-world abstract-value selection generates for
/// every backend, with the closed sum carrying every concrete descendant and
/// no abstract intermediate as a variant.
#[test]
fn closed_world_abstract_selection_generates_the_full_closed_sum() {
    for language in ["ada", "rust", "cpp"] {
        let output_root = output_dir(&format!("abstract-{language}"));
        let output = generate(
            "abstract.xsd",
            "abstract.yaml",
            language,
            "closed-schema",
            &output_root,
        );
        assert_eq!(output.status.code(), Some(0), "{language}");

        let stdout = stdout_of(&output);
        // Holder + Base are selected; the five descendants plus the nested
        // BaseB sum and NestedNote are generated support.
        assert!(stdout.contains("contract-selected types: 2"), "{language}");
        assert!(stdout.contains("generated support types: 8"), "{language}");

        let combined = generated_files(&output_root)
            .into_iter()
            .map(|(_, contents)| contents)
            .collect::<String>();
        for descendant in [
            "ConcreteA",
            "ConcreteParent",
            "ConcreteLeaf",
            "MiddleLeaf",
            "ConcreteB",
        ] {
            assert!(combined.contains(descendant), "{language} {descendant}");
        }
    }
}

/// Sections 46/48/49: the Task 026 zero-descendant optional generates under
/// closed-schema (ordinary elision applies) and is NOT READY under
/// open-extensions, where nothing may be written.
#[test]
fn zero_descendant_optional_follows_task_026_world_semantics() {
    let closed = output_dir("uninhabited-closed");
    let closed_output = generate(
        "uninhabited.xsd",
        "uninhabited.yaml",
        "rust",
        "closed-schema",
        &closed,
    );
    assert_eq!(closed_output.status.code(), Some(0));
    let source = single_source(&closed);
    // The abstract declaration is retained in the projected model, but Task
    // 026 elides its storage rather than inventing a payload.
    assert!(source.contains("pub struct OptionalHolder"));
    assert!(!source.contains("SidecarPoint"));

    let open = output_dir("uninhabited-open");
    let open_output = generate(
        "uninhabited.xsd",
        "uninhabited.yaml",
        "rust",
        "open-extensions",
        &open,
    );
    assert_eq!(open_output.status.code(), Some(1));
    assert!(!open.exists());
}

/// Section 58: a contract with zero OMS Message exchanges succeeds and emits
/// zero UCI type files rather than a fabricated boilerplate-only file.
#[test]
fn non_uci_only_contract_generates_no_type_files() {
    let output_root = output_dir("non-uci");
    let output = generate(
        "root.xsd",
        "non-uci.yaml",
        "rust",
        "closed-schema",
        &output_root,
    );
    assert_eq!(output.status.code(), Some(0));

    let stdout = stdout_of(&output);
    assert!(stdout.contains("selected oms messages: 0"));
    assert!(stdout.contains("contract-selected types: 0"));
    assert!(stdout.contains("generated support types: 0"));
    assert!(stdout.contains("generated 0 file(s)"));
    assert!(
        !output_root.exists(),
        "zero files means nothing is created at all"
    );
}

// ---------------------------------------------------------------------
// Sections 47/48/49/50 -- extension mapping reuse
// ---------------------------------------------------------------------

/// The Task 030 private-overlay fixtures, whose contract declares two logical
/// extension IDs that only `--extension ID=PATH` can bind.
fn plan_fixture(name: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures/service-plan")
        .join(name)
}

fn generate_with_extensions(extension_args: &[&str], output: &Path) -> Output {
    let mut command = cli();
    command
        .arg("service-generate")
        .arg("--schema")
        .arg(plan_fixture("root.xsd"))
        .arg("--contract")
        .arg(plan_fixture("service-extension.yaml"));
    for argument in extension_args {
        command.arg(argument);
    }
    command
        .args(["--language", "rust", "--world", "closed-schema"])
        .arg("--output")
        .arg(output)
        .output()
        .expect("CLI should run")
}

fn extension_mapping(id: &str, fixture_name: &str) -> String {
    format!("{id}={}", plan_fixture(fixture_name).display())
}

/// Sections 47/48: a private message reachable only through an overlay
/// generates normally, and REVERSING the --extension command-line order
/// leaves output byte-identical because Task 030 contract order -- not shell
/// history -- drives Task 029 overlay composition.
#[test]
fn private_extension_selection_generates_and_ignores_option_order() {
    let a = extension_mapping("ext-a-1.0", "private-overlay.xsd");
    let b = extension_mapping("ext-b-2.0", "private-overlay-b.xsd");

    let forward = output_dir("ext-forward");
    let reverse = output_dir("ext-reverse");
    let forward_output =
        generate_with_extensions(&["--extension", &a, "--extension", &b], &forward);
    let reverse_output =
        generate_with_extensions(&["--extension", &b, "--extension", &a], &reverse);

    assert_eq!(forward_output.status.code(), Some(0));
    assert_eq!(reverse_output.status.code(), Some(0));
    // The private message's own type is generated: no public/private
    // distinction survives schema composition.
    assert!(single_source(&forward).contains("PrivateReportType"));
    assert_eq!(generated_files(&forward), generated_files(&reverse));
}

/// Section 49: the exact-matching rules are reused verbatim, so a missing
/// mapping, an undeclared mapping, a duplicate ID, and a bare path all fail.
#[test]
fn extension_set_must_match_the_contract_exactly() {
    let a = extension_mapping("ext-a-1.0", "private-overlay.xsd");
    let b = extension_mapping("ext-b-2.0", "private-overlay-b.xsd");
    let bare = plan_fixture("private-overlay.xsd").display().to_string();
    let unknown = extension_mapping("ext-z-9.9", "private-overlay.xsd");

    for (label, arguments, expected) in [
        (
            "missing",
            vec!["--extension", a.as_str()],
            "no '--extension ext-b-2.0=PATH' mapping was supplied",
        ),
        (
            "undeclared",
            vec![
                "--extension",
                a.as_str(),
                "--extension",
                b.as_str(),
                "--extension",
                unknown.as_str(),
            ],
            "does not declare extension",
        ),
        (
            "duplicate",
            vec![
                "--extension",
                a.as_str(),
                "--extension",
                a.as_str(),
                "--extension",
                b.as_str(),
            ],
            "duplicate '--extension' identifier",
        ),
        (
            "bare-path",
            vec!["--extension", bare.as_str()],
            "expected ID=PATH mapping",
        ),
    ] {
        let output_root = output_dir(&format!("ext-{label}"));
        let output = generate_with_extensions(&arguments, &output_root);
        assert_ne!(output.status.code(), Some(0), "{label} should fail");
        let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
        assert!(stderr.contains(expected), "{label}: {stderr}");
        assert!(!output_root.exists(), "{label} must write nothing");
    }
}

// ---------------------------------------------------------------------
// Sections 50/51/52/53/83/84 -- command-line surface
// ---------------------------------------------------------------------

/// Sections 51/52/84: every one of --schema, --contract, --language, --world,
/// and --output is required, with a deterministic usage failure naming the
/// missing option. There is no implicit world and no implicit language.
#[test]
fn every_required_option_is_required() {
    let all = [
        ("--schema", fixture("root.xsd").display().to_string()),
        ("--contract", fixture("both.yaml").display().to_string()),
        ("--language", "rust".to_owned()),
        ("--world", "closed-schema".to_owned()),
        (
            "--output",
            output_dir("missing-option").display().to_string(),
        ),
    ];
    for omitted in 0..all.len() {
        let mut command = cli();
        command.arg("service-generate");
        for (index, (option, value)) in all.iter().enumerate() {
            if index != omitted {
                command.arg(option).arg(value);
            }
        }
        let output = command.output().expect("CLI should run");
        assert_eq!(output.status.code(), Some(2), "{}", all[omitted].0);
        let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
        assert!(
            stderr.contains(&format!("missing required option '{}'", all[omitted].0)),
            "{}: {stderr}",
            all[omitted].0
        );
    }
}

/// Sections 50/52/53/84: invalid values and a raw --overlay fail as usage
/// errors, and a repeated singular option is rejected rather than silently
/// letting the last one win.
#[test]
fn invalid_and_rejected_options_fail_deterministically() {
    let output_root = output_dir("bad-options").display().to_string();
    let schema = fixture("root.xsd").display().to_string();
    let contract = fixture("both.yaml").display().to_string();

    for (label, extra, expected) in [
        (
            "language",
            vec!["--language", "go", "--world", "closed-schema"],
            "unsupported language 'go'",
        ),
        (
            "world",
            vec!["--language", "rust", "--world", "closed"],
            "unsupported generation world 'closed'",
        ),
        (
            // Section 50: contract-declared extension IDs stay authoritative.
            "overlay",
            vec![
                "--language",
                "rust",
                "--world",
                "closed-schema",
                "--overlay",
                "extra.xsd",
            ],
            "unknown option '--overlay'",
        ),
        (
            "duplicate-schema",
            vec![
                "--language",
                "rust",
                "--world",
                "closed-schema",
                "--schema",
                "again.xsd",
            ],
            "duplicate option '--schema'",
        ),
        (
            "duplicate-output",
            vec![
                "--language",
                "rust",
                "--world",
                "closed-schema",
                "--output",
                "again",
            ],
            "duplicate option '--output'",
        ),
    ] {
        let output = cli()
            .args([
                "service-generate",
                "--schema",
                &schema,
                "--contract",
                &contract,
                "--output",
                &output_root,
            ])
            .args(extra)
            .output()
            .expect("CLI should run");
        assert_eq!(output.status.code(), Some(2), "{label}");
        let stderr = String::from_utf8(output.stderr).expect("diagnostic should be UTF-8");
        assert!(stderr.contains(expected), "{label}: {stderr}");
    }
}

/// Section 83: dedicated help documents the whole surface and states the
/// non-goals explicitly, so nobody expects a CAL wrapper from this command.
#[test]
fn dedicated_help_documents_the_surface_and_non_goals() {
    let output = cli()
        .args(["service-generate", "--help"])
        .output()
        .expect("CLI should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help should be UTF-8");

    for option in [
        "--schema PATH",
        "--contract PATH",
        "--extension ID=PATH",
        "--language LANGUAGE",
        "--world WORLD",
        "--output DIR",
    ] {
        assert!(stdout.contains(option), "{option}");
    }
    assert!(stdout.contains("no raw --overlay PATH option here"));
    assert!(stdout.contains("READINESS IS CHECKED FIRST:"));
    assert!(stdout.contains("SELECTED TYPES ONLY:"));
    assert!(stdout.contains("No CAL facade"));
}

// ---------------------------------------------------------------------
// Sections 34/35/36/65/89 -- the selected output actually compiles
// ---------------------------------------------------------------------

/// Generate the ready fixture for one language and return the output root.
fn generate_ready(language: &str, label: &str) -> PathBuf {
    let output_root = output_dir(label);
    let output = generate(
        "root.xsd",
        "both.yaml",
        language,
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{language} generation should succeed"
    );
    output_root
}

/// Section 34: the selected Rust source compiles, and a selected payload
/// value can be constructed through the generated public constructors.
#[test]
fn selected_rust_output_compiles_and_constructs_a_payload() {
    let output_root = generate_ready("rust", "compile-rust");
    let probe = output_root.join("probe.rs");
    std::fs::write(
        &probe,
        format!(
            "{}\n\npub fn probe() -> PayloadA {{\n    PayloadA {{ shared: SharedType {{ label: LabelType::Primary, count: BoundedI64::new(1).unwrap() }} }}\n}}\n",
            std::fs::read_to_string(output_root.join("test.rs")).expect("generated Rust source")
        ),
    )
    .expect("write Rust probe");

    let status = Command::new("rustc")
        .current_dir(&output_root)
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--deny",
            "warnings",
        ])
        .arg("probe.rs")
        .status()
        .expect("rustc should be available in a Rust workspace");
    assert!(status.success(), "selected Rust output must compile");
}

/// Section 35: the selected C++ source compiles under strict C++17, and a
/// selected payload value is constructed.
#[test]
fn selected_cpp_output_compiles_under_strict_cpp17() {
    if Command::new("c++").arg("--version").output().is_err() {
        return;
    }
    let output_root = generate_ready("cpp", "compile-cpp");
    let probe = output_root.join("probe.cpp");
    std::fs::write(
        &probe,
        "#include \"test.hpp\"\n\nint main() {\n    const auto count = decltype(urn::test::SharedType::count)::create(1);\n    if (!count) return 1;\n    const urn::test::PayloadA payload{{urn::test::LabelType::Primary, *count}};\n    return payload.shared.count.value() == 1 ? 0 : 1;\n}\n",
    )
    .expect("write C++ probe");

    let status = Command::new("c++")
        .current_dir(&output_root)
        .args([
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-pedantic-errors",
            "-fsyntax-only",
            "probe.cpp",
        ])
        .status()
        .expect("c++ reported a version, so it must be runnable");
    assert!(status.success(), "selected C++ output must compile");
}

/// Section 36: the selected Ada source compiles under GNAT, exercising one
/// selected payload declaration.
///
/// GNAT is not present on the stock CI image, so like the Task 029 overlay
/// probe this compile step is skipped when it is absent; the generation
/// assertions above run unconditionally.
#[test]
fn selected_ada_output_compiles_under_gnat() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        return;
    }
    let output_root = generate_ready("ada", "compile-ada");
    std::fs::write(
        output_root.join("probe.adb"),
        "with Urn.Test; use Urn.Test;\n\nprocedure Probe is\n   Value : constant PayloadA :=\n     (Shared => (Label => Primary, Count => 1));\nbegin\n   pragma Assert (Value.Shared.Count = 1);\n   null;\nend Probe;\n",
    )
    .expect("write Ada probe");

    let status = Command::new("gnatmake")
        .current_dir(&output_root)
        .args(["-gnatwa", "-gnata", "probe.adb"])
        .status()
        .expect("GNAT reported a version, so it must be runnable");
    assert!(status.success(), "selected Ada output must compile");
}

/// Generate the Task 033 constrained-float service for one language.
fn generate_constrained_float(language: &str, label: &str) -> PathBuf {
    let output_root = output_dir(label);
    let output = generate(
        "constrained-float.xsd",
        "constrained-float.yaml",
        language,
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{language} constrained-float generation should succeed"
    );
    output_root
}

/// Task 033 section 45: a contract whose entire selected closure is bound-only
/// constrained floats becomes READY in all three backends.
///
/// The point of this test is *propagation*. No Task 033 change was made to
/// `service_plan.rs`, `service_readiness.rs`, or `service_generation.rs`; the
/// new capability has to arrive through the single shared coverage snapshot
/// and the ordinary backends. The fixture also contains an unselected
/// `xs:duration` that no backend can render, so a READY result here cannot be
/// a whole-schema accident.
#[test]
fn task033_constrained_float_service_is_ready_in_every_backend() {
    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("constrained-float.xsd"))
            .arg("--contract")
            .arg(fixture("constrained-float.yaml"))
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        assert_eq!(
            check.status.code(),
            Some(0),
            "{language} service-check should succeed"
        );
        let report = String::from_utf8(check.stdout).expect("UTF-8 report");
        assert!(
            report.contains("status: READY"),
            "{language} must be READY after Task 033: {report}"
        );
        assert!(
            report.contains("selected type closure: 6")
                && report.contains("renderable selected types: 6"),
            "{language} must render every selected type: {report}"
        );
    }
}

/// Task 033: the selected Rust output compiles and its checked constructors
/// actually reject out-of-domain values.
#[test]
fn task033_selected_rust_float_bounds_hold() {
    let root = generate_constrained_float("rust", "task033-rust");
    std::fs::write(
        root.join("probe.rs"),
        "include!(\"test.rs\");\n\nfn main() {\n    assert!(AltitudeMeters::new(-6378237.0).is_some());\n    assert!(AltitudeMeters::new(-6378238.0).is_none());\n    assert!(BurnRate::new(0.0).is_none());\n    assert!(BurnRate::new(1.0).is_some());\n    assert!(ThrustRatio::new(0.5).is_some());\n    assert!(ThrustRatio::new(1.5).is_none());\n    assert!(DerivedRate::new(10.0).is_some());\n    assert!(DerivedRate::new(10.5).is_none());\n}\n",
    )
    .expect("write Rust probe");
    let status = Command::new("rustc")
        .current_dir(&root)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .status()
        .expect("rustc should be available in a Rust workspace");
    assert!(status.success(), "selected Rust output must compile");
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Rust bounds must hold"
    );
}

/// Task 033: the selected C++ output compiles under strict C++17 and its
/// checked factories reject out-of-domain values.
#[test]
fn task033_selected_cpp_float_bounds_hold() {
    let root = generate_constrained_float("cpp", "task033-cpp");
    std::fs::write(
        root.join("probe.cpp"),
        "#include \"test.hpp\"\n\nint main() {\n    if (urn::test::BurnRate::create(0.0)) return 1;\n    if (!urn::test::BurnRate::create(1.0)) return 1;\n    if (urn::test::AltitudeMeters::create(-6378238.0)) return 1;\n    if (!urn::test::DerivedRate::create(10.0)) return 1;\n    if (urn::test::DerivedRate::create(10.5)) return 1;\n    return 0;\n}\n",
    )
    .expect("write C++ probe");
    let status = Command::new("c++")
        .current_dir(&root)
        .args([
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-pedantic-errors",
            "-o",
            "probe",
            "probe.cpp",
        ])
        .status()
        .expect("c++ must be runnable");
    assert!(status.success(), "selected C++ output must compile");
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected C++ bounds must hold"
    );
}

/// Task 033: the selected Ada output compiles and its predicates are enforced
/// for a client that does *not* pass `-gnata`, which is exactly what the
/// generated spec's own `Assertion_Policy` exists to guarantee.
///
/// The probe drives the private representation through its only public
/// construction path, `Create`, and reads back through `Value`. Neither a
/// scalar conversion nor an inherited operator is available to it.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task033_selected_ada_float_predicates_hold_under_gnat() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        return;
    }
    let root = generate_constrained_float("ada", "task033-ada");
    std::fs::write(
        root.join("probe.adb"),
        "with Urn.Test; use Urn.Test;\nwith Interfaces;\nuse type Interfaces.IEEE_Float_64;\n\nprocedure Probe is\n   function Rejects_Zero_Burn return Boolean is\n      Held : BurnRate;\n   begin\n      Held := Create (0.0);\n      return Value (Held) /= 0.0;\n   exception\n      when others => return True;\n   end Rejects_Zero_Burn;\n\n   Good : constant BurnRate := Create (1.0);\nbegin\n   --  Read back through the public accessor; no conversion is available.\n   if Value (Good) /= 1.0 then\n      raise Program_Error;\n   end if;\n   if not Rejects_Zero_Burn then\n      raise Program_Error;\n   end if;\nend Probe;\n",
    )
    .expect("write Ada probe");
    let status = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "probe.adb"])
        .status()
        .expect("GNAT reported a version, so it must be runnable");
    assert!(status.success(), "selected Ada output must compile");
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Ada predicates must be enforced"
    );
}

/// Task 033 correction: the selected Ada output must not offer a client any
/// unchecked way to build a `BurnRate`, so the two historical bypasses are
/// compile errors rather than runtime failures.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task033_selected_ada_float_rejects_public_bypasses() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        return;
    }
    let root = generate_constrained_float("ada", "task033-ada-bypass");
    for (unit, body, expected) in [
        (
            "direct_conversion",
            "   Bad : BurnRate := BurnRate (0.0);\n",
            "invalid conversion",
        ),
        (
            "inherited_arithmetic",
            "   A : BurnRate := Create (1.0);\n   B : BurnRate := Create (1.0);\n   C : BurnRate := A + B;\n",
            "no applicable operator",
        ),
    ] {
        let file = format!("{unit}.adb");
        let procedure = unit
            .split('_')
            .map(|word| {
                let mut characters = word.chars();
                match characters.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join("_");
        std::fs::write(
            root.join(&file),
            format!(
                "with Urn.Test; use Urn.Test;\n\nprocedure {procedure} is\n{body}begin\n   null;\nend {procedure};\n"
            ),
        )
        .expect("write Ada bypass probe");
        let output = Command::new("gnatmake")
            .current_dir(&root)
            .args(["-q", &file])
            .output()
            .expect("GNAT reported a version, so it must be runnable");
        assert!(
            !output.status.success(),
            "{unit} bypass must not compile against the selected Ada output"
        );
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            diagnostics.contains(expected),
            "{unit} must be rejected for representation hiding \
             (expected a diagnostic mentioning {expected:?}), got:\n{diagnostics}"
        );
    }
}

/// Task 034 section: a contract whose selected closure carries non-nillable
/// `0..1` named-type Record fields becomes READY in all three backends.
///
/// The point of this test is *propagation*. No Task 034 change was made to
/// `service_plan.rs`, `service_readiness.rs`, or `service_generation.rs`; the
/// new capability has to arrive through the single shared coverage snapshot
/// and the ordinary backends. Ada is the meaningful case -- it was NOT READY
/// for this contract before Task 034 -- while Rust and C++ prove nothing
/// regressed.
///
/// The fixture also contains an unselected `xs:duration` that no backend can
/// render, so a READY result here cannot be a whole-schema accident.
#[test]
fn task034_optional_named_service_is_ready_in_every_backend() {
    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("optional-named.xsd"))
            .arg("--contract")
            .arg(fixture("optional-named.yaml"))
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        assert_eq!(
            check.status.code(),
            Some(0),
            "{language} service-check should succeed"
        );
        let report = String::from_utf8(check.stdout).expect("UTF-8 report");
        assert!(
            report.contains("status: READY"),
            "{language} must be READY after Task 034: {report}"
        );
        assert!(
            report.contains("selected type closure: 4")
                && report.contains("renderable selected types: 4"),
            "{language} must render every selected type: {report}"
        );
    }
}

/// The unselected `xs:duration` really is unrenderable, so ordinary
/// whole-schema generation still fails for Ada on the very same file that
/// service-generate handles. Without this, READY above could be explained by
/// the schema simply being fully supported.
#[test]
fn task034_optional_named_whole_schema_generation_still_fails() {
    let output = cli()
        .arg("generate")
        .arg("--schema")
        .arg(fixture("optional-named.xsd"))
        .args(["--language", "ada", "--world", "closed-schema"])
        .arg("--output")
        .arg(output_dir("task034-ada-whole"))
        .output()
        .expect("generate must run");
    assert_ne!(
        output.status.code(),
        Some(0),
        "an unrenderable unselected declaration must still fail whole-schema generation"
    );
    let diagnostics = String::from_utf8(output.stderr).expect("UTF-8 diagnostic");
    assert!(
        diagnostics.contains("UnrelatedDuration"),
        "the unselected declaration must be the stated reason: {diagnostics}"
    );
}

/// Task 034: the selected Ada output really compiles under GNAT, with the
/// per-field wrappers this task generates.
///
/// This is a permanent GNAT-backed gate, so it follows the existing policy:
/// skipped only where GNAT is genuinely absent, and never silently skipped
/// when `AMS_GRA_REQUIRE_GNAT` says CI installed it.
#[test]
fn task034_selected_ada_optional_named_compiles_under_gnat() {
    let output_root = output_dir("task034-ada");
    let output = generate(
        "optional-named.xsd",
        "optional-named.yaml",
        "ada",
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "ada optional-named generation should succeed"
    );

    let spec = std::fs::read_to_string(output_root.join("urn-test.ads"))
        .expect("the selected Ada spec must exist");
    // One wrapper per optional named field, named after the emitted owner.
    for helper in [
        "type SensorPayload_Maybe_Mode_Optional (Is_Present : Boolean := False) is record",
        "type SensorPayload_Maybe_Details_Optional (Is_Present : Boolean := False) is record",
        "type SensorPayload_Maybe_Samples_Optional (Is_Present : Boolean := False) is record",
    ] {
        assert!(spec.contains(helper), "missing {helper}:\n{spec}");
    }
    // The required named field is untouched by Task 034.
    assert!(spec.contains("Required_Mode : Mode;"), "{spec}");

    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
        );
        return;
    }
    let status = Command::new("gnatmake")
        .current_dir(&output_root)
        .args(["-gnatwa", "-c", "urn-test.ads"])
        .status()
        .expect("GNAT reported a version, so it must be runnable");
    assert!(status.success(), "selected Ada output must compile");
}

/// Task 035 section: a contract whose selected closure carries non-nillable
/// `0..1` **direct primitive** Record fields becomes READY in all three
/// backends.
///
/// As in Task 034 the point is *propagation*: no Task 035 change was made to
/// `service_plan.rs`, `service_readiness.rs`, or `service_generation.rs`, so the
/// new capability has to arrive through the single shared coverage snapshot and
/// the ordinary backends. Ada is the meaningful case -- it was NOT READY for
/// this contract before Task 035 -- while Rust and C++ prove nothing regressed.
///
/// The fixture also contains an unselected `xs:duration` that no backend can
/// render, so a READY result here cannot be a whole-schema accident.
#[test]
fn task035_optional_primitive_service_is_ready_in_every_backend() {
    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("optional-primitive.xsd"))
            .arg("--contract")
            .arg(fixture("optional-primitive.yaml"))
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        assert_eq!(
            check.status.code(),
            Some(0),
            "{language} service-check should succeed"
        );
        let report = String::from_utf8(check.stdout).expect("UTF-8 report");
        assert!(
            report.contains("status: READY"),
            "{language} must be READY after Task 035: {report}"
        );
        assert!(
            report.contains("selected type closure: 2")
                && report.contains("renderable selected types: 2"),
            "{language} must render every selected type: {report}"
        );
    }
}

/// The unselected `xs:duration` really is unrenderable, so ordinary whole-schema
/// generation still fails for Ada on the very same file that service-generate
/// handles. Without this, READY above could be explained by the schema simply
/// being fully supported.
#[test]
fn task035_optional_primitive_whole_schema_generation_still_fails() {
    let output = cli()
        .arg("generate")
        .arg("--schema")
        .arg(fixture("optional-primitive.xsd"))
        .args(["--language", "ada", "--world", "closed-schema"])
        .arg("--output")
        .arg(output_dir("task035-ada-whole"))
        .output()
        .expect("generate must run");
    assert_ne!(
        output.status.code(),
        Some(0),
        "an unrenderable unselected declaration must still fail whole-schema generation"
    );
    let diagnostics = String::from_utf8(output.stderr).expect("UTF-8 diagnostic");
    assert!(
        diagnostics.contains("UnrelatedDuration"),
        "the unselected declaration must be the stated reason: {diagnostics}"
    );
}

/// Task 035: the selected Ada output really compiles under GNAT, with the
/// per-field direct primitive wrappers this task generates -- including the
/// inherited `Version` field, whose wrapper is named after the **emitted**
/// owner. That is the synthetic counterpart of the UCI
/// `MissionID_Type`/`VersionedID_Type` boundary Task 035 unblocked, expressed
/// without special-casing any UCI name.
#[test]
fn task035_selected_ada_optional_primitive_compiles_under_gnat() {
    let output_root = output_dir("task035-ada");
    let output = generate(
        "optional-primitive.xsd",
        "optional-primitive.yaml",
        "ada",
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "ada optional-primitive generation should succeed"
    );

    let spec = std::fs::read_to_string(output_root.join("urn-test.ads"))
        .expect("the selected Ada spec must exist");
    // One wrapper per optional direct primitive field, named after the emitted
    // owner -- including the field inherited from `VersionedBase`.
    for helper in [
        "type PrimitivePayload_Version_Optional (Is_Present : Boolean := False) is record",
        "type PrimitivePayload_Maybe_Bool_Optional (Is_Present : Boolean := False) is record",
        "type PrimitivePayload_Maybe_Signed_Optional (Is_Present : Boolean := False) is record",
        "type PrimitivePayload_Maybe_Unsigned_Optional (Is_Present : Boolean := False) is record",
        "type PrimitivePayload_Maybe_F32_Optional (Is_Present : Boolean := False) is record",
        "type PrimitivePayload_Maybe_F64_Optional (Is_Present : Boolean := False) is record",
        "type PrimitivePayload_Maybe_Binary_Optional (Is_Present : Boolean := False) is record",
    ] {
        assert!(spec.contains(helper), "missing {helper}:\n{spec}");
    }
    // Direct optional String keeps the shared `Optional_String`, unchanged.
    assert!(
        spec.contains("Maybe_String : Optional_String;"),
        "direct optional String must keep Optional_String:\n{spec}"
    );
    assert!(
        !spec.contains("Maybe_String_Optional"),
        "direct optional String must not gain a per-field wrapper:\n{spec}"
    );
    // The required direct primitive field is untouched by Task 035.
    assert!(spec.contains("Required_Bool : Boolean;"), "{spec}");

    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but GNAT is not runnable"
        );
        return;
    }
    let status = Command::new("gnatmake")
        .current_dir(&output_root)
        .args(["-gnatwa", "-c", "urn-test.ads"])
        .status()
        .expect("GNAT reported a version, so it must be runnable");
    assert!(status.success(), "selected Ada output must compile");
}

/// Generate the Task 036 temporal service for one language.
fn generate_temporal(language: &str, label: &str) -> PathBuf {
    let output_root = output_dir(label);
    let output = generate(
        "temporal.xsd",
        "temporal.yaml",
        language,
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{language} temporal generation should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output_root
}

/// Task 036: a contract whose selected closure carries the supported named
/// DateTime Zulu profile becomes READY in all three backends.
///
/// The point of this test is *propagation*. No Task 036 change was made to
/// `service_plan.rs`, `service_readiness.rs`, or `service_generation.rs`; the
/// new capability has to arrive through the single shared coverage snapshot
/// and the ordinary backends. The fixture also declares an unselected `Time`
/// carrying the *same* `.+Z` pattern text, plus an unselected `Duration`, so a
/// READY result here cannot be a whole-schema accident and cannot be an
/// accidental widening to every temporal kind.
#[test]
fn task036_temporal_service_is_ready_in_every_backend() {
    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("temporal.xsd"))
            .arg("--contract")
            .arg(fixture("temporal.yaml"))
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        assert_eq!(
            check.status.code(),
            Some(0),
            "{language} service-check should succeed"
        );
        let report = stdout_of(&check);
        assert!(
            report.contains("status: READY"),
            "{language} must be READY for the temporal contract:\n{report}"
        );
        // The unsupported temporal declarations are outside the selection, so
        // they must not appear in the projected closure at all.
        for unselected in ["WallClock", "Span", "UnselectedTemporalPayload"] {
            assert!(
                !report.contains(unselected),
                "{language} must not project the unselected {unselected}"
            );
        }
    }
}

/// Task 036: selected generation emits the Ada package **body** alongside the
/// spec, and the projected output carries no unselected temporal declaration.
#[test]
fn task036_selected_ada_generation_includes_the_package_body() {
    let root = generate_temporal("ada", "task036-ada-files");
    let spec = std::fs::read_to_string(root.join("urn-test.ads")).expect("spec must be generated");
    let body = std::fs::read_to_string(root.join("urn-test.adb")).expect("body must be generated");

    assert!(spec.contains("type Instant is private;"));
    assert!(spec.contains("function Create (Value : String) return Instant;"));
    assert!(body.contains("package body Urn.Test is"));
    assert!(body.contains("function Create (Value : String) return Instant is"));
    // Both supported declarations are projected, which is what makes Ada
    // `Create` / `Value` overload resolution load-bearing here.
    assert!(spec.contains("type Deadline is private;"));
    assert!(body.contains("function Create (Value : String) return Deadline is"));

    for unselected in ["WallClock", "Span", "Unselected"] {
        assert!(!spec.contains(unselected), "spec leaked {unselected}");
        assert!(!body.contains(unselected), "body leaked {unselected}");
    }
}

/// Task 036: the selected Rust output compiles and validates at runtime.
#[test]
fn task036_selected_rust_output_validates_at_runtime() {
    let root = generate_temporal("rust", "task036-rust");
    std::fs::write(
        root.join("probe.rs"),
        "include!(\"test.rs\");\n\nfn main() {\n\
         \x20   let value = Instant::new(\"  2026-09-20T12:34:56Z \").expect(\"valid\");\n\
         \x20   assert_eq!(value.as_str(), \"2026-09-20T12:34:56Z\");\n\
         \x20   assert!(Instant::new(\"2026-02-30T00:00:00Z\").is_none());\n\
         \x20   assert!(Instant::new(\"garbageZ\").is_none());\n\
         \x20   assert!(Instant::new(\"2026-09-20T12:34:56\").is_none());\n\
         }\n",
    )
    .expect("write Rust probe");
    let status = Command::new("rustc")
        .current_dir(&root)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .status()
        .expect("rustc must be available");
    assert!(status.success(), "selected Rust output must compile");
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Rust validation must hold"
    );
}

/// Task 036: the selected C++ output compiles under strict C++17 and validates
/// at runtime.
#[test]
fn task036_selected_cpp_output_validates_at_runtime() {
    let root = generate_temporal("cpp", "task036-cpp");
    std::fs::write(
        root.join("probe.cpp"),
        "#include \"test.hpp\"\n#include <cstdio>\n\n\
         int main() {\n\
         \x20   auto value = urn::test::Instant::create(\"  2026-09-20T12:34:56Z \");\n\
         \x20   if (!value || value->value() != \"2026-09-20T12:34:56Z\") return 1;\n\
         \x20   if (urn::test::Instant::create(\"2026-02-30T00:00:00Z\")) return 1;\n\
         \x20   if (urn::test::Instant::create(\"garbageZ\")) return 1;\n\
         \x20   if (urn::test::Instant::create(\"2026-09-20T12:34:56\")) return 1;\n\
         \x20   std::puts(\"ok\");\n\
         \x20   return 0;\n\
         }\n",
    )
    .expect("write C++ probe");
    let output = Command::new("c++")
        .current_dir(&root)
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
        .expect("a C++ compiler must be available");
    assert!(
        output.status.success(),
        "selected C++ output must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected C++ validation must hold"
    );
}

/// Task 036: the selected Ada output compiles and raises `Constraint_Error` on
/// an invalid value, with no unchecked construction path available.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task036_selected_ada_output_validates_under_gnat() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let root = generate_temporal("ada", "task036-ada-runtime");
    std::fs::write(
        root.join("probe.adb"),
        "with Ada.Text_IO;\nwith Urn.Test;\n\n\
         procedure Probe is\n\
         \x20  Good : constant Urn.Test.Instant :=\n\
         \x20    Urn.Test.Create (\"  2026-09-20T12:34:56Z \");\n\
         begin\n\
         \x20  if Urn.Test.Value (Good) /= \"2026-09-20T12:34:56Z\" then\n\
         \x20     Ada.Text_IO.Put_Line (\"bad normalization\");\n\
         \x20     return;\n\
         \x20  end if;\n\
         \x20  declare\n\
         \x20     Bad : constant Urn.Test.Instant :=\n\
         \x20       Urn.Test.Create (\"2026-02-30T00:00:00Z\");\n\
         \x20  begin\n\
         \x20     Ada.Text_IO.Put_Line (\"must have raised: \" & Urn.Test.Value (Bad));\n\
         \x20  end;\n\
         exception\n\
         \x20  when Constraint_Error => Ada.Text_IO.Put_Line (\"ok\");\n\
         end Probe;\n",
    )
    .expect("write Ada probe");
    let compile = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "probe.adb"])
        .output()
        .expect("GNAT reported a version, so it must be runnable");
    assert!(
        compile.status.success(),
        "selected Ada output must compile:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(root.join("probe"))
        .output()
        .expect("probe must run");
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    assert!(
        stdout.trim() == "ok",
        "selected Ada validation must hold:\n{stdout}"
    );
}

/// Task 036: the selected Ada output offers no unchecked way to build an
/// `Instant`, so the representation bypasses are compile errors.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task036_selected_ada_temporal_rejects_public_bypasses() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let root = generate_temporal("ada", "task036-ada-bypass");
    for (unit, body) in [
        // The private record's component is not visible, so no aggregate can
        // name it.
        ("aggregate", "   Bad : Instant := (Lexical => <>);\n"),
        // No conversion from String exists; only `Create` can build one.
        (
            "conversion",
            "   Bad : Instant := Instant (\"2026-99-99T99:99:99Z\");\n",
        ),
    ] {
        let file = format!("{unit}_probe.adb");
        let procedure = format!("{}{}_Probe", unit[..1].to_uppercase(), &unit[1..]);
        std::fs::write(
            root.join(&file),
            format!(
                "with Urn.Test; use Urn.Test;\n\nprocedure {procedure} is\n{body}begin\n   null;\nend {procedure};\n"
            ),
        )
        .expect("write Ada bypass probe");
        let output = Command::new("gnatmake")
            .current_dir(&root)
            .args(["-q", &file])
            .output()
            .expect("GNAT reported a version, so it must be runnable");
        assert!(
            !output.status.success(),
            "{unit} bypass must not compile against the selected Ada output"
        );
    }
}

/// Generate the Task 037 String-profile service for one language.
fn generate_string_profile(language: &str, label: &str) -> PathBuf {
    let output_root = output_dir(label);
    let output = generate(
        "string-profile.xsd",
        "string-profile.yaml",
        language,
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{language} String-profile generation should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output_root
}

/// Task 037: a contract whose selected closure carries the supported named
/// schema-version String profile becomes READY in all three backends.
///
/// The point of this test is *propagation*. No Task 037 change was made to
/// `service_plan.rs`, `service_readiness.rs`, or `service_generation.rs`; the
/// new capability has to arrive through the single shared coverage snapshot
/// and the ordinary backends. The fixture also declares an unselected
/// `length + pattern` String, so a READY result here cannot be a whole-schema
/// accident and cannot be an accidental widening to every constrained String.
#[test]
fn task037_string_profile_service_is_ready_in_every_backend() {
    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("string-profile.xsd"))
            .arg("--contract")
            .arg(fixture("string-profile.yaml"))
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        assert_eq!(
            check.status.code(),
            Some(0),
            "{language} service-check should succeed"
        );
        let report = stdout_of(&check);
        assert!(
            report.contains("status: READY"),
            "{language} must be READY for the String-profile contract:\n{report}"
        );
        // The unsupported constrained String is outside the selection, so it
        // must not appear in the projected closure at all.
        for unselected in ["Identifier", "UnselectedVersionPayload"] {
            assert!(
                !report.contains(unselected),
                "{language} must not project the unselected {unselected}"
            );
        }
    }
}

/// Task 037: selected generation emits the Ada package **body** alongside the
/// spec, and the projected output carries no unselected String declaration.
#[test]
fn task037_selected_ada_generation_includes_the_package_body() {
    let root = generate_string_profile("ada", "task037-ada-files");
    let spec = std::fs::read_to_string(root.join("urn-test.ads")).expect("spec must be generated");
    let body = std::fs::read_to_string(root.join("urn-test.adb")).expect("body must be generated");

    assert!(spec.contains("type SchemaVersion is private;"));
    assert!(spec.contains("function Create (Value : String) return SchemaVersion;"));
    assert!(body.contains("package body Urn.Test is"));
    assert!(body.contains("function Create (Value : String) return SchemaVersion is"));
    // Both supported declarations are projected, which is what makes Ada
    // `Create` / `Value` overload resolution load-bearing here.
    assert!(spec.contains("type PeerVersion is private;"));
    assert!(body.contains("function Create (Value : String) return PeerVersion is"));
    // No generic regex engine is pulled in.
    assert!(!body.contains("Regpat"), "body must not use GNAT.Regpat");

    for unselected in ["Identifier", "Unselected"] {
        assert!(!spec.contains(unselected), "spec leaked {unselected}");
        assert!(!body.contains(unselected), "body leaked {unselected}");
    }
}

/// Task 037: the selected Rust output compiles and validates at runtime.
#[test]
fn task037_selected_rust_output_validates_at_runtime() {
    let root = generate_string_profile("rust", "task037-rust");
    std::fs::write(
        root.join("probe.rs"),
        "include!(\"test.rs\");\n\nfn main() {\n\
         \x20   let value = SchemaVersion::new(\"002.5.0\").expect(\"valid\");\n\
         \x20   assert_eq!(value.as_str(), \"002.5.0\");\n\
         \x20   assert!(SchemaVersion::new(\"000.001.000.000\").is_none());\n\
         \x20   assert!(SchemaVersion::new(\" 002.5.0\").is_none());\n\
         \x20   assert!(SchemaVersion::new(\"002.5\").is_none());\n\
         \x20   assert_eq!(value, SchemaVersion::new(\"002.5.0\").unwrap());\n\
         }\n",
    )
    .expect("write Rust probe");
    let status = Command::new("rustc")
        .current_dir(&root)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .status()
        .expect("rustc must be available");
    assert!(status.success(), "selected Rust output must compile");
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Rust validation must hold"
    );
}

/// Task 037: the selected C++ output compiles under strict C++17 and validates
/// at runtime.
#[test]
fn task037_selected_cpp_output_validates_at_runtime() {
    let root = generate_string_profile("cpp", "task037-cpp");
    std::fs::write(
        root.join("probe.cpp"),
        "#include \"test.hpp\"\n#include <cstdio>\n\n\
         int main() {\n\
         \x20   auto value = urn::test::SchemaVersion::create(\"002.5.0\");\n\
         \x20   if (!value || value->value() != \"002.5.0\") return 1;\n\
         \x20   if (urn::test::SchemaVersion::create(\"000.001.000.000\")) return 1;\n\
         \x20   if (urn::test::SchemaVersion::create(\" 002.5.0\")) return 1;\n\
         \x20   if (urn::test::SchemaVersion::create(\"002.5\")) return 1;\n\
         \x20   std::puts(\"ok\");\n\
         \x20   return 0;\n\
         }\n",
    )
    .expect("write C++ probe");
    let output = Command::new("c++")
        .current_dir(&root)
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
        .expect("a C++ compiler must be available");
    assert!(
        output.status.success(),
        "selected C++ output must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected C++ validation must hold"
    );
}

/// Task 037: the selected Ada output compiles and raises `Constraint_Error` on
/// an invalid value.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task037_selected_ada_output_validates_under_gnat() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let root = generate_string_profile("ada", "task037-ada-runtime");
    std::fs::write(
        root.join("probe.adb"),
        "with Ada.Text_IO;\nwith Urn.Test;\n\n\
         procedure Probe is\n\
         \x20  Good : constant Urn.Test.SchemaVersion := Urn.Test.Create (\"002.5.0\");\n\
         \x20  Rejected : Boolean := False;\n\
         begin\n\
         \x20  if Urn.Test.Value (Good) /= \"002.5.0\" then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         \x20  begin\n\
         \x20     declare\n\
         \x20        Bad : constant Urn.Test.SchemaVersion :=\n\
         \x20          Urn.Test.Create (\"000.001.000.000\");\n\
         \x20     begin\n\
         \x20        if Urn.Test.Value (Bad)'Length >= 0 then\n\
         \x20           null;\n\
         \x20        end if;\n\
         \x20     end;\n\
         \x20  exception\n\
         \x20     when Constraint_Error => Rejected := True;\n\
         \x20  end;\n\
         \x20  if not Rejected then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         \x20  Ada.Text_IO.Put_Line (\"ok\");\n\
         end Probe;\n",
    )
    .expect("write Ada probe");
    let output = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "probe.adb"])
        .output()
        .expect("gnatmake must run");
    assert!(
        output.status.success(),
        "selected Ada output must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Ada validation must hold"
    );
}

/// Generate the Task 038 UUID service for one language.
fn generate_uuid(language: &str, label: &str) -> PathBuf {
    let output_root = output_dir(label);
    let output = generate(
        "uuid.xsd",
        "uuid.yaml",
        language,
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{language} UUID generation should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output_root
}

/// Task 038: a contract whose selected closure carries the supported named UUID
/// String profile becomes READY in all three backends.
///
/// The point of this test is *propagation*. No Task 038 change was made to
/// `service_plan.rs`, `service_readiness.rs`, or `service_generation.rs`; the
/// new capability has to arrive through the single shared coverage snapshot and
/// the ordinary backends. The fixture also declares an unselected general
/// hexadecimal String, so a READY result here cannot be a whole-schema accident
/// and cannot be an accidental widening to every `length + pattern` String.
#[test]
fn task038_uuid_service_is_ready_in_every_backend() {
    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("uuid.xsd"))
            .arg("--contract")
            .arg(fixture("uuid.yaml"))
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        assert_eq!(
            check.status.code(),
            Some(0),
            "{language} service-check should succeed"
        );
        let report = stdout_of(&check);
        assert!(
            report.contains("status: READY"),
            "{language} must be READY for the UUID contract:\n{report}"
        );
        // The unsupported constrained String is outside the selection, so it
        // must not appear in the projected closure at all.
        for unselected in ["LooseIdentifier", "UnselectedTrackPayload"] {
            assert!(
                !report.contains(unselected),
                "{language} must not project the unselected {unselected}"
            );
        }
    }
}

/// Task 038: selected generation emits the Ada package **body** alongside the
/// spec, and the projected output carries no unselected String declaration.
#[test]
fn task038_selected_ada_generation_includes_the_package_body() {
    let root = generate_uuid("ada", "task038-ada-files");
    let spec = std::fs::read_to_string(root.join("urn-test.ads")).expect("spec must be generated");
    let body = std::fs::read_to_string(root.join("urn-test.adb")).expect("body must be generated");

    assert!(spec.contains("type Uuid is private;"));
    assert!(spec.contains("function Create (Value : String) return Uuid;"));
    assert!(body.contains("package body Urn.Test is"));
    assert!(body.contains("function Create (Value : String) return Uuid is"));
    // Several supported declarations are projected, spanning BOTH String
    // profiles, which is what makes Ada `Create` / `Value` overload resolution
    // load-bearing here.
    assert!(spec.contains("type PeerUuid is private;"));
    assert!(spec.contains("type SchemaVersion is private;"));
    assert!(body.contains("function Create (Value : String) return PeerUuid is"));
    assert!(body.contains("function Create (Value : String) return SchemaVersion is"));
    // No generic regex engine is pulled in.
    assert!(!body.contains("Regpat"), "body must not use GNAT.Regpat");

    for unselected in ["LooseIdentifier", "Unselected"] {
        assert!(!spec.contains(unselected), "spec leaked {unselected}");
        assert!(!body.contains(unselected), "body leaked {unselected}");
    }
}

/// Task 038: the selected Rust output compiles and validates at runtime.
///
/// The rejected cases are chosen to prove the *authoritative* profile is in
/// force: a bad version nibble and a bad variant nibble are both well-formed
/// 8-4-4-4-12 hexadecimal values, so only the real `[1-5]` / `[89abAB]`
/// classes reject them. The nil UUID is accepted, via its own branch.
#[test]
fn task038_selected_rust_output_validates_at_runtime() {
    let root = generate_uuid("rust", "task038-rust");
    std::fs::write(
        root.join("probe.rs"),
        "include!(\"test.rs\");\n\nfn main() {\n\
         \x20   let value = Uuid::new(\"123e4567-e89b-12d3-a456-426614174000\").expect(\"valid\");\n\
         \x20   assert_eq!(value.as_str(), \"123e4567-e89b-12d3-a456-426614174000\");\n\
         \x20   // The nil UUID is accepted by its own branch.\n\
         \x20   assert!(Uuid::new(\"00000000-0000-0000-0000-000000000000\").is_some());\n\
         \x20   // Version nibble outside [1-5].\n\
         \x20   assert!(Uuid::new(\"123e4567-e89b-02d3-a456-426614174000\").is_none());\n\
         \x20   assert!(Uuid::new(\"123e4567-e89b-62d3-a456-426614174000\").is_none());\n\
         \x20   // Variant nibble outside [89abAB].\n\
         \x20   assert!(Uuid::new(\"123e4567-e89b-12d3-c456-426614174000\").is_none());\n\
         \x20   // All-f fails BOTH classes.\n\
         \x20   assert!(Uuid::new(\"ffffffff-ffff-ffff-ffff-ffffffffffff\").is_none());\n\
         \x20   assert!(Uuid::new(\" 123e4567-e89b-12d3-a456-42661417400\").is_none());\n\
         \x20   assert!(Uuid::new(\"123e4567e89b12d3a456426614174000\").is_none());\n\
         \x20   assert_eq!(value, Uuid::new(\"123e4567-e89b-12d3-a456-426614174000\").unwrap());\n\
         \x20   // Case is preserved and remains significant.\n\
         \x20   let upper = Uuid::new(\"123E4567-E89B-12D3-A456-42661417400F\").expect(\"valid\");\n\
         \x20   assert_eq!(upper.as_str(), \"123E4567-E89B-12D3-A456-42661417400F\");\n\
         \x20   assert_ne!(upper, Uuid::new(\"123e4567-e89b-12d3-a456-42661417400f\").unwrap());\n\
         }\n",
    )
    .expect("write Rust probe");
    let status = Command::new("rustc")
        .current_dir(&root)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .status()
        .expect("rustc must be available");
    assert!(status.success(), "selected Rust output must compile");
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Rust validation must hold"
    );
}

/// Task 038: the selected C++ output compiles under strict C++17 and validates
/// at runtime.
#[test]
fn task038_selected_cpp_output_validates_at_runtime() {
    let root = generate_uuid("cpp", "task038-cpp");
    std::fs::write(
        root.join("probe.cpp"),
        "#include \"test.hpp\"\n#include <cstdio>\n\n\
         int main() {\n\
         \x20   auto value = urn::test::Uuid::create(\"123e4567-e89b-12d3-a456-426614174000\");\n\
         \x20   if (!value || value->value() != \"123e4567-e89b-12d3-a456-426614174000\") return 1;\n\
         \x20   if (!urn::test::Uuid::create(\"00000000-0000-0000-0000-000000000000\")) return 1;\n\
         \x20   if (urn::test::Uuid::create(\"123e4567-e89b-02d3-a456-426614174000\")) return 1;\n\
         \x20   if (urn::test::Uuid::create(\"123e4567-e89b-62d3-a456-426614174000\")) return 1;\n\
         \x20   if (urn::test::Uuid::create(\"123e4567-e89b-12d3-c456-426614174000\")) return 1;\n\
         \x20   if (urn::test::Uuid::create(\"ffffffff-ffff-ffff-ffff-ffffffffffff\")) return 1;\n\
         \x20   if (urn::test::Uuid::create(\" 123e4567-e89b-12d3-a456-42661417400\")) return 1;\n\
         \x20   if (urn::test::Uuid::create(\"123e4567e89b12d3a456426614174000\")) return 1;\n\
         \x20   auto upper = urn::test::Uuid::create(\"123E4567-E89B-12D3-A456-42661417400F\");\n\
         \x20   if (!upper || upper->value() != \"123E4567-E89B-12D3-A456-42661417400F\") return 1;\n\
         \x20   std::puts(\"ok\");\n\
         \x20   return 0;\n\
         }\n",
    )
    .expect("write C++ probe");
    let output = Command::new("c++")
        .current_dir(&root)
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
        .expect("a C++ compiler must be available");
    assert!(
        output.status.success(),
        "selected C++ output must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected C++ validation must hold"
    );
}

/// Task 038: the selected Ada output compiles and raises `Constraint_Error` on
/// an invalid value, while accepting the nil UUID.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task038_selected_ada_output_validates_under_gnat() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let root = generate_uuid("ada", "task038-ada-runtime");
    std::fs::write(
        root.join("probe.adb"),
        "with Ada.Text_IO;\nwith Urn.Test;\n\n\
         procedure Probe is\n\
         \x20  Good : constant Urn.Test.Uuid :=\n\
         \x20    Urn.Test.Create (\"123e4567-e89b-12d3-a456-426614174000\");\n\
         \x20  Nil : constant Urn.Test.Uuid :=\n\
         \x20    Urn.Test.Create (\"00000000-0000-0000-0000-000000000000\");\n\
         \x20  Rejected : Natural := 0;\n\
         \x20  --  Each of these is a well-formed 8-4-4-4-12 hexadecimal value,\n\
         \x20  --  so only the authoritative version/variant classes reject them.\n\
         \x20  type Case_Names is (Bad_Version, Bad_Variant, All_F);\n\
         \x20  function Text (Item : Case_Names) return String is\n\
         \x20    (case Item is\n\
         \x20       when Bad_Version => \"123e4567-e89b-02d3-a456-426614174000\",\n\
         \x20       when Bad_Variant => \"123e4567-e89b-12d3-c456-426614174000\",\n\
         \x20       when All_F       => \"ffffffff-ffff-ffff-ffff-ffffffffffff\");\n\
         begin\n\
         \x20  if Urn.Test.Value (Good) /= \"123e4567-e89b-12d3-a456-426614174000\"\n\
         \x20    or else Urn.Test.Value (Nil) /= \"00000000-0000-0000-0000-000000000000\"\n\
         \x20  then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         \x20  for Item in Case_Names loop\n\
         \x20     begin\n\
         \x20        declare\n\
         \x20           Bad : constant Urn.Test.Uuid := Urn.Test.Create (Text (Item));\n\
         \x20        begin\n\
         \x20           if Urn.Test.Value (Bad)'Length >= 0 then\n\
         \x20              null;\n\
         \x20           end if;\n\
         \x20        end;\n\
         \x20     exception\n\
         \x20        when Constraint_Error => Rejected := Rejected + 1;\n\
         \x20     end;\n\
         \x20  end loop;\n\
         \x20  if Rejected /= 3 then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         \x20  Ada.Text_IO.Put_Line (\"ok\");\n\
         end Probe;\n",
    )
    .expect("write Ada probe");
    let output = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "probe.adb"])
        .output()
        .expect("gnatmake must run");
    assert!(
        output.status.success(),
        "selected Ada output must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Ada validation must hold"
    );
}

/// Generate the Task 039 visible-ASCII service for one language.
fn generate_visible_ascii(language: &str, label: &str) -> PathBuf {
    let output_root = output_dir(label);
    let output = generate(
        "visible-ascii.xsd",
        "visible-ascii.yaml",
        language,
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{language} visible-ASCII generation should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output_root
}

/// Generate the Task 041 whitespace-visible service for one language.
fn generate_whitespace_visible(language: &str, label: &str) -> PathBuf {
    let output_root = output_dir(label);
    let output = generate(
        "whitespace-visible.xsd",
        "whitespace-visible.yaml",
        language,
        "closed-schema",
        &output_root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{language} whitespace-visible generation should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output_root
}

/// Task 041: a contract selecting whitespace-visible String declarations becomes
/// READY in all three backends.
///
/// The point of this test is *propagation*. No Task 041 change was made to
/// `service_plan.rs`, `service_readiness.rs`, or `service_generation.rs`; the new
/// capability arrives through the single shared coverage snapshot and the
/// ordinary backends. The fixture also declares an UNEVIDENCED whitespace-visible
/// triple outside the selection -- collapse paired with UCI 2.6's minimum of one
/// -- so a READY result here cannot be a whole-schema accident and cannot be an
/// accidental widening to every agreeing facet combination.
#[test]
fn task041_whitespace_visible_service_is_ready_in_every_backend() {
    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("whitespace-visible.xsd"))
            .arg("--contract")
            .arg(fixture("whitespace-visible.yaml"))
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        assert_eq!(
            check.status.code(),
            Some(0),
            "{language} service-check should succeed"
        );
        let report = stdout_of(&check);
        assert!(
            report.contains("status: READY"),
            "{language} must be READY for the whitespace-visible contract:\n{report}"
        );
        // The unevidenced triple is outside the selection, so it must not
        // contaminate the projected closure.
        assert!(
            !report.contains("UnobservedCollapsedText"),
            "{language} must not project the unselected shape:\n{report}"
        );
    }
}

/// Task 041: an UNSUPPORTED selected neighbour still BLOCKS and produces no
/// partial output.
///
/// Selecting the unevidenced triple's own message must fail closed, and the
/// failure must leave no generated artifact behind. This is the other half of the
/// readiness claim: the `READY` above has to mean something.
#[test]
fn task041_an_unsupported_selected_neighbour_still_blocks_generation() {
    let contract = output_dir("task041-blocked-contract").join("unselected.yaml");
    std::fs::create_dir_all(contract.parent().expect("parent")).expect("create contract dir");
    // The same contract shape, but selecting the message whose closure carries
    // the UNEVIDENCED whitespace-visible triple.
    let source = std::fs::read_to_string(fixture("whitespace-visible.yaml"))
        .expect("read the supported contract");
    std::fs::write(
        &contract,
        source.replace("message: RemarksReport", "message: UnselectedRemarksReport"),
    )
    .expect("write blocked contract");

    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("whitespace-visible.xsd"))
            .arg("--contract")
            .arg(&contract)
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        let report = stdout_of(&check);
        assert!(
            report.contains("status: NOT READY"),
            "{language} must block the unevidenced triple:\n{report}"
        );
        assert!(
            report.contains("UnobservedCollapsedText"),
            "{language} must name the blocker:\n{report}"
        );

        // And generation must produce NO partial output. The CLI is invoked
        // directly here because the shared `generate` helper resolves its
        // contract argument through `fixture()`, and this contract is generated
        // into a temporary directory rather than shipped.
        let out = output_dir(&format!("task041-blocked-{language}"));
        let generated = cli()
            .arg("service-generate")
            .arg("--schema")
            .arg(fixture("whitespace-visible.xsd"))
            .arg("--contract")
            .arg(&contract)
            .args([
                OsString::from("--language"),
                OsString::from(language),
                OsString::from("--world"),
                OsString::from("closed-schema"),
                OsString::from("--output"),
                OsString::from(&out),
            ])
            .output()
            .expect("CLI should run");
        assert_ne!(
            generated.status.code(),
            Some(0),
            "{language} generation must fail for an unsupported selection"
        );
        let produced: Vec<_> = std::fs::read_dir(&out)
            .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        assert!(
            produced.is_empty(),
            "{language} must not leave partial output: {produced:?}"
        );
    }
}

/// Task 041: the selected Ada generation emits both policies' bodies, and the
/// projected output carries no unselected declaration.
#[test]
fn task041_selected_ada_generation_emits_both_whitespace_policies() {
    let root = generate_whitespace_visible("ada", "task041-ada-files");
    let spec = std::fs::read_to_string(root.join("urn-test.ads")).expect("spec must be generated");
    let body = std::fs::read_to_string(root.join("urn-test.adb")).expect("body must be generated");

    for name in [
        "CollapsedRemarks",
        "CollapsedNarrative",
        "PreservedRemarks",
        "PreservedNarrative",
        "PreservedQuery",
    ] {
        assert!(
            spec.contains(&format!("type {name} is private;")),
            "spec must project {name}"
        );
        assert!(
            spec.contains(&format!("function Create (Value : String) return {name};")),
            "spec must project {name}'s Create"
        );
    }
    // Both maxima and both minima are projected, so the carriers really do carry
    // their own bounds rather than a single shared pair.
    assert!(body.contains("Max_Length : constant := 1024;"));
    assert!(body.contains("Max_Length : constant := 4096;"));
    assert!(body.contains("Min_Length : constant := 0;"));
    assert!(body.contains("Min_Length : constant := 1;"));
    // The collapse half normalizes into a BOUNDED buffer; nothing is sized by
    // client input.
    assert!(body.contains("Normalized : String (1 .. Max_Length);"));
    assert!(
        !body.contains("String (1 .. Value'Length)"),
        "the buffer must not be sized by client input"
    );
    // Documentation distinguishes the two policies.
    assert!(spec.contains("collapse-normalized form of Create's argument"));
    assert!(spec.contains("The stored representation, exactly as supplied."));
    // No generic regex engine is pulled in.
    assert!(!body.contains("Regpat"), "body must not use GNAT.Regpat");

    for unselected in ["UnobservedCollapsedText", "Unselected"] {
        assert!(!spec.contains(unselected), "spec leaked {unselected}");
        assert!(!body.contains(unselected), "body leaked {unselected}");
    }
}

/// Task 041: the selected Rust client constructs and reads the new values.
///
/// Exercises required, optional, bounded and unbounded occurrences through the
/// ordinary generated containers -- there is no profile-specific container path.
#[test]
fn task041_selected_rust_client_constructs_and_reads_the_new_values() {
    let root = generate_whitespace_visible("rust", "task041-rust");
    std::fs::write(root.join("probe.rs"), TASK041_RUST_PROBE).expect("write Rust probe");
    let compiled = Command::new("rustc")
        .current_dir(&root)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .output()
        .expect("rustc must run");
    assert!(
        compiled.status.success(),
        "selected Rust output must compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Rust validation must hold"
    );
}

/// The Task 041 selected-Rust client probe.
const TASK041_RUST_PROBE: &str = r##"include!("test.rs");

fn main() {
    // COLLAPSE: the stored value is the NORMALIZED one, not the argument.
    let summary = CollapsedRemarks::new("  Target   sighted  ").expect("valid");
    assert_eq!(summary.as_str(), "Target sighted");
    // minLength 0, so the empty string is a valid value of this type.
    assert_eq!(CollapsedRemarks::new("").expect("empty is valid").as_str(), "");
    // A raw argument longer than maxLength is accepted when it normalizes short.
    let raw = String::from("a") + &" ".repeat(4000) + "b";
    assert!(raw.len() > 1024);
    assert_eq!(CollapsedRemarks::new(&raw).expect("normalizes").as_str(), "a b");

    // PRESERVE: LF and CR survive, and nothing is trimmed.
    let detail = PreservedRemarks::new("  line1\nline2\r  ").expect("valid");
    assert_eq!(detail.as_str(), "  line1\nline2\r  ");
    // minLength 1, so the empty string is INVALID here.
    assert!(PreservedRemarks::new("").is_none());
    // TAB is outside the class under preserve, but normalizes under collapse.
    assert!(PreservedRemarks::new("a\tb").is_none());
    assert_eq!(CollapsedRemarks::new("a\tb").expect("valid").as_str(), "a b");

    // The UCI 2.5 query shape: preserve policy with a ZERO minimum.
    assert_eq!(PreservedQuery::new("").expect("valid").as_str(), "");
    assert_eq!(PreservedQuery::new("  ").expect("valid").as_str(), "  ");

    // The two maxima genuinely differ.
    let between = "x".repeat(2000);
    assert!(CollapsedRemarks::new(&between).is_none());
    assert!(CollapsedNarrative::new(&between).is_some());
    assert!(PreservedNarrative::new(&between).is_some());

    // Container composition through the ordinary generated machinery.
    let payload = RemarksPayload {
        summary: summary.clone(),
        detail: detail.clone(),
        addendum: Some(CollapsedNarrative::new(" extra  text ").expect("valid")),
        query: Some(PreservedQuery::new("q=1").expect("valid")),
        version: SchemaVersion::new("001.1.0").expect("valid"),
        notes: String::from("ordinary"),
        history: BoundedVec::new(vec![
            CollapsedRemarks::new(" h1 ").expect("valid"),
            CollapsedRemarks::new("h2").expect("valid"),
        ])
        .expect("within 0..4"),
        anchors: BoundedVec::new(vec![
            PreservedRemarks::new("a1").expect("valid"),
            PreservedRemarks::new(" a2 ").expect("valid"),
        ])
        .expect("meets the minimum of 2"),
        log: UnboundedVec::new(vec![PreservedNarrative::new("entry\n").expect("valid")])
            .expect("no minimum"),
    };
    // Read every value back out, including the normalized and preserved forms.
    assert_eq!(payload.summary.as_str(), "Target sighted");
    assert_eq!(payload.detail.as_str(), "  line1\nline2\r  ");
    assert_eq!(payload.addendum.as_ref().expect("present").as_str(), "extra text");
    assert_eq!(payload.query.as_ref().expect("present").as_str(), "q=1");
    assert_eq!(payload.history.as_slice()[0].as_str(), "h1");
    assert_eq!(payload.anchors.as_slice()[1].as_str(), " a2 ");
    assert_eq!(payload.log.as_slice()[0].as_str(), "entry\n");
    // A bounded field below its positive minimum is rejected.
    assert!(BoundedVec::<PreservedRemarks, 2, 3>::new(vec![
        PreservedRemarks::new("only").expect("valid")
    ])
    .is_none());
    // Equality follows the STORED value.
    assert_eq!(summary, CollapsedRemarks::new("Target sighted").expect("valid"));
    assert_ne!(detail, PreservedRemarks::new("line1\nline2").expect("valid"));
    println!("ok");
}
"##;

/// Task 041: the selected C++ client constructs and reads the new values, and
/// keeps the Task 040 lifecycle guarantees after copying and rvalue operations.
#[test]
fn task041_selected_cpp_client_constructs_and_reads_the_new_values() {
    let root = generate_whitespace_visible("cpp", "task041-cpp");
    std::fs::write(root.join("probe.cpp"), TASK041_CPP_PROBE).expect("write C++ probe");
    let compiled = Command::new("c++")
        .current_dir(&root)
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
        .expect("a C++ compiler must be available");
    assert!(
        compiled.status.success(),
        "selected C++ output must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected C++ validation must hold"
    );
}

/// The Task 041 selected-C++ client probe.
///
/// Control-character inputs are built with EXPLICIT lengths so an embedded NUL
/// case is not accidentally truncated into a shorter, valid input.
const TASK041_CPP_PROBE: &str = r##"#include "test.hpp"
#include <cassert>
#include <cstdio>
#include <string>
#include <utility>

using namespace urn::test;

int main() {
    // COLLAPSE: the stored value is the NORMALIZED one.
    const auto summary = CollapsedRemarks::create("  Target   sighted  ");
    assert(summary && summary->value() == "Target sighted");
    // minLength 0, so the empty string is a valid value of this type.
    const auto empty = CollapsedRemarks::create("");
    assert(empty && empty->value().empty());
    // A raw argument longer than maxLength is accepted when it normalizes short.
    const std::string raw = std::string("a") + std::string(4000, ' ') + "b";
    assert(raw.size() > 1024);
    const auto normalized = CollapsedRemarks::create(raw);
    assert(normalized && normalized->value() == "a b");

    // PRESERVE: LF and CR survive, and nothing is trimmed.
    const auto detail = PreservedRemarks::create("  line1\nline2\r  ");
    assert(detail && detail->value() == "  line1\nline2\r  ");
    // minLength 1, so the empty string is INVALID here.
    assert(!PreservedRemarks::create("").has_value());
    // TAB is outside the class under preserve, but normalizes under collapse.
    assert(!PreservedRemarks::create("a\tb").has_value());
    const auto tabbed = CollapsedRemarks::create("a\tb");
    assert(tabbed && tabbed->value() == "a b");

    // The UCI 2.5 query shape: preserve policy with a ZERO minimum.
    const auto query = PreservedQuery::create("  ");
    assert(query && query->value() == "  ");

    // The two maxima genuinely differ.
    assert(!CollapsedRemarks::create(std::string(2000, 'x')).has_value());
    assert(CollapsedNarrative::create(std::string(2000, 'x')).has_value());

    // Invalid controls, with EXPLICIT lengths so NUL is not truncated.
    for (const char bad : {'\0', '\v', '\f', '\x7f'}) {
        const std::string s = std::string("a") + std::string(1, bad) + "b";
        assert(s.size() == 3);
        assert(!CollapsedRemarks::create(s).has_value());
        assert(!PreservedRemarks::create(s).has_value());
    }
    // High-bit bytes: U+00A0 is 0xC2 0xA0, both negative as a signed char.
    assert(!CollapsedRemarks::create(std::string("a\302\240b", 4)).has_value());

    // Task 040 lifecycle: source AND destination keep their validated values
    // after copying and after rvalue operations.
    auto source = *CollapsedRemarks::create(" keep  me ");
    auto dest = source;
    assert(source.value() == "keep me" && dest.value() == "keep me");
    auto moved = std::move(source);
    assert(source.value() == "keep me" && moved.value() == "keep me");
    dest = std::move(moved);
    assert(moved.value() == "keep me" && dest.value() == "keep me");

    // Container composition through the ordinary generated machinery.
    //
    // The record is AGGREGATE-initialized, which is itself part of the
    // guarantee: the carriers have no default constructor, so a
    // `RemarksPayload payload;` declaration does not compile and every member
    // must be supplied from a validated value.
    const RemarksPayload payload{
        *summary,
        *detail,
        CollapsedNarrative::create(" extra  text "),
        query,
        *SchemaVersion::create("001.1.0"),
        "ordinary",
        *BoundedVector<CollapsedRemarks, 0, 4>::create(
            {*CollapsedRemarks::create(" h1 "), *CollapsedRemarks::create("h2")}),
        *BoundedVector<PreservedRemarks, 2, 3>::create(
            {*PreservedRemarks::create("a1"), *PreservedRemarks::create(" a2 ")}),
        *UnboundedVector<PreservedNarrative, 0>::create(
            {*PreservedNarrative::create("entry\n")}),
    };
    assert(payload.summary.value() == "Target sighted");
    assert(payload.detail.value() == "  line1\nline2\r  ");
    assert(payload.addendum->value() == "extra text");
    assert(payload.query->value() == "  ");
    assert(payload.history.values().at(0).value() == "h1");
    assert(payload.anchors.values().at(1).value() == " a2 ");
    assert(payload.log.values().at(0).value() == "entry\n");
    // A bounded field below its positive minimum is rejected. Bound to a name
    // first: a braced initializer's commas would otherwise be read as extra
    // arguments by the `assert` macro.
    const auto too_few = BoundedVector<PreservedRemarks, 2, 3>::create(
        {*PreservedRemarks::create("only")});
    assert(!too_few.has_value());
    std::puts("ok");
    return 0;
}
"##;

/// Task 041: the selected Ada client constructs, reads, and exercises the
/// sequences, under GNAT.
///
/// Covers bounded empty/partial/full, the positive-minimum bounded shape whose
/// `Clear` stays absent, and unbounded growth, copying, clearing and reuse.
#[test]
fn task041_selected_ada_client_exercises_the_sequences_under_gnat() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let root = generate_whitespace_visible("ada", "task041-ada-runtime");
    // The positive-minimum bounded member must NOT expose Clear.
    let spec = std::fs::read_to_string(root.join("urn-test.ads")).expect("spec must be generated");
    assert!(
        !spec.contains("procedure Clear (Container : in out RemarksPayload_Anchors_Sequence)"),
        "a positive-minimum bounded member must not expose Clear"
    );
    assert!(
        spec.contains("procedure Clear (Container : in out RemarksPayload_History_Sequence)"),
        "a zero-minimum bounded member keeps Clear"
    );

    std::fs::write(root.join("probe.adb"), TASK041_ADA_PROBE).expect("write Ada probe");
    let output = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "probe.adb"])
        .output()
        .expect("gnatmake must run");
    assert!(
        output.status.success(),
        "selected Ada output must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Command::new(root.join("probe"))
        .output()
        .expect("probe must run");
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    assert!(
        run.status.success() && stdout.trim() == "ok",
        "selected Ada validation must hold:\n{stdout}"
    );
}

/// The Task 041 selected-Ada client probe.
const TASK041_ADA_PROBE: &str = r##"with Ada.Text_IO; use Ada.Text_IO;
with Urn.Test;
procedure Probe is
   Failures : Natural := 0;
   procedure Check (Got, Want, Label : String) is
   begin
      if Got /= Want then
         Put_Line ("mismatch: " & Label & " got [" & Got & "]");
         Failures := Failures + 1;
      end if;
   end Check;

   --  COLLAPSE: the stored value is the NORMALIZED one.
   Summary : constant Urn.Test.CollapsedRemarks :=
     Urn.Test.Create ("  Target   sighted  ");
   --  minLength 0, so Create ("") SUCCEEDS for this profile. This is a VALUE
   --  fact; default construction remains prohibited as an API policy.
   Empty : constant Urn.Test.CollapsedRemarks := Urn.Test.Create ("");
   --  PRESERVE: LF and CR survive and nothing is trimmed.
   Detail : constant Urn.Test.PreservedRemarks :=
     Urn.Test.Create ("  line1" & ASCII.LF & "line2" & ASCII.CR & "  ");
   Query : constant Urn.Test.PreservedQuery := Urn.Test.Create ("  ");
begin
   Check (Urn.Test.Value (Summary), "Target sighted", "collapse normalizes");
   Check (Urn.Test.Value (Empty), "", "minLength 0 accepts empty");
   Check (Urn.Test.Value (Detail),
     "  line1" & ASCII.LF & "line2" & ASCII.CR & "  ", "preserve keeps text");
   Check (Urn.Test.Value (Query), "  ", "preserve does not trim");

   --  BOUNDED, zero minimum: partial, full, then Clear and reuse.
   declare
      History : Urn.Test.RemarksPayload_History_Sequence :=
        Urn.Test.To_Sequence
          ((1 => Urn.Test.Create (" h1 "), 2 => Urn.Test.Create ("h2")));
   begin
      if Urn.Test.Length (History) /= 2 then
         Put_Line ("bounded length"); Failures := Failures + 1;
      end if;
      Check (Urn.Test.Value (Urn.Test.Element (History, 1)), "h1",
        "bounded element normalized");
      Urn.Test.Append (History, Urn.Test.Create ("  h3  "));
      Urn.Test.Append (History, Urn.Test.Create ("h4"));
      if Urn.Test.Length (History) /= 4 then
         Put_Line ("bounded full"); Failures := Failures + 1;
      end if;
      Check (Urn.Test.Value (Urn.Test.Element (History, 3)), "h3",
        "appended element normalized");
      --  Full: a fifth value must be rejected.
      begin
         Urn.Test.Append (History, Urn.Test.Create ("h5"));
         Put_Line ("bounded overflow must be rejected");
         Failures := Failures + 1;
      exception
         when Constraint_Error => null;
      end;
      --  Clear and reuse, which the zero-minimum shape permits.
      Urn.Test.Clear (History);
      if Urn.Test.Length (History) /= 0 then
         Put_Line ("bounded clear"); Failures := Failures + 1;
      end if;
      Urn.Test.Append (History, Urn.Test.Create (" reused "));
      Check (Urn.Test.Value (Urn.Test.Element (History, 1)), "reused",
        "bounded reuse");
   end;

   --  BOUNDED, POSITIVE minimum: copying and assignment preserve the values,
   --  and a too-short sequence is rejected.
   declare
      Anchors : constant Urn.Test.RemarksPayload_Anchors_Sequence :=
        Urn.Test.To_Sequence
          ((1 => Urn.Test.Create ("a1"), 2 => Urn.Test.Create (" a2 ")));
      Copied : Urn.Test.RemarksPayload_Anchors_Sequence := Anchors;
   begin
      Check (Urn.Test.Value (Urn.Test.Element (Copied, 2)), " a2 ",
        "positive-minimum copy preserves");
      Copied := Anchors;
      Check (Urn.Test.Value (Urn.Test.Element (Copied, 1)), "a1",
        "positive-minimum assignment preserves");
      begin
         declare
            Short : constant Urn.Test.RemarksPayload_Anchors_Sequence :=
              Urn.Test.To_Sequence ((1 => Urn.Test.Create ("only")));
         begin
            Put_Line ("too few values must be rejected:" &
              Urn.Test.Length (Short)'Image);
            Failures := Failures + 1;
         end;
      exception
         when Constraint_Error => null;
      end;
   end;

   --  UNBOUNDED: growth, copying, clearing and reuse.
   declare
      Log : Urn.Test.RemarksPayload_Log_Sequence;
      Copy : Urn.Test.RemarksPayload_Log_Sequence;
   begin
      Urn.Test.Reserve_Capacity (Log, 8);
      for Index in 1 .. 5 loop
         Urn.Test.Append (Log, Urn.Test.Create ("entry" & ASCII.LF));
      end loop;
      if Urn.Test.Length (Log) /= 5 then
         Put_Line ("unbounded growth"); Failures := Failures + 1;
      end if;
      Check (Urn.Test.Value (Urn.Test.Element (Log, 3)), "entry" & ASCII.LF,
        "unbounded element preserves LF");
      Copy := Log;
      Urn.Test.Clear (Log);
      if Urn.Test.Length (Log) /= 0 or else Urn.Test.Length (Copy) /= 5 then
         Put_Line ("unbounded clear must not disturb the copy");
         Failures := Failures + 1;
      end if;
      Urn.Test.Append (Log, Urn.Test.Create ("reused"));
      Check (Urn.Test.Value (Urn.Test.Element (Log, 1)), "reused",
        "unbounded reuse");
   end;

   if Failures /= 0 then
      raise Program_Error;
   end if;
   Put_Line ("ok");
end Probe;
"##;

/// Task 039: a contract whose selected closure carries supported visible-ASCII
/// String declarations becomes READY in all three backends.
///
/// The point of this test is *propagation*. No Task 039 change was made to
/// `service_plan.rs`, `service_readiness.rs`, or `service_generation.rs`; the
/// new capability has to arrive through the single shared coverage snapshot and
/// the ordinary backends. The fixture also declares an unselected
/// `whiteSpace=collapse` String, so a READY result here cannot be a
/// whole-schema accident and cannot be an accidental widening to every
/// `minLength + maxLength + pattern` String.
#[test]
fn task039_visible_ascii_service_is_ready_in_every_backend() {
    for language in ["rust", "cpp", "ada"] {
        let check = cli()
            .arg("service-check")
            .arg("--schema")
            .arg(fixture("visible-ascii.xsd"))
            .arg("--contract")
            .arg(fixture("visible-ascii.yaml"))
            .args(["--language", language, "--world", "closed-schema"])
            .output()
            .expect("service-check must run");
        assert_eq!(
            check.status.code(),
            Some(0),
            "{language} service-check should succeed"
        );
        let report = stdout_of(&check);
        assert!(
            report.contains("status: READY"),
            "{language} must be READY for the visible-ASCII contract:\n{report}"
        );
        // The unsupported constrained String is outside the selection, so it
        // must not appear in the projected closure.
        assert!(
            !report.contains("CollapsingText"),
            "{language} must not project the unselected String:\n{report}"
        );
    }
}

/// Task 039: the selected Ada generation emits the body alongside the spec, and
/// the projected output carries no unselected String declaration.
#[test]
fn task039_selected_ada_generation_includes_the_package_body() {
    let root = generate_visible_ascii("ada", "task039-ada-files");
    let spec = std::fs::read_to_string(root.join("urn-test.ads")).expect("spec must be generated");
    let body = std::fs::read_to_string(root.join("urn-test.adb")).expect("body must be generated");

    assert!(spec.contains("type Callsign is private;"));
    assert!(spec.contains("function Create (Value : String) return Callsign;"));
    assert!(body.contains("package body Urn.Test is"));
    assert!(body.contains("function Create (Value : String) return Callsign is"));
    // Several supported declarations are projected at DIFFERENT bounds, which
    // is what makes Ada `Create` / `Value` overload resolution load-bearing
    // here, and what proves the profile is genuinely parameterized.
    assert!(spec.contains("type ShortLabel is private;"));
    assert!(spec.contains("type CountryCode is private;"));
    assert!(spec.contains("type SchemaVersion is private;"));
    assert!(body.contains("Max_Length : constant := 256;"));
    assert!(body.contains("Max_Length : constant := 32;"));
    assert!(body.contains("Min_Length : constant := 2;"));
    assert!(body.contains("Max_Length : constant := 4;"));
    // No generic regex engine is pulled in.
    assert!(!body.contains("Regpat"), "body must not use GNAT.Regpat");

    for unselected in ["CollapsingText", "Unselected"] {
        assert!(!spec.contains(unselected), "spec leaked {unselected}");
        assert!(!body.contains(unselected), "body leaked {unselected}");
    }
}

/// Task 039: the selected Rust output compiles and validates at runtime.
///
/// The accepted cases include a value with leading and trailing SPACE, because
/// `xs:string` carries `whiteSpace = preserve` and U+0020 is inside `[ -~]`:
/// those spaces are part of the value and must survive unchanged.
#[test]
fn task039_selected_rust_output_validates_at_runtime() {
    let root = generate_visible_ascii("rust", "task039-rust");
    std::fs::write(
        root.join("probe.rs"),
        "include!(\"test.rs\");\n\nfn main() {\n\
         \x20   let value = Callsign::new(\"Falcon-1 {alpha}\").expect(\"valid\");\n\
         \x20   assert_eq!(value.as_str(), \"Falcon-1 {alpha}\");\n\
         \x20   // whiteSpace = preserve: the spaces are part of the value.\n\
         \x20   let spaced = Callsign::new(\"  padded  \").expect(\"valid\");\n\
         \x20   assert_eq!(spaced.as_str(), \"  padded  \");\n\
         \x20   assert!(Callsign::new(\" \").is_some());\n\
         \x20   // The exact class boundaries.\n\
         \x20   assert!(Callsign::new(\"\\u{20}\").is_some());\n\
         \x20   assert!(Callsign::new(\"~\").is_some());\n\
         \x20   assert!(Callsign::new(\"\\u{1f}\").is_none());\n\
         \x20   assert!(Callsign::new(\"\\u{7f}\").is_none());\n\
         \x20   // Controls and non-ASCII are rejected.\n\
         \x20   assert!(Callsign::new(\"bad\\ttab\").is_none());\n\
         \x20   assert!(Callsign::new(\"bad\\nline\").is_none());\n\
         \x20   assert!(Callsign::new(\"caf\\u{e9}\").is_none());\n\
         \x20   assert!(Callsign::new(\"\\u{1f600}\").is_none());\n\
         \x20   // Both length facets.\n\
         \x20   assert!(Callsign::new(\"\").is_none());\n\
         \x20   assert!(Callsign::new(&\"x\".repeat(256)).is_some());\n\
         \x20   assert!(Callsign::new(&\"x\".repeat(257)).is_none());\n\
         \x20   // The bounds really are per-declaration.\n\
         \x20   assert!(ShortLabel::new(&\"x\".repeat(33)).is_none());\n\
         \x20   assert!(CountryCode::new(\"x\").is_none());\n\
         \x20   assert!(CountryCode::new(\"US\").is_some());\n\
         \x20   // Equality is value equality, and spacing stays significant.\n\
         \x20   assert_eq!(value, Callsign::new(\"Falcon-1 {alpha}\").unwrap());\n\
         \x20   assert_ne!(Callsign::new(\"abc\").unwrap(), Callsign::new(\"abc \").unwrap());\n\
         }\n",
    )
    .expect("write Rust probe");
    let status = Command::new("rustc")
        .current_dir(&root)
        .args(["--edition", "2021", "-o", "probe", "probe.rs"])
        .status()
        .expect("rustc must be available");
    assert!(status.success(), "selected Rust output must compile");
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Rust validation must hold"
    );
}

/// Task 039: the selected C++ output compiles under strict C++17 and validates
/// at runtime.
#[test]
fn task039_selected_cpp_output_validates_at_runtime() {
    let root = generate_visible_ascii("cpp", "task039-cpp");
    std::fs::write(
        root.join("probe.cpp"),
        "#include \"test.hpp\"\n#include <cstdio>\n#include <string>\n\n\
         int main() {\n\
         \x20   auto value = urn::test::Callsign::create(\"Falcon-1 {alpha}\");\n\
         \x20   if (!value || value->value() != \"Falcon-1 {alpha}\") return 1;\n\
         \x20   // whiteSpace = preserve: the spaces are part of the value.\n\
         \x20   auto spaced = urn::test::Callsign::create(\"  padded  \");\n\
         \x20   if (!spaced || spaced->value() != \"  padded  \") return 1;\n\
         \x20   if (!urn::test::Callsign::create(\" \")) return 1;\n\
         \x20   // The exact class boundaries.\n\
         \x20   if (!urn::test::Callsign::create(\"~\")) return 1;\n\
         \x20   if (urn::test::Callsign::create(std::string(\"\\037\", 1))) return 1;\n\
         \x20   if (urn::test::Callsign::create(std::string(\"\\177\", 1))) return 1;\n\
         \x20   // Controls and non-ASCII are rejected.\n\
         \x20   if (urn::test::Callsign::create(\"bad\\ttab\")) return 1;\n\
         \x20   if (urn::test::Callsign::create(\"bad\\nline\")) return 1;\n\
         \x20   if (urn::test::Callsign::create(\"caf\\303\\251\")) return 1;\n\
         \x20   // Both length facets.\n\
         \x20   if (urn::test::Callsign::create(\"\")) return 1;\n\
         \x20   if (!urn::test::Callsign::create(std::string(256, 'x'))) return 1;\n\
         \x20   if (urn::test::Callsign::create(std::string(257, 'x'))) return 1;\n\
         \x20   // The bounds really are per-declaration.\n\
         \x20   if (urn::test::ShortLabel::create(std::string(33, 'x'))) return 1;\n\
         \x20   if (urn::test::CountryCode::create(\"x\")) return 1;\n\
         \x20   if (!urn::test::CountryCode::create(\"US\")) return 1;\n\
         \x20   std::puts(\"ok\");\n\
         \x20   return 0;\n\
         }\n",
    )
    .expect("write C++ probe");
    let output = Command::new("c++")
        .current_dir(&root)
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
        .expect("a C++ compiler must be available");
    assert!(
        output.status.success(),
        "selected C++ output must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected C++ validation must hold"
    );
}

/// Task 039: the selected Ada output compiles and raises `Constraint_Error` on
/// an invalid value, while accepting a value whose leading and trailing SPACE
/// are part of it.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task039_selected_ada_output_validates_under_gnat() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let root = generate_visible_ascii("ada", "task039-ada-runtime");
    std::fs::write(
        root.join("probe.adb"),
        "with Ada.Text_IO;\nwith Urn.Test;\n\n\
         procedure Probe is\n\
         \x20  Good : constant Urn.Test.Callsign :=\n\
         \x20    Urn.Test.Create (\"Falcon-1 {alpha}\");\n\
         \x20  --  whiteSpace = preserve, and SPACE is inside [ -~].\n\
         \x20  Spaced : constant Urn.Test.Callsign := Urn.Test.Create (\"  padded  \");\n\
         \x20  Rejected : Natural := 0;\n\
         \x20  type Case_Names is (Empty, Tabbed, Delete_Char, Too_Long);\n\
         \x20  function Text (Item : Case_Names) return String is\n\
         \x20    (case Item is\n\
         \x20       when Empty       => \"\",\n\
         \x20       when Tabbed      => \"bad\" & ASCII.HT & \"tab\",\n\
         \x20       when Delete_Char => (1 => Character'Val (127)),\n\
         \x20       when Too_Long    => (1 .. 257 => 'x'));\n\
         begin\n\
         \x20  if Urn.Test.Value (Good) /= \"Falcon-1 {alpha}\"\n\
         \x20    or else Urn.Test.Value (Spaced) /= \"  padded  \"\n\
         \x20  then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         \x20  for Item in Case_Names loop\n\
         \x20     begin\n\
         \x20        declare\n\
         \x20           Bad : constant Urn.Test.Callsign := Urn.Test.Create (Text (Item));\n\
         \x20        begin\n\
         \x20           if Urn.Test.Value (Bad)'Length >= 0 then\n\
         \x20              null;\n\
         \x20           end if;\n\
         \x20        end;\n\
         \x20     exception\n\
         \x20        when Constraint_Error => Rejected := Rejected + 1;\n\
         \x20     end;\n\
         \x20  end loop;\n\
         \x20  if Rejected /= 4 then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         \x20  Ada.Text_IO.Put_Line (\"ok\");\n\
         end Probe;\n",
    )
    .expect("write Ada probe");
    let output = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "probe.adb"])
        .output()
        .expect("gnatmake must run");
    assert!(
        output.status.success(),
        "selected Ada output must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected Ada validation must hold"
    );
}

/// Task 040: the *contract-selected* C++ output carries the special-member
/// policy, and a selected carrier keeps a valid value through an rvalue
/// operation.
///
/// The backend-level probes prove the templates are right. This one proves the
/// correction propagates through the ordinary `service-generate` path that a
/// consumer actually uses, rather than only through a directly invoked backend.
#[test]
fn task040_selected_cpp_output_preserves_the_validated_value() {
    let root = generate_visible_ascii("cpp", "task040-cpp");
    std::fs::write(
        root.join("probe.cpp"),
        "#include \"test.hpp\"\n#include <cstdio>\n#include <optional>\n\
         #include <string>\n#include <utility>\n\n\
         using urn::test::Callsign;\n\
         using urn::test::CountryCode;\n\n\
         int main() {\n\
         \x20   auto made = Callsign::create(\"Falcon-1\");\n\
         \x20   if (!made) return 1;\n\
         \x20   Callsign origin = *made;\n\n\
         \x20   // Construction from an rvalue must not strip the source.\n\
         \x20   Callsign taken = std::move(origin);\n\
         \x20   if (taken.value() != \"Falcon-1\") return 1;\n\
         \x20   if (origin.value() != \"Falcon-1\") return 1;\n\
         \x20   // The source must still satisfy its own validator.\n\
         \x20   if (!Callsign::create(origin.value())) return 1;\n\n\
         \x20   // Copying the source AFTER the rvalue operation must not\n\
         \x20   // propagate an invalid representation.\n\
         \x20   Callsign copied = origin;\n\
         \x20   if (copied.value() != \"Falcon-1\") return 1;\n\n\
         \x20   // Assignment from an rvalue, same requirement.\n\
         \x20   Callsign target = *Callsign::create(\"Eagle-2\");\n\
         \x20   target = std::move(origin);\n\
         \x20   if (target.value() != \"Falcon-1\") return 1;\n\
         \x20   if (origin.value() != \"Falcon-1\") return 1;\n\n\
         \x20   // The 2..4 profile, whose minimum is above one character.\n\
         \x20   auto country = CountryCode::create(\"US\");\n\
         \x20   if (!country) return 1;\n\
         \x20   CountryCode moved_country = std::move(*country);\n\
         \x20   if (moved_country.value() != \"US\") return 1;\n\
         \x20   if (!CountryCode::create(country->value())) return 1;\n\n\
         \x20   // std::optional composition still behaves.\n\
         \x20   std::optional<Callsign> boxed = Callsign::create(\"Hawk-3\");\n\
         \x20   std::optional<Callsign> moved_box = std::move(boxed);\n\
         \x20   if (!moved_box || moved_box->value() != \"Hawk-3\") return 1;\n\
         \x20   if (!boxed || boxed->value() != \"Hawk-3\") return 1;\n\n\
         \x20   std::puts(\"ok\");\n\
         \x20   return 0;\n\
         }\n",
    )
    .expect("write C++ probe");
    let output = Command::new("c++")
        .current_dir(&root)
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
        .expect("a C++ compiler must be available");
    assert!(
        output.status.success(),
        "selected C++ output must compile under strict C++17:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(root.join("probe"))
            .status()
            .expect("probe must run")
            .success(),
        "selected C++ carriers must keep their validated value"
    );
}

/// Task 040: the *contract-selected* Ada output rejects an unchecked default
/// declaration while every legitimate construction path still works.
///
/// Built without `-gnata`, so this cannot pass as a client assertion effect.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task040_selected_ada_output_requires_explicit_initialization() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let root = generate_visible_ascii("ada", "task040-ada");
    std::fs::write(
        root.join("probe.adb"),
        "with Ada.Text_IO;\nwith Urn.Test;\n\n\
         procedure Probe is\n\
         \x20  use Ada.Text_IO;\n\
         \x20  Failures : Natural := 0;\n\n\
         \x20  --  Positive controls: Create, copy initialization, assignment.\n\
         \x20  Made : constant Urn.Test.Callsign := Urn.Test.Create (\"Falcon-1\");\n\
         \x20  Copied : constant Urn.Test.Callsign := Made;\n\
         \x20  Assigned : Urn.Test.Callsign := Urn.Test.Create (\"Eagle-2\");\n\
         \x20  --  The 2..4 profile, whose minimum is above one character.\n\
         \x20  Country : constant Urn.Test.CountryCode := Urn.Test.Create (\"US\");\n\
         begin\n\
         \x20  Assigned := Made;\n\
         \x20  if Urn.Test.Value (Made) /= \"Falcon-1\"\n\
         \x20    or else Urn.Test.Value (Copied) /= \"Falcon-1\"\n\
         \x20    or else Urn.Test.Value (Assigned) /= \"Falcon-1\"\n\
         \x20    or else Urn.Test.Value (Country) /= \"US\"\n\
         \x20  then\n\
         \x20     Put_Line (\"a legitimate construction path failed\");\n\
         \x20     Failures := Failures + 1;\n\
         \x20  end if;\n\n\
         \x20  --  The negative controls: an ordinary default declaration must\n\
         \x20  --  fail explicitly rather than yield empty, invalid text.\n\
         \x20  --\n\
         \x20  --  The declaration lives in a nested procedure because an\n\
         \x20  --  exception raised while elaborating a declarative part\n\
         \x20  --  propagates PAST that block's own handler.\n\
         \x20  declare\n\
         \x20     procedure Default_Callsign is\n\
         \x20        Bad : Urn.Test.Callsign;\n\
         \x20     begin\n\
         \x20        Put_Line (\"UNCHECKED: [\" & Urn.Test.Value (Bad) & \"]\");\n\
         \x20     end Default_Callsign;\n\
         \x20  begin\n\
         \x20     Default_Callsign;\n\
         \x20     Put_Line (\"UNCHECKED: default Callsign did not raise\");\n\
         \x20     Failures := Failures + 1;\n\
         \x20  exception\n\
         \x20     when Program_Error => null;\n\
         \x20  end;\n\n\
         \x20  --  The same for the 2..4 profile.\n\
         \x20  declare\n\
         \x20     procedure Default_Country is\n\
         \x20        Bad : Urn.Test.CountryCode;\n\
         \x20     begin\n\
         \x20        Put_Line (\"UNCHECKED: [\" & Urn.Test.Value (Bad) & \"]\");\n\
         \x20     end Default_Country;\n\
         \x20  begin\n\
         \x20     Default_Country;\n\
         \x20     Put_Line (\"UNCHECKED: default CountryCode did not raise\");\n\
         \x20     Failures := Failures + 1;\n\
         \x20  exception\n\
         \x20     when Program_Error => null;\n\
         \x20  end;\n\n\
         \x20  if Failures /= 0 then\n\
         \x20     raise Program_Error;\n\
         \x20  end if;\n\
         \x20  Put_Line (\"ok\");\n\
         end Probe;\n",
    )
    .expect("write Ada probe");
    let output = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "probe.adb"])
        .output()
        .expect("gnatmake must run");
    assert!(
        output.status.success(),
        "selected Ada output must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Command::new(root.join("probe"))
        .output()
        .expect("probe must run");
    let text = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success() && text.contains("ok"),
        "selected Ada carriers must require explicit initialization:\n{text}"
    );
    assert!(
        !text.contains("UNCHECKED"),
        "a default declaration must not yield a usable value:\n{text}"
    );
}

/// Task 040 corrective: the *contract-selected* Ada output can actually build
/// and read back a REPEATED field of validated carriers.
///
/// This is deliberately not a compile-only check. The reviewed head produced a
/// spec that compiled for some schemas yet raised `Program_Error` from inside
/// the container the moment a second valid carrier was appended, because the
/// definite vector default-initialized the replacement array it allocates when
/// growing. A regression that merely compiled the spec would not have caught
/// it, so this probe constructs, grows, copies, clears, and reads.
///
/// Built without `-gnata`, so it cannot pass as a client assertion effect.
///
/// Skipped only where GNAT is absent, as elsewhere in this file.
#[test]
fn task040_selected_ada_repeated_field_is_constructible() {
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "AMS_GRA_REQUIRE_GNAT is set but gnatmake is unavailable"
        );
        return;
    }
    let root = generate_visible_ascii("ada", "task040-ada-repeated");
    std::fs::write(root.join("probe.adb"), ADA_REPEATED_PROBE).expect("write Ada probe");
    let output = Command::new("gnatmake")
        .current_dir(&root)
        .args(["-q", "-f", "probe.adb"])
        .output()
        .expect("gnatmake must run");
    assert!(
        output.status.success(),
        "selected Ada output with a repeated field must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = Command::new(root.join("probe"))
        .output()
        .expect("probe must run");
    let mut text = String::from_utf8_lossy(&run.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&run.stderr));
    assert!(
        run.status.success() && text.contains("ok"),
        "a selected repeated field of validated carriers must be usable:\n{text}"
    );
    assert!(
        !text.contains("PROGRAM_ERROR"),
        "appending valid carriers must not raise from container storage:\n{text}"
    );
}

/// The selected-generation repeated-field probe.
const ADA_REPEATED_PROBE: &str = r##"with Ada.Text_IO;
with Ada.Strings.Unbounded;
with Urn.Test;

procedure Probe is
   use Ada.Text_IO;
   Failures : Natural := 0;

   procedure Check (Got, Want, Label : String) is
   begin
      if Got /= Want then
    Put_Line ("wrong " & Label & ": [" & Got & "]");
    Failures := Failures + 1;
      end if;
   end Check;
begin
   --  A 0..unbounded field of validated carriers: empty, then grown well past
   --  several capacity boundaries. This is the exact reproduction.
   declare
      Aliases : Urn.Test.TrackPayload_Aliases_Sequence;
   begin
      if Urn.Test.Length (Aliases) /= 0 then
    Put_Line ("a zero-minimum sequence must start empty");
    Failures := Failures + 1;
      end if;

      for I in 1 .. 48 loop
    Urn.Test.Append (Aliases, Urn.Test.Create ("A" & I'Image));
      end loop;
      if Urn.Test.Length (Aliases) /= 48 then
    Put_Line ("growth lost elements");
    Failures := Failures + 1;
      end if;
      Check (Urn.Test.Value (Urn.Test.Element (Aliases, 1)), "A 1", "first");
      Check (Urn.Test.Value (Urn.Test.Element (Aliases, 48)), "A 48", "last");

      Urn.Test.Reserve_Capacity (Aliases, 256);
      Check
   (Urn.Test.Value (Urn.Test.Element (Aliases, 48)), "A 48", "after reserve");

      declare
    Copied : Urn.Test.TrackPayload_Aliases_Sequence := Aliases;
      begin
    Check
      (Urn.Test.Value (Urn.Test.Element (Copied, 9)), "A 9", "copied");
    Urn.Test.Clear (Copied);
    Urn.Test.Append (Copied, Urn.Test.Create ("REUSED"));
    Check
      (Urn.Test.Value (Urn.Test.Element (Copied, 1)), "REUSED", "reused");
      end;

      --  A whole selected message value carrying the populated sequence.
      declare
    Report : constant Urn.Test.TrackPayload :=
      (Primary   => Urn.Test.Create ("Falcon-1"),
       Alternate => (Is_Present => False),
       Label     => (Is_Present => False),
       Country   => Urn.Test.Create ("US"),
       Version   => Urn.Test.Create ("002.5.0"),
       Notes     => Ada.Strings.Unbounded.To_Unbounded_String ("n"),
       Aliases   => Aliases,
       Slots     => <>,
       Anchors   =>
         Urn.Test.To_Sequence
           ((Urn.Test.Create ("K-1"), Urn.Test.Create ("K-2"))));
      begin
    Check
      (Urn.Test.Value (Urn.Test.Element (Report.Aliases, 2)), "A 2",
       "message alias");
    --  The 0..3 bounded field is validly EMPTY here: no placeholder
    --  carrier is required to occupy its unused slots.
    if Urn.Test.Length (Report.Slots) /= 0 then
       Put_Line ("an empty bounded field must have length zero");
       Failures := Failures + 1;
    end if;
    --  The 2..3 field carries exactly the two supplied values.
    if Urn.Test.Length (Report.Anchors) /= 2 then
       Put_Line ("a positive-minimum field must carry what was supplied");
       Failures := Failures + 1;
    end if;
    Check
      (Urn.Test.Value (Urn.Test.Element (Report.Anchors, 2)), "K-2",
       "message anchor");
      end;
   end;

   --  The 0..3 bounded field, empty then partially then fully populated,
   --  entirely through the generated API. There is no writable occupancy
   --  and no reachable slot, so a claimed-but-empty position cannot be
   --  written at all.
   declare
      Slots : Urn.Test.TrackPayload_Slots_Sequence;
   begin
      if Urn.Test.Length (Slots) /= 0 then
    Put_Line ("a zero-minimum bounded field must start empty");
    Failures := Failures + 1;
      end if;
      Urn.Test.Append (Slots, Urn.Test.Create ("S-1"));
      Check (Urn.Test.Value (Urn.Test.Element (Slots, 1)), "S-1",
        "bounded partial");
      Urn.Test.Append (Slots, Urn.Test.Create ("S-2"));
      Urn.Test.Append (Slots, Urn.Test.Create ("S-3"));
      Check (Urn.Test.Value (Urn.Test.Element (Slots, 3)), "S-3",
        "bounded full");

      --  maxOccurs is enforced by the generated sequence itself.
      begin
    Urn.Test.Append (Slots, Urn.Test.Create ("S-4"));
    Put_Line ("appending past maxOccurs must be rejected");
    Failures := Failures + 1;
      exception
    when Constraint_Error => null;
      end;

      --  Reading past the logical length is rejected rather than returning
      --  spare capacity.
      begin
    declare
       Unused : constant String :=
         Urn.Test.Value (Urn.Test.Element (Slots, 4));
    begin
       Put_Line ("out-of-range read must be rejected: " & Unused);
       Failures := Failures + 1;
    end;
      exception
    when Constraint_Error => null;
      end;

      Urn.Test.Clear (Slots);
      if Urn.Test.Length (Slots) /= 0 then
    Put_Line ("clearing a zero-minimum field must empty it");
    Failures := Failures + 1;
      end if;
   end;

   --  The 2..3 field: construction with too few values is rejected, and a
   --  valid construction still works (the positive control).
   declare
      Anchors : constant Urn.Test.TrackPayload_Anchors_Sequence :=
    Urn.Test.To_Sequence
      ((Urn.Test.Create ("A-1"), Urn.Test.Create ("A-2"),
        Urn.Test.Create ("A-3")));
      Copied  : Urn.Test.TrackPayload_Anchors_Sequence := Anchors;
   begin
      Check (Urn.Test.Value (Urn.Test.Element (Anchors, 3)), "A-3",
        "required full");
      Check (Urn.Test.Value (Urn.Test.Element (Copied, 1)), "A-1",
        "required copied");
      Copied := Anchors;
      Check (Urn.Test.Value (Urn.Test.Element (Copied, 2)), "A-2",
        "required assigned");
      begin
    declare
       Short : constant Urn.Test.TrackPayload_Anchors_Sequence :=
         Urn.Test.To_Sequence ((1 => Urn.Test.Create ("A-1")));
    begin
       Put_Line
         ("too few values must be rejected:" & Urn.Test.Length (Short)'Image);
       Failures := Failures + 1;
    end;
      exception
    when Constraint_Error => null;
      end;
   end;

   if Failures /= 0 then
      raise Program_Error;
   end if;
   Put_Line ("ok");
end Probe;
"##;
