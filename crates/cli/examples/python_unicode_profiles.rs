//! Inspect actual normalized pinned declarations using the shared classifier.
use ams_gra_oms_codegen_core::{StringProfile, UnicodeStringProfile, string_profile};
use ams_gra_oms_ir::{PrimitiveKind, TypeKind};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let schema = load_schema_set(Path::new(&args[1])).expect("pinned schema");
    let rows: Vec<_> = schema.types.iter().filter_map(|decl| {
        if decl.kind != TypeKind::Primitive(PrimitiveKind::String) { return None; }
        let StringProfile::Unicode(profile) = string_profile(PrimitiveKind::String, &decl.constraints).ok()?? else { return None; };
        assert_eq!(decl.constraints, UnicodeStringProfile::ALL[usize::from(profile.id())].constraints());
        Some(serde_json::json!({"name": decl.name.local_name, "namespace": decl.name.namespace_uri,
            "profile": profile.id(), "length": profile.length(),
            "constraints_debug": format!("{:?}", decl.constraints),
            "patterns": decl.constraints.lexical.pattern_groups.iter().map(|group|
                group.alternatives.iter().map(|pattern| &pattern.expression).collect::<Vec<_>>()
            ).collect::<Vec<_>>(),
            "white_space": format!("{:?}", decl.constraints.lexical.white_space)}))
    }).collect();
    println!("{}", serde_json::to_string(&rows).unwrap());
}
