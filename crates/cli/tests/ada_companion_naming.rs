//! Task 065: production naming evidence, including the immutable parent defect.
use ams_gra_oms_codegen_core::{
    BackendLanguage, CoverageAnalysis, GenerationWorld, TypeEmission, backend_preflight,
    plan_type_emissions, project_service_generation_schema, resolve_service_plan,
    unsafe_named_declarations,
};
use ams_gra_oms_ir::TypeRefTarget;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{collections::BTreeSet, path::PathBuf, process::Command};

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

#[test]
fn task065_synthetic_collision_repaired() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/companion-collision.xsd");
    let schema = load_schema_set(&path).unwrap();
    let emissions = plan_type_emissions(&schema, CLOSED).unwrap();
    assert!(emissions.iter().any(
        |e| matches!(e, TypeEmission::AbstractValue(p) if p.declaration.name.local_name == "Base")
    ));
    assert!(backend_preflight(&schema, BackendLanguage::Ada, CLOSED).is_ok());
    let unsafe_names = unsafe_named_declarations(&schema, BackendLanguage::Ada, CLOSED);
    assert!(unsafe_names.is_empty());
    let source = ams_gra_oms_backend_ada::generate(&schema, CLOSED).unwrap();
    assert!(source.contains("type Dispatch_Kind is"));
    assert!(source.contains("Dispatch_Choice_Value_Kind"));
    assert!(source.contains("when Dispatch_Choice_Value_Kind =>"));
    for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
        assert!(backend_preflight(&schema, language, CLOSED).is_ok());
    }
    println!("TASK065 SYNTHETIC COLLISION: REPAIRED");
}

#[test]
fn task065_semantic_name_matrix_and_gnat() {
    use ams_gra_oms_codegen_core::ada_closed_sum_literal_name;
    use ams_gra_oms_ir::{EnumVariant, TypeKind};
    let directory = std::env::temp_dir().join(format!("task065-matrix-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let template = include_str!("../../../tests/fixtures/service-generate/companion-collision.xsd");
    for owner in ["Dispatch", "Selection", "DISPATCH", "Dispatch_Choice_Value"] {
        let file = directory.join("input.xsd");
        std::fs::write(&file, template.replace("Dispatch", owner)).unwrap();
        let mut schema = load_schema_set(&file).unwrap();
        let descendant = schema
            .types
            .iter()
            .find(|d| d.name.local_name == owner)
            .unwrap()
            .clone();
        let chosen = ada_closed_sum_literal_name(&descendant).unwrap();
        assert_eq!(chosen, format!("{owner}_Choice_Value_Kind"));
        let mut record = descendant.clone();
        record.kind = TypeKind::Record { fields: Vec::new() };
        assert_eq!(
            ada_closed_sum_literal_name(&record).unwrap(),
            format!("{owner}_Kind")
        );
        let source = ams_gra_oms_backend_ada::generate(&schema, CLOSED).unwrap();
        assert!(source.contains(&format!("type {owner}_Kind is")));
        assert!(source.contains(&format!("type Base (Kind : Base_Kind := {chosen})")));
        assert!(source.contains(&format!("when {chosen} =>")));
        assert!(backend_preflight(&schema, BackendLanguage::Ada, CLOSED).is_ok());
        assert!(unsafe_named_declarations(&schema, BackendLanguage::Ada, CLOSED).is_empty());
        assert_eq!(
            source,
            ams_gra_oms_backend_ada::generate(&schema, CLOSED).unwrap()
        );
        schema.types.reverse();
        let reversed = ams_gra_oms_backend_ada::generate(&schema, CLOSED).unwrap();
        assert!(reversed.contains(&format!("when {chosen} =>")));
        let mut unrelated = record.clone();
        unrelated.name.local_name = "Unrelated".into();
        unrelated.base_type = None;
        schema.types.push(unrelated.clone());
        unrelated.name.local_name = "Other".into();
        schema.types.push(unrelated);
        for peer in ["FirstPeer", "SecondPeer"] {
            let mut choice = descendant.clone();
            choice.name.local_name = peer.into();
            choice.base_type = None;
            schema.types.push(choice);
        }
        let full = ams_gra_oms_backend_ada::generate(&schema, CLOSED).unwrap();
        assert!(full.contains("type Unrelated is record\n      null;"));
        assert!(full.contains("type Other is record\n      null;"));
        assert!(full.contains(&format!("when {chosen} =>")));
        assert!(full.contains("type FirstPeer_Kind is"));
        assert!(full.contains("type SecondPeer_Kind is"));
        schema.types.retain(|d| {
            !["Other", "Unrelated", "FirstPeer", "SecondPeer"].contains(&d.name.local_name.as_str())
        });
        assert!(
            ams_gra_oms_backend_ada::generate(&schema, CLOSED)
                .unwrap()
                .contains(&format!("when {chosen} =>"))
        );
        // A deliberately authored fixed escape collision must still fail closed,
        // including Ada case-insensitive equivalence. No random/numeric retries.
        let mut occupied = record.clone();
        occupied.base_type = None;
        occupied.name.local_name = chosen.to_ascii_lowercase();
        schema.types.push(occupied);
        assert!(backend_preflight(&schema, BackendLanguage::Ada, CLOSED).is_err());
        schema.types.pop();
        // The old-policy literal-vs-companion conflict under different case.
        let mut enumeration = record.clone();
        enumeration.base_type = None;
        enumeration.name.local_name = "Flags".into();
        enumeration.kind = TypeKind::Enumeration {
            variants: vec![EnumVariant {
                wire_value: format!("{}_Kind", owner.to_ascii_lowercase()),
                documentation: None,
            }],
        };
        schema.types.push(enumeration);
        assert!(backend_preflight(&schema, BackendLanguage::Ada, CLOSED).is_err());
        schema.types.pop();
        if Command::new("gnatmake").arg("--version").output().is_ok() {
            let probe = directory.join(owner.to_ascii_lowercase());
            std::fs::create_dir_all(&probe).unwrap();
            std::fs::write(probe.join("urn.ads"), "package Urn is end Urn;\n").unwrap();
            std::fs::write(probe.join("urn-companion.ads"), &source).unwrap();
            let result = Command::new("gnatmake")
                .current_dir(&probe)
                .args([
                    "-q",
                    "-gnat2022",
                    "-gnatwe",
                    "-gnato",
                    "-gnatc",
                    "urn-companion.ads",
                ])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        } else {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "GNAT required"
            );
        }
    }
    println!("TASK065 SEMANTIC MATRIX AND GNAT: PASSED");
}

fn xsd_files(directory: &std::path::Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() && path.file_name().unwrap() != ".git" {
            xsd_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "xsd") {
            files.push(path);
        }
    }
}

#[test]
fn task065_broad_rename_design_gate_inventory() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    xsd_files(&repository.join("crates"), &mut files);
    xsd_files(&repository.join("tests"), &mut files);
    xsd_files(&repository.join("examples"), &mut files);
    files.sort();
    let mut successful = 0;
    let mut affected = BTreeSet::new();
    let mut choice_artifacts = 0;
    let mut sum_artifacts = 0;
    for path in files {
        let Ok(schema) = load_schema_set(&path) else {
            continue;
        };
        for world in [CLOSED, GenerationWorld::OpenExtensions] {
            if ams_gra_oms_backend_ada::generate(&schema, world).is_err() {
                continue;
            }
            successful += 1;
            let emissions = plan_type_emissions(&schema, world).unwrap();
            let mut companions = Vec::new();
            for e in &emissions {
                match e {
                    TypeEmission::AbstractValue(p) => {
                        sum_artifacts += 1;
                        companions.push(format!("{}:sum", p.declaration.name.local_name));
                    }
                    TypeEmission::Declaration(d)
                        if matches!(d.kind, ams_gra_oms_ir::TypeKind::Choice { .. }) =>
                    {
                        choice_artifacts += 1;
                        companions.push(format!("{}:choice", d.name.local_name));
                    }
                    _ => {}
                }
            }
            if !companions.is_empty() {
                let relative = path
                    .strip_prefix(&repository)
                    .unwrap()
                    .display()
                    .to_string();
                affected.insert(relative.clone());
                println!("RENAME-FIXTURE\t{relative}\t{world:?}\t{companions:?}");
            }
        }
    }
    println!(
        "RENAME-SUMMARY\tsuccessful_fixture_world_cells={successful}\taffected_fixture_files={}\tchoice_artifact_cells={choice_artifacts}\tsum_artifact_cells={sum_artifacts}",
        affected.len()
    );
    println!("TASK065 BROAD RENAME DESIGN INVENTORY: PASSED");
}

fn contract(name: &str, version: &str) -> ams_gra_oms_service_contract::Contract {
    ams_gra_oms_service_contract::parse_yaml(&format!(
        "contract_version: \"0.1\"\nservice:\n  name: task065\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{version}\"\n  uci_schema_version: \"{version}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {name}\n        topic: t\n        timing:\n          kind: asynchronous\n"
    )).unwrap()
}

#[test]
fn task065_pinned_naming_evidence() {
    let roots = [
        (
            "2.5",
            "AMS_GRA_UCI_2_5_ROOT",
            "ac9430499e1107371345e04430895c8c9f18578c1a6b022958ca43ae8aa7bf27",
        ),
        (
            "2.6",
            "AMS_GRA_UCI_2_6_ROOT",
            "af54ce724c4fe869c8208c86985c0b768d74d581691e21d66c88bb6cfe59955b",
        ),
    ];
    if roots
        .iter()
        .any(|(_, variable, _)| std::env::var_os(variable).is_none())
    {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_TASK065_PINNED").is_none(),
            "both pinned roots required"
        );
        eprintln!("SKIPPED Task065: both pinned roots required");
        return;
    }
    for (release, variable, digest) in roots {
        let root = PathBuf::from(std::env::var_os(variable).unwrap());
        let hash = Command::new("sha256sum").arg(&root).output().unwrap();
        assert!(hash.status.success());
        assert_eq!(
            String::from_utf8_lossy(&hash.stdout)
                .split_whitespace()
                .next(),
            Some(digest)
        );
        let schema = load_schema_set(&root).unwrap();
        for name in ["QueryPET", "QueryType"] {
            let d = schema
                .types
                .iter()
                .find(|d| d.name.local_name == name)
                .unwrap();
            println!(
                "IDENTITY\t{release}\t{name}\tabstract={}\tbase={:?}\tkind={:?}\tsource={:?}",
                d.is_abstract, d.base_type, d.kind, d.source
            );
        }
        match plan_type_emissions(&schema, CLOSED) {
            Ok(emissions) => {
                for e in emissions {
                    if let TypeEmission::AbstractValue(p) = e {
                        println!(
                            "SUM\t{release}\t{}\t{:?}",
                            p.declaration.name.local_name,
                            p.concrete_descendants
                                .iter()
                                .map(|d| (&d.name.local_name, &d.kind))
                                .collect::<Vec<_>>()
                        );
                    }
                }
            }
            Err(e) => println!("EMISSION-PLAN\t{release}\t{e}"),
        }
        for world in [CLOSED, GenerationWorld::OpenExtensions] {
            let coverage = CoverageAnalysis::new(&schema, world).unwrap();
            let mut names = Vec::new();
            let mut unsafe_names = Vec::new();
            for language in BackendLanguage::ALL {
                let result = backend_preflight(&schema, language, world);
                assert!(
                    result.is_ok(),
                    "{release} {world:?} {language:?}: {result:?}"
                );
                println!("PREFLIGHT\t{release}\t{world:?}\t{language:?}\t{result:?}");
                if let Err(e) = result {
                    println!("DIAGNOSTIC\t{release}\t{world:?}\t{language:?}\t{e}");
                }
                let unsafe_set = unsafe_named_declarations(&schema, language, world);
                assert!(unsafe_set.is_empty());
                println!("UNSAFE\t{release}\t{world:?}\t{language:?}\t{unsafe_set:?}");
                let row = format!(
                    "COVERAGE\t{release}\t{world:?}\t{language:?}\t{:?}",
                    coverage.backend_coverage(language).unwrap()
                );
                assert!(
                    include_str!("../../../tests/fixtures/string/task065-current-coverage.tsv")
                        .lines()
                        .any(|expected| expected == row),
                    "{row}"
                );
                println!("{row}");
                names.push(coverage.renderable_message_closure_names(language).unwrap());
                unsafe_names.push(unsafe_set);
            }
            assert_eq!(names[1], names[2]);
            assert_eq!(names[0], names[1]);
            let ada_only: BTreeSet<_> = unsafe_names[0]
                .difference(&unsafe_names[1])
                .filter(|n| !unsafe_names[2].contains(*n))
                .collect();
            println!("ADA-ONLY-UNSAFE\t{release}\t{world:?}\t{ada_only:?}");
            let gap: Vec<_> = names[1].difference(&names[0]).collect();
            println!("GAP-COUNT\t{release}\t{world:?}\t{}", gap.len());
            let mut sizes = Vec::new();
            for name in gap {
                let m = schema.messages.iter().find(|m| m.name == *name).unwrap();
                let TypeRefTarget::Named(payload) = &m.payload_type.target else {
                    panic!("named payload")
                };
                let closure = coverage.dependency_closure(payload).unwrap();
                let attribution: BTreeSet<_> = closure
                    .iter()
                    .filter(|d| ada_only.contains(&d.name))
                    .map(|d| d.name.local_name.as_str())
                    .collect();
                assert!(!attribution.is_empty());
                println!(
                    "GAP\t{release}\t{world:?}\t{}\t{attribution:?}",
                    name.local_name
                );
                let plan =
                    resolve_service_plan(&contract(&name.local_name, release), &schema).unwrap();
                match project_service_generation_schema(&plan, &schema, world) {
                    Ok(p) => {
                        let count =
                            p.selected_type_names().len() + p.generated_support_type_names().len();
                        println!(
                            "GAP-PROJECTION\t{release}\t{world:?}\t{}\t{count}\t{:?}",
                            name.local_name,
                            backend_preflight(p.schema(), BackendLanguage::Ada, world)
                        );
                        sizes.push((count, name.local_name.clone()));
                    }
                    Err(e) => println!(
                        "GAP-PROJECTION-ERROR\t{release}\t{world:?}\t{}\t{e}",
                        name.local_name
                    ),
                }
            }
            sizes.sort();
            println!("SMALLEST\t{release}\t{world:?}\t{:?}", sizes.first());
        }
        // A former full-schema gap witness was already projected-name-safe.
        // Do not present this service as a newly READY projected service.
        let analysis = CoverageAnalysis::new(&schema, CLOSED).unwrap();
        let former: Vec<_> =
            include_str!("../../../tests/fixtures/string/task062-ada-full-schema-gap.tsv")
                .lines()
                .filter_map(|line| line.strip_prefix(&format!("{release}\t")))
                .collect();
        let mut sizes = Vec::new();
        for message in former {
            let m = schema
                .messages
                .iter()
                .find(|m| m.name.local_name == message)
                .unwrap();
            let TypeRefTarget::Named(payload) = &m.payload_type.target else {
                panic!("named payload")
            };
            assert!(
                analysis
                    .dependency_closure(payload)
                    .unwrap()
                    .iter()
                    .any(|d| d.name.local_name == "QueryPET")
            );
            let plan = resolve_service_plan(&contract(message, release), &schema).unwrap();
            let p = project_service_generation_schema(&plan, &schema, CLOSED).unwrap();
            sizes.push((
                p.selected_type_names().len() + p.generated_support_type_names().len(),
                message,
            ));
        }
        sizes.sort();
        let (count, message) = sizes[0];
        println!("FORMER-GAP-SMALLEST\t{release}\t{message}\t{count}");
        let plan = resolve_service_plan(&contract(message, release), &schema).unwrap();
        let p = project_service_generation_schema(&plan, &schema, CLOSED).unwrap();
        assert!(backend_preflight(&schema, BackendLanguage::Ada, CLOSED).is_ok());
        for language in BackendLanguage::ALL {
            let readiness = ams_gra_oms_codegen_core::analyze_service_readiness(
                &plan, &schema, language, CLOSED,
            )
            .unwrap();
            assert!(
                readiness.is_ready(),
                "{release} {language:?}: {readiness:?}"
            );
            assert!(backend_preflight(p.schema(), language, CLOSED).is_ok());
        }
        let out =
            std::env::temp_dir().join(format!("task065-vertical-{release}-{}", std::process::id()));
        let args: Vec<std::ffi::OsString> = vec![
            "service-generate".into(),
            "--schema".into(),
            root.as_os_str().to_owned(),
            "--contract".into(),
            out.with_extension("yaml").into_os_string(),
            "--language".into(),
            "ada".into(),
            "--world".into(),
            "closed-schema".into(),
            "--output".into(),
            out.as_os_str().to_owned(),
        ];
        // Serialize the same production contract without adding a dependency.
        let yaml = format!(
            "contract_version: \"0.1\"\nservice:\n  name: task065\n  version: \"0.1.0\"\n  kind: service\nstandards:\n  oms_version: \"{release}\"\n  uci_schema_version: \"{release}\"\nfunctions:\n  - id: f\n    name: F\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e\n        kind: oms_message\n        direction: output\n        mandate: mandatory\n        message: {message}\n        topic: t\n        timing:\n          kind: asynchronous\n"
        );
        std::fs::write(out.with_extension("yaml"), yaml).unwrap();
        for language in ["ada", "rust", "cpp"] {
            let check_args: Vec<std::ffi::OsString> = vec![
                "service-check".into(),
                "--schema".into(),
                root.as_os_str().to_owned(),
                "--contract".into(),
                out.with_extension("yaml").into_os_string(),
                "--language".into(),
                language.into(),
                "--world".into(),
                "closed-schema".into(),
            ];
            let mut report = Vec::new();
            ams_gra_codegen_oms::run(check_args, &mut report).unwrap();
            assert!(
                String::from_utf8_lossy(&report)
                    .lines()
                    .any(|l| l == "status: READY")
            );
            println!("SERVICE-CHECK\t{release}\t{language}\tREADY");
        }
        let mut stdout = Vec::new();
        ams_gra_codegen_oms::run(args, &mut stdout).unwrap();
        println!("{}", String::from_utf8_lossy(&stdout));
        if Command::new("gnatmake").arg("--version").output().is_ok() {
            let result = Command::new("gnatmake")
                .current_dir(&out)
                .args([
                    "-c",
                    "-q",
                    "-gnat2022",
                    "-gnatwe",
                    "-gnato",
                    "service_api.ads",
                ])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            for body in std::fs::read_dir(&out)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|e| e == "adb"))
            {
                let result = Command::new("gnatmake")
                    .current_dir(&out)
                    .args(["-c", "-q", "-gnat2022", "-gnatwe", "-gnato"])
                    .arg(body.file_name().unwrap())
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "{}{}",
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        } else {
            assert!(
                std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
                "GNAT required"
            );
        }
        println!("UCI {release} TASK065 AUTHORIZATION MODEL API GNAT: PASSED");
        println!("UCI {release} TASK065 NAMING EVIDENCE: PASSED");
    }
}
