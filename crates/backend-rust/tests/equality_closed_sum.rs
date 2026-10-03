//! Generic equality propagation through stored closed sums, not schema names.
use ams_gra_oms_backend_rust::generate;
use ams_gra_oms_codegen_core::GenerationWorld;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::process::Command;

#[test]
fn absent_only_storage_preserves_equality_in_closed_world() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../xsd-frontend/tests/fixtures");
    for (fixture, name) in [
        ("backend-uninhabited-abstract-optional.xsd", "Holder"),
        (
            "backend-uninhabited-abstract-inherited.xsd",
            "ConcreteHolder",
        ),
        (
            "backend-uninhabited-abstract-composes-closed-sum.xsd",
            "Holder",
        ),
    ] {
        let schema = load_schema_document(&root.join(fixture)).unwrap();
        let model = generate(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        assert!(
            model.contains(&format!(
                "#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {name}"
            )),
            "{fixture}"
        );
        // The same abstract value cannot be erased in an open generation world.
        assert!(
            generate(&schema, GenerationWorld::OpenExtensions).is_err(),
            "{fixture}"
        );
        let dir =
            std::env::temp_dir().join(format!("task060-elided-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("model.rs"), model).unwrap();
        let built = Command::new("rustc")
            .current_dir(&dir)
            .args([
                "--edition=2021",
                "-Dwarnings",
                "--crate-type=lib",
                "model.rs",
            ])
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn closed_sum_container_traits_follow_concrete_payload_traits() {
    let dir = std::env::temp_dir().join(format!("task060-equality-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (kind, derive) in [
        ("xs:boolean", "#[derive(Debug, Clone, PartialEq, Eq)]"),
        ("xs:double", "#[derive(Debug, Clone, PartialEq)]"),
        ("xs:dateTime", "#[derive(Debug, Clone)]"),
    ] {
        let schema = format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:derive" targetNamespace="urn:derive" elementFormDefault="qualified">
<xs:complexType name="Base" abstract="true"><xs:sequence><xs:element name="Flag" type="xs:boolean"/></xs:sequence></xs:complexType>
<xs:complexType name="Leaf"><xs:complexContent><xs:extension base="t:Base"><xs:sequence><xs:element name="Value" type="{kind}"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType>
<xs:complexType name="Middle"><xs:sequence><xs:element name="Items" type="t:Base" minOccurs="0" maxOccurs="unbounded"/></xs:sequence></xs:complexType>
<xs:complexType name="Pick"><xs:choice><xs:element name="Middle" type="t:Middle"/><xs:element name="Flag" type="xs:boolean"/></xs:choice></xs:complexType>
<xs:complexType name="Parent"><xs:sequence><xs:element name="Pick" type="t:Pick" minOccurs="0"/><xs:element name="Batch" type="t:Middle" minOccurs="0" maxOccurs="4"/></xs:sequence></xs:complexType>
</xs:schema>"#
        );
        std::fs::write(dir.join("schema.xsd"), schema).unwrap();
        let ir = load_schema_document(&dir.join("schema.xsd")).unwrap();
        let model = generate(&ir, GenerationWorld::ClosedSchemaSet).unwrap();
        for name in ["Base", "Leaf", "Middle", "Pick", "Parent"] {
            let category = if ["Base", "Pick"].contains(&name) {
                "enum"
            } else {
                "struct"
            };
            assert!(
                model.contains(&format!("{derive}\npub {category} {name}")),
                "{kind} {name}"
            );
        }
        let mut reversed = ir.clone();
        reversed.types.reverse();
        let other = generate(&reversed, GenerationWorld::ClosedSchemaSet).unwrap();
        for name in ["Base", "Middle", "Parent"] {
            let category = if name == "Base" { "enum" } else { "struct" };
            assert!(other.contains(&format!("{derive}\npub {category} {name}")));
        }
        std::fs::write(dir.join("model.rs"), model).unwrap();
        let output = Command::new("rustc")
            .current_dir(&dir)
            .args([
                "--edition=2021",
                "-Dwarnings",
                "--crate-type=lib",
                "model.rs",
                "-o",
                "model.rlib",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}
