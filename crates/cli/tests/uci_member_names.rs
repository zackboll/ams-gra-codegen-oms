//! Task 054: effective emitted Record-field / Choice-alternative identifier
//! inventory on the REAL pinned UCI roots, plus the whole-schema first-blocker
//! probe after remapping.
//!
//! Runs only when the caller supplies a pinned root (a wrong root is a
//! failure, never a skip):
//!
//! * `AMS_GRA_UCI_2_5_ROOT`: open-arsenal/uci/standard v2.5 @ 093610b7...,
//!   root SHA-256 `ac943049...` (`scripts/fetch-pinned-uci-2.5.sh`);
//! * `AMS_GRA_UCI_2_6_ROOT`: v2.6 @ 78eb61b6..., root SHA-256 `af54ce72...`
//!   (`scripts/fetch-pinned-uci-2.6.sh`).
//!
//! The inventory comes from the shared `structural_member_name_inventory`,
//! which walks the same emission plan and storage classification as
//! generated-name preflight. Exact expectations are asserted so a
//! normalization or policy change is noticed.

use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, GenerationWorld, StructuralMemberKind, StructuralMemberNameRecord,
    structural_member_name_inventory,
};
use ams_gra_oms_ir::SchemaIr;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

const UCI_25_SHA256: &str = "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27";
const UCI_26_SHA256: &str = "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b";

const WORLDS: [(GenerationWorld, &str); 2] = [
    (GenerationWorld::ClosedSchemaSet, "closed-schema"),
    (GenerationWorld::OpenExtensions, "open-extensions"),
];

const LANGUAGES: [BackendLanguage; 3] = [
    BackendLanguage::Ada,
    BackendLanguage::Rust,
    BackendLanguage::Cpp,
];

fn pinned_root(variable: &str, sha256: &str) -> Option<SchemaIr> {
    let Some(root) = std::env::var_os(variable) else {
        eprintln!("SKIPPED: {variable} is not set");
        return None;
    };
    let root = PathBuf::from(root);
    let digest = Command::new("sha256sum")
        .arg(&root)
        .output()
        .expect("sha256sum runs");
    assert_eq!(
        String::from_utf8_lossy(&digest.stdout)
            .split_whitespace()
            .next(),
        Some(sha256),
        "{variable} {} is not the pinned root",
        root.display()
    );
    Some(load_schema_set(&root).expect("pinned UCI root loads"))
}

/// Per-cell summary counts.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Summary {
    record_fields: usize,
    choice_alternatives: usize,
    inherited: usize,
    syntax_invalid: usize,
    reserved: usize,
    escaped: usize,
    escape_illegal: usize,
    collisions: usize,
}

fn summarize(rows: &[StructuralMemberNameRecord]) -> Summary {
    let mut summary = Summary::default();
    for row in rows {
        match row.kind {
            StructuralMemberKind::RecordField => summary.record_fields += 1,
            StructuralMemberKind::ChoiceAlternative => summary.choice_alternatives += 1,
        }
        summary.inherited += usize::from(row.inherited);
        summary.syntax_invalid += usize::from(row.ordinary.is_none());
        summary.reserved += usize::from(row.ordinary_reserved);
        summary.escaped += usize::from(row.escaped());
        summary.escape_illegal += usize::from(row.ordinary_reserved && row.generated.is_none());
        summary.collisions += usize::from(row.region_collision);
    }
    summary
}

/// `kind source -> ordinary => final` for every unsafe ordinary candidate.
fn distinct_unsafe(rows: &[StructuralMemberNameRecord]) -> BTreeSet<String> {
    rows.iter()
        .filter(|row| row.ordinary_reserved || row.ordinary.is_none())
        .map(|row| {
            format!(
                "{} {} -> {} => {}",
                row.kind.label(),
                row.source_name,
                row.ordinary.as_deref().unwrap_or("<invalid>"),
                row.generated.as_deref().unwrap_or("<none>")
            )
        })
        .collect()
}

struct Cell {
    label: String,
    summary: Summary,
    unsafe_names: BTreeSet<String>,
}

/// Print the whole inventory for one release and return per-cell summaries.
fn inventory(release: &str, schema: &SchemaIr) -> Vec<Cell> {
    let mut cells = Vec::new();
    for (world, world_label) in WORLDS {
        for language in LANGUAGES {
            let rows = structural_member_name_inventory(schema, language, world);
            let summary = summarize(&rows);
            let label = format!("{release} {world_label} {}", language.name());
            println!(
                "{label}: record-fields {} choice-alternatives {} inherited {} \
                 syntax-invalid {} reserved {} escaped {} escape-illegal {} \
                 post-remap-collisions {}",
                summary.record_fields,
                summary.choice_alternatives,
                summary.inherited,
                summary.syntax_invalid,
                summary.reserved,
                summary.escaped,
                summary.escape_illegal,
                summary.collisions
            );
            for row in rows.iter().filter(|row| {
                row.ordinary_reserved || row.ordinary.is_none() || row.region_collision
            }) {
                println!(
                    "  ROW {label} | {} | {} | {} | {} | ordinary {} | reserved {} | final {} | collision {}",
                    row.owner.local_name,
                    row.source_name,
                    row.kind.label(),
                    if row.inherited { "inherited" } else { "local" },
                    row.ordinary.as_deref().unwrap_or("<syntax-invalid>"),
                    row.ordinary_reserved,
                    row.generated.as_deref().unwrap_or("<none>"),
                    row.region_collision
                );
            }
            let unsafe_names = distinct_unsafe(&rows);
            for name in &unsafe_names {
                println!("  DISTINCT {label} | {name}");
            }
            cells.push(Cell {
                label,
                summary,
                unsafe_names,
            });
        }
    }
    cells
}

/// Evidence gate + outcome: zero syntax-invalid members, every reserved
/// candidate escaped to a legal spelling, zero post-remap collisions.
fn assert_gate(cells: &[Cell]) {
    for cell in cells {
        let (label, summary) = (&cell.label, &cell.summary);
        assert_eq!(summary.syntax_invalid, 0, "{label}: syntax-invalid members");
        assert_eq!(summary.escape_illegal, 0, "{label}: illegal escapes");
        assert_eq!(summary.reserved, summary.escaped, "{label}");
        assert_eq!(summary.collisions, 0, "{label}: post-remap collisions");
    }
}

fn cell<'a>(cells: &'a [Cell], label: &str) -> &'a Cell {
    cells
        .iter()
        .find(|cell| cell.label == label)
        .unwrap_or_else(|| panic!("no cell {label}"))
}

fn backend(language: BackendLanguage) -> Box<dyn Backend> {
    match language {
        BackendLanguage::Ada => Box::new(ams_gra_oms_backend_ada::AdaBackend),
        BackendLanguage::Rust => Box::new(ams_gra_oms_backend_rust::RustBackend),
        BackendLanguage::Cpp => Box::new(ams_gra_oms_backend_cpp::CppBackend),
    }
}

/// The whole-schema generation outcome per backend/world after remapping.
fn first_blockers(release: &str, schema: &SchemaIr) -> Vec<(String, String)> {
    let mut blockers = Vec::new();
    for (world, world_label) in WORLDS {
        for language in LANGUAGES {
            let outcome = match backend(language).generate(schema, world) {
                Ok(_) => "GENERATED".to_owned(),
                Err(error) => error.message,
            };
            let label = format!("{release} {world_label} {}", language.name());
            println!("FIRST BLOCKER {label}: {outcome}");
            blockers.push((label, outcome));
        }
    }
    blockers
}

/// No whole-schema first blocker may still be a structural-member reserved
/// word: those are exactly what Task 054 remaps.
fn assert_no_member_reserved_blocker(blockers: &[(String, String)]) {
    for (label, outcome) in blockers {
        assert!(
            !(outcome.contains("generates reserved word") && outcome.contains("in the members of")),
            "{label}: a member reserved word is still the first blocker: {outcome}"
        );
    }
}

/// Distinct unsafe source members, identical in 2.5 and 2.6 and in both
/// worlds (only occurrence counts differ).
const ADA_DISTINCT: [&str; 15] = [
    "Choice alternative All -> All => Alternative_All",
    "Choice alternative And -> And => Alternative_And",
    "Choice alternative Body -> Body => Alternative_Body",
    "Choice alternative Not -> Not => Alternative_Not",
    "Choice alternative Or -> Or => Alternative_Or",
    "Choice alternative Range -> Range => Alternative_Range",
    "Choice alternative Task -> Task => Alternative_Task",
    "Record field All -> All => Field_All",
    "Record field Begin -> Begin => Field_Begin",
    "Record field End -> End => Field_End",
    "Record field Function -> Function => Field_Function",
    "Record field Range -> Range => Field_Range",
    "Record field Task -> Task => Field_Task",
    "Record field Type -> Type => Field_Type",
    "Record field When -> When => Field_When",
];
const RUST_DISTINCT: [&str; 2] = [
    "Record field Type -> type => field_type",
    "Record field Yield -> yield => field_yield",
];
const CPP_DISTINCT: [&str; 4] = [
    "Record field Auto -> auto => field_auto",
    "Record field Delete -> delete => field_delete",
    "Record field Friend -> friend => field_friend",
    "Record field Operator -> operator => field_operator",
];

/// `(cell label suffix, record fields, choice alternatives, inherited, reserved)`.
type Expected = (&'static str, usize, usize, usize, usize);

fn assert_cells(release: &str, cells: &[Cell], expected: &[Expected]) {
    assert_eq!(cells.len(), expected.len());
    for (suffix, record_fields, choice_alternatives, inherited, reserved) in expected {
        let label = format!("{release} {suffix}");
        let found = cell(cells, &label);
        assert_eq!(
            found.summary,
            Summary {
                record_fields: *record_fields,
                choice_alternatives: *choice_alternatives,
                inherited: *inherited,
                syntax_invalid: 0,
                reserved: *reserved,
                escaped: *reserved,
                escape_illegal: 0,
                collisions: 0,
            },
            "{label}"
        );
        let distinct: &[&str] = if suffix.ends_with("Ada") {
            &ADA_DISTINCT
        } else if suffix.ends_with("Rust") {
            &RUST_DISTINCT
        } else {
            &CPP_DISTINCT
        };
        assert_eq!(
            found
                .unsafe_names
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            distinct,
            "{label}"
        );
    }
    assert_gate(cells);
}

#[test]
fn task054_real_uci_2_5_member_identifier_inventory() {
    let Some(schema) = pinned_root("AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256) else {
        return;
    };
    let cells = inventory("UCI 2.5", &schema);
    assert_cells(
        "UCI 2.5",
        &cells,
        &[
            ("closed-schema Ada", 17614, 1468, 6037, 58),
            ("closed-schema Rust", 17614, 1468, 6037, 10),
            ("closed-schema C++", 17614, 1468, 6037, 7),
            ("open-extensions Ada", 17551, 1468, 6025, 58),
            ("open-extensions Rust", 17551, 1468, 6025, 10),
            ("open-extensions C++", 17551, 1468, 6025, 7),
        ],
    );
    println!("UCI 2.5 MEMBER IDENTIFIER INVENTORY: PASSED");
}

#[test]
fn task054_real_uci_2_6_member_identifier_inventory() {
    let Some(schema) = pinned_root("AMS_GRA_UCI_2_6_ROOT", UCI_26_SHA256) else {
        return;
    };
    let cells = inventory("UCI 2.6", &schema);
    assert_cells(
        "UCI 2.6",
        &cells,
        &[
            ("closed-schema Ada", 17778, 1482, 6178, 58),
            ("closed-schema Rust", 17778, 1482, 6178, 10),
            ("closed-schema C++", 17778, 1482, 6178, 6),
            ("open-extensions Ada", 17715, 1482, 6166, 58),
            ("open-extensions Rust", 17715, 1482, 6166, 10),
            ("open-extensions C++", 17715, 1482, 6166, 6),
        ],
    );
    println!("UCI 2.6 MEMBER IDENTIFIER INVENTORY: PASSED");
}

/// The six (x2 worlds) whole-schema first blockers AFTER remapping. None may
/// be a structural-member reserved word any more; each is recorded exactly,
/// and each is a pre-existing boundary the reserved-member failure used to
/// mask. Task 054 does not fix them.
const FIRST_BLOCKERS: [(&str, &str); 6] = [
    (
        "closed-schema Ada",
        // Task065 supersedes the historical Task054 naming blocker; the
        // original measurement remains in its historical documentation.
        "unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants",
    ),
    (
        "closed-schema Rust",
        "unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants",
    ),
    (
        "closed-schema C++",
        "unsupported abstract structural value: abstract value target SourceCommandEXT has no concrete structural descendants",
    ),
    (
        "open-extensions Ada",
        "unsupported abstract structural value: abstract value CapabilityCommandBaseType is not closed under open-extensions generation; external derived types cannot be represented",
    ),
    (
        "open-extensions Rust",
        "unsupported abstract structural value: abstract value CapabilityCommandBaseType is not closed under open-extensions generation; external derived types cannot be represented",
    ),
    (
        "open-extensions C++",
        "unsupported abstract structural value: abstract value CapabilityCommandBaseType is not closed under open-extensions generation; external derived types cannot be represented",
    ),
];

#[test]
fn task054_real_uci_whole_schema_first_blockers() {
    for (release, variable, sha) in [
        ("UCI 2.5", "AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256),
        ("UCI 2.6", "AMS_GRA_UCI_2_6_ROOT", UCI_26_SHA256),
    ] {
        let Some(schema) = pinned_root(variable, sha) else {
            continue;
        };
        let blockers = first_blockers(release, &schema);
        assert_no_member_reserved_blocker(&blockers);
        for (suffix, expected) in FIRST_BLOCKERS {
            let label = format!("{release} {suffix}");
            let (_, outcome) = blockers
                .iter()
                .find(|(cell, _)| *cell == label)
                .unwrap_or_else(|| panic!("no cell {label}"));
            assert_eq!(outcome, expected, "{label}");
        }
        println!("{release} WHOLE-SCHEMA FIRST BLOCKER PROBE: PASSED");
    }
}

fn run_ok(command: &mut Command, label: &str) {
    let output = command.output().unwrap_or_else(|e| panic!("{label}: {e}"));
    assert!(
        output.status.success(),
        "{label}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Real category-A selection (`real-member-remapping.yaml`):
/// SystemOrbitalElementSetRequest (Ada `Begin`/`End` -> `Field_Begin`/
/// `Field_End`) and ApprovalRequestStatus (C++ `operator` ->
/// `field_operator`). Before Task 054 the Ada and C++ selections were NOT
/// READY on exactly those members. Now each backend is READY, generates, and
/// the generated model compiles in its host language.
#[test]
fn task054_real_uci_category_a_selection_generates_and_compiles() {
    let Some(root) = std::env::var_os("AMS_GRA_UCI_2_5_ROOT") else {
        eprintln!("SKIPPED: AMS_GRA_UCI_2_5_ROOT is not set");
        return;
    };
    // Verify the pin before trusting any result.
    pinned_root("AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256).expect("pinned root");
    let root = PathBuf::from(root);
    let contract = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/real-member-remapping.yaml");
    for language in ["ada", "rust", "cpp"] {
        let out = std::env::temp_dir().join(format!("ams-gra-oms-task054-real-{language}"));
        let _ = std::fs::remove_dir_all(&out);
        let common = [
            "--schema",
            root.to_str().unwrap(),
            "--contract",
            contract.to_str().unwrap(),
            "--language",
            language,
            "--world",
            "closed-schema",
        ];
        let check = Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
            .arg("service-check")
            .args(common)
            .output()
            .expect("CLI runs");
        let check_text = String::from_utf8_lossy(&check.stdout);
        assert!(
            check_text.lines().any(|line| line == "status: READY"),
            "{language}: {check_text}"
        );
        run_ok(
            Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
                .arg("service-generate")
                .args(common)
                .args(["--output", out.to_str().unwrap()]),
            &format!("{language} service-generate"),
        );
        match language {
            "ada" => {
                let spec = std::fs::read_to_string(out.join("programs-oam.ads")).unwrap();
                for needle in [
                    "      Field_Begin : DateTimeRangeType_Field_Begin_Optional;",
                    "      Field_End : DateTimeRangeType_Field_End_Optional;",
                ] {
                    assert!(spec.contains(needle), "missing {needle:?}");
                }
                if Command::new("gnatmake").arg("--version").output().is_err() {
                    assert!(
                        std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                        "GNAT required"
                    );
                    continue;
                }
                run_ok(
                    Command::new("gnatmake").current_dir(&out).args([
                        "-q",
                        "-gnat2012",
                        "-gnatwa",
                        "-gnatc",
                        "programs-oam.ads",
                        "service_api.ads",
                    ]),
                    "GNAT semantic check",
                );
            }
            "rust" => {
                let model = std::fs::read_to_string(out.join("oam.rs")).unwrap();
                // No pinned category-A closure reaches a Rust-reserved member;
                // `begin`/`end` are legal Rust and stay unchanged.
                assert!(model.contains("    pub begin: Option<DateTimeType>,"));
                assert!(!model.contains("r#"), "no raw identifiers");
                run_ok(
                    Command::new("rustc").current_dir(&out).args([
                        "--edition",
                        "2024",
                        "--crate-type",
                        "lib",
                        "-D",
                        "warnings",
                        "oam.rs",
                    ]),
                    "rustc -D warnings",
                );
            }
            _ => {
                let header = std::fs::read_to_string(out.join("oam.hpp")).unwrap();
                assert!(header.contains(" field_operator;"), "field_operator");
                assert!(!header.contains(" operator;"));
                std::fs::write(
                    out.join("probe.cpp"),
                    "#include \"oam.hpp\"\nint main() { return 0; }\n",
                )
                .unwrap();
                run_ok(
                    Command::new("c++").current_dir(&out).args([
                        "-std=c++17",
                        "-Wall",
                        "-Wextra",
                        "-Werror",
                        "-pedantic-errors",
                        "-fsyntax-only",
                        "probe.cpp",
                    ]),
                    "strict C++17",
                );
            }
        }
    }
    println!("UCI 2.5 REAL CATEGORY-A MEMBER REMAPPING: PASSED");
}
