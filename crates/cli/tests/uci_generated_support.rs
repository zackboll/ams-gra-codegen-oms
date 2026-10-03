//! Task 056: readiness / generated-support parity on the REAL pinned UCI
//! roots (Deep CI only).
//!
//! Runs only when the caller supplies the pinned roots (a wrong root is a
//! failure, never a skip):
//!
//! * `AMS_GRA_UCI_2_5_ROOT`: open-arsenal/uci/standard v2.5 @ 093610b7...,
//!   root SHA-256 `ac943049...` (`scripts/fetch-pinned-uci-2.5.sh`);
//! * `AMS_GRA_UCI_2_6_ROOT`: v2.6 @ 78eb61b6..., root SHA-256 `af54ce72...`
//!   (`scripts/fetch-pinned-uci-2.6.sh`).
//!
//! Before Task 056 a single-message `OrderOfBattle` contract was READY in all
//! three backends while `service-generate` failed in the backend on a
//! declaration reached only through generated-support expansion. These tests
//! pin the corrected verdict and the Task 054 category-A regression set.

use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, GenerationWorld, analyze_service_readiness,
    project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::SchemaIr;
use ams_gra_oms_service_contract::{Contract, parse_yaml};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::PathBuf;
use std::process::Command;

const UCI_25_SHA256: &str = "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27";
const UCI_26_SHA256: &str = "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b";
const WORLD: GenerationWorld = GenerationWorld::ClosedSchemaSet;

const LANGUAGES: [(BackendLanguage, &str); 3] = [
    (BackendLanguage::Ada, "ada"),
    (BackendLanguage::Rust, "rust"),
    (BackendLanguage::Cpp, "cpp"),
];

/// Verify the pin, then load. `None` only when the variable is unset.
fn pinned_root(variable: &str, sha256: &str) -> Option<(PathBuf, SchemaIr)> {
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
    let schema = load_schema_set(&root).expect("pinned UCI root loads");
    Some((root, schema))
}

/// A single-OMS-message contract for `message` against UCI `version`.
fn contract_yaml(message: &str, version: &str) -> String {
    format!(
        "contract_version: \"0.1\"\nservice:\n  name: Task 056 {message}\n  version: \"0.1.0\"\n  \
         kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \
         \"{version}\"\nfunctions:\n  - id: mission-data\n    name: Mission Data\n    category: \
         specific\n    applicability: applicable\n    exchanges:\n      - id: report-input\n        \
         kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: \
         {message}\n        topic: mission.report\n        timing:\n          kind: asynchronous\n"
    )
}

fn contract(message: &str, version: &str) -> Contract {
    parse_yaml(&contract_yaml(message, version)).expect("test contract is portable-valid")
}

fn backend(language: BackendLanguage) -> Box<dyn Backend> {
    match language {
        BackendLanguage::Ada => Box::new(ams_gra_oms_backend_ada::AdaBackend),
        BackendLanguage::Rust => Box::new(ams_gra_oms_backend_rust::RustBackend),
        BackendLanguage::Cpp => Box::new(ams_gra_oms_backend_cpp::CppBackend),
    }
}

/// Measured expectations for one release (identical in all three backends).
struct Expected {
    release: &'static str,
    version: &'static str,
    variable: &'static str,
    sha256: &'static str,
    selected_types: usize,
    support_types: usize,
    unsupported_support: usize,
}

const RELEASES: [Expected; 2] = [
    Expected {
        release: "UCI 2.5",
        version: "2.5",
        variable: "AMS_GRA_UCI_2_5_ROOT",
        sha256: UCI_25_SHA256,
        selected_types: 55,
        support_types: 442,
        // Task 057: 42 -> 39. `EphemerisOrbitalModelType` (direct
        // `IntegratorStepSize`), `OrbitalEphemerisParametersReferenceType`
        // (direct `EphemerisResultsStepSize`) and the named `DurationType`
        // are renderable, so the historical Ada "type reference
        // Primitive(Duration)" first blocker is gone and the shared String
        // profile boundary is now first, exactly as in 2.6.
        //
        // Task 058: 39 -> 12. The 27 bounded-ASCII String declarations in
        // the support set (led by `AircraftIdentifierType`) are renderable;
        // the next blocker is `IMO_NumberType` (`IMO[0-9]{7}`, a literal
        // prefix, deliberately out of scope).
        // Task 059: nine deterministic ASCII shapes become renderable. Only
        // three alternation profiles remain in the 442-name support set.
        unsupported_support: 0,
    },
    Expected {
        release: "UCI 2.6",
        version: "2.6",
        variable: "AMS_GRA_UCI_2_6_ROOT",
        sha256: UCI_26_SHA256,
        selected_types: 56,
        support_types: 442,
        // Task 057: 40 -> 39; only the named `DurationType` became
        // renderable (2.6 has no direct xs:duration).
        //
        // Task 058: 39 -> 12, the same 27 bounded-ASCII declarations as 2.5;
        // Task 059: nine more support declarations are renderable; the
        // remaining three require union/alternation semantics.
        unsupported_support: 0,
    },
];

/// Current-state regression: Task 060 closes the final three support blockers.
/// Historical Task 056/058/059 measurements remain in their evidence documents.
#[test]
fn task056_real_uci_order_of_battle_support_parity() {
    let mut ran = false;
    for expected in &RELEASES {
        let Some((_root, schema)) = pinned_root(expected.variable, expected.sha256) else {
            continue;
        };
        ran = true;
        let contract = contract("OrderOfBattle", expected.version);
        let plan = resolve_service_plan(&contract, &schema).expect("plan resolves");
        let projection =
            project_service_generation_schema(&plan, &schema, WORLD).expect("projection succeeds");
        assert_eq!(
            projection.selected_type_names().len(),
            expected.selected_types
        );
        assert_eq!(
            projection.generated_support_type_names().len(),
            expected.support_types
        );
        for (language, label) in LANGUAGES {
            let readiness =
                analyze_service_readiness(&plan, &schema, language, WORLD).expect("readiness");
            let cell = format!("{} {label}", expected.release);
            // The contract-selected facts are exactly the pre-Task-056 ones.
            assert_eq!(readiness.selected_messages_total, 1, "{cell}");
            assert_eq!(readiness.selected_messages_renderable, 1, "{cell}");
            assert_eq!(readiness.selected_types_total, expected.selected_types);
            assert_eq!(readiness.selected_types_renderable, expected.selected_types);
            assert!(readiness.unsupported_types.is_empty(), "{cell}");
            assert!(readiness.blocked_messages.is_empty(), "{cell}");
            assert!(readiness.is_ready(), "{cell}");
            assert_eq!(
                readiness.generated_support_types_total,
                expected.support_types
            );
            assert_eq!(
                readiness.generated_support_types_renderable,
                expected.support_types
            );
            assert_eq!(
                readiness.unsupported_generated_support_types.len(),
                expected.unsupported_support
            );
            assert!(readiness.backend_blocker.is_none());
            assert!(readiness.service_api_blocker.is_none());
            let files = backend(language)
                .generate(projection.schema(), WORLD)
                .expect("Task 060 support renders");
            assert!(!files.is_empty());
            println!(
                "{cell} ORDEROFBATTLE: selected {} support {} unsupported support 0 READY",
                expected.selected_types, expected.support_types
            );
        }
        println!(
            "{} ORDEROFBATTLE GENERATED SUPPORT PARITY: PASSED",
            expected.release
        );
    }
    if !ran {
        eprintln!("SKIPPED: no pinned UCI root is set");
    }
}

/// The 35 Task 054 category-A messages (single-message contracts).
const CATEGORY_A: [&str; 35] = [
    "ActionCapabilityStatus",
    "ApprovalRequestStatus",
    "CapabilityCoverageAreaRequest",
    "CargoDeliveryCapabilityStatus",
    "CommTerminalCapabilityStatus",
    "ControlInterfacesCommand",
    "CounterSpaceCapabilityStatus",
    "CryptoStatus",
    "EarthOrientationParameters",
    "EffectCapabilityStatus",
    "EntityNotification",
    "EntityOrbitalElementSetRequest",
    "FlightCapabilityStatus",
    "GatewayCapabilityStatus",
    "NavigationCapabilityStatus",
    "OpNotification",
    "OpaqueCapabilityStatus",
    "OrbitChangeCapabilityStatus",
    "OrbitalSurveillanceCapabilityStatus",
    "OrbitalSurveillanceSensorCapabilityStatus",
    "OrderOfBattle",
    "PackageStatus",
    "ProductProcessingFunctionStatus",
    "ReferenceCapabilityStatus",
    "ResponseCapabilityStatus",
    "SelfDefenseStatus",
    "SpaceWeather",
    "StoreCarriageCapabilityStatus",
    "StoreManagementStatus",
    "StrikeCapabilityStatus",
    "SystemDeploymentCapabilityStatus",
    "SystemNotification",
    "SystemOrbitalElementSetRequest",
    "TacticalOrderCapabilityStatus",
    "TurretStatus",
];

/// Section 25: the Task 054 category-A set, through the library (no CLI
/// subprocess per message). All 35 messages now remain READY, including
/// `OrderOfBattle` after Task 060 closes its support blockers. Readiness
/// and backend generation agree for all 105 cells.
#[test]
fn task056_real_uci_category_a_readiness_matches_generation() {
    let Some((_, schema)) = pinned_root("AMS_GRA_UCI_2_5_ROOT", UCI_25_SHA256) else {
        return;
    };
    for (language, label) in LANGUAGES {
        let mut ready = Vec::new();
        let mut not_ready = Vec::new();
        for message in CATEGORY_A {
            let plan = resolve_service_plan(&contract(message, "2.5"), &schema).expect("plan");
            let readiness =
                analyze_service_readiness(&plan, &schema, language, WORLD).expect("readiness");
            let projection =
                project_service_generation_schema(&plan, &schema, WORLD).expect("projection");
            let generated = backend(language).generate(projection.schema(), WORLD);
            assert_eq!(
                readiness.is_ready(),
                generated.is_ok(),
                "{label} {message}: readiness/generation disagree: {:?}",
                generated.err()
            );
            if readiness.is_ready() {
                ready.push(message);
            } else {
                // Never a selected-model blocker: only generated support.
                assert!(readiness.unsupported_types.is_empty(), "{label} {message}");
                assert!(readiness.blocked_messages.is_empty(), "{label} {message}");
                assert!(!readiness.unsupported_generated_support_types.is_empty());
                not_ready.push(message);
            }
        }
        assert!(not_ready.is_empty(), "{label}: {not_ready:?}");
        assert_eq!(ready.len(), 35, "{label}");
        println!(
            "UCI 2.5 CATEGORY-A {label}: READY+generated {} NOT READY {:?}",
            ready.len(),
            not_ready
        );
    }
    println!("UCI 2.5 CATEGORY-A GENERATED SUPPORT PARITY: PASSED");
}
