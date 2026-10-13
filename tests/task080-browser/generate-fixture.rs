// Task-local fixture driver: the same validated IR-only Alias/List forms as
// crates/schema-docs/tests/explorer.rs, rendered by the production exporter.
use ams_gra_oms_ir::{Cardinality, QualifiedName, TypeKind, TypeRef};
use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut schema =
        ams_gra_oms_xsd_frontend::load_schema_set_with_overlays(Path::new(&args[1]), &[]).unwrap();
    for t in &mut schema.types {
        match t.name.local_name.as_str() {
            "Alias" => {
                t.kind =
                    TypeKind::Alias(TypeRef::named(QualifiedName::new("urn:explorer", "Enum")));
                t.base_type = None;
            }
            "Items" => {
                t.kind = TypeKind::List {
                    item_type: TypeRef::named(QualifiedName::new("urn:explorer", "Alias")),
                    cardinality: Cardinality {
                        min_occurs: 2,
                        max_occurs: Some(8),
                    },
                };
                t.base_type = None;
            }
            _ => {}
        }
    }
    let mut other = schema
        .types
        .iter()
        .find(|t| t.name.local_name == "Child")
        .unwrap()
        .clone();
    other.name = QualifiedName::new("urn:other", "Child");
    schema.namespaces.push(ams_gra_oms_ir::NamespaceDecl {
        uri: "urn:other".into(),
        preferred_prefix: None,
    });
    schema.types.push(other);
    let root = schema
        .types
        .iter_mut()
        .find(|t| t.name.local_name == "Root")
        .unwrap();
    if let TypeKind::Record { fields } = &mut root.kind {
        let mut duplicate = fields[0].clone();
        duplicate.name = "Inherited".into();
        fields.push(duplicate);
        let mut other = fields[0].clone();
        other.name = "Other".into();
        other.type_ref = TypeRef::named(QualifiedName::new("urn:other", "Child"));
        fields.push(other);
    }
    let output = Path::new(&args[2]);
    for file in ams_gra_oms_schema_docs::generate(&schema).unwrap() {
        let path = output.join(file.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, file.contents).unwrap();
    }
}
