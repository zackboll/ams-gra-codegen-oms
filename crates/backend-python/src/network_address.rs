//! Python's deliberately narrow subset of the existing shared String profiles.
use crate::error;
use ams_gra_oms_codegen_core::{
    CodegenError, StringProfile, StructuredAsciiAlphabet as Alphabet, StructuredAsciiFacets,
    StructuredAsciiRepetition as Repetition, StructuredAsciiSegment as Segment,
    factor_delimited_ascii, string_profile,
};
use ams_gra_oms_ir::{PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRefTarget};
use std::collections::BTreeSet;

pub(crate) fn profile(declaration: &TypeDecl) -> Option<&'static str> {
    if declaration.kind != TypeKind::Primitive(PrimitiveKind::String) {
        return None;
    }
    match string_profile(PrimitiveKind::String, &declaration.constraints).ok()?? {
        StringProfile::Ipv6Address => Some("_check_ipv6"),
        StringProfile::AlternatingAscii(profile) => {
            if profile.facets != StructuredAsciiFacets::Range(7, 15) || profile.groups.len() != 1 {
                return None;
            }
            let product = factor_delimited_ascii(&profile.groups[0])?;
            let class =
                |ranges, count| Segment::Class(Alphabet::Finite(ranges), Repetition::Exact(count));
            // Prove all five octet bodies and the complete four-way product.
            // Neither a declaration name nor merely the delimiter/arity suffices.
            let expected = [
                vec![class(&[(b'0', b'9')], 1)],
                vec![class(&[(b'1', b'9')], 1), class(&[(b'0', b'9')], 1)],
                vec![Segment::Literal("1"), class(&[(b'0', b'9')], 2)],
                vec![
                    Segment::Literal("2"),
                    class(&[(b'0', b'4')], 1),
                    class(&[(b'0', b'9')], 1),
                ],
                vec![Segment::Literal("25"), class(&[(b'0', b'5')], 1)],
            ];
            (product.delimiter == b'.'
                && product.components == 4
                && product.alternatives.len() == expected.len()
                && expected
                    .iter()
                    .all(|body| product.alternatives.contains(body)))
            .then_some("_check_ipv4")
        }
        _ => None,
    }
}

pub(crate) fn uses(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|decl| profile(decl).is_some())
}

pub(crate) fn ancestry(schema: &SchemaIr, declaration: &TypeDecl) -> Result<(), CodegenError> {
    let expected = profile(declaration).ok_or_else(|| error("unknown address profile"))?;
    let mut current = declaration;
    let mut seen = BTreeSet::new();
    loop {
        if current.is_abstract || !seen.insert(&current.name) || profile(current) != Some(expected)
        {
            return Err(error("unproven network-address restriction ancestry"));
        }
        match current.base_type.as_ref().map(|base| &base.target) {
            None | Some(TypeRefTarget::Primitive(PrimitiveKind::String)) => return Ok(()),
            Some(TypeRefTarget::Named(name)) => {
                current = schema
                    .types
                    .iter()
                    .find(|parent| parent.name == *name)
                    .ok_or_else(|| error("missing network-address restriction base"))?;
            }
            _ => return Err(error("incompatible network-address restriction base")),
        }
    }
}
