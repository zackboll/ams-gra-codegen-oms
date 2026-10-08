//! Task 065: component/subtype hiding is legal by qualification, not rejection.
use ams_gra_oms_codegen_core::{Backend, BackendLanguage, GenerationWorld, backend_preflight};
use ams_gra_oms_ir::SchemaIr;
use std::{path::Path, process::Command};

const WORLD: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn schema(directory: &Path, member: &str, target: &str, shape: &str, occurs: &str) -> SchemaIr {
    let path = directory.join("input.xsd");
    let content = format!("<xs:element name=\"{member}\" type=\"t:{target}\" {occurs}/>");
    let owner = if shape == "inherited" {
        format!(
            "<xs:complexType name=\"Ancestor\" abstract=\"true\"><xs:sequence>{content}</xs:sequence></xs:complexType>\
             <xs:complexType name=\"Holder\"><xs:complexContent><xs:extension base=\"t:Ancestor\"><xs:sequence/></xs:extension></xs:complexContent></xs:complexType>"
        )
    } else {
        let tag = if shape == "choice" {
            "choice"
        } else {
            "sequence"
        };
        format!("<xs:complexType name=\"Holder\"><xs:{tag}>{content}</xs:{tag}></xs:complexType>")
    };
    std::fs::write(
        &path,
        format!(
            "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" xmlns:t=\"urn:test:shadow\" targetNamespace=\"urn:test:shadow\">\
             <xs:simpleType name=\"{target}\"><xs:restriction base=\"xs:long\"/></xs:simpleType>{owner}</xs:schema>"
        ),
    )
    .unwrap();
    ams_gra_oms_xsd_frontend::load_schema_set(&path).unwrap()
}

fn compile(directory: &Path, schema: &SchemaIr, planted: Option<(&str, &str)>) {
    let files = ams_gra_oms_backend_ada::AdaBackend
        .generate(schema, WORLD)
        .unwrap();
    let mut spec = None;
    for file in files {
        let mut contents = file.contents;
        if let Some((old, new)) = planted {
            contents = contents.replace(old, new);
        }
        if contents.contains("type Holder") {
            spec = Some(file.relative_path.clone());
        }
        std::fs::write(directory.join(file.relative_path), contents).unwrap();
    }
    if Command::new("gnatmake").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
            "GNAT required"
        );
        return;
    }
    let result = Command::new("gnatmake")
        .current_dir(directory)
        .args(["-f", "-c", "-q", "-gnatc", "-gnat2022", "-gnatwe"])
        .arg(spec.unwrap())
        .output()
        .unwrap();
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::write(directory.join("gnat.log"), &diagnostics).unwrap();
    if planted.is_some() {
        assert!(
            !result.status.success(),
            "planted hidden subtype unexpectedly compiled"
        );
        assert!(
            diagnostics.contains("cannot be used before end of record declaration")
                || diagnostics.contains("subtype mark required in this context"),
            "{diagnostics}"
        );
        println!("PLANTED BEFORE: {diagnostics}");
    } else {
        assert!(result.status.success(), "{diagnostics}");
        for body in std::fs::read_dir(directory)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "adb"))
        {
            let result = Command::new("gnatmake")
                .current_dir(directory)
                .args(["-f", "-c", "-q", "-gnat2022", "-gnatwe"])
                .arg(body.file_name().unwrap())
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}

#[test]
fn task065_component_hiding_compile_matrix() {
    let root = std::env::temp_dir().join(format!("task065-hiding-{}", std::process::id()));
    let mut count = 0;
    for shape in ["record", "choice", "inherited"] {
        for member in ["Foo", "fOO"] {
            for (storage, occurs) in [
                ("required", ""),
                ("optional", "minOccurs=\"0\""),
                ("bounded", "minOccurs=\"0\" maxOccurs=\"3\""),
                ("unbounded", "minOccurs=\"0\" maxOccurs=\"unbounded\""),
            ] {
                // Optional Choice alternatives use a different admission policy.
                if shape == "choice" && storage == "optional" {
                    continue;
                }
                let directory = root.join(format!("{shape}-{member}-{storage}"));
                std::fs::create_dir_all(&directory).unwrap();
                let schema = schema(&directory, member, "Foo", shape, occurs);
                assert!(backend_preflight(&schema, BackendLanguage::Ada, WORLD).is_ok());
                let source = ams_gra_oms_backend_ada::generate(&schema, WORLD).unwrap();
                if storage == "required" {
                    assert!(source.contains(&format!("{member} : Test.Shadow.Foo;")));
                } else {
                    assert!(
                        !source.contains("Test.Shadow.Foo"),
                        "unnecessary qualification"
                    );
                    assert!(source.contains(&format!("{member} : Holder_{member}_")));
                }
                compile(&directory, &schema, None);
                count += 1;
                println!("COMPONENT MATRIX\t{shape}\t{member}\t{storage}\tPASSED");
            }
        }
    }
    assert_eq!(count, 22);
    // The internal optional component can itself hide the named type Value.
    let directory = root.join("optional-value");
    std::fs::create_dir_all(&directory).unwrap();
    let schema = schema(&directory, "Other", "Value", "record", "minOccurs=\"0\"");
    let source = ams_gra_oms_backend_ada::generate(&schema, WORLD).unwrap();
    assert!(source.contains("Other : Holder_Other_Optional;"));
    assert!(source.contains("Value : Test.Shadow.Value;"));
    compile(&directory, &schema, None);
    println!("TASK065 COMPONENT COMPILE MATRIX: PASSED (23 cells)");
}

#[test]
fn task065_component_hiding_planted_failure_controls() {
    for shape in ["record", "choice"] {
        for member in ["Foo", "fOO"] {
            let directory = std::env::temp_dir().join(format!(
                "task065-planted-{}-{shape}-{member}",
                std::process::id()
            ));
            std::fs::create_dir_all(&directory).unwrap();
            let schema = schema(&directory, member, "Foo", shape, "");
            let old = format!("{member} : Foo;");
            let new = format!("{member} : Test.Shadow.Foo;");
            println!("BEFORE {old}\nAFTER {new}");
            compile(&directory, &schema, Some((&new, &old)));
        }
    }
    println!("TASK065 COMPONENT PLANTED FAILURE CONTROLS: PASSED (4 cells)");
}

#[test]
fn task065_component_nonhiding_output_stability() {
    for shape in ["record", "choice"] {
        let directory =
            std::env::temp_dir().join(format!("task065-stable-{}-{shape}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let schema = schema(&directory, "Other", "Foo", shape, "");
        let source = ams_gra_oms_backend_ada::generate(&schema, WORLD).unwrap();
        assert!(source.contains("Other : Foo;"));
        assert!(!source.contains("Test.Shadow.Foo"));
        compile(&directory, &schema, None);
    }
    println!("TASK065 COMPONENT NONHIDING STABILITY: PASSED (2 cells)");
}

#[test]
fn task065_preceding_component_hiding_controls() {
    for shape in ["record", "choice"] {
        for member in ["Foo", "fOO"] {
            let directory = std::env::temp_dir().join(format!(
                "task065-preceding-{}-{shape}-{member}",
                std::process::id()
            ));
            std::fs::create_dir_all(&directory).unwrap();
            let mut schema = schema(&directory, member, "Foo", shape, "");
            let owner = schema
                .types
                .iter_mut()
                .find(|d| d.name.local_name == "Holder")
                .unwrap();
            let fields = match &mut owner.kind {
                ams_gra_oms_ir::TypeKind::Record { fields } => fields,
                ams_gra_oms_ir::TypeKind::Choice { alternatives } => alternatives,
                _ => unreachable!(),
            };
            let mut other = fields[0].clone();
            other.name = "Other".to_owned();
            fields.push(other);
            let source = ams_gra_oms_backend_ada::generate(&schema, WORLD).unwrap();
            assert!(source.contains("Other : Test.Shadow.Foo;"));
            compile(&directory, &schema, None);
            compile(
                &directory,
                &schema,
                Some(("Other : Test.Shadow.Foo;", "Other : Foo;")),
            );
        }
    }
    println!("TASK065 PRECEDING COMPONENT CONTROLS: PASSED (4 cells)");
}

#[test]
fn task065_discriminant_hiding_controls() {
    for (shape, target, occurs) in [
        ("choice", "Kind", ""),
        ("record", "Is_Present", "minOccurs=\"0\""),
    ] {
        let directory = std::env::temp_dir().join(format!(
            "task065-discriminant-{}-{target}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let schema = schema(&directory, "Other", target, shape, occurs);
        let source = ams_gra_oms_backend_ada::generate(&schema, WORLD).unwrap();
        let component = if shape == "choice" { "Other" } else { "Value" };
        let new = format!("{component} : Test.Shadow.{target};");
        let old = format!("{component} : {target};");
        assert!(source.contains(&new));
        compile(&directory, &schema, None);
        compile(&directory, &schema, Some((&new, &old)));
    }
    println!("TASK065 DISCRIMINANT CONTROLS: PASSED (2 cells)");
}

#[test]
fn task065_package_prefix_hiding_control() {
    let directory =
        std::env::temp_dir().join(format!("task065-package-prefix-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let schema = schema(&directory, "Test", "Test", "record", "");
    let source = ams_gra_oms_backend_ada::generate(&schema, WORLD).unwrap();
    assert!(source.contains("Test : Standard.Test.Shadow.Test;"));
    compile(&directory, &schema, None);
    compile(
        &directory,
        &schema,
        Some((
            "Test : Standard.Test.Shadow.Test;",
            "Test : Test.Shadow.Test;",
        )),
    );
    println!("TASK065 PACKAGE PREFIX CONTROL: PASSED (1 cell)");
}
