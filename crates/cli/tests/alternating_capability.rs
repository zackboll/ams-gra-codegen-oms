//! Task 060 shared classification/coverage/backend parity, including neighbors.
use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, CoverageAnalysis, GenerationWorld, StringProfile,
    analyze_service_readiness, resolve_service_plan, string_profile,
};
use ams_gra_oms_ir::PrimitiveKind;
use ams_gra_oms_xsd_frontend::load_schema_document;

#[test]
fn admitted_union_is_real_backend_capability_and_neighbors_fail_closed() {
    let path = std::env::temp_dir().join(format!("task060-capability-{}.xsd", std::process::id()));
    for (pattern, min, supported) in [
        ("[A-Z0-9]{5}|UNKN|NONE", 4, true),
        ("[A-Z0-9]{5}|UNKN", 4, false),
        ("[A-Z0-9]{5}|UNKN|NONE|XXXX", 4, false),
        ("[A-Z0-9]{5}|UNKN|NOPE", 4, false),
        ("[A-Z0-9]{5}|UNKN|NONE", 3, false),
        ("[A-Z0-9]{5}|UNKN|NONE", 4, false),
    ] {
        // The final duplicate adds a derived restriction group: same text is
        // not evidence for a different normalized restriction-level shape.
        let derived = if pattern == "[A-Z0-9]{5}|UNKN|NONE" && min == 4 && !supported {
            "<xs:simpleType name=\"Derived\"><xs:restriction base=\"t:Renamed\"><xs:pattern value=\"NONE\"/></xs:restriction></xs:simpleType>"
        } else {
            ""
        };
        std::fs::write(&path,format!("<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" xmlns:t=\"urn:task060:capability\" targetNamespace=\"urn:task060:capability\"><xs:simpleType name=\"Renamed\"><xs:restriction base=\"xs:string\"><xs:minLength value=\"{min}\"/><xs:maxLength value=\"5\"/><xs:pattern value=\"{pattern}\"/></xs:restriction></xs:simpleType>{derived}</xs:schema>")).unwrap();
        let schema = load_schema_document(&path).unwrap();
        let declaration = schema.types.last().unwrap();
        let verdict = string_profile(PrimitiveKind::String, &declaration.constraints);
        assert_eq!(
            matches!(verdict, Ok(Some(StringProfile::AlternatingAscii(_)))),
            supported
        );
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            let coverage = analysis.backend_coverage(language).unwrap();
            assert_eq!(
                coverage.declarations_fully_renderable == schema.types.len(),
                supported
            );
            let backend: Box<dyn Backend> = match language {
                BackendLanguage::Ada => Box::new(ams_gra_oms_backend_ada::AdaBackend),
                BackendLanguage::Rust => Box::new(ams_gra_oms_backend_rust::RustBackend),
                BackendLanguage::Cpp => Box::new(ams_gra_oms_backend_cpp::CppBackend),
            };
            assert_eq!(
                backend
                    .generate(&schema, GenerationWorld::ClosedSchemaSet)
                    .is_ok(),
                supported
            );
        }
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn selected_and_generated_support_readiness_share_exact_boundary() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate/codec-alternating-ascii.xsd");
    let schema = load_schema_document(&path).unwrap();
    let contract = ams_gra_oms_service_contract::parse_yaml(
        &std::fs::read_to_string(path.with_extension("yaml")).unwrap(),
    )
    .unwrap();
    let plan = resolve_service_plan(&contract, &schema).unwrap();
    for language in BackendLanguage::ALL {
        let ready =
            analyze_service_readiness(&plan, &schema, language, GenerationWorld::ClosedSchemaSet)
                .unwrap();
        assert!(ready.is_ready(), "{language:?}: {ready:?}");
        assert!(ready.unsupported_types.is_empty());
        assert!(ready.unsupported_generated_support_types.is_empty());
    }
    for mutation in 0..3 {
        let mut changed = schema.clone();
        let declaration = changed
            .types
            .iter_mut()
            .find(|d| d.name.local_name == "NotationType")
            .unwrap();
        match mutation {
            0 => declaration.constraints.lexical.pattern_groups[0].alternatives[0]
                .expression
                .push_str("|XXXX"),
            1 => declaration.constraints.min_length = Some(3),
            _ => declaration
                .constraints
                .lexical
                .pattern_groups
                .push(declaration.constraints.lexical.pattern_groups[0].clone()),
        }
        let plan = resolve_service_plan(&contract, &changed).unwrap();
        for language in BackendLanguage::ALL {
            let readiness = analyze_service_readiness(
                &plan,
                &changed,
                language,
                GenerationWorld::ClosedSchemaSet,
            )
            .unwrap();
            assert!(!readiness.is_ready());
            assert!(
                readiness
                    .unsupported_types
                    .iter()
                    .any(|name| name.local_name == "NotationType")
            );
        }
    }
}

#[test]
fn generated_support_string_controls_final_readiness_not_selected_counts() {
    let path = std::env::temp_dir().join(format!("task060-support-{}.xsd", std::process::id()));
    let yaml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/service-generate/codec-alternating-ascii.yaml"),
    )
    .unwrap();
    let contract = ams_gra_oms_service_contract::parse_yaml(&yaml).unwrap();
    for (pattern, supported) in [
        ("[A-Z0-9]{5}|UNKN|NONE", true),
        ("[A-Z0-9]{5}|UNKN|NOPE", false),
    ] {
        std::fs::write(&path,format!(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="https://www.vdl.afrl.af.mil/programs/oam" targetNamespace="https://www.vdl.afrl.af.mil/programs/oam" elementFormDefault="qualified">
<xs:simpleType name="SupportString"><xs:restriction base="xs:string"><xs:minLength value="4"/><xs:maxLength value="5"/><xs:pattern value="{pattern}"/></xs:restriction></xs:simpleType>
<xs:complexType name="Base" abstract="true"><xs:sequence><xs:element name="Flag" type="xs:boolean"/></xs:sequence></xs:complexType>
<xs:complexType name="Concrete"><xs:complexContent><xs:extension base="t:Base"><xs:sequence><xs:element name="Text" type="t:SupportString"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType>
<xs:complexType name="Payload"><xs:sequence><xs:element name="Item" type="t:Base"/></xs:sequence></xs:complexType>
<xs:element name="AlternatingNotice" type="t:Payload" t:version="001.000.000.000"/>
</xs:schema>"#)).unwrap();
        let schema = load_schema_document(&path).unwrap();
        let plan = resolve_service_plan(&contract, &schema).unwrap();
        let projection = ams_gra_oms_codegen_core::project_service_generation_schema(
            &plan,
            &schema,
            GenerationWorld::ClosedSchemaSet,
        )
        .unwrap();
        assert!(
            projection
                .generated_support_type_names()
                .iter()
                .any(|name| name.local_name == "SupportString")
        );
        assert!(
            !projection
                .selected_type_names()
                .iter()
                .any(|name| name.local_name == "SupportString")
        );
        for language in BackendLanguage::ALL {
            let ready = analyze_service_readiness(
                &plan,
                &schema,
                language,
                GenerationWorld::ClosedSchemaSet,
            )
            .unwrap();
            assert_eq!(ready.selected_types_renderable, ready.selected_types_total);
            assert_eq!(ready.is_ready(), supported, "{language:?} {ready:?}");
            assert_eq!(
                ready.unsupported_generated_support_types.is_empty(),
                supported
            );
            if !supported {
                assert_eq!(
                    ready.unsupported_generated_support_types[0].local_name,
                    "SupportString"
                );
            }
            let backend: Box<dyn Backend> = match language {
                BackendLanguage::Ada => Box::new(ams_gra_oms_backend_ada::AdaBackend),
                BackendLanguage::Rust => Box::new(ams_gra_oms_backend_rust::RustBackend),
                BackendLanguage::Cpp => Box::new(ams_gra_oms_backend_cpp::CppBackend),
            };
            assert_eq!(
                backend
                    .generate(projection.schema(), GenerationWorld::ClosedSchemaSet)
                    .is_ok(),
                supported
            );
        }
    }
    std::fs::remove_file(path).unwrap();
}
