//! Normalized IR-only alias chains and scalar payloads use the same renderer.
use ams_gra_oms_backend_python::{PythonBackend, generate_service_codec};
use ams_gra_oms_codegen_core::{
    Backend, GenerationWorld, build_service_api_model, project_service_generation_schema,
    resolve_service_plan,
};
use ams_gra_oms_ir::{NumericValue, TypeKind, TypeRef};
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_document;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn alias_chains_and_unique_scalar_payloads_preserve_exact_types_and_wire_values() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut schema =
        load_schema_document(&root.join("tests/fixtures/python-codec/oam.xsd")).unwrap();
    let original = schema
        .types
        .iter()
        .find(|ty| ty.name.local_name == "Big")
        .unwrap()
        .clone();
    let mut last = original.name.clone();
    for name in ["Alias", "AliasChain"] {
        let mut alias = original.clone();
        alias.name.local_name = name.into();
        alias.base_type = None;
        alias.kind = TypeKind::Alias(TypeRef::named(last));
        last = alias.name.clone();
        schema.types.push(alias);
    }
    let mut direct = original.clone();
    direct.name.local_name = "DirectAlias".into();
    direct.base_type = None;
    direct.kind = TypeKind::Alias(TypeRef::primitive(ams_gra_oms_ir::PrimitiveKind::String));
    schema.types.push(direct.clone());
    let unsigned = schema
        .types
        .iter_mut()
        .find(|ty| ty.name.local_name == "Unsigned")
        .unwrap();
    unsigned.constraints.max_inclusive = Some(NumericValue::Integer(i128::from(u64::MAX)));
    let unsigned_name = unsigned.name.clone();
    // Every selected payload is unique except two message identities share the alias chain.
    let mut contract_text =
        fs::read_to_string(root.join("tests/fixtures/python-codec/service.yaml")).unwrap();
    contract_text.push('\n');
    schema.messages[0].payload_type = TypeRef::named(last);
    schema.messages[1].payload_type = schema.messages[0].payload_type.clone();
    for (message, target) in [
        ("DirectReport", direct.name),
        ("UnsignedReport", unsigned_name),
        (
            "EnumReport",
            schema
                .types
                .iter()
                .find(|ty| ty.name.local_name == "State")
                .unwrap()
                .name
                .clone(),
        ),
    ] {
        let mut msg = schema.messages[0].clone();
        msg.name.local_name = message.into();
        msg.payload_type = TypeRef::named(target);
        schema.messages.push(msg);
        contract_text.push_str(&format!("      - id: {}\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {message}\n        topic: scalar\n        timing: {{kind: asynchronous}}\n", message.to_ascii_lowercase()));
    }
    let plan = resolve_service_plan(&parse_yaml(&contract_text).unwrap(), &schema).unwrap();
    for world in [
        GenerationWorld::ClosedSchemaSet,
        GenerationWorld::OpenExtensions,
    ] {
        let projection = project_service_generation_schema(&plan, &schema, world).unwrap();
        let model = build_service_api_model(&plan, projection.schema(), world).unwrap();
        let files = generate_service_codec(&model, projection.schema()).unwrap();
        assert_eq!(
            files,
            generate_service_codec(&model, projection.schema()).unwrap()
        );
        assert_eq!(
            files[0].contents.matches("def encode_AliasChain(").count(),
            1
        );
        let dest = std::env::temp_dir().join(format!(
            "py004-alias-{}-{}",
            std::process::id(),
            world.label()
        ));
        fs::create_dir(&dest).unwrap();
        for file in PythonBackend
            .generate(projection.schema(), world)
            .unwrap()
            .into_iter()
            .chain(files)
        {
            fs::write(dest.join(file.relative_path), file.contents).unwrap();
        }
        let probe = r#"import service_codec as c, model as m
from unittest import TestCase
t=TestCase()
v=m.AliasChain(m.Alias(m.Big(10**100)))
t.assertEqual(c.encode_AliasChain(v), '1'+'0'*100)
t.assertEqual(c.decode_AliasChain('1'+'0'*100),v)
t.assertIs(type(c.decode_AliasChain('5').value),m.Alias)
t.assertIs(type(c.decode_AliasChain('5').value.value),m.Big)
t.assertEqual(c.encode_DirectAlias(m.DirectAlias('λ')), '"λ"')
t.assertEqual(c.decode_DirectAlias('"λ"'),m.DirectAlias('λ'))
t.assertEqual(c.encode_Unsigned(m.Unsigned(2**64-1)), '18446744073709551615')
t.assertEqual(c.decode_Unsigned('18446744073709551615'),m.Unsigned(2**64-1))
t.assertEqual(c.decode_State('"3-D"'),m.State.schema_3_D)
t.assertEqual(c.encode_State(m.State.schema_class),'"class"')
for text in ['true','"5"','null','1.0']:
    with t.assertRaises(c.CodecError): c.decode_AliasChain(text)
for text in ['-1','18446744073709551616','true']:
    with t.assertRaises(c.CodecError): c.decode_Unsigned(text)
with t.assertRaises(c.CodecError): c.encode_AliasChain(m.Alias(m.Big(1)))
"#;
        for optimized in [false, true] {
            let mut cmd =
                Command::new(std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into()));
            if optimized {
                cmd.arg("-O");
            }
            let result = cmd.args(["-c", probe]).current_dir(&dest).output().unwrap();
            assert!(result.status.success(), "{result:?}");
        }
        fs::remove_dir_all(&dest).unwrap();
    }
}
