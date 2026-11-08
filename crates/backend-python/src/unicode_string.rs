//! Exact shared Task066 profiles; no Python-specific classifier or Unicode policy.
use crate::{error, names};
use ams_gra_oms_codegen_core::{
    CodegenError, StringProfile, UNICODE31_ND, UnicodeStringProfile, UnicodeStringToken,
    string_profile,
};
use ams_gra_oms_ir::{PrimitiveKind, SchemaIr, TypeDecl, TypeKind, TypeRefTarget};
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub(crate) fn profile(declaration: &TypeDecl) -> Option<UnicodeStringProfile> {
    if declaration.kind != TypeKind::Primitive(PrimitiveKind::String) {
        return None;
    }
    match string_profile(PrimitiveKind::String, &declaration.constraints).ok()?? {
        StringProfile::Unicode(profile) => Some(profile),
        _ => None,
    }
}

pub(crate) fn uses(schema: &SchemaIr) -> bool {
    schema.types.iter().any(|decl| profile(decl).is_some())
}

pub(crate) fn ancestry(schema: &SchemaIr, declaration: &TypeDecl) -> Result<(), CodegenError> {
    let expected = profile(declaration).ok_or_else(|| error("unknown Unicode String profile"))?;
    let mut current = declaration;
    let mut seen = BTreeSet::new();
    loop {
        if current.is_abstract || !seen.insert(&current.name) || profile(current) != Some(expected)
        {
            return Err(error("unproven Unicode String restriction ancestry"));
        }
        match current.base_type.as_ref().map(|base| &base.target) {
            None | Some(TypeRefTarget::Primitive(PrimitiveKind::String)) => return Ok(()),
            Some(TypeRefTarget::Named(name)) => {
                current = schema
                    .types
                    .iter()
                    .find(|parent| parent.name == *name)
                    .ok_or_else(|| error("missing Unicode String restriction base"))?;
            }
            _ => return Err(error("incompatible Unicode String restriction base")),
        }
    }
}

pub(crate) fn render_runtime(output: &mut String, schema: &SchemaIr) {
    if !uses(schema) {
        return;
    }
    output.push_str("\n\n# Unicode 3.1 Nd and ordered languages derived from codegen-core.\n_unicode31_nd = (\n");
    for (first, last) in UNICODE31_ND {
        writeln!(output, "    (0x{first:x}, 0x{last:x}),").unwrap();
    }
    output.push_str(")\n_unicode31_profiles = {\n");
    for profile in UnicodeStringProfile::ALL {
        if !schema
            .types
            .iter()
            .any(|decl| self::profile(decl) == Some(profile))
        {
            continue;
        }
        writeln!(output, "    {}: ({}, (", profile.id(), profile.length()).unwrap();
        for branch in profile.branches() {
            output.push_str("        (");
            for token in branch {
                match token {
                    UnicodeStringToken::Ascii(set) => output.push_str(&names::literal(set)),
                    UnicodeStringToken::DecimalDigit => output.push_str("None"),
                }
                output.push_str(", ");
            }
            output.push_str("),\n");
        }
        output.push_str("    )),\n");
    }
    output.push_str("}\n");
    output.push_str(include_str!("unicode31_runtime.py"));
}
