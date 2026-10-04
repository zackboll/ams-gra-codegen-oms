use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, CoverageAnalysis, GenerationWorld, StringProfile,
    analyze_service_codec, analyze_service_readiness, build_service_api_model,
    ipv6_address_constraints, resolve_service_plan, string_profile,
};
use ams_gra_oms_ir::PrimitiveKind;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::Path;

fn backend(language: BackendLanguage) -> Box<dyn Backend> {
    match language {
        BackendLanguage::Ada => Box::new(ams_gra_oms_backend_ada::AdaBackend),
        BackendLanguage::Rust => Box::new(ams_gra_oms_backend_rust::RustBackend),
        BackendLanguage::Cpp => Box::new(ams_gra_oms_backend_cpp::CppBackend),
    }
}

#[test]
fn task062_name_free_exact_profile_and_neighbors_share_all_capability_gates() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = repo.join("tests/fixtures/service-generate/codec-ipv6.xsd");
    let contract = ams_gra_oms_service_contract::parse_yaml(
        &std::fs::read_to_string(repo.join("tests/fixtures/service-generate/codec-ipv6.yaml"))
            .unwrap(),
    )
    .unwrap();
    let schema = load_schema_document(&fixture).unwrap();
    let mut neighbors = vec![ipv6_address_constraints()];
    let mut c = neighbors[0].clone();
    c.min_length = Some(3);
    neighbors.push(c);
    let mut c = neighbors[0].clone();
    c.max_length = Some(44);
    neighbors.push(c);
    let mut c = neighbors[0].clone();
    c.lexical.pattern_groups[0].alternatives[0]
        .expression
        .push_str("|.*");
    neighbors.push(c);
    for (index, constraints) in neighbors.into_iter().enumerate() {
        let mut changed = schema.clone();
        let declaration = changed
            .types
            .iter_mut()
            .find(|d| d.name.local_name == "AddressType")
            .unwrap();
        declaration.constraints = constraints;
        assert_eq!(
            string_profile(PrimitiveKind::String, &declaration.constraints)
                .ok()
                .flatten(),
            if index == 0 {
                Some(StringProfile::Ipv6Address)
            } else {
                None
            }
        );
        let plan = resolve_service_plan(&contract, &changed).unwrap();
        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let coverage = CoverageAnalysis::new(&changed, world).unwrap();
            for language in BackendLanguage::ALL {
                assert_eq!(
                    analyze_service_readiness(&plan, &changed, language, world)
                        .unwrap()
                        .is_ready(),
                    index == 0
                );
                assert_eq!(
                    backend(language).generate(&changed, world).is_ok(),
                    index == 0
                );
                assert_eq!(
                    coverage
                        .backend_coverage(language)
                        .unwrap()
                        .message_closures_renderable,
                    usize::from(index == 0)
                );
            }
        }
        assert_eq!(
            analyze_service_codec(
                &build_service_api_model(&plan, &changed, GenerationWorld::ClosedSchemaSet)
                    .unwrap(),
                &changed,
                BackendLanguage::Rust,
                GenerationWorld::ClosedSchemaSet
            )
            .unwrap()
            .is_ready(),
            index == 0
        );
    }
    // The fixture deliberately names the admitted declaration AddressType,
    // not IPv6_AddressType; renaming cannot alter effective constraints.
    println!("TASK062 SHARED IPV6 CAPABILITY: PASSED");
}

#[test]
fn task062_generated_support_only_ipv6_controls_final_readiness() {
    use ams_gra_oms_codegen_core::{UCI_IPV6_PATTERN, project_service_generation_schema};
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let yaml =
        std::fs::read_to_string(repo.join("tests/fixtures/service-generate/codec-ipv6.yaml"))
            .unwrap();
    let contract = ams_gra_oms_service_contract::parse_yaml(&yaml).unwrap();
    let path = std::env::temp_dir().join(format!("task062-support-{}.xsd", std::process::id()));
    for (pattern, supported) in [
        (UCI_IPV6_PATTERN.to_owned(), true),
        (format!("{UCI_IPV6_PATTERN}|.*"), false),
    ] {
        std::fs::write(&path, format!(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="https://www.vdl.afrl.af.mil/programs/oam" targetNamespace="https://www.vdl.afrl.af.mil/programs/oam" elementFormDefault="qualified">
<xs:simpleType name="IPv6_AddressType"><xs:restriction base="xs:string"><xs:minLength value="2"/><xs:maxLength value="45"/><xs:pattern value="{pattern}"/></xs:restriction></xs:simpleType>
<xs:complexType name="Base" abstract="true"><xs:sequence><xs:element name="Flag" type="xs:boolean"/></xs:sequence></xs:complexType>
<xs:complexType name="Concrete"><xs:complexContent><xs:extension base="t:Base"><xs:sequence><xs:element name="Address" type="t:IPv6_AddressType"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType>
<xs:complexType name="Payload"><xs:sequence><xs:element name="Item" type="t:Base"/></xs:sequence></xs:complexType>
<xs:element name="Ipv6Notice" type="t:Payload" t:version="001.000.000.000"/>
</xs:schema>"#)).unwrap();
        let schema = load_schema_document(&path).unwrap();
        let plan = resolve_service_plan(&contract, &schema).unwrap();
        let world = GenerationWorld::ClosedSchemaSet;
        let projection = project_service_generation_schema(&plan, &schema, world).unwrap();
        assert!(
            projection
                .generated_support_type_names()
                .iter()
                .any(|n| n.local_name == "IPv6_AddressType")
        );
        assert!(
            !projection
                .selected_type_names()
                .iter()
                .any(|n| n.local_name == "IPv6_AddressType")
        );
        for language in BackendLanguage::ALL {
            let r = analyze_service_readiness(&plan, &schema, language, world).unwrap();
            assert_eq!(r.selected_types_renderable, r.selected_types_total);
            assert_eq!(r.is_ready(), supported);
            assert_eq!(r.unsupported_generated_support_types.is_empty(), supported);
            assert_eq!(
                backend(language)
                    .generate(projection.schema(), world)
                    .is_ok(),
                supported
            );
        }
    }
    std::fs::remove_file(path).unwrap();
    println!("TASK062 SUPPORT-ONLY IPV6 FAIL-CLOSED: PASSED");
}

#[test]
fn task062_production_source_size_and_private_helper_names() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut schema =
        load_schema_document(&repo.join("tests/fixtures/service-generate/codec-ipv6.xsd")).unwrap();
    schema.types.retain(|d| d.name.local_name == "AddressType");
    schema.messages.clear();
    for language in BackendLanguage::ALL {
        let files = backend(language)
            .generate(&schema, GenerationWorld::ClosedSchemaSet)
            .unwrap();
        let bytes: usize = files.iter().map(|f| f.contents.len()).sum();
        let lines: usize = files.iter().map(|f| f.contents.lines().count()).sum();
        println!("TASK062 PRODUCTION {language:?} SINGLE CARRIER: {bytes} bytes {lines} lines");
        assert!(bytes < 16_000 && lines < 350, "pathological source");
    }
    // Names coinciding with private scanner helpers must remain available for
    // ordinary schema declarations. No generated global helper is reserved.
    let address = schema.types[0].clone();
    for name in [
        "Hex_Byte",
        "Hex_Group",
        "Hex_Colon",
        "IPv4_Octet",
        "Embedded_IPv4",
        "Suffix",
        "After_Prefix",
        "Valid",
        "Ipv6AddressModel",
    ] {
        let mut declaration = address.clone();
        declaration.name.local_name = name.to_owned();
        declaration.kind = ams_gra_oms_ir::TypeKind::Primitive(PrimitiveKind::Boolean);
        declaration.base_type = None;
        declaration.constraints = ams_gra_oms_ir::ConstraintSet::default();
        schema.types.push(declaration);
    }
    for language in BackendLanguage::ALL {
        assert!(
            backend(language)
                .generate(&schema, GenerationWorld::ClosedSchemaSet)
                .is_ok()
        );
    }
    println!("TASK062 PRIVATE IPV6 HELPERS NAME PREFLIGHT: PASSED");
}
