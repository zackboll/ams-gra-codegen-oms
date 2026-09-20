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
