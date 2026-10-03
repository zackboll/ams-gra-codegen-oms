//! Task 058: bounded-ASCII String profiles on the REAL pinned UCI roots
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
//! Everything is read from the NORMALIZED IR. The inventory enumerates every
//! named constrained `xs:string` declaration that is not one of the five
//! earlier profiles: the 61 per release that are now
//! `StringProfile::BoundedAscii`, and the 19 that still fail closed after
//! Task 059 (27 additional shapes now use `StructuredAscii`), each
//! printed with its exact normalized pattern text and facets. The member list
//! below is ASSERTED against the classifier's output; it never drives it.

use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, ServiceMessageBlocker, StringProfile,
    TypeEmission, analyze_service_readiness, backend_preflight, effective_choice_alternatives,
    effective_record_fields, project_service_generation_schema, resolve_service_plan,
    string_profile, unsafe_named_declarations,
};
use ams_gra_oms_ir::{
    ConstraintSet, PrimitiveKind, QualifiedName, SchemaIr, TypeDecl, TypeKind, TypeRefTarget,
};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

const UCI_25_SHA256: &str = "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27";
const UCI_26_SHA256: &str = "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b";

/// The 61 declarations (identical in both releases) the evidence gate found
/// in the strict shape. Compared with the classifier's verdicts, exactly.
const MEMBERS: [&str; 61] = [
    "AO_PIM_CodeType",
    "ATO_DMPI_IdentifierType",
    "ATO_PackageIdentificationType",
    "ATO_TargetFacilityNameType",
    "AircraftIdentifierType",
    "AlphanumericDashSpaceUnderscoreString20Type",
    "AlphanumericDashSpaceUnderscoreStringLength12Type",
    "AlphanumericDashSpaceUnderscoreStringLength15Type",
    "AlphanumericDashSpaceUnderscoreStringLength20Type",
    "AlphanumericSpaceStringLength15Type",
    "AlphanumericString20Type",
    "AlphanumericString4Type",
    "AlphanumericString54Type",
    "AlphanumericString6Type",
    "AlphanumericStringLength4Type",
    "AlphanumericStringLength7Type",
    "EOB_CED_NameType",
    "EOB_CED_WeaponSystemType",
    "EmptyType",
    "ICAO_AirfieldIdentifierType",
    "IJMS_TrackNumberType",
    "LaunchPieceType",
    "Link16_VoiceCallSignType",
    "MISP_ClassificationType",
    "MMSI_NumberType",
    "NITF_ACFTB_SensorIdentifierType",
    "NITF_ClassificationAuthorityType",
    "NITF_ClassificationTextType",
    "NITF_ControlAndHandlingType",
    "NITF_IPON_IID2_ProjectCodeType",
    "NITF_ImageSourceType",
    "NITF_OriginatorNameType",
    "NITF_OriginatorPhoneType",
    "NameSpecialCharacterRestrictionType",
    "NumericStringLength5Type",
    "NumericStringLength6Type",
    "OB_ActivityCodeType",
    "OB_CodeWordType",
    "OB_FacilityNameType",
    "OB_LastCollectorType",
    "OB_RecordOwnerType",
    "OctalStringLength4Type",
    "OneUpNumberType",
    "OperatorPhoneNumberType",
    "TailNumberType",
    "UCI_SchemaComponentNameType",
    "USMTF_AircraftCallSignType",
    "USMTF_ExerciseNicknameType",
    "USMTF_MessageSerialNumberType",
    "USMTF_MissionNumberType",
    "USMTF_OperationCodewordType",
    "USMTF_OriginatorType",
    "USMTF_UnitDesignatorType",
    "UnitNameType",
    "UserIdentifierType",
    "VisibleStringLength10Type",
    "VisibleStringLength12Type",
    "VisibleStringLength15Type",
    "VisibleStringLength17Type",
    "VisibleStringLength20Type",
    "VisibleStringLength80Type",
];

/// Neighbours the scope gate deliberately leaves unsupported; each must still
/// fail closed in both releases.
const EXCLUDED_NEIGHBOURS: [&str; 6] = [
    "NotationType",
    "MilitaryGridType",
    "RecordOriginatorType",
    "FIPS_CountryCodeType",
    "IPv4_AddressType",
    "IPv6_AddressType",
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

/// `(depth, "Name <- Base <- ... <- xs:String")`.
fn ancestry(schema: &SchemaIr, declaration: &TypeDecl) -> (usize, String) {
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
            TypeRefTarget::Primitive(kind) => links.push(format!("xs:{kind:?}")),
        }
    }
    (links.len() - 1, links.join(" <- "))
}

fn facets(c: &ConstraintSet) -> String {
    let patterns: Vec<Vec<String>> = c
        .lexical
        .pattern_groups
        .iter()
        .map(|group| {
            group
                .alternatives
                .iter()
                .map(|a| format!("{:?}:{}", a.dialect, a.expression.escape_default()))
                .collect()
        })
        .collect();
    format!(
        "patterns={patterns:?} length={:?} minLength={:?} maxLength={:?} whiteSpace={:?} \
         effectiveWhiteSpace={:?} numeric={}",
        c.length,
        c.min_length,
        c.max_length,
        c.lexical.white_space,
        c.lexical.effective_white_space(PrimitiveKind::String),
        c.min_inclusive.is_some()
            || c.max_inclusive.is_some()
            || c.min_exclusive.is_some()
            || c.max_exclusive.is_some(),
    )
}

fn profile(declaration: &TypeDecl) -> Option<Result<Option<StringProfile>, ()>> {
    matches!(declaration.kind, TypeKind::Primitive(PrimitiveKind::String))
        .then(|| string_profile(PrimitiveKind::String, &declaration.constraints).map_err(|_| ()))
}

fn is_member(declaration: &TypeDecl) -> bool {
    matches!(
        profile(declaration),
        Some(Ok(Some(StringProfile::BoundedAscii { .. })))
    )
}

/// `(declared references, effective occurrences in emitted owners)` per
/// named target, inherited members included.
fn references(schema: &SchemaIr) -> BTreeMap<QualifiedName, (usize, usize)> {
    let mut counts: BTreeMap<QualifiedName, (usize, usize)> = BTreeMap::new();
    for declaration in &schema.types {
        let local: &[_] = match &declaration.kind {
            TypeKind::Record { fields } => fields,
            TypeKind::Choice { alternatives } => alternatives,
            _ => &[],
        };
        for member in local {
            if let TypeRefTarget::Named(name) = &member.type_ref.target {
                counts.entry(name.clone()).or_default().0 += 1;
            }
        }
        if !TypeEmission::Declaration(declaration).emits_own_top_level_name() {
            continue;
        }
        let effective = match declaration.kind {
            TypeKind::Record { .. } => effective_record_fields(schema, &declaration.name),
            TypeKind::Choice { .. } => effective_choice_alternatives(schema, &declaration.name),
            _ => continue,
        };
        for member in effective.into_iter().flatten() {
            if let TypeRefTarget::Named(name) = &member.type_ref.target {
                counts.entry(name.clone()).or_default().1 += 1;
            }
        }
    }
    counts
}

/// The section 1 evidence gate, re-run on every Deep CI build.
fn bounded_inventory(label: &str, schema: &SchemaIr) {
    let references = references(schema);
    let analysis =
        CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet).expect("coverage");
    // Message closures reaching each constrained String declaration.
    let mut reach: BTreeMap<QualifiedName, usize> = BTreeMap::new();
    for message in &schema.messages {
        let TypeRefTarget::Named(payload) = &message.payload_type.target else {
            continue;
        };
        let Ok(closure) = analysis.dependency_closure(payload) else {
            continue;
        };
        for declaration in closure {
            if matches!(
                profile(declaration),
                Some(Ok(Some(StringProfile::BoundedAscii { .. }))) | Some(Err(()))
            ) {
                *reach.entry(declaration.name.clone()).or_default() += 1;
            }
        }
    }
    let mut members = BTreeSet::new();
    let mut unsupported = BTreeSet::new();
    let mut alphabets = BTreeSet::new();
    for declaration in &schema.types {
        let verdict = match profile(declaration) {
            Some(Ok(Some(StringProfile::BoundedAscii { alphabet, length }))) => {
                alphabets.insert(alphabet);
                members.insert(declaration.name.local_name.clone());
                format!("BoundedAscii {alphabet:?} {length:?}")
            }
            Some(Err(())) => {
                unsupported.insert(declaration.name.local_name.clone());
                "UNSUPPORTED".to_owned()
            }
            _ => continue,
        };
        let (depth, chain) = ancestry(schema, declaration);
        let (declared, effective) = references
            .get(&declaration.name)
            .copied()
            .unwrap_or_default();
        println!(
            "{label} CONSTRAINED STRING: {} | {}:{} | depth={depth} {chain} | {} | \
             refs declared={declared} effective={effective} | messages={} | {verdict}",
            declaration.name.local_name,
            declaration
                .source
                .document
                .rsplit('/')
                .next()
                .unwrap_or_default(),
            declaration.source.line.unwrap_or_default(),
            facets(&declaration.constraints),
            reach.get(&declaration.name).copied().unwrap_or_default(),
        );
        if is_member(declaration) {
            assert_eq!(depth, 1, "{label} {}", declaration.name.local_name);
            assert!(chain.ends_with("<- xs:String"), "{chain}");
            assert_eq!(declaration.constraints.lexical.white_space, None);
        }
    }
    println!(
        "{label} BOUNDED ASCII SUMMARY: members={} alphabets={} still-unsupported={}",
        members.len(),
        alphabets.len(),
        unsupported.len()
    );
    let expected: BTreeSet<String> = MEMBERS.iter().map(|&m| m.to_owned()).collect();
    assert_eq!(members, expected, "{label}: member set drifted");
    assert_eq!(alphabets.len(), 19, "{label}");
    assert_eq!(unsupported.len(), 19, "{label}");
    for neighbour in EXCLUDED_NEIGHBOURS {
        assert!(unsupported.contains(neighbour), "{label} {neighbour}");
    }
    // Direct field-local constrained xs:string is out of scope: none exist.
    let direct = schema
        .types
        .iter()
        .flat_map(|d| match &d.kind {
            TypeKind::Record { fields } => fields.as_slice(),
            TypeKind::Choice { alternatives } => alternatives.as_slice(),
            _ => &[],
        })
        .filter(|m| {
            m.type_ref.target == TypeRefTarget::Primitive(PrimitiveKind::String)
                && m.constraints != ConstraintSet::default()
        })
        .count();
    assert_eq!(direct, 0, "{label}");
    // No reference to a member carries field-local facets or nillable.
    for declaration in &schema.types {
        let local: &[_] = match &declaration.kind {
            TypeKind::Record { fields } => fields,
            TypeKind::Choice { alternatives } => alternatives,
            _ => &[],
        };
        for member in local {
            if let TypeRefTarget::Named(name) = &member.type_ref.target
                && expected.contains(&name.local_name)
            {
                assert!(
                    member.constraints == ConstraintSet::default() && !member.nillable,
                    "{label} {}.{}",
                    declaration.name.local_name,
                    member.name
                );
            }
        }
    }
}

#[test]
fn task058_real_uci_2_5_bounded_ascii_inventory() {
    let Some(schema) = pinned_root("AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256) else {
        return;
    };
    bounded_inventory("UCI 2.5", &schema);
    println!("UCI 2.5 BOUNDED ASCII INVENTORY: PASSED");
}

#[test]
fn task058_real_uci_2_6_bounded_ascii_inventory() {
    let Some(schema) = pinned_root("AMS_GRA_UCI_2_6_ROOT", UCI_26_SHA256) else {
        return;
    };
    bounded_inventory("UCI 2.6", &schema);
    println!("UCI 2.6 BOUNDED ASCII INVENTORY: PASSED");
}

#[test]
fn task058_closed_schema_ada_gap_evidence() {
    let world = GenerationWorld::ClosedSchemaSet;
    for (variable, digest, version, ada_count, peer_count) in [
        ("AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256, "2.5", 552, 555),
        ("AMS_GRA_UCI_2_6_ROOT", UCI_26_SHA256, "2.6", 553, 556),
    ] {
        let Some(schema) = pinned_root(variable, digest) else {
            continue;
        };
        let analysis = CoverageAnalysis::new(&schema, world).expect("coverage");
        for (language, count) in [
            (BackendLanguage::Ada, ada_count),
            (BackendLanguage::Rust, peer_count),
            (BackendLanguage::Cpp, peer_count),
        ] {
            assert_eq!(
                analysis
                    .backend_coverage(language)
                    .unwrap()
                    .message_closures_renderable,
                count
            );
        }
        let ada_unsafe = unsafe_named_declarations(&schema, BackendLanguage::Ada, world);
        let rust_unsafe = unsafe_named_declarations(&schema, BackendLanguage::Rust, world);
        let cpp_unsafe = unsafe_named_declarations(&schema, BackendLanguage::Cpp, world);
        let ada = analysis
            .renderable_message_closure_names(BackendLanguage::Ada)
            .unwrap();
        let rust = analysis
            .renderable_message_closure_names(BackendLanguage::Rust)
            .unwrap();
        let cpp = analysis
            .renderable_message_closure_names(BackendLanguage::Cpp)
            .unwrap();
        assert_eq!(rust, cpp);
        assert_eq!(rust.len(), peer_count);
        assert_eq!(ada.len(), ada_count);
        let expected = [
            "Authorization",
            "AuthorizationRequest",
            "CommSupportActivity",
        ];
        let difference: Vec<_> = rust
            .difference(&ada)
            .map(|n| n.local_name.as_str())
            .collect();
        assert_eq!(difference, expected, "{version}");
        assert!(ada.difference(&rust).next().is_none());
        let ada_only: Vec<_> = ada_unsafe
            .difference(&rust_unsafe)
            .filter(|n| !cpp_unsafe.contains(*n))
            .map(|n| n.local_name.as_str())
            .collect();
        assert_eq!(ada_only, ["QueryPET", "QueryType"]);
        let error = backend_preflight(&schema, BackendLanguage::Ada, world)
            .expect_err("full-schema Ada name conflict")
            .to_string();
        assert_eq!(
            error,
            "Ada names \"QueryType companion\" and \"QueryType\" both generate \
             \"QueryType_Kind\" in the generated top-level scope"
        );
        assert!(backend_preflight(&schema, BackendLanguage::Rust, world).is_ok());
        assert!(backend_preflight(&schema, BackendLanguage::Cpp, world).is_ok());
        for name in expected {
            let message = schema
                .messages
                .iter()
                .find(|m| m.name.local_name == name)
                .unwrap();
            let TypeRefTarget::Named(payload) = &message.payload_type.target else {
                panic!("named payload")
            };
            let closure = analysis.dependency_closure(payload).unwrap();
            let blockers: Vec<_> = closure
                .iter()
                .filter(|d| {
                    ada_unsafe.contains(&d.name)
                        && !rust_unsafe.contains(&d.name)
                        && !cpp_unsafe.contains(&d.name)
                })
                .map(|d| d.name.local_name.as_str())
                .collect();
            let members: Vec<_> = closure
                .iter()
                .filter(|d| is_member(d))
                .map(|d| d.name.local_name.as_str())
                .collect();
            assert_eq!(blockers, ["QueryPET"], "{version} {name}");
            assert_eq!(
                members,
                [
                    "AlphanumericDashSpaceUnderscoreStringLength15Type",
                    "EmptyType"
                ],
                "{version} {name}"
            );
            assert!(!closure.iter().any(|d| d.name.local_name == "QueryType"));
            assert!(members.iter().all(|n| {
                !ada_unsafe
                    .iter()
                    .chain(&rust_unsafe)
                    .chain(&cpp_unsafe)
                    .any(|unsafe_name| unsafe_name.local_name == *n)
            }));
            let plan =
                resolve_service_plan(&single_message_contract(name, version), &schema).unwrap();
            let projection = project_service_generation_schema(&plan, &schema, world).unwrap();
            let projected_unsafe =
                unsafe_named_declarations(projection.schema(), BackendLanguage::Ada, world);
            assert!(
                projection
                    .schema()
                    .types
                    .iter()
                    .any(|d| d.name.local_name == "QueryPET")
            );
            assert!(
                !projection
                    .schema()
                    .types
                    .iter()
                    .any(|d| d.name.local_name == "QueryType")
            );
            assert!(!projected_unsafe.iter().any(|n| n.local_name == "QueryPET"));
            assert!(backend_preflight(projection.schema(), BackendLanguage::Ada, world).is_ok());
            for language in BackendLanguage::ALL {
                assert_eq!(verdict(&plan, &schema, language), "A", "{version} {name}");
            }
        }
        println!("UCI {version} CLOSED-SCHEMA ADA GAP: PASSED {difference:?}");
    }
}

/// A single-message contract for `message` against UCI `version`.
fn single_message_contract(message: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    let yaml = format!(
        "contract_version: \"0.1\"\nservice:\n  name: task058-impact\n  version: \"0.1.0\"\n  \
         kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \
         \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    \
         applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        \
         direction: output\n        mandate: mandatory\n        message: {message}\n        \
         topic: t\n        timing:\n          kind: asynchronous\n"
    );
    ams_gra_oms_service_contract::parse_yaml(&yaml).expect("contract")
}

/// One backend's verdict: `A` READY; `B(x)` NOT READY on selected
/// declaration `x`; `B(support x)` on generated support only; `C(..)`
/// topology / projection / preflight.
fn verdict(
    plan: &ams_gra_oms_codegen_core::ServicePlan,
    schema: &SchemaIr,
    language: BackendLanguage,
) -> String {
    match analyze_service_readiness(plan, schema, language, GenerationWorld::ClosedSchemaSet) {
        Err(error) => {
            let text = format!("{error:?}");
            if text.contains("cyclic generated value dependencies") {
                "C(cyclic value dependencies)".to_owned()
            } else {
                format!("C({})", text.chars().take(80).collect::<String>())
            }
        }
        Ok(readiness) if readiness.is_ready() => "A".to_owned(),
        Ok(readiness) => match readiness.blocked_messages.first().map(|b| &b.blocker) {
            Some(ServiceMessageBlocker::Declaration(name)) => format!("B({})", name.local_name),
            Some(other) => format!("C({other})"),
            None => readiness
                .unsupported_generated_support_types
                .first()
                .map_or_else(
                    || "C(preflight/API)".to_owned(),
                    |name| format!("B(support {})", name.local_name),
                ),
        },
    }
}

/// Per-backend verdicts for a message whose selected or generated-support
/// surface holds a Task 058 member; `None` when it holds none.
fn classify_message(
    version: &str,
    schema: &SchemaIr,
    analysis: &CoverageAnalysis<'_>,
    message: &ams_gra_oms_ir::MessageDecl,
) -> Option<String> {
    let name = &message.name.local_name;
    let plan = resolve_service_plan(&single_message_contract(name, version), schema).ok()?;
    let reaches =
        match project_service_generation_schema(&plan, schema, GenerationWorld::ClosedSchemaSet) {
            Ok(projection) => projection.schema().types.iter().any(is_member),
            Err(_) => {
                let TypeRefTarget::Named(payload) = &message.payload_type.target else {
                    return None;
                };
                analysis
                    .dependency_closure(payload)
                    .is_ok_and(|closure| closure.into_iter().any(is_member))
            }
        };
    if !reaches {
        return None;
    }
    let mut verdicts = Vec::new();
    for language in BackendLanguage::ALL {
        let verdict = verdict(&plan, schema, language);
        // No Task 058 member may remain a blocker.
        for member in MEMBERS {
            assert!(
                verdict != format!("B({member})") && verdict != format!("B(support {member})"),
                "{version} {name} {language:?}: {verdict}"
            );
        }
        verdicts.push(format!("{language:?}={verdict}"));
    }
    Some(verdicts.join(" "))
}

/// Every message, classified in parallel (each is independent, read-only).
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
                            classify_message(version, schema, analysis, message)
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
        println!("{label} BOUNDED ASCII MESSAGE IMPACT: {message} {line}");
    }
    classes
}

#[test]
fn task058_real_uci_bounded_ascii_message_impact() {
    for (label, version, variable, sha256) in [
        ("UCI 2.5", "2.5", "AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256),
        ("UCI 2.6", "2.6", "AMS_GRA_UCI_2_6_ROOT", UCI_26_SHA256),
    ] {
        let Some(schema) = pinned_root(variable, sha256) else {
            continue;
        };
        let started = std::time::Instant::now();
        let classes = message_impact(label, version, &schema);
        let all_a: Vec<&str> = classes
            .iter()
            .filter(|(_, line)| line == "Ada=A Rust=A Cpp=A")
            .map(|(message, _)| message.as_str())
            .collect();
        let mut distribution: BTreeMap<String, usize> = BTreeMap::new();
        for (message, line) in &classes {
            // `Ada=<v> Rust=<v> Cpp=<v>`; a verdict may itself contain spaces.
            let (ada, rest) = line
                .strip_prefix("Ada=")
                .and_then(|rest| rest.split_once(" Rust="))
                .expect("Ada verdict");
            let (rust, cpp) = rest.split_once(" Cpp=").expect("Rust verdict");
            let verdicts: BTreeSet<&str> = [ada, rust, cpp].into_iter().collect();
            // Readiness is backend-uniform for every message here.
            assert_eq!(verdicts.len(), 1, "{label} {message}: {line}");
            *distribution
                .entry(verdicts.into_iter().next().expect("one").to_owned())
                .or_default() += 1;
        }
        println!(
            "{label} BOUNDED ASCII MESSAGE IMPACT SUMMARY: reaching={} A={} in {:.1}s; A=[{}]",
            classes.len(),
            all_a.len(),
            started.elapsed().as_secs_f64(),
            all_a.join(",")
        );
        for (verdict, count) in &distribution {
            println!("{label} BOUNDED ASCII VERDICT DISTRIBUTION: {verdict} {count}");
        }
        println!("{label} BOUNDED ASCII MESSAGE IMPACT: RECORDED");
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

/// Section 12: the smallest real message made READY by Task 058, UCI 2.5
/// `AMTI_SettingsCommand` (61 selected declarations, no generated support;
/// previously NOT READY on `EmptyType` in every backend). READY in Ada, Rust
/// and C++ (Rust codec READY), generated, and compiled in each host language
/// with a zero-length `EmptyType` constructed through the generated API.
#[test]
fn task058_real_uci_newly_ready_service_generates_and_compiles() {
    let Some(_) = pinned_root("AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256) else {
        return;
    };
    let root = PathBuf::from(std::env::var_os("AMS_GRA_UCI_2_5_ROOT").expect("root"));
    let contract = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/real-bounded-ascii-amti-settings.yaml");
    for language in ["ada", "rust", "cpp"] {
        let out = std::env::temp_dir().join(format!("ams-gra-oms-task058-real-amti-{language}"));
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
        assert!(report.contains("selected type closure: 61"), "{report}");
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
        compile_generated(language, &out);
        let _ = std::fs::remove_dir_all(&out);
    }
    println!("UCI 2.5 REAL NEWLY-READY BOUNDED ASCII SERVICE: PASSED");
}

fn compile_generated(language: &str, out: &std::path::Path) {
    match language {
        "rust" => {
            let model = std::fs::read_to_string(out.join("oam.rs")).expect("model");
            assert!(model.contains("pub unassignall: Option<EmptyType>,"));
            assert!(model.contains("const LENGTH: usize = 0;"));
            let codec = std::fs::read_to_string(out.join("service_codec.rs")).expect("codec");
            assert!(codec.contains("super::model::EmptyType::new(x)"));
            std::fs::write(
                out.join("probe.rs"),
                "#![allow(dead_code)]\n#[path = \"oam.rs\"]\nmod oam;\nfn main() {\n    \
                 assert_eq!(oam::EmptyType::new(\"\").expect(\"empty\").as_str(), \"\");\n    \
                 assert!(oam::EmptyType::new(\" \").is_none());\n}\n",
            )
            .expect("probe");
            run_ok(
                Command::new("rustc").current_dir(out).args([
                    "--edition",
                    "2021",
                    "-D",
                    "warnings",
                    "-o",
                    "probe",
                    "probe.rs",
                ]),
                "rustc",
            );
            run_ok(&mut Command::new(out.join("probe")), "rust probe");
        }
        "cpp" => {
            std::fs::write(
                out.join("client.cpp"),
                "#include \"oam.hpp\"\nint main() {\n  auto e = programs::oam::EmptyType::create(\"\");\n  \
                 return e && e->value().empty() && !programs::oam::EmptyType::create(\" \") ? 0 : 1;\n}\n",
            )
            .expect("probe");
            run_ok(
                Command::new("c++").current_dir(out).args([
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
                Command::new("gnatmake").current_dir(out).args([
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
}
