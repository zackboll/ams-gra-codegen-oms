use super::*;
#[path = "../../../tests/task060_cases.rs"]
mod corpus;
use std::process::Command;

#[test]
fn standalone_task060_compiler_probe() {
    let dir = std::env::temp_dir().join(format!("task060-rust-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut source = String::new();
    let mut probe = String::from("fn main() {\n");
    for (name, profile) in corpus::profiles() {
        let model = render_rust_alternating_ascii(&name, &profile);
        println!(
            "TASK060 Rust {name}: {} bytes {} lines",
            model.len(),
            model.lines().count()
        );
        source.push_str(&model);
        for (text, valid) in corpus::cases(&profile) {
            writeln!(
                probe,
                "assert_eq!({name}::new({text:?}).is_some(), {valid}, {text:?});"
            )
            .unwrap();
            if valid {
                writeln!(
                    probe,
                    "assert_eq!({name}::new({text:?}).unwrap().as_str(), {text:?});"
                )
                .unwrap();
            }
        }
    }
    probe.push_str("}\n");
    source.push_str(&probe);
    std::fs::write(dir.join("probe.rs"), &source).unwrap();
    let build = Command::new("rustc")
        .current_dir(&dir)
        .args(["--edition=2024", "-Dwarnings", "probe.rs", "-o", "probe"])
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(Command::new(dir.join("probe")).status().unwrap().success());
    // Flip one expectation, recompile, and require the probe to catch it.
    let wrong = source.replacen(".is_some(), false", ".is_some(), true", 1);
    assert_ne!(wrong, source);
    std::fs::write(dir.join("probe.rs"), wrong).unwrap();
    assert!(
        Command::new("rustc")
            .current_dir(&dir)
            .args(["--edition=2024", "-Dwarnings", "probe.rs", "-o", "probe"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        !Command::new(dir.join("probe"))
            .output()
            .unwrap()
            .status
            .success()
    );
    std::fs::write(dir.join("probe.rs"), source).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn factored_task060_source_sizes() {
    for (name, profile) in corpus::profiles() {
        let source = render_rust_alternating_ascii(&name, &profile);
        println!(
            "TASK060 rust {name}: {} bytes {} lines",
            source.len(),
            source.lines().count()
        );
        if profile.groups[0].len() == 625 {
            assert!(source.len() < 25_000, "IPv4 remained unfactored");
        }
    }
}

#[test]
fn recursive_equality_analysis_terminates_and_declines_both_traits() {
    let dir = std::env::temp_dir().join(format!("task060-cycle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("schema.xsd"),r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:cycle" targetNamespace="urn:cycle"><xs:complexType name="Node"><xs:sequence><xs:element name="Children" type="t:Node" minOccurs="0" maxOccurs="unbounded"/></xs:sequence></xs:complexType></xs:schema>"#).unwrap();
    let schema = ams_gra_oms_xsd_frontend::load_schema_document(&dir.join("schema.xsd")).unwrap();
    let declaration = &schema.types[0];
    assert!(!declaration_supports_partial_eq(
        &schema,
        declaration,
        &mut Vec::new(),
        GenerationWorld::ClosedSchemaSet
    ));
    assert!(!declaration_supports_eq(
        &schema,
        declaration,
        &mut Vec::new(),
        GenerationWorld::ClosedSchemaSet
    ));
    // Normal generation may reject recursive by-value topology before rendering.
    // Exercise the production derive decision over a Rust Vec storage cycle.
    let mut source = String::new();
    source.push_str(&format!(
        "#[derive(Debug, Clone{})]\npub struct Node {{ pub children: Vec<Node> }}",
        structural_derives(&schema, declaration, GenerationWorld::ClosedSchemaSet)
    ));
    std::fs::write(dir.join("model.rs"), source).unwrap();
    let result = Command::new("rustc")
        .current_dir(&dir)
        .args([
            "-Dwarnings",
            "--crate-type=lib",
            "model.rs",
            "-o",
            "model.rlib",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn generated_group_and_carrier_composes_with_production_service_codec() {
    // Keep test dependencies linked so the generated-code compiler can use
    // their Cargo-built artifacts without a public renderer/admission seam.
    let _ = serde_json::json!(null);
    let _ = std::mem::size_of::<ams_gra_oms_runtime_rust::CodecError>();
    use ams_gra_oms_codegen_core::{
        AlternatingAsciiProfile, StructuredAsciiFacets, build_service_api_model,
        resolve_service_plan,
    };
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/codec-alternating-ascii.xsd");
    let schema = ams_gra_oms_xsd_frontend::load_schema_document(&fixture).unwrap();
    let contract = ams_gra_oms_service_contract::parse_yaml(
        &std::fs::read_to_string(fixture.with_extension("yaml")).unwrap(),
    )
    .unwrap();
    let plan = resolve_service_plan(&contract, &schema).unwrap();
    let api = build_service_api_model(&plan, &schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let codec = generate_service_codec(&api, &schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let model = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
    let old = render_rust_alternating_ascii(
        "NotationType",
        ams_gra_oms_codegen_core::alternating_ascii_rows()
            .iter()
            .find(|row| row.patterns[0] == "[A-Z0-9]{5}|UNKN|NONE")
            .map(|row| &row.profile)
            .unwrap(),
    );
    let branch = |text| vec![StructuredAsciiSegment::Literal(text)];
    let synthetic = AlternatingAsciiProfile {
        facets: StructuredAsciiFacets::Exact(2),
        groups: vec![
            vec![branch("AB"), branch("AC")],
            vec![branch("AB"), branch("AD")],
        ],
    };
    assert_eq!(model.matches(&old).count(), 1);
    let assembled = model.replacen(
        &old,
        &render_rust_alternating_ascii("NotationType", &synthetic),
        1,
    );
    let dir = std::env::temp_dir().join(format!("task060-and-codec-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("model.rs"), assembled).unwrap();
    std::fs::write(dir.join("service_codec.rs"), codec).unwrap();
    std::fs::write(dir.join("probe.rs"),r#"
pub mod model;
pub mod service_codec;
use ams_gra_oms_runtime_rust::{CodecError, OmsJsonCodec};
use serde_json::{json, Value};
fn decode(value: &Value) -> Result<model::AlternatingPayload, CodecError> { service_codec::ServiceCodec.decode_payload(value) }
fn main() {
    let document=json!({"Notation":"AB","Origin":"E","Address":"1.2.3.4"});
    let accepted=decode(&document).unwrap();
    assert_eq!(service_codec::ServiceCodec.encode_payload(&accepted).unwrap(),document);
    for bad in [json!("AC"),json!("AD"),json!("AE"),json!(42),json!(true),json!(null),json!([]),json!({})] {
        let mut changed=document.clone();changed["Notation"]=bad;assert!(decode(&changed).is_err());
    }
}
"#).unwrap();
    // Let Cargo resolve a coherent dependency graph rather than picking an
    // arbitrary rlib from a target directory containing multiple feature builds.
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../runtime-rust");
    std::fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "task060-and-generated"
version = "0.1.0"
edition = "2021"
[workspace]
[[bin]]
name = "probe"
path = "probe.rs"
[dependencies]
ams-gra-oms-runtime-rust = {{ path = {:?} }}
serde_json = "1"
"#,
            runtime
        ),
    )
    .unwrap();
    let built = Command::new("cargo")
        .current_dir(&dir)
        .args(["run", "--offline", "--quiet"])
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .env("RUSTFLAGS", "-Dwarnings")
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    std::fs::remove_dir_all(dir).unwrap();
}
