use ams_gra_oms_codegen_core::*;
use ams_gra_oms_ir::*;
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::Path;
fn plan(s: &SchemaIr, m: &str) -> ServicePlan {
    let c=parse_yaml(&format!("contract_version: \"0.1\"\nservice:\n  name: Audit\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n  - id: f1\n    name: Audit\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {m}\n        topic: audit\n        timing:\n          kind: asynchronous\n")).unwrap();
    resolve_service_plan(&c, s).unwrap()
}
fn main() {
    let dir = Path::new("/tmp/task068/fixtures/synthetic");
    std::fs::create_dir_all(dir).unwrap();
    let public = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam" xmlns:t="urn:audit" targetNamespace="urn:audit" elementFormDefault="qualified"><xs:complexType name="Base" abstract="true"/><xs:complexType name="PublicA"><xs:complexContent><xs:extension base="t:Base"><xs:sequence><xs:element name="PublicFlag" type="xs:boolean"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType><xs:complexType name="Holder"><xs:sequence><xs:element name="Value" type="t:Base"/></xs:sequence></xs:complexType><xs:element uci:version="1.0" name="HolderReport" type="t:Holder"/></xs:schema>"#;
    let private = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:uci="https://www.vdl.afrl.af.mil/programs/oam" xmlns:t="urn:audit" targetNamespace="urn:audit" elementFormDefault="qualified"><xs:complexType name="PrivateB"><xs:complexContent><xs:extension base="t:Base"><xs:sequence><xs:element name="Flag" type="xs:boolean"/><xs:element name="Enabled" type="xs:boolean"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType></xs:schema>"#;
    std::fs::write(dir.join("public.xsd"), public).unwrap();
    std::fs::write(dir.join("private.xsd"), private).unwrap();
    std::fs::write(
        dir.join("generic.xsd"),
        public.replace("PublicA", "ConcreteA"),
    )
    .unwrap();
    let generic = load_schema_set(&dir.join("generic.xsd")).unwrap();
    let generic_plan = plan(&generic, "HolderReport");
    let closed = project_service_generation_schema(
        &generic_plan,
        &generic,
        GenerationWorld::ClosedSchemaSet,
    )
    .unwrap();
    assert_eq!(
        closed.generated_support_type_names()[0].local_name,
        "ConcreteA"
    );
    assert!(matches!(
        project_service_generation_schema(&generic_plan, &generic, GenerationWorld::OpenExtensions),
        Err(ServiceGenerationError::AbstractValue(
            AbstractValueProjectionError::NotClosedUnderOpenExtensions(target)
        )) if target == QualifiedName::new("urn:audit", "Base")
    ));
    for language in [
        BackendLanguage::Ada,
        BackendLanguage::Rust,
        BackendLanguage::Cpp,
    ] {
        assert!(
            analyze_service_readiness(
                &generic_plan,
                &generic,
                language,
                GenerationWorld::ClosedSchemaSet
            )
            .unwrap()
            .is_ready()
        );
        assert!(
            !analyze_service_readiness(
                &generic_plan,
                &generic,
                language,
                GenerationWorld::OpenExtensions
            )
            .unwrap()
            .is_ready()
        );
    }
    ams_gra_oms_backend_ada::generate(closed.schema(), GenerationWorld::ClosedSchemaSet).unwrap();
    ams_gra_oms_backend_rust::generate(closed.schema(), GenerationWorld::ClosedSchemaSet).unwrap();
    ams_gra_oms_backend_cpp::generate(closed.schema(), GenerationWorld::ClosedSchemaSet).unwrap();
    println!(
        "GENERIC ConcreteA: Closed support/model/readiness succeeds; Open exact Base error in all backends"
    );
    for overlay in [false, true] {
        let s = ams_gra_oms_xsd_frontend::load_schema_set_with_overlays(
            &dir.join("public.xsd"),
            &if overlay {
                vec![dir.join("private.xsd")]
            } else {
                vec![]
            },
        )
        .unwrap();
        let p = plan(&s, "HolderReport");
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let projection = project_service_generation_schema(&p, &s, world);
            println!(
                "overlay={overlay} world={world:?} projection={:?}",
                projection
                    .as_ref()
                    .map(|p| p.generated_support_type_names())
            );
            for l in [
                BackendLanguage::Ada,
                BackendLanguage::Rust,
                BackendLanguage::Cpp,
            ] {
                let r = analyze_service_readiness(&p, &s, l, world).unwrap();
                println!(
                    "{l:?} ready={} blocker={:?}",
                    r.is_ready(),
                    r.backend_blocker
                );
            }
            if let Ok(projection) = projection {
                let s = projection.schema();
                let d = dir.join(if overlay { "private" } else { "public" });
                std::fs::create_dir_all(&d).unwrap();
                std::fs::write(
                    d.join("model.rs"),
                    ams_gra_oms_backend_rust::generate(s, world).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    d.join("model.hpp"),
                    ams_gra_oms_backend_cpp::generate(s, world).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    d.join("urn-audit.ads"),
                    ams_gra_oms_backend_ada::generate(s, world).unwrap(),
                )
                .unwrap();
            }
        }
    }
    let zero=public.replace("<xs:complexType name=\"PublicA\"><xs:complexContent><xs:extension base=\"t:Base\"><xs:sequence><xs:element name=\"PublicFlag\" type=\"xs:boolean\"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType>","").replace("Base","ExtensionBase").replace("name=\"Value\" type=\"t:ExtensionBase\"","name=\"Extensions\" type=\"t:ExtensionBase\" minOccurs=\"0\" maxOccurs=\"unbounded\"");
    std::fs::write(dir.join("zero.xsd"), zero).unwrap();
    let s = load_schema_set(&dir.join("zero.xsd")).unwrap();
    let p = plan(&s, "HolderReport");
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        println!(
            "ZERO {world:?} {:?}",
            project_service_generation_schema(&p, &s, world)
        );
    }
}
