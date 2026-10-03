//! Shared classification -> coverage -> readiness -> generation boundary.
use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, CoverageAnalysis, GenerationWorld, TemporalProfile,
    analyze_service_readiness, project_service_generation_schema, resolve_service_plan,
    temporal_profile,
};
use ams_gra_oms_ir::{
    ConstraintSet, NumericValue, PatternExpression, PatternGroup, PrimitiveKind, TypeKind, TypeRef,
    WhiteSpacePolicy,
};
use std::path::Path;
const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn fixture() -> (
    ams_gra_oms_ir::SchemaIr,
    ams_gra_oms_service_contract::Contract,
) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/temporal/time-zulu.xsd");
    (
        ams_gra_oms_xsd_frontend::load_schema_document(&path).unwrap(),
        ams_gra_oms_service_contract::parse_yaml(
            &std::fs::read_to_string(path.with_extension("yaml")).unwrap(),
        )
        .unwrap(),
    )
}
fn backend(language: BackendLanguage) -> Box<dyn Backend> {
    match language {
        BackendLanguage::Ada => Box::new(ams_gra_oms_backend_ada::AdaBackend),
        BackendLanguage::Rust => Box::new(ams_gra_oms_backend_rust::RustBackend),
        BackendLanguage::Cpp => Box::new(ams_gra_oms_backend_cpp::CppBackend),
    }
}

#[test]
fn task061_exact_named_time_and_neighbors_share_the_boundary() {
    let (schema, contract) = fixture();
    let clock = schema
        .types
        .iter()
        .position(|d| d.name.local_name == "Clock")
        .unwrap();
    let exact = schema.types[clock].constraints.clone();
    let mut neighbors = vec![exact.clone(), ConstraintSet::default()];
    let mut different = exact.clone();
    different.lexical.pattern_groups[0].alternatives[0].expression = ".*Z".into();
    neighbors.push(different);
    let mut alternatives = exact.clone();
    alternatives.lexical.pattern_groups[0]
        .alternatives
        .push(PatternExpression::xml_schema(".+Z"));
    neighbors.push(alternatives);
    let mut groups = exact.clone();
    groups.lexical.pattern_groups.push(PatternGroup {
        alternatives: vec![PatternExpression::xml_schema(".+Z")],
    });
    neighbors.push(groups);
    {
        let policy = WhiteSpacePolicy::Collapse;
        let mut c = exact.clone();
        c.lexical.white_space = Some(policy);
        neighbors.push(c);
    }
    neighbors.push(ConstraintSet {
        min_inclusive: Some(NumericValue::Integer(0)),
        ..exact.clone()
    });
    neighbors.push(ConstraintSet {
        length: Some(9),
        ..exact
    });
    for (index, constraints) in neighbors.into_iter().enumerate() {
        let supported = index == 0;
        let mut changed = schema.clone();
        changed.types[clock].constraints = constraints.clone();
        assert_eq!(
            temporal_profile(PrimitiveKind::Time, &constraints),
            if supported {
                Ok(Some(TemporalProfile::TimeZulu))
            } else {
                Err(ams_gra_oms_codegen_core::TemporalProfileError::TimeUnsupported)
            }
        );
        if changed.validate().is_err() {
            assert!(!supported);
            for language in BackendLanguage::ALL {
                assert!(backend(language).generate(&changed, CLOSED).is_err());
            }
            continue;
        }
        let plan = resolve_service_plan(&contract, &changed).unwrap();
        let coverage = CoverageAnalysis::new(&changed, CLOSED).unwrap();
        for language in BackendLanguage::ALL {
            let r = analyze_service_readiness(&plan, &changed, language, CLOSED).unwrap();
            assert_eq!(r.is_ready(), supported, "{index} {language:?}: {r:?}");
            assert_eq!(
                coverage
                    .backend_coverage(language)
                    .unwrap()
                    .declarations_fully_renderable
                    == changed.types.len(),
                supported
            );
            assert_eq!(
                backend(language).generate(&changed, CLOSED).is_ok(),
                supported
            );
        }
        if !supported {
            // A UCI-looking name is never admission evidence.
            changed.types[clock].name.local_name = "TimeType".into();
            assert!(
                temporal_profile(PrimitiveKind::Time, &changed.types[clock].constraints).is_err()
            );
        }
    }
    // The IR has only XmlSchema in PatternDialect: a wrong dialect is not
    // representable. Explicit dialect equality remains in the shared classifier.
    assert_eq!(
        schema.types[clock].constraints.lexical.pattern_groups[0].alternatives[0].dialect,
        ams_gra_oms_ir::PatternDialect::XmlSchema
    );
}

#[test]
fn task061_generated_support_only_time_blocks_final_readiness() {
    let (mut schema, contract) = fixture();
    let clock = schema
        .types
        .iter()
        .find(|d| d.name.local_name == "Clock")
        .unwrap()
        .clone();
    let mut unsupported = clock.clone();
    unsupported.name.local_name = "SupportTime".into();
    unsupported.constraints = ConstraintSet::default();
    schema.types.push(unsupported.clone());
    let concrete = schema
        .types
        .iter_mut()
        .find(|d| d.name.local_name == "ConcreteClock")
        .unwrap();
    let TypeKind::Record { fields } = &mut concrete.kind else {
        panic!()
    };
    fields[0].type_ref = TypeRef::named(unsupported.name.clone());
    let plan = resolve_service_plan(&contract, &schema).unwrap();
    let projection = project_service_generation_schema(&plan, &schema, CLOSED).unwrap();
    assert!(
        projection
            .generated_support_type_names()
            .contains(&unsupported.name)
    );
    assert!(!projection.selected_type_names().contains(&unsupported.name));
    for language in BackendLanguage::ALL {
        let r = analyze_service_readiness(&plan, &schema, language, CLOSED).unwrap();
        assert_eq!(r.selected_types_renderable, r.selected_types_total);
        assert!(!r.is_ready());
        assert!(
            r.unsupported_generated_support_types
                .contains(&unsupported.name)
        );
        assert!(
            backend(language)
                .generate(projection.schema(), CLOSED)
                .is_err()
        );
    }
}

#[test]
fn task061_direct_time_is_still_unsupported_in_records_and_choices() {
    let (schema, contract) = fixture();
    for owner in ["Base", "Pick"] {
        let mut changed = schema.clone();
        let d = changed
            .types
            .iter_mut()
            .find(|d| d.name.local_name == owner)
            .unwrap();
        let fields = match &mut d.kind {
            TypeKind::Record { fields }
            | TypeKind::Choice {
                alternatives: fields,
            } => fields,
            _ => panic!(),
        };
        fields[0].type_ref = TypeRef::primitive(PrimitiveKind::Time);
        let plan = resolve_service_plan(&contract, &changed).unwrap();
        for language in BackendLanguage::ALL {
            assert!(
                !analyze_service_readiness(&plan, &changed, language, CLOSED)
                    .unwrap()
                    .is_ready()
            );
            assert!(backend(language).generate(&changed, CLOSED).is_err());
        }
    }
}

#[test]
fn task061_private_scanner_helpers_reserve_no_global_names() {
    let (schema, _) = fixture();
    let clock = schema
        .types
        .iter()
        .find(|d| d.name.local_name == "Clock")
        .unwrap();
    for name in ["Valid", "Two", "Collapse", "XmlSchemaTimeParser"] {
        let mut changed = schema.clone();
        let mut declaration = clock.clone();
        declaration.name.local_name = name.into();
        declaration.kind = TypeKind::Primitive(PrimitiveKind::Boolean);
        declaration.base_type = None;
        declaration.constraints = ConstraintSet::default();
        changed.types.push(declaration);
        for language in BackendLanguage::ALL {
            backend(language)
                .generate(&changed, CLOSED)
                .unwrap_or_else(|error| panic!("{language:?} {name}: {error}"));
        }
    }
    // Projecting an unrelated selected message drops the unused Time carrier,
    // so no scanner is emitted or unnecessarily reserved.
    let mut absent = schema.clone();
    let unrelated = absent
        .types
        .iter()
        .find(|d| d.name.local_name == "Unrelated")
        .unwrap()
        .name
        .clone();
    absent.messages[0].payload_type = TypeRef::named(unrelated);
    let (_, contract) = fixture();
    let plan = resolve_service_plan(&contract, &absent).unwrap();
    let projection = project_service_generation_schema(&plan, &absent, CLOSED).unwrap();
    for language in BackendLanguage::ALL {
        let generated = backend(language)
            .generate(projection.schema(), CLOSED)
            .unwrap();
        assert!(!generated.iter().any(|f| f.contents.contains("fn valid(")
            || f.contents.contains("static bool valid(")
            || f.contents.contains("function Valid (")));
    }
}
