//! Execute the real frontend -> IR -> Python backend pipeline; Python is required,
//! never silently skipped. Set PYTHON to select a particular 3.11+ interpreter.

use ams_gra_oms_backend_python::{MODEL_FILE, PythonBackend, generate};
use ams_gra_oms_codegen_core::{
    Backend, GenerationWorld, build_service_api_model, project_service_generation_schema,
    resolve_service_plan,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, EnumVariant, NamespaceDecl, NumericValue, PrimitiveKind, SchemaIr,
    TypeKind, TypeRef,
};
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;
const OPEN: GenerationWorld = GenerationWorld::OpenExtensions;

fn fixture(path: &str) -> SchemaIr {
    load_schema_document(&Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .expect("valid fixture IR")
}

fn track() -> SchemaIr {
    fixture("../xsd-frontend/tests/fixtures/track.xsd")
}

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "py001-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn execute(schema: &SchemaIr, script: &str) {
    let temp = Temp::new();
    let files = PythonBackend.generate(schema, CLOSED).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].relative_path, Path::new(MODEL_FILE));
    std::fs::write(temp.0.join(MODEL_FILE), &files[0].contents).unwrap();
    std::fs::write(temp.0.join("probe.py"), format!(
        "import sys\nassert sys.version_info >= (3, 11)\nimport py_compile\npy_compile.compile('model.py', doraise=True)\nimport model\nfrom model import *\nimport typing\nfor name, value in vars(model).items():\n    if isinstance(value, type) and value.__module__ == 'model':\n        typing.get_type_hints(value)\n{script}"
    )).unwrap();
    // -I isolates user environment/site imports; the probe explicitly admits
    // only its temporary model directory. No pip or generated runtime package.
    for optimized in [false, true] {
        let mut command =
            Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
        command.arg("-I");
        if optimized {
            command.arg("-O");
        }
        let output = command.args(["-c", "import runpy, sys; sys.path.insert(0, '.'); runpy.run_path('probe.py', run_name='__main__')"])
            .current_dir(&temp.0).output().expect("Python 3.11+ must be installed");
        println!(
            "Python runtime probe (optimized={optimized}):\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "Python runtime probe failed");
    }
}

#[test]
fn real_track_compiles_imports_and_validates_at_runtime() {
    execute(&track(), include_str!("track_runtime.py"));
}

#[test]
fn scalar_subset_compiles_imports_and_validates_at_runtime() {
    let mut schema = fixture("tests/fixtures/scalars.xsd");
    // The existing frontend admits unsignedInt, not unsignedLong. Exercise the
    // backend's full shared u64 domain through normalized IR without widening
    // the frontend's independently qualified XSD subset.
    let mut wide = schema.types[3].clone();
    wide.name.local_name = "Unsigned64".to_owned();
    wide.constraints.max_inclusive = Some(NumericValue::Integer(i128::from(u64::MAX)));
    schema.types.push(wide);
    let mut unbounded = schema.types[3].clone();
    unbounded.name.local_name = "Unbounded_Unsigned".to_owned();
    unbounded.constraints = ConstraintSet::default();
    schema.types.push(unbounded);
    // Alias is an IR facility, not an excuse to add another XSD parser.
    let mut alias = schema.types[0].clone();
    alias.name.local_name = "Flag_Alias".to_owned();
    alias.base_type = None;
    alias.kind = TypeKind::Alias(TypeRef::named(schema.types[0].name.clone()));
    schema.types.insert(0, alias); // planner must move it after Flag
    let mut chain = schema.types[0].clone();
    chain.name.local_name = "Flag_Chain".to_owned();
    chain.kind = TypeKind::Alias(TypeRef::named(schema.types[0].name.clone()));
    schema.types.insert(0, chain);
    let mut direct_alias = schema.types[0].clone();
    direct_alias.name.local_name = "Direct_Alias".to_owned();
    direct_alias.kind = TypeKind::Alias(TypeRef::primitive(PrimitiveKind::String));
    schema.types.push(direct_alias);
    execute(&schema, include_str!("scalars_runtime.py"));
}

#[test]
fn golden_output_is_byte_identical_and_world_is_explicit() {
    let schema = track();
    let first = PythonBackend.generate(&schema, CLOSED).unwrap();
    assert_eq!(first, PythonBackend.generate(&schema, CLOSED).unwrap());
    assert_eq!(first, PythonBackend.generate(&schema, OPEN).unwrap());
    assert_eq!(first[0].contents, include_str!("expected/track.py"));
    let mut relocated = schema;
    for declaration in &mut relocated.types {
        declaration.source.document = "/different/absolute/location.xsd".to_owned();
    }
    assert_eq!(first, PythonBackend.generate(&relocated, CLOSED).unwrap());
}

fn rejected(schema: &SchemaIr) {
    let temp = Temp::new();
    for world in [CLOSED, OPEN] {
        // The normal artifact sink is reachable only after successful preflight.
        let result = PythonBackend.generate(schema, world).map(|files| {
            for file in files {
                std::fs::write(temp.0.join(file.relative_path), file.contents).unwrap();
            }
        });
        assert!(result.is_err(), "unexpectedly accepted {schema:?}");
        assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
    }
}

#[test]
fn negative_schema_fixtures_return_errors_and_no_artifacts() {
    for path in [
        "tests/fixtures/unsupported-choice.xsd",
        "tests/fixtures/unsupported-nillable.xsd",
        "tests/fixtures/unsupported-pattern.xsd",
        "../xsd-frontend/tests/fixtures/backend-abstract-value-reference.xsd",
        "../xsd-frontend/tests/fixtures/backend-uninhabited-abstract-optional.xsd",
        "../xsd-frontend/tests/fixtures/backend-base-only-abstract-ancestry.xsd",
    ] {
        rejected(&fixture(path));
    }
    let schema = fixture("../xsd-frontend/tests/fixtures/backend-abstract-value-reference.xsd");
    let error = generate(&schema, OPEN).unwrap_err();
    assert!(error.message.contains("open"), "{error}");
}

#[test]
fn unsupported_ir_semantics_fail_closed_without_artifacts() {
    let baseline = track();
    let mut cases = Vec::new();
    for kind in [
        PrimitiveKind::Decimal,
        PrimitiveKind::Float32,
        PrimitiveKind::Float64,
        PrimitiveKind::Binary,
        PrimitiveKind::DateTime,
        PrimitiveKind::Time,
        PrimitiveKind::Duration,
    ] {
        let mut schema = baseline.clone();
        schema.types[0].kind = TypeKind::Primitive(kind);
        schema.types[0].base_type = None;
        schema.types[0].constraints = ConstraintSet::default();
        cases.push(schema);
    }
    for constraints in [
        ConstraintSet {
            min_exclusive: Some(NumericValue::Integer(0)),
            ..ConstraintSet::default()
        },
        ConstraintSet {
            min_inclusive: Some(NumericValue::Integer(1)),
            ..ConstraintSet::default()
        },
        ConstraintSet {
            max_length: Some(2),
            ..ConstraintSet::default()
        },
        ConstraintSet {
            min_inclusive: Some(NumericValue::Integer(i128::from(i64::MIN) - 1)),
            max_inclusive: Some(NumericValue::Integer(0)),
            ..ConstraintSet::default()
        },
    ] {
        let mut schema = baseline.clone();
        schema.types[0].constraints = constraints;
        cases.push(schema);
    }
    let mut unbounded = baseline.clone();
    let TypeKind::Record { fields } = &mut unbounded.types[2].kind else {
        panic!()
    };
    fields[3].cardinality.max_occurs = None;
    cases.push(unbounded);
    let mut named_constraints = baseline.clone();
    let TypeKind::Record { fields } = &mut named_constraints.types[2].kind else {
        panic!()
    };
    fields[0].constraints.min_length = Some(1);
    cases.push(named_constraints);
    let mut list = baseline.clone();
    list.types[2].kind = TypeKind::List {
        item_type: TypeRef::primitive(PrimitiveKind::String),
        cardinality: Cardinality::REQUIRED_ONE,
    };
    cases.push(list);
    let mut alias = baseline.clone();
    alias.types[0].base_type = None;
    alias.types[0].constraints = ConstraintSet::default();
    alias.types[0].kind = TypeKind::Alias(TypeRef::named(alias.types[2].name.clone()));
    cases.push(alias);
    let mut namespaces = baseline.clone();
    namespaces.namespaces.push(NamespaceDecl {
        uri: "urn:other".to_owned(),
        preferred_prefix: None,
    });
    cases.push(namespaces);
    let mut invalid = baseline.clone();
    let TypeKind::Record { fields } = &mut invalid.types[2].kind else {
        panic!()
    };
    fields[0].type_ref =
        TypeRef::named(ams_gra_oms_ir::QualifiedName::new("urn:missing", "Missing"));
    cases.push(invalid);
    let mut cycle = baseline.clone();
    let self_ref = TypeRef::named(cycle.types[0].name.clone());
    cycle.types[0].base_type = None;
    cycle.types[0].constraints = ConstraintSet::default();
    cycle.types[0].kind = TypeKind::Alias(self_ref);
    cases.push(cycle);
    let mut ancestry = baseline.clone();
    ancestry.types[0].base_type = Some(TypeRef::named(ancestry.types[1].name.clone()));
    cases.push(ancestry);
    for schema in cases {
        rejected(&schema);
    }
}

#[test]
fn python_specific_name_collisions_fail_closed() {
    let mut schema = track();
    schema.types[0].name.local_name = "int".to_owned();
    let renamed = schema.types[0].name.clone();
    // Keep references valid so Python preflight, not resolution, rejects it.
    let TypeKind::Record { fields } = &mut schema.types[2].kind else {
        panic!()
    };
    fields[0].type_ref = TypeRef::named(renamed);
    assert!(
        generate(&schema, CLOSED)
            .unwrap_err()
            .message
            .contains("Python name collision")
    );
    rejected(&schema);

    let mut schema = track();
    let TypeKind::Record { fields } = &mut schema.types[2].kind else {
        panic!()
    };
    fields[2].name = "Sensor-Ids".to_owned();
    rejected(&schema);

    let mut schema = track();
    let TypeKind::Record { fields } = &mut schema.types[2].kind else {
        panic!()
    };
    fields[2].name = "int".to_owned();
    assert!(
        generate(&schema, CLOSED)
            .unwrap_err()
            .message
            .contains("Python name collision")
    );
    rejected(&schema);

    for values in [
        vec!["a-b", "a_b"],
        vec!["same", "same"],
        vec!["name", "other"],
    ] {
        let mut schema = track();
        schema.types[1].kind = TypeKind::Enumeration {
            variants: values
                .into_iter()
                .map(|value| EnumVariant {
                    wire_value: value.to_owned(),
                    documentation: None,
                })
                .collect(),
        };
        rejected(&schema);
    }
    let mut schema = track();
    let mut extra = schema.types[0].clone();
    extra.name.local_name = "Track-Id".to_owned();
    schema.types.push(extra);
    rejected(&schema);

    for reserved in ["tuple", "self", "item"] {
        let mut schema = track();
        let mut extra = schema.types[0].clone();
        extra.name.local_name = reserved.to_owned();
        schema.types.push(extra);
        assert!(
            generate(&schema, CLOSED)
                .unwrap_err()
                .message
                .contains("Python name collision")
        );
        rejected(&schema);
    }
}

#[test]
fn zero_oms_service_api_is_generated_without_a_model() {
    let schema = SchemaIr::empty();
    let contract = ams_gra_oms_service_contract::parse_yaml(
        "contract_version: '0.1'\nservice:\n  name: Python\n  version: '1.0'\n  kind: isolator\nstandards:\n  oms_version: '2.5'\n  uci_schema_version: '2.5'\nfunctions:\n  - id: f\n    name: Function\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: signal\n        kind: special_signal\n        direction: output\n        mandate: mandatory\n        name: Signal\n        timing:\n          kind: asynchronous\n"
    ).unwrap();
    let plan = resolve_service_plan(&contract, &schema).unwrap();
    let projection = project_service_generation_schema(&plan, &schema, CLOSED).unwrap();
    let model = build_service_api_model(&plan, projection.schema(), CLOSED).unwrap();
    let files = PythonBackend.generate_service_api(&model, &schema).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].relative_path, Path::new("service_api.py"));
    assert!(files[0].contents.contains("SERVICE_KIND = \"isolator\""));
    assert!(!files[0].contents.contains("Payload"));
    assert!(!files[0].contents.contains("import model"));
}
