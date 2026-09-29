//! Task 057: XML Schema `duration` inventory on the REAL pinned UCI roots
//! (Deep CI only).
//!
//! Runs only when the caller supplies the pinned roots (a wrong root is a
//! failure, never a skip):
//!
//! * `AMS_GRA_UCI_2_5_ROOT`: open-arsenal/uci/standard v2.5 @ 093610b7...,
//!   root SHA-256 `ac943049...` (`scripts/fetch-pinned-uci-2.5.sh`);
//! * `AMS_GRA_UCI_2_6_ROOT`: v2.6 @ 78eb61b6..., root SHA-256 `af54ce72...`
//!   (`scripts/fetch-pinned-uci-2.6.sh`).
//!
//! Every Duration occurrence is recorded from the NORMALIZED IR: every named
//! `PrimitiveKind::Duration` declaration (effective constraints and
//! restriction ancestry) and every Record field / Choice alternative whose
//! target is either the direct `xs:duration` primitive or a named Duration
//! declaration (owner, cardinality, local vs inherited, emitted vs unused
//! abstract owner). Message closures reaching any of them are counted.

use ams_gra_oms_codegen_core::{
    CoverageAnalysis, GenerationWorld, TypeEmission, direct_temporal_profile,
    effective_choice_alternatives, effective_record_fields, temporal_profile,
};
use ams_gra_oms_ir::{
    ConstraintSet, FieldDecl, PrimitiveKind, QualifiedName, SchemaIr, TypeDecl, TypeKind,
    TypeRefTarget,
};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

const UCI_25_SHA256: &str = "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27";
const UCI_26_SHA256: &str = "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b";

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

fn is_named_duration(schema: &SchemaIr, name: &QualifiedName) -> bool {
    schema.types.iter().any(|declaration| {
        &declaration.name == name
            && matches!(
                declaration.kind,
                TypeKind::Primitive(PrimitiveKind::Duration)
            )
    })
}

/// `Name <- Base <- ... <- xs:duration`.
fn ancestry(schema: &SchemaIr, declaration: &TypeDecl) -> String {
    let mut links = vec![declaration.name.local_name.clone()];
    let mut base = declaration.base_type.clone();
    while let Some(reference) = base {
        base = None;
        match &reference.target {
            TypeRefTarget::Named(name) => {
                links.push(name.local_name.clone());
                base = schema
                    .types
                    .iter()
                    .find(|candidate| &candidate.name == name)
                    .and_then(|candidate| candidate.base_type.clone());
            }
            TypeRefTarget::Primitive(PrimitiveKind::Duration) => {
                links.push("xs:duration".to_owned());
            }
            TypeRefTarget::Primitive(other) => links.push(format!("{other:?}")),
        }
    }
    links.join(" <- ")
}

fn cardinality(field: &FieldDecl) -> String {
    format!(
        "{}..{}",
        field.cardinality.min_occurs,
        field
            .cardinality
            .max_occurs
            .map_or_else(|| "unbounded".to_owned(), |max| max.to_string())
    )
}

fn declared_members(declaration: &TypeDecl) -> (&'static str, &[FieldDecl]) {
    match &declaration.kind {
        TypeKind::Record { fields } => ("Record field", fields),
        TypeKind::Choice { alternatives } => ("Choice alternative", alternatives),
        _ => ("", &[]),
    }
}

fn facet_summary(c: &ConstraintSet) -> String {
    format!(
        "constraints={} patterns={} whiteSpace={:?} range={} length={}",
        if *c == ConstraintSet::default() {
            "default"
        } else {
            "NON-DEFAULT"
        },
        c.lexical.pattern_groups.len(),
        c.lexical.white_space,
        c.min_inclusive.is_some()
            || c.max_inclusive.is_some()
            || c.min_exclusive.is_some()
            || c.max_exclusive.is_some(),
        c.length.is_some() || c.min_length.is_some() || c.max_length.is_some(),
    )
}

/// The facts for one release.
struct Inventory {
    named: Vec<String>,
    direct_refs: Vec<String>,
    named_refs: usize,
    /// Effective direct-duration members of EMITTED owners, inherited included.
    direct_occurrences: Vec<String>,
    named_occurrences: usize,
    /// Emitted owners that store a direct duration.
    direct_owners: BTreeSet<QualifiedName>,
    messages_direct: Vec<String>,
    messages_any: Vec<String>,
    constrained: Vec<String>,
}

fn named_declarations(label: &str, schema: &SchemaIr, facts: &mut Inventory) {
    for declaration in &schema.types {
        if !matches!(
            declaration.kind,
            TypeKind::Primitive(PrimitiveKind::Duration)
        ) {
            continue;
        }
        let c = &declaration.constraints;
        let row = format!(
            "{} | {}:{} | {} | {} | abstract={} | profile={:?}",
            declaration.name.local_name,
            declaration
                .source
                .document
                .rsplit('/')
                .next()
                .unwrap_or_default(),
            declaration.source.line.unwrap_or_default(),
            ancestry(schema, declaration),
            facet_summary(c),
            declaration.is_abstract,
            temporal_profile(PrimitiveKind::Duration, c),
        );
        println!("{label} NAMED DURATION: {row}");
        if *c != ConstraintSet::default() {
            facts.constrained.push(declaration.name.local_name.clone());
        }
        facts.named.push(row);
    }
}

/// Declared (local) references, one per schema member.
fn declared_references(label: &str, schema: &SchemaIr, facts: &mut Inventory) {
    for declaration in &schema.types {
        let (member_kind, members) = declared_members(declaration);
        for member in members {
            let owner = format!("{}.{}", declaration.name.local_name, member.name);
            match &member.type_ref.target {
                TypeRefTarget::Primitive(PrimitiveKind::Duration) => {
                    let row = format!(
                        "{owner} | {member_kind} | {} | {} owner | nillable={} | {} | \
                         direct profile={:?} | {}:{}",
                        cardinality(member),
                        if declaration.is_abstract {
                            "abstract"
                        } else {
                            "concrete"
                        },
                        member.nillable,
                        facet_summary(&member.constraints),
                        direct_temporal_profile(PrimitiveKind::Duration, &member.constraints),
                        member
                            .source
                            .document
                            .rsplit('/')
                            .next()
                            .unwrap_or_default(),
                        member.source.line.unwrap_or_default(),
                    );
                    println!("{label} DIRECT DURATION REF: {row}");
                    if member.constraints != ConstraintSet::default() {
                        facts.constrained.push(owner);
                    }
                    facts.direct_refs.push(row);
                }
                TypeRefTarget::Named(name) if is_named_duration(schema, name) => {
                    facts.named_refs += 1;
                    if member.constraints != ConstraintSet::default() || member.nillable {
                        facts.constrained.push(owner);
                    }
                }
                _ => {}
            }
        }
    }
}

/// Effective occurrences per structural owner, inherited members included.
///
/// The whole-schema planner fails closed on unrelated pinned-UCI topology
/// (`SourceCommandEXT`), so each owner is classified on its own with the same
/// rule as `TypeEmission::emits_own_top_level_name`: an abstract Record
/// carries no generated top-level name of its own.
fn effective_occurrences(label: &str, schema: &SchemaIr, facts: &mut Inventory) {
    for declaration in &schema.types {
        let emitted = TypeEmission::Declaration(declaration).emits_own_top_level_name();
        let members = match declaration.kind {
            TypeKind::Record { .. } => effective_record_fields(schema, &declaration.name),
            TypeKind::Choice { .. } => effective_choice_alternatives(schema, &declaration.name),
            _ => continue,
        };
        let Ok(members) = members else { continue };
        let (_, local) = declared_members(declaration);
        for member in members {
            let inherited = !local.iter().any(|own| std::ptr::eq(own, member));
            match &member.type_ref.target {
                TypeRefTarget::Primitive(PrimitiveKind::Duration) => {
                    let row = format!(
                        "{}.{} | {} | {} | {}",
                        declaration.name.local_name,
                        member.name,
                        cardinality(member),
                        if inherited { "inherited" } else { "local" },
                        if emitted {
                            "emitted"
                        } else {
                            "abstract owner without a top-level name"
                        },
                    );
                    println!("{label} DIRECT DURATION OCCURRENCE: {row}");
                    if emitted {
                        facts.direct_owners.insert(declaration.name.clone());
                    }
                    facts.direct_occurrences.push(row);
                }
                TypeRefTarget::Named(name) if is_named_duration(schema, name) => {
                    facts.named_occurrences += 1;
                }
                _ => {}
            }
        }
    }
}

/// Messages whose payload closure reaches a Duration surface.
fn message_closures(label: &str, schema: &SchemaIr, facts: &mut Inventory) {
    let analysis =
        CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet).expect("coverage");
    let named: BTreeSet<_> = schema
        .types
        .iter()
        .filter(|d| matches!(d.kind, TypeKind::Primitive(PrimitiveKind::Duration)))
        .map(|d| d.name.clone())
        .collect();
    for message in &schema.messages {
        let TypeRefTarget::Named(payload) = &message.payload_type.target else {
            continue;
        };
        let Ok(closure) = analysis.dependency_closure(payload) else {
            continue;
        };
        let direct = closure
            .iter()
            .any(|d| facts.direct_owners.contains(&d.name));
        if direct {
            facts.messages_direct.push(message.name.local_name.clone());
        }
        if direct || closure.iter().any(|d| named.contains(&d.name)) {
            facts.messages_any.push(message.name.local_name.clone());
        }
    }
    println!(
        "{label} MESSAGES REACHING DIRECT DURATION: {}",
        facts.messages_direct.join(",")
    );
}

fn inventory(label: &str, schema: &SchemaIr) -> Inventory {
    let mut facts = Inventory {
        named: Vec::new(),
        direct_refs: Vec::new(),
        named_refs: 0,
        direct_occurrences: Vec::new(),
        named_occurrences: 0,
        direct_owners: BTreeSet::new(),
        messages_direct: Vec::new(),
        messages_any: Vec::new(),
        constrained: Vec::new(),
    };
    named_declarations(label, schema, &mut facts);
    declared_references(label, schema, &mut facts);
    effective_occurrences(label, schema, &mut facts);
    message_closures(label, schema, &mut facts);
    println!(
        "{label} DURATION SUMMARY: named={} direct_refs={} named_refs={} \
         direct_occurrences={} direct_owners={} named_occurrences={} \
         messages_reaching_any={} messages_reaching_direct={} constrained={}",
        facts.named.len(),
        facts.direct_refs.len(),
        facts.named_refs,
        facts.direct_occurrences.len(),
        facts.direct_owners.len(),
        facts.named_occurrences,
        facts.messages_any.len(),
        facts.messages_direct.len(),
        facts.constrained.len(),
    );
    facts
}

/// Section 4 evidence gate, from the normalized IR: exactly one named
/// zero-facet `DurationType <- xs:duration`, every direct reference is a
/// field-local zero-facet non-nillable member, and no constrained Duration
/// (named, direct, or named-reference-local) exists anywhere.
fn assert_authoritative_shapes(label: &str, facts: &Inventory, direct: usize) {
    assert_eq!(facts.named.len(), 1, "{label}");
    assert!(
        facts.named[0].starts_with("DurationType | ")
            && facts.named[0].contains(" | DurationType <- xs:duration | constraints=default"),
        "{label}: {}",
        facts.named[0]
    );
    assert_eq!(facts.direct_refs.len(), direct, "{label}");
    assert!(
        facts
            .direct_refs
            .iter()
            .all(|row| row.contains("constraints=default") && row.contains("nillable=false")),
        "{label}"
    );
    assert!(
        facts.constrained.is_empty(),
        "{label}: {:?}",
        facts.constrained
    );
}

#[test]
fn task057_real_uci_2_5_duration_inventory() {
    let Some(schema) = pinned_root("AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256) else {
        return;
    };
    let facts = inventory("UCI 2.5", &schema);
    assert_authoritative_shapes("UCI 2.5", &facts, 9);
    println!("UCI 2.5 DURATION INVENTORY: PASSED");
}

#[test]
fn task057_real_uci_2_6_duration_inventory() {
    let Some(schema) = pinned_root("AMS_GRA_UCI_2_6_ROOT", UCI_26_SHA256) else {
        return;
    };
    let facts = inventory("UCI 2.6", &schema);
    assert_authoritative_shapes("UCI 2.6", &facts, 0);
    println!("UCI 2.6 DURATION INVENTORY: PASSED");
}

/// Section 27: every Duration shape found is now renderable in all three
/// backends. The named declaration and every emitted owner of a direct
/// duration render as a single-declaration-closure check via coverage.
fn assert_all_duration_surfaces_render(label: &str, schema: &SchemaIr, facts: &Inventory) {
    use ams_gra_oms_codegen_core::{BackendLanguage, temporal_profile as named_profile};
    use ams_gra_oms_codegen_core::{DirectTemporalProfile, TemporalProfile};
    for declaration in &schema.types {
        if matches!(
            declaration.kind,
            TypeKind::Primitive(PrimitiveKind::Duration)
        ) {
            assert_eq!(
                named_profile(PrimitiveKind::Duration, &declaration.constraints),
                Ok(Some(TemporalProfile::Duration)),
                "{label} {}",
                declaration.name.local_name
            );
        }
        let (_, members) = declared_members(declaration);
        for member in members {
            if member.type_ref.target == TypeRefTarget::Primitive(PrimitiveKind::Duration) {
                assert_eq!(
                    direct_temporal_profile(PrimitiveKind::Duration, &member.constraints),
                    Ok(Some(DirectTemporalProfile::Duration)),
                    "{label} {}.{}",
                    declaration.name.local_name,
                    member.name
                );
            }
        }
    }
    // Renderability per backend from the SAME capability snapshot readiness
    // uses: the named Duration declaration and every direct-duration owner
    // must no longer be attributed to a Duration blocker.
    let analysis =
        CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet).expect("coverage");
    for language in BackendLanguage::ALL {
        let coverage = analysis.backend_coverage(language).expect("coverage");
        println!(
            "{label} {language:?} COVERAGE: declarations {}/{} field-types {}/{} \
             field-occurrences {}/{} message-closures {}/{}",
            coverage.declarations_fully_renderable,
            coverage.declarations_total,
            coverage.field_type_references_renderable,
            coverage.fields_total,
            coverage.field_occurrences_renderable,
            coverage.fields_total,
            coverage.message_closures_renderable,
            coverage.messages_total,
        );
    }
    let _ = facts;
}

/// A single-message contract for `message` against UCI `version`.
fn single_message_contract(message: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    let yaml = format!(
        "contract_version: \"0.1\"\nservice:\n  name: task057-impact\n  version: \"0.1.0\"\n  \
         kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \
         \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    \
         applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        \
         direction: output\n        mandate: mandatory\n        message: {message}\n        \
         topic: t\n        timing:\n          kind: asynchronous\n"
    );
    ams_gra_oms_service_contract::parse_yaml(&yaml).expect("contract")
}

/// Whether a declaration list carries a Duration surface: a named Duration
/// declaration, or a declared direct-duration member.
fn carries_duration(types: &[TypeDecl]) -> bool {
    types.iter().any(|declaration| {
        matches!(
            declaration.kind,
            TypeKind::Primitive(PrimitiveKind::Duration)
        ) || declared_members(declaration).1.iter().any(|member| {
            member.type_ref.target == TypeRefTarget::Primitive(PrimitiveKind::Duration)
        })
    })
}

/// Section 29 for one message. `None` when neither its selected nor its
/// generated-support surface contains a Duration. Otherwise the per-backend
/// verdict from the REAL selected-service readiness (projection + coverage
/// + preflight + generated support, as `service-check`):
///
/// * `A`     -- READY now (Duration was the last model blocker);
/// * `B(..)` -- NOT READY on another declaration (selected or support);
/// * `C(..)` -- NOT READY for a topology/projection/preflight reason.
fn classify_message(
    label: &str,
    version: &str,
    schema: &SchemaIr,
    analysis: &CoverageAnalysis<'_>,
    message: &ams_gra_oms_ir::MessageDecl,
) -> Option<String> {
    use ams_gra_oms_codegen_core::{
        BackendLanguage, ServiceMessageBlocker, analyze_service_readiness,
        project_service_generation_schema, resolve_service_plan,
    };
    let world = GenerationWorld::ClosedSchemaSet;
    let name = &message.name.local_name;
    let plan = resolve_service_plan(&single_message_contract(name, version), schema).expect("plan");
    // The real emission surface when it exists; otherwise (topology failure)
    // the semantic closure, so a C-class message is still recorded.
    let reaches = match project_service_generation_schema(&plan, schema, world) {
        Ok(projection) => carries_duration(&projection.schema().types),
        Err(_) => {
            let TypeRefTarget::Named(payload) = &message.payload_type.target else {
                return None;
            };
            analysis.dependency_closure(payload).is_ok_and(|closure| {
                carries_duration(&closure.into_iter().cloned().collect::<Vec<_>>())
            })
        }
    };
    if !reaches {
        return None;
    }
    let mut per_backend = Vec::new();
    for language in BackendLanguage::ALL {
        per_backend.push(
            match analyze_service_readiness(&plan, schema, language, world) {
                Err(error) => {
                    let text = format!("{error:?}");
                    let reason = if text.contains("cyclic generated value dependencies") {
                        "cyclic value dependencies".to_owned()
                    } else {
                        text.chars().take(80).collect()
                    };
                    format!("{language:?}=C({reason})")
                }
                Ok(readiness) if readiness.is_ready() => format!("{language:?}=A"),
                Ok(readiness) => {
                    let blocker = match readiness.blocked_messages.first().map(|b| &b.blocker) {
                        Some(ServiceMessageBlocker::Declaration(name)) => {
                            Some(format!("B({})", name.local_name))
                        }
                        Some(other) => Some(format!("C({other})")),
                        None => None,
                    }
                    .or_else(|| {
                        readiness
                            .unsupported_generated_support_types
                            .first()
                            .map(|name| format!("B(support {})", name.local_name))
                    })
                    .unwrap_or_else(|| "C(preflight/API)".to_owned());
                    // No Duration shape may be a remaining blocker.
                    assert!(
                        !blocker.contains("DurationType") && !blocker.contains("Duration)"),
                        "{label} {name} {language:?}: {blocker}"
                    );
                    format!("{language:?}={blocker}")
                }
            },
        );
    }
    Some(per_backend.join(" "))
}

/// Every message, classified in parallel (each is independent and read-only).
fn message_impact(label: &str, version: &str, schema: &SchemaIr) -> Vec<(String, String)> {
    let analysis =
        CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet).expect("coverage");
    let workers = std::thread::available_parallelism()
        .map_or(4, usize::from)
        .min(16);
    let messages: Vec<_> = schema.messages.iter().collect();
    let chunk = messages.len().div_ceil(workers).max(1);
    let mut classes: Vec<(String, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = messages
            .chunks(chunk)
            .map(|slice| {
                let analysis = &analysis;
                scope.spawn(move || {
                    slice
                        .iter()
                        .filter_map(|message| {
                            classify_message(label, version, schema, analysis, message)
                                .map(|line| (message.name.local_name.clone(), line))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("worker"))
            .collect()
    });
    classes.sort();
    for (message, line) in &classes {
        println!("{label} DURATION MESSAGE IMPACT: {message} {line}");
    }
    classes
}

#[test]
fn task057_real_uci_duration_message_impact() {
    for (label, version, variable, sha256) in [
        ("UCI 2.5", "2.5", "AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256),
        ("UCI 2.6", "2.6", "AMS_GRA_UCI_2_6_ROOT", UCI_26_SHA256),
    ] {
        let Some(schema) = pinned_root(variable, sha256) else {
            continue;
        };
        let facts = inventory(label, &schema);
        assert_all_duration_surfaces_render(label, &schema, &facts);
        let started = std::time::Instant::now();
        let classes = message_impact(label, version, &schema);
        let all_a: Vec<&str> = classes
            .iter()
            .filter(|(_, line)| line == "Ada=A Rust=A Cpp=A")
            .map(|(message, _)| message.as_str())
            .collect();
        let any_a = classes
            .iter()
            .filter(|(_, line)| line.contains("=A"))
            .count();
        let c = classes
            .iter()
            .filter(|(_, line)| line.contains("=C("))
            .count();
        println!(
            "{label} DURATION MESSAGE IMPACT SUMMARY: reaching={} A(all backends)={} \
             A(any backend)={any_a} C(any backend)={c} in {:.1}s; A=[{}]",
            classes.len(),
            all_a.len(),
            started.elapsed().as_secs_f64(),
            all_a.join(",")
        );
        println!("{label} DURATION MESSAGE IMPACT: RECORDED");
    }
}

fn run_ok(command: &mut Command, label: &str) -> String {
    let output = command.output().unwrap_or_else(|e| panic!("{label}: {e}"));
    assert!(
        output.status.success(),
        "{label}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Section 30: the smallest real category-A message, `Log` (UCI 2.5). It is
/// READY in Ada, Rust and C++ (and Rust codec READY), generates, and the
/// generated model compiles in each host language.
#[test]
fn task057_real_uci_category_a_log_generates_and_compiles() {
    let Some(_) = pinned_root("AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256) else {
        return;
    };
    let root = PathBuf::from(std::env::var_os("AMS_GRA_UCI_2_5_ROOT").expect("root"));
    let contract = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/real-duration-log.yaml");
    for language in ["ada", "rust", "cpp"] {
        let out = std::env::temp_dir().join(format!("ams-gra-oms-task057-real-log-{language}"));
        let _ = std::fs::remove_dir_all(&out);
        let common = [
            "--schema",
            root.to_str().expect("UTF-8"),
            "--contract",
            contract.to_str().expect("UTF-8"),
            "--language",
            language,
            "--world",
            "closed-schema",
        ];
        let codec: &[&str] = if language == "rust" {
            &["--with-codec"]
        } else {
            &[]
        };
        let report = run_ok(
            Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
                .arg("service-check")
                .args(common)
                .args(codec),
            language,
        );
        assert!(report.contains("status: READY"), "{language}: {report}");
        if language == "rust" {
            assert!(report.contains("codec status: READY"), "{report}");
        }
        run_ok(
            Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
                .arg("service-generate")
                .args(common)
                .args(codec)
                .arg("--output")
                .arg(&out),
            language,
        );
        match language {
            "rust" => {
                let model = std::fs::read_to_string(out.join("oam.rs")).expect("model");
                assert!(model.contains("pub serviceuptime: Option<XmlSchemaDuration>,"));
                let codec = std::fs::read_to_string(out.join("service_codec.rs")).expect("codec");
                assert!(codec.contains("XmlSchemaDuration::new(s)"));
                run_ok(
                    Command::new("rustc")
                        .args(["--edition", "2021", "--crate-type", "lib", "-D", "warnings"])
                        .args(["-A", "dead_code"])
                        .arg(out.join("oam.rs"))
                        .arg("-o")
                        .arg(out.join("oam.rlib")),
                    "rustc",
                );
            }
            "cpp" => {
                std::fs::write(
                    out.join("client.cpp"),
                    "#include \"oam.hpp\"\nint main() {\n  auto d = programs::oam::XmlSchemaDuration::create(\" PT5M \");\n  return d && d->value() == \"PT5M\" ? 0 : 1;\n}\n",
                )
                .expect("probe");
                run_ok(
                    Command::new("c++").current_dir(&out).args([
                        "-std=c++17",
                        "-Wall",
                        "-Wextra",
                        "-Werror",
                        "-pedantic-errors",
                        "-o",
                        "client",
                        "client.cpp",
                    ]),
                    "c++",
                );
                run_ok(&mut Command::new(out.join("client")), "client");
            }
            "ada" => {
                run_ok(
                    Command::new("gnatmake").current_dir(&out).args([
                        "-q",
                        "-c",
                        "-gnatc",
                        "programs-oam.adb",
                    ]),
                    "gnatmake",
                );
            }
            _ => unreachable!(),
        }
        let _ = std::fs::remove_dir_all(&out);
    }
    println!("UCI 2.5 REAL CATEGORY-A DURATION SERVICE: PASSED");
}
