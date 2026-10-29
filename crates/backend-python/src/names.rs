//! Python's own deterministic naming policy, independent of other backends.

use crate::error;
use ams_gra_oms_codegen_core::CodegenError;
use ams_gra_oms_ir::{SchemaIr, TypeDecl, TypeKind};
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub(crate) fn symbol(original: &str, member: bool) -> String {
    let mut name: String = original
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                if member { ch.to_ascii_lowercase() } else { ch }
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty()
        || !name.as_bytes()[0].is_ascii_alphabetic()
        || KEYWORDS.contains(&name.as_str())
    {
        name.insert_str(0, "schema_");
    }
    name
}

const KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

pub(crate) fn literal(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            ch if ch.is_ascii() && !ch.is_ascii_control() => output.push(ch),
            ch => write!(output, "\\U{:08x}", u32::from(ch)).unwrap(),
        }
    }
    output.push('"');
    output
}

fn insert(seen: &mut BTreeSet<String>, original: &str, member: bool) -> Result<(), CodegenError> {
    let name = symbol(original, member);
    if !seen.insert(name.clone()) {
        return Err(error(format!(
            "Python name collision: {original:?} -> {name}"
        )));
    }
    Ok(())
}

pub(crate) fn validate(schema: &SchemaIr, declarations: &[&TypeDecl]) -> Result<(), CodegenError> {
    // Builtins used by annotations, decorators and runtime checks must never be
    // shadowed by a schema declaration. Leading-underscore runtime names cannot
    // be emitted by symbol(). No foreign-language preflight is consulted.
    let mut top: BTreeSet<String> = [
        "bool",
        "str",
        "int",
        "tuple",
        "type",
        "object",
        "len",
        "TypeError",
        "ValueError",
        // Generated method/loop local names cannot also denote schema types.
        "self",
        "item",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if crate::uses_float64(schema) {
        top.insert("float".to_owned());
        // A lowercase declaration would collide with the private codec helper.
        top.insert("float64".to_owned());
    }
    for declaration in declarations {
        insert(&mut top, &declaration.name.local_name, false)?;
        match &declaration.kind {
            TypeKind::Record { .. } => {
                // An annotation is evaluated in the class body. A field named
                // `int`, `tuple`, or a lowercase schema type would shadow a
                // later annotation with its dataclasses.Field instance.
                let mut members: BTreeSet<String> = ["bool", "str", "int", "tuple"]
                    .into_iter()
                    .map(str::to_owned)
                    .chain(
                        declarations
                            .iter()
                            .map(|declaration| symbol(&declaration.name.local_name, false)),
                    )
                    .collect();
                if crate::uses_float64(schema) {
                    members.insert("float".to_owned());
                }
                for field in crate::record_fields(schema, declaration)? {
                    insert(&mut members, &field.name, true)?;
                }
            }
            TypeKind::Enumeration { variants } => {
                let mut members = ["name", "value", "mro"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                let mut values = BTreeSet::new();
                for variant in variants {
                    insert(&mut members, &variant.wire_value, false)?;
                    if !values.insert(&variant.wire_value) {
                        return Err(error("duplicate enumeration value"));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_symbols_and_literals_are_deterministic() {
        assert_eq!(symbol("class", true), "schema_class");
        assert_eq!(symbol("9-lives", false), "schema_9_lives");
        assert_eq!(symbol("__post_init__", true), "schema___post_init__");
        assert_eq!(
            literal("a\"\\\n\0é"),
            "\"a\\\"\\\\\\U0000000a\\U00000000\\U000000e9\""
        );
    }
}
