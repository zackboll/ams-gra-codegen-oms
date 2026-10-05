use ams_gra_oms_codegen_core::{
    StringProfile, UNICODE31_ND, UnicodeStringProfile, string_profile, unicode31_decimal_digit,
};
use ams_gra_oms_ir::{NumericValue, PrimitiveKind, WhiteSpacePolicy};

#[test]
fn task066_authority_membership_and_independent_profile_corpus() {
    let rows = include_str!("../../../tests/fixtures/string/task066-unicode31-nd.tsv");
    let ranges: Vec<_> = rows
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let (a, b) = l.split_once('\t').unwrap();
            (
                u32::from_str_radix(a, 16).unwrap(),
                u32::from_str_radix(b, 16).unwrap(),
            )
        })
        .collect();
    assert_eq!(UNICODE31_ND, ranges);
    assert_eq!(ranges.len(), 21);
    assert_eq!(ranges.iter().map(|(a, b)| b - a + 1).sum::<u32>(), 248);
    for c in 0..=0x10FFFF {
        let expected = ranges.iter().any(|&(a, b)| (a..=b).contains(&c));
        assert_eq!(unicode31_decimal_digit(c), expected, "U+{c:X}");
    }
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/string/task066-corpus.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let cells: Vec<_> = line.split('\t').collect();
        let profile = UnicodeStringProfile::ALL[cells[0].parse::<usize>().unwrap()];
        let bytes: Vec<_> = cells[2]
            .as_bytes()
            .chunks(2)
            .map(|h| u8::from_str_radix(std::str::from_utf8(h).unwrap(), 16).unwrap())
            .collect();
        let expected = cells[1] == "1";
        let accepted = std::str::from_utf8(&bytes).is_ok_and(|text| profile.accepts(text));
        assert_eq!(accepted, expected, "{line}");
        count += 1;
    }
    println!("TASK066 CORE CORPUS: PASSED ({count} cases)");
}

#[test]
fn task066_exact_classifier_and_fail_closed_neighbors() {
    for p in UnicodeStringProfile::ALL {
        let c = p.constraints();
        assert_eq!(
            string_profile(PrimitiveKind::String, &c),
            Ok(Some(StringProfile::Unicode(p)))
        );
        let mut neighbors = Vec::new();
        for length in [
            None,
            Some(p.length() as u64 - 1),
            Some(p.length() as u64 + 1),
        ] {
            let mut n = c.clone();
            n.length = length;
            neighbors.push(n);
        }
        for space in [
            WhiteSpacePolicy::Preserve,
            WhiteSpacePolicy::Replace,
            WhiteSpacePolicy::Collapse,
        ] {
            let mut n = c.clone();
            n.lexical.white_space = Some(space);
            neighbors.push(n);
        }
        let mut n = c.clone();
        n.min_length = Some(0);
        neighbors.push(n);
        let mut n = c.clone();
        n.max_length = Some(100);
        neighbors.push(n);
        let mut n = c.clone();
        n.min_inclusive = Some(NumericValue::Integer(0));
        neighbors.push(n);
        let mut n = c.clone();
        n.lexical.pattern_groups[0].alternatives[0]
            .expression
            .push(' ');
        neighbors.push(n);
        let mut n = c.clone();
        n.lexical.pattern_groups[0].alternatives.pop();
        neighbors.push(n);
        let mut n = c.clone();
        let a = n.lexical.pattern_groups[0].alternatives[0].clone();
        n.lexical.pattern_groups[0].alternatives.push(a);
        neighbors.push(n);
        let mut n = c.clone();
        let g = n.lexical.pattern_groups[0].clone();
        n.lexical.pattern_groups.push(g);
        neighbors.push(n);
        for n in neighbors {
            assert!(
                string_profile(PrimitiveKind::String, &n).is_err(),
                "{p:?} {n:?}"
            );
        }
        assert_ne!(
            string_profile(PrimitiveKind::Boolean, &c),
            Ok(Some(StringProfile::Unicode(p)))
        );
    }
    assert_eq!("2١000101".chars().count(), 8);
    assert_eq!("2١000101".len(), 9);
    assert!(UnicodeStringProfile::Date.accepts("2١000101"));
    assert!(!UnicodeStringProfile::Date.accepts("١0000101"));
}

#[test]
fn task066_support_names_and_renamed_profile_are_preflight_consistent() {
    use ams_gra_oms_codegen_core::{
        BackendLanguage, GenerationWorld, backend_preflight, unsafe_named_declarations,
    };
    use ams_gra_oms_ir::{NamespaceDecl, QualifiedName, SchemaIr, SourceRef, TypeDecl, TypeKind};
    let source = SourceRef {
        document: "synthetic".into(),
        line: None,
    };
    let mut schema = SchemaIr {
        namespaces: vec![NamespaceDecl {
            uri: "urn:unicode".into(),
            preferred_prefix: None,
        }],
        types: vec![TypeDecl {
            name: QualifiedName::new("urn:unicode", "Renamed"),
            is_abstract: false,
            base_type: None,
            kind: TypeKind::Primitive(PrimitiveKind::String),
            constraints: UnicodeStringProfile::Date.constraints(),
            documentation: None,
            source: source.clone(),
        }],
        messages: vec![],
        schema_version: None,
    };
    for language in BackendLanguage::ALL {
        assert!(backend_preflight(&schema, language, GenerationWorld::ClosedSchemaSet).is_ok());
        let helper = if language == BackendLanguage::Ada {
            "Unicode31_String_Valid"
        } else {
            "Unicode31StringValidator"
        };
        let mut d = schema.types[0].clone();
        d.name = QualifiedName::new("urn:unicode", helper);
        d.kind = TypeKind::Primitive(PrimitiveKind::Boolean);
        d.constraints = Default::default();
        schema.types.push(d);
        assert!(backend_preflight(&schema, language, GenerationWorld::ClosedSchemaSet).is_err());
        assert!(
            !unsafe_named_declarations(&schema, language, GenerationWorld::ClosedSchemaSet)
                .is_empty()
        );
        schema.types.pop();
    }
    let mut d = schema.types[0].clone();
    d.name.local_name = "NITF_DateType".into();
    d.constraints.length = Some(9);
    schema.types.push(d);
    assert!(string_profile(PrimitiveKind::String, &schema.types[1].constraints).is_err());
}
