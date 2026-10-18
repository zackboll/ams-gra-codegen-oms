//! Guarded OMS payload-body codecs over the existing public Ada model API.
use crate::{ada_identifier, ada_type, error, package_name, record_field_name};
use ams_gra_oms_codegen_core::{
    ADA_SEQUENCE_APPEND, ADA_SEQUENCE_ELEMENT, ADA_SEQUENCE_LENGTH, ADA_SEQUENCE_TO_SEQUENCE,
    AdaCodecRuntime, BackendLanguage, CodegenError, GeneratedFile, GenerationWorld,
    ServiceApiModel, TypeEmission, analyze_service_codec_with_runtime,
    effective_choice_alternatives, effective_record_fields, emissions_emit_direct_duration,
    generated_choice_alternative_name, generated_enum_variant_name, oms_json_member_name,
    plan_type_emissions, string_profile,
};
use ams_gra_oms_ir::{OccurrenceShape, PrimitiveKind, SchemaIr, TypeKind, TypeRefTarget};
use std::{collections::BTreeSet, fmt::Write as _, path::PathBuf};

/// Select decoder operations at emission time. Schema names and completed
/// source never pass through textual substitutions. Encoders always use DOM.
#[derive(Clone, Copy)]
struct Decoder {
    value: &'static str,
    array: &'static str,
    get: &'static str,
    length: &'static str,
    map: &'static str,
    reader: &'static str,
    errors: &'static str,
    runtime_check: &'static str,
}

impl Decoder {
    fn new(occurrences: bool) -> Self {
        if occurrences {
            Self {
                value: "Ingress.Value",
                array: "Ingress.Value_Array",
                get: "Ingress.Get",
                length: "Ingress.Length",
                map: "Ingress.Map_JSON_Object",
                reader: "Ingress.Read",
                errors: "Ingress.Validation_Error | AMS_GRA_OMS_JSON_Float32.Validation_Error",
                runtime_check: "      AMS_GRA_OMS_JSON_Float32.Require_Platform;\n",
            }
        } else {
            Self {
                value: "JSON_Value",
                array: "JSON_Array",
                get: "Get",
                length: "Length",
                map: "Map_JSON_Object",
                reader: "AMS_GRA_OMS_JSON_Guard.Read",
                errors: "AMS_GRA_OMS_JSON_Guard.Validation_Error",
                runtime_check: "",
            }
        }
    }
}

fn literal(text: &str) -> String {
    let parts: Vec<_> = text
        .bytes()
        .map(|byte| format!("Character'Val ({byte})"))
        .collect();
    if parts.is_empty() {
        "\"\"".into()
    } else {
        format!(
            "String'({})",
            parts
                .iter()
                .enumerate()
                .map(|(i, part)| format!("{} => {part}", i + 1))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

// Emitted only when an operation in the projected output uses Binary.
fn hex_binary_helpers(decoder: Decoder) -> String {
    let mut helpers = String::from(
        r#"
   function Encode_Hex_Binary (Item : Model.Binary_Vectors.Vector) return JSON_Value is
      use type Ada.Containers.Count_Type;
      Hex_Digits : constant String := "0123456789ABCDEF";
   begin
      if Item.Length > Ada.Containers.Count_Type (Natural'Last / 2) then
         raise Codec_Error with "hexBinary too large";
      end if;
      declare
         Text : String (1 .. Natural (Item.Length) * 2);
         Offset : Natural := 0;
      begin
         for Octet of Item loop
            Text (Offset + 1) := Hex_Digits (Natural (Octet) / 16 + 1);
            Text (Offset + 2) := Hex_Digits (Natural (Octet) mod 16 + 1);
            Offset := Offset + 2;
         end loop;
         return Create (Text);
      end;
   end Encode_Hex_Binary;

"#,
    );
    writeln!(
        helpers,
        "   function Decode_Hex_Binary (Value : {}) return Model.Binary_Vectors.Vector is",
        decoder.value
    )
    .unwrap();
    helpers.push_str(
        r#"      Text : constant String := Scalar_String (Value);
      First : Natural := Text'First;
      Last : Natural := Text'Last;
      Result : Model.Binary_Vectors.Vector;
      function XML_Space (C : Character) return Boolean is
        (C = ' ' or else C = Character'Val (9) or else
         C = Character'Val (10) or else C = Character'Val (13));
      function Nibble (C : Character) return Natural is
      begin
         case C is
            when '0' .. '9' => return Character'Pos (C) - Character'Pos ('0');
            when 'A' .. 'F' => return Character'Pos (C) - Character'Pos ('A') + 10;
            when 'a' .. 'f' => return Character'Pos (C) - Character'Pos ('a') + 10;
            when others => raise Codec_Error with "invalid hexBinary digit";
         end case;
      end Nibble;
   begin
      -- Trim XML whitespace. Any internal whitespace survives collapse and
      -- is rejected by Nibble, without an intermediate lexical allocation.
      while First <= Last and then XML_Space (Text (Last)) loop
         Last := Last - 1;
      end loop;
      while First <= Last and then XML_Space (Text (First)) loop
         First := First + 1;
      end loop;
      declare
         Count : constant Natural := (if First > Last then 0 else Last - First + 1);
      begin
         if Count mod 2 /= 0 then
            raise Codec_Error with "odd hexBinary digit count";
         end if;
         Result.Reserve_Capacity (Ada.Containers.Count_Type (Count / 2));
         for Pair in 1 .. Count / 2 loop
            declare
               Offset : constant Natural := First + (Pair - 1) * 2;
            begin
               Result.Append (Interfaces.Unsigned_8
                 (Nibble (Text (Offset)) * 16 + Nibble (Text (Offset + 1))));
            end;
         end loop;
      end;
      return Result;
   end Decode_Hex_Binary;
"#,
    );
    helpers
}

/// Render both artifacts in memory after the shared capability preflight.
///
/// # Errors
/// Returns the first codec boundary or model naming error.
pub fn generate_service_codec(
    model: &ServiceApiModel,
    schema: &SchemaIr,
    world: GenerationWorld,
) -> Result<Vec<GeneratedFile>, CodegenError> {
    generate_service_codec_with_runtime(model, schema, world, AdaCodecRuntime::Portable)
}

/// Render against an explicit deployment runtime; portable calls stay unchanged.
///
/// # Errors
/// Returns the first codec boundary or model naming error.
pub fn generate_service_codec_with_runtime(
    model: &ServiceApiModel,
    schema: &SchemaIr,
    world: GenerationWorld,
    runtime: AdaCodecRuntime,
) -> Result<Vec<GeneratedFile>, CodegenError> {
    crate::validate_schema(schema, world)?;
    ams_gra_oms_codegen_core::validate_service_api_names(model, BackendLanguage::Ada)
        .map_err(|e| error(e.to_string()))?;
    ams_gra_oms_codegen_core::validate_service_api_artifacts(model, schema, BackendLanguage::Ada)
        .map_err(|e| error(e.to_string()))?;
    let readiness =
        analyze_service_codec_with_runtime(model, schema, BackendLanguage::Ada, world, runtime)
            .map_err(|e| error(e.to_string()))?;
    if let Some(blocker) = readiness.blocker {
        return Err(error(blocker.to_string()));
    }
    if !model.has_oms_exchanges() {
        return Ok(Vec::new());
    }
    let package = package_name(schema)?;
    let float32 = schema.types.iter().any(|d| match &d.kind {
        TypeKind::Primitive(PrimitiveKind::Float32) => true,
        TypeKind::Record { fields, .. } => fields.iter().any(|f| {
            matches!(
                f.type_ref.target,
                TypeRefTarget::Primitive(PrimitiveKind::Float32)
            )
        }),
        _ => false,
    });
    let decoder = Decoder::new(float32);
    let Decoder {
        value,
        get,
        map,
        reader,
        errors,
        runtime_check,
        ..
    } = decoder;
    let payloads: BTreeSet<_> = model
        .functions()
        .iter()
        .flat_map(|f| f.exchanges())
        .filter_map(|e| e.oms_binding())
        .map(|b| b.payload_name())
        .collect();
    let mut spec = format!(
        "with {package};\npackage Service_Codec is\n   --  Invalid payloads raise Codec_Error; internal defects are not hidden.\n   --  Duplicate keys retain the provider's last-value-wins semantics.\n   Codec_Error : exception;\n"
    );
    let mut body = format!(
        "with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;\nwith Interfaces;\nwith GNATCOLL.JSON; use GNATCOLL.JSON;\nwith AMS_GRA_OMS_JSON_Guard;\n{}package body Service_Codec is\n   package Model renames {package};\n",
        if float32 {
            "with AMS_GRA_OMS_JSON_Float32;\nwith AMS_GRA_OMS_JSON_Occurrences;\n"
        } else {
            ""
        }
    );
    if float32 {
        body.push_str("   package Ingress renames AMS_GRA_OMS_JSON_Occurrences;\n   function Scalar_Float32 (Value : Ingress.Value) return Interfaces.IEEE_Float_32 is\n   begin\n      return Ingress.Floating (Value);\n   exception\n      when Ingress.Validation_Error | AMS_GRA_OMS_JSON_Float32.Validation_Error => raise Codec_Error with \"invalid Float32\";\n   end Scalar_Float32;\n");
    }
    body.push_str(&format!(
        r#"
   procedure Require_Kind (Value : {value}; Expected : JSON_Value_Type) is
   begin
      if Value.Kind /= Expected then
         raise Codec_Error with "wrong JSON kind";
      end if;
   end Require_Kind;
   function Scalar_Boolean (Value : {value}) return Boolean is
   begin
      Require_Kind (Value, JSON_Boolean_Type);
      return {get} (Value);
   end Scalar_Boolean;
   function Scalar_String (Value : {value}) return String is
   begin
      Require_Kind (Value, JSON_String_Type);
      return {get} (Value);
   end Scalar_String;
   function Scalar_Integer (Value : {value}) return Long_Long_Integer is
   begin
      Require_Kind (Value, JSON_Int_Type);
      return {get} (Value);
   end Scalar_Integer;
   function Member (Value : {value}; Key : String) return {value} is
   begin
      if not Value.Has_Field (Key) then
         raise Codec_Error with "missing required member: " & Key;
      end if;
      return Value.Get (Key);
   end Member;
"#,
    ));
    let emissions = plan_type_emissions(schema, world)?;
    if emissions_emit_direct_duration(schema, &emissions, world) {
        body.push_str(&format!(
            "   function Scalar_Duration (Value : {value}) return Model.XML_Schema_Duration is\n      Text : constant String := Scalar_String (Value);\n   begin\n      begin\n         return Model.Create (Text);\n      exception\n         when Constraint_Error => raise Codec_Error with \"invalid Duration\";\n      end;\n   end Scalar_Duration;\n",
        ));
    }
    if ams_gra_oms_codegen_core::emissions_emit_direct_date_time(schema, &emissions, world) {
        body.push_str(&format!("   function Scalar_Date_Time (Value : {value}) return Model.XML_Schema_Date_Time is\n      Text : constant String := Scalar_String (Value);\n   begin\n      return Model.Create (Text);\n   exception\n      when Constraint_Error => raise Codec_Error with \"invalid direct DateTime\";\n   end Scalar_Date_Time;\n"));
    }
    for emission in &emissions {
        if let TypeEmission::Declaration(d) = emission {
            if !emission.emits_own_top_level_name() {
                continue;
            }
            let n = ada_identifier(&d.name.local_name)?;
            writeln!(body, "   function To_JSON_{n} (Item : Model.{n}) return JSON_Value;\n   function From_JSON_{n} (Value : {value}) return Model.{n};").unwrap();
        }
    }
    for emission in &emissions {
        let TypeEmission::Declaration(d) = emission else {
            continue;
        };
        if !emission.emits_own_top_level_name() {
            continue;
        }
        let n = ada_identifier(&d.name.local_name)?;
        let (enc, dec) = match &d.kind {
            TypeKind::Primitive(k) => {
                let checked = *k == PrimitiveKind::String
                    && string_profile(*k, &d.constraints)
                        .map_err(|e| error(e.to_string()))?
                        .is_some();
                let enc: String = match k {
                    PrimitiveKind::Binary => {
                        if ams_gra_oms_codegen_core::binary_length_domain(*k, &d.constraints)
                            .map_err(|e| error(e.to_string()))?
                            .is_some()
                        {
                            "Encode_Hex_Binary (Model.Value (Item))".into()
                        } else {
                            "Encode_Hex_Binary (Item.Value)".into()
                        }
                    }
                    PrimitiveKind::Float32 => {
                        let value = if ams_gra_oms_codegen_core::floating_domain(*k, &d.constraints)
                            .map_err(|e| error(e.to_string()))?
                            .is_some()
                        {
                            "Model.Value (Item)".to_owned()
                        } else {
                            "Interfaces.IEEE_Float_32 (Item)".to_owned()
                        };
                        format!("AMS_GRA_OMS_JSON_Float32.Encode ({value})")
                    }
                    PrimitiveKind::Duration => "Create (Model.Value (Item))".into(),
                    PrimitiveKind::DateTime => "Create (Model.Value (Item))".into(),
                    PrimitiveKind::String if checked => "Create (Model.Value (Item))".into(),
                    PrimitiveKind::String => "Create (To_String (Unbounded_String (Item)))".into(),
                    PrimitiveKind::Boolean => "Create (Boolean (Item))".into(),
                    _ => "Create (Long_Long_Integer (Item))".into(),
                };
                let dec = match k {
                    PrimitiveKind::Binary => {
                        if ams_gra_oms_codegen_core::binary_length_domain(*k, &d.constraints)
                            .map_err(|e| error(e.to_string()))?
                            .is_some()
                        {
                            String::from(
                                "      declare\n         Octets : constant Model.Binary_Vectors.Vector := Decode_Hex_Binary (Value);\n      begin\n         begin\n            return Model.Create (Octets);\n         exception\n            when Constraint_Error => raise Codec_Error with \"invalid Binary length\";\n         end;\n      end;",
                            )
                        } else {
                            format!("      return Model.{n}'(Value => Decode_Hex_Binary (Value));")
                        }
                    }
                    PrimitiveKind::Float32 => {
                        if ams_gra_oms_codegen_core::floating_domain(*k, &d.constraints)
                            .map_err(|e| error(e.to_string()))?
                            .is_some()
                        {
                            "      declare\n         Number : constant Interfaces.IEEE_Float_32 := Scalar_Float32 (Value);\n      begin\n         return Model.Create (Number);\n      exception\n         when Ada.Assertions.Assertion_Error => raise Codec_Error with \"invalid checked Float32\";\n      end;".to_owned()
                        } else {
                            format!("      return Model.{n} (Scalar_Float32 (Value));")
                        }
                    }
                    PrimitiveKind::Duration => String::from(
                        "      declare\n         Text : constant String := Scalar_String (Value);\n      begin\n         begin\n            return Model.Create (Text);\n         exception\n            when Constraint_Error => raise Codec_Error with \"invalid Duration\";\n         end;\n      end;",
                    ),
                    PrimitiveKind::DateTime => String::from(
                        "      declare\n         Text : constant String := Scalar_String (Value);\n      begin\n         return Model.Create (Text);\n      exception\n         when Constraint_Error => raise Codec_Error with \"invalid named DateTime\";\n      end;",
                    ),
                    PrimitiveKind::String if checked => String::from(
                        "      declare\n         Text : constant String := Scalar_String (Value);\n      begin\n         return Model.Create (Text);\n      exception\n         when Constraint_Error => raise Codec_Error with \"invalid checked String\";\n      end;",
                    ),
                    PrimitiveKind::String => format!(
                        "      return Model.{n} (To_Unbounded_String (Scalar_String (Value)));"
                    ),
                    PrimitiveKind::Boolean => {
                        format!("      return Model.{n} (Scalar_Boolean (Value));")
                    }
                    _ => format!(
                        "      declare\n         Number : constant Long_Long_Integer := Scalar_Integer (Value);\n      begin\n         return Model.{n} (Number);\n      exception\n         when Constraint_Error => raise Codec_Error with \"integer outside model range\";\n      end;"
                    ),
                };
                (format!("      return {enc};"), dec)
            }
            TypeKind::Enumeration { variants } => {
                let mut enc = "      case Item is\n".to_owned();
                let mut dec = "      declare\n         Text : constant String := Scalar_String (Value);\n      begin\n".to_owned();
                for v in variants {
                    let id = generated_enum_variant_name(BackendLanguage::Ada, &v.wire_value)
                        .ok_or_else(|| error("invalid enum"))?;
                    writeln!(
                        enc,
                        "         when Model.{id} => return Create ({});",
                        literal(&v.wire_value)
                    )
                    .unwrap();
                    writeln!(
                        dec,
                        "         if Text = {} then return Model.{id}; end if;",
                        literal(&v.wire_value)
                    )
                    .unwrap();
                }
                enc.push_str("      end case;");
                dec.push_str(
                    "         raise Codec_Error with \"invalid enumeration\";\n      end;",
                );
                (enc, dec)
            }
            TypeKind::Record { .. } => {
                let fields = effective_record_fields(schema, &d.name)
                    .map_err(|e| error(format!("{e:?}")))?;
                let mut enc = "      declare\n         Result : constant JSON_Value := Create_Object;\n      begin\n".to_owned();
                let mut dec = format!(
                    "      declare\n         procedure Check_Member (Name : String; Child : {value}) is\n            pragma Unreferenced (Child);\n         begin\n"
                );
                let keys: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        literal(&oms_json_member_name(
                            f.wire_name().namespace_uri.unwrap(),
                            &f.name,
                        ))
                    })
                    .collect();
                let allowed = if keys.is_empty() {
                    "False".into()
                } else {
                    keys.iter()
                        .map(|key| format!("Name = {key}"))
                        .collect::<Vec<_>>()
                        .join(" or else ")
                };
                writeln!(dec, "            if not ({allowed}) then raise Codec_Error with \"unknown member: \" & Name; end if;\n         end Check_Member;\n      begin\n         Require_Kind (Value, JSON_Object_Type);\n         {map} (Value, Check_Member'Access);").unwrap();
                let mut aggregates = Vec::new();
                for (f, key) in fields.iter().zip(keys) {
                    let field = record_field_name(&f.name)?;
                    if matches!(f.cardinality.shape(), OccurrenceShape::Bounded { max, .. } if max > 1)
                        || matches!(f.cardinality.shape(), OccurrenceShape::Unbounded { .. })
                    {
                        let (statements, helper) = repeated_member(f, &n, &field, &key, decoder)?;
                        enc.push_str(&statements);
                        let at = dec.find("         procedure Check_Member").unwrap();
                        dec.insert_str(at, &helper);
                        aggregates.push(format!("{field} => Sequence_{field}"));
                        continue;
                    }
                    let optional = f.cardinality.shape() == OccurrenceShape::OptionalOne;
                    let item = format!("Item.{field}{}", if optional { ".Value" } else { "" });
                    let input = format!("Member (Value, {key})");
                    let (encode, decode) = match &f.type_ref.target {
                        TypeRefTarget::Named(q) => (
                            format!("To_JSON_{} ({item})", q.local_name),
                            format!("From_JSON_{} ({input})", q.local_name),
                        ),
                        TypeRefTarget::Primitive(PrimitiveKind::String) => (
                            format!("Create (To_String ({item}))"),
                            format!("To_Unbounded_String (Scalar_String ({input}))"),
                        ),
                        TypeRefTarget::Primitive(PrimitiveKind::Boolean) => (
                            format!("Create ({item})"),
                            format!("Scalar_Boolean ({input})"),
                        ),
                        TypeRefTarget::Primitive(PrimitiveKind::Float32) => (
                            format!("AMS_GRA_OMS_JSON_Float32.Encode ({item})"),
                            format!("Scalar_Float32 ({input})"),
                        ),
                        TypeRefTarget::Primitive(PrimitiveKind::Duration) => (
                            format!("Create (Model.Value ({item}))"),
                            format!("Scalar_Duration ({input})"),
                        ),
                        TypeRefTarget::Primitive(PrimitiveKind::Binary) => (
                            format!("Encode_Hex_Binary ({item})"),
                            format!("Decode_Hex_Binary ({input})"),
                        ),
                        TypeRefTarget::Primitive(PrimitiveKind::DateTime) => (
                            format!("Create (Model.Value ({item}))"),
                            format!("Scalar_Date_Time ({input})"),
                        ),
                        TypeRefTarget::Primitive(_) => {
                            let ty = ada_type(&f.type_ref)?;
                            // Field-local numeric subtypes are checked before conversion.
                            let helper = format!("Number_{field}");
                            let mut check = String::new();
                            if let Some(domain) =
                                ams_gra_oms_codegen_core::inclusive_integral_domain(
                                    match f.type_ref.target {
                                        TypeRefTarget::Primitive(k) => k,
                                        _ => unreachable!(),
                                    },
                                    &f.constraints,
                                )
                                .map_err(error)?
                            {
                                let (min, max) = match domain { ams_gra_oms_codegen_core::InclusiveIntegralDomain::Signed { min, max } => (min.to_string(), max.to_string()), ams_gra_oms_codegen_core::InclusiveIntegralDomain::Unsigned { min, max } => (min.to_string(), max.to_string()) };
                                check = format!(
                                    "if Number < {min} or else Number > {max} then raise Codec_Error with \"integer range\"; end if;"
                                );
                            }
                            // Local helper goes into this Record's declarative part.
                            let at = dec.find("         procedure Check_Member").unwrap();
                            dec.insert_str(at, &format!("         function {helper} return {ty} is\n            Number : constant Long_Long_Integer := Scalar_Integer ({input});\n         begin\n            {check}\n            return {ty} (Number);\n         end {helper};\n"));
                            (format!("Create (Long_Long_Integer ({item}))"), helper)
                        }
                    };
                    if optional {
                        writeln!(enc, "         if Item.{field}.Is_Present then Result.Set_Field ({key}, {encode}); end if;").unwrap();
                    } else {
                        writeln!(enc, "         Result.Set_Field ({key}, {encode});").unwrap();
                    }
                    aggregates.push(format!("{field} => {}", if optional { format!("(if Value.Has_Field ({key}) then (Is_Present => True, Value => {decode}) else (Is_Present => False))") } else { decode }));
                }
                enc.push_str("         return Result;\n      end;");
                writeln!(
                    dec,
                    "         return Model.{n}'({});\n      end;",
                    if aggregates.is_empty() {
                        "null record".into()
                    } else {
                        aggregates.join(",\n            ")
                    }
                )
                .unwrap();
                (enc, dec)
            }
            TypeKind::Choice { .. } => {
                let alternatives = effective_choice_alternatives(schema, &d.name)
                    .map_err(|e| error(e.to_string()))?;
                let mut enc = "      declare\n         Result : constant JSON_Value := Create_Object;\n      begin\n         case Item.Kind is\n".to_owned();
                let mut helpers = String::new();
                let mut branches = String::new();
                for (index, f) in alternatives.iter().enumerate() {
                    let field = generated_choice_alternative_name(BackendLanguage::Ada, &f.name)
                        .ok_or_else(|| error("invalid Choice alternative identifier"))?;
                    let key = literal(&oms_json_member_name(
                        f.wire_name().namespace_uri.unwrap(),
                        &f.name,
                    ));
                    let item = format!("Item.{field}");
                    let input = format!("Member (Value, {key})");
                    let (encode, decode) = match &f.type_ref.target {
                        TypeRefTarget::Named(q) => {
                            let name = ada_identifier(&q.local_name)?;
                            (
                                format!("To_JSON_{name} ({item})"),
                                format!("From_JSON_{name} ({input})"),
                            )
                        }
                        TypeRefTarget::Primitive(PrimitiveKind::String) => (
                            format!("Create (To_String ({item}))"),
                            format!("To_Unbounded_String (Scalar_String ({input}))"),
                        ),
                        TypeRefTarget::Primitive(PrimitiveKind::Boolean) => (
                            format!("Create ({item})"),
                            format!("Scalar_Boolean ({input})"),
                        ),
                        TypeRefTarget::Primitive(kind) => {
                            let ty = ada_type(&f.type_ref)?;
                            let helper = format!("Number_{index}");
                            let mut check = String::new();
                            if let Some(domain) =
                                ams_gra_oms_codegen_core::inclusive_integral_domain(
                                    *kind,
                                    &f.constraints,
                                )
                                .map_err(error)?
                            {
                                let (min, max) = match domain {
                                    ams_gra_oms_codegen_core::InclusiveIntegralDomain::Signed { min, max } => (min.to_string(), max.to_string()),
                                    ams_gra_oms_codegen_core::InclusiveIntegralDomain::Unsigned { min, max } => (min.to_string(), max.to_string()),
                                };
                                check = format!(
                                    "if Number < {min} or else Number > {max} then raise Codec_Error with \"integer range\"; end if;"
                                );
                            }
                            writeln!(helpers, "         function {helper} return {ty} is\n            Number : constant Long_Long_Integer := Scalar_Integer ({input});\n         begin\n            {check}\n            return {ty} (Number);\n         end {helper};").unwrap();
                            (format!("Create (Long_Long_Integer ({item}))"), helper)
                        }
                    };
                    writeln!(
                        enc,
                        "            when Model.{field}_Kind => Result.Set_Field ({key}, {encode});"
                    )
                    .unwrap();
                    writeln!(branches, "         if Value.Has_Field ({key}) then\n            return Model.{n}'(Kind => Model.{field}_Kind, {field} => {decode});\n         end if;").unwrap();
                }
                enc.push_str("         end case;\n         return Result;\n      end;");
                let dec = format!(
                    "      declare\n         Count : Natural := 0;\n         procedure Count_Member (Name : String; Child : {value}) is\n            pragma Unreferenced (Name, Child);\n         begin\n            Count := Count + 1;\n         end Count_Member;\n{helpers}      begin\n         Require_Kind (Value, JSON_Object_Type);\n         {map} (Value, Count_Member'Access);\n         if Count /= 1 then raise Codec_Error with \"Choice requires exactly one member\"; end if;\n{branches}         raise Codec_Error with \"unknown Choice alternative\";\n      end;"
                );
                (enc, dec)
            }
            _ => return Err(error("unsupported codec passed readiness")),
        };
        writeln!(body, "   function To_JSON_{n} (Item : Model.{n}) return JSON_Value is\n   begin\n{enc}\n   end To_JSON_{n};\n   function From_JSON_{n} (Value : {value}) return Model.{n} is\n   begin\n{dec}\n   end From_JSON_{n};").unwrap();
    }
    for q in payloads {
        let n = ada_identifier(&q.local_name)?;
        writeln!(spec, "   function Encode_{n} (Item : {package}.{n}) return String;\n   function Decode_{n} (Text : String) return {package}.{n};").unwrap();
        writeln!(body, "   function Encode_{n} (Item : {package}.{n}) return String is\n   begin\n{runtime_check}      return AMS_GRA_OMS_JSON_Guard.Write (To_JSON_{n} (Item));\n   exception\n      when AMS_GRA_OMS_JSON_Guard.Validation_Error => raise Codec_Error with \"invalid JSON output\";\n   end Encode_{n};\n   function Decode_{n} (Text : String) return {package}.{n} is\n   begin\n{runtime_check}      return From_JSON_{n} ({reader} (Text));\n   exception\n      when {errors} => raise Codec_Error with \"invalid JSON document\";\n   end Decode_{n};").unwrap();
    }
    if body.contains("Encode_Hex_Binary (") {
        body.insert_str(0, "with Ada.Containers;\n");
        let at = body.find("   function To_JSON_").unwrap();
        body.insert_str(at, &hex_binary_helpers(decoder));
    }
    spec.push_str("end Service_Codec;\n");
    body.push_str("end Service_Codec;\n");
    if float32 && body.contains("Ada.Assertions.Assertion_Error") {
        body = format!("with Ada.Assertions;\n{body}");
    }
    Ok(vec![
        GeneratedFile {
            relative_path: PathBuf::from("service_codec.ads"),
            contents: spec,
        },
        GeneratedFile {
            relative_path: PathBuf::from("service_codec.adb"),
            contents: body,
        },
    ])
}

/// Repeated members use the model's published owner-specific storage API.
fn repeated_member(
    field: &ams_gra_oms_ir::FieldDecl,
    owner: &str,
    name: &str,
    key: &str,
    decoder: Decoder,
) -> Result<(String, String), CodegenError> {
    let stem = format!("{owner}_{name}");
    let min = field.cardinality.min_occurs;
    let positive_unbounded = field.cardinality.max_occurs.is_none() && min > 0;
    let (encode, decode) = match &field.type_ref.target {
        TypeRefTarget::Named(q) => {
            let ty = ada_identifier(&q.local_name)?;
            (
                format!("To_JSON_{ty} (Element_Item)"),
                format!("From_JSON_{ty} (Child)"),
            )
        }
        TypeRefTarget::Primitive(PrimitiveKind::String) => (
            "Create (To_String (Element_Item))".into(),
            "To_Unbounded_String (Scalar_String (Child))".into(),
        ),
        TypeRefTarget::Primitive(PrimitiveKind::Boolean) => (
            "Create (Element_Item)".into(),
            "Scalar_Boolean (Child)".into(),
        ),
        TypeRefTarget::Primitive(PrimitiveKind::Float32) => (
            "AMS_GRA_OMS_JSON_Float32.Encode (Element_Item)".into(),
            "Scalar_Float32 (Child)".into(),
        ),
        TypeRefTarget::Primitive(PrimitiveKind::DateTime) => (
            "Create (Model.Value (Element_Item))".into(),
            "Scalar_Date_Time (Child)".into(),
        ),
        TypeRefTarget::Primitive(PrimitiveKind::Duration) => (
            "Create (Model.Value (Element_Item))".into(),
            "Scalar_Duration (Child)".into(),
        ),
        TypeRefTarget::Primitive(kind) => {
            let ty = ada_type(&field.type_ref)?;
            let check = match ams_gra_oms_codegen_core::inclusive_integral_domain(
                *kind,
                &field.constraints,
            )
            .map_err(error)?
            {
                Some(ams_gra_oms_codegen_core::InclusiveIntegralDomain::Signed { min, max }) => {
                    format!(
                        "if Number < {min} or else Number > {max} then raise Codec_Error with \"integer range\"; end if;"
                    )
                }
                Some(ams_gra_oms_codegen_core::InclusiveIntegralDomain::Unsigned { min, max }) => {
                    format!(
                        "if Number < {min} or else Number > {max} then raise Codec_Error with \"integer range\"; end if;"
                    )
                }
                None => String::new(),
            };
            let decode = format!(
                "declare\n                  Number : constant Long_Long_Integer := Scalar_Integer (Child);\n               begin\n                  {check}\n                  return {ty} (Number);\n               end;"
            );
            return repeated_member_render(
                field,
                &stem,
                name,
                key,
                positive_unbounded,
                "Create (Long_Long_Integer (Element_Item))",
                &decode,
                true,
                decoder,
            );
        }
    };
    repeated_member_render(
        field,
        &stem,
        name,
        key,
        positive_unbounded,
        &encode,
        &decode,
        false,
        decoder,
    )
}

#[allow(clippy::too_many_arguments)]
fn repeated_member_render(
    field: &ams_gra_oms_ir::FieldDecl,
    stem: &str,
    name: &str,
    key: &str,
    positive_unbounded: bool,
    encode: &str,
    decode: &str,
    decode_statement: bool,
    decoder: Decoder,
) -> Result<(String, String), CodegenError> {
    let Decoder {
        value,
        array,
        get,
        length: json_length,
        ..
    } = decoder;
    let min = field.cardinality.min_occurs;
    let length = ADA_SEQUENCE_LENGTH;
    let element = ADA_SEQUENCE_ELEMENT;
    let append = ADA_SEQUENCE_APPEND;
    let constructor = ADA_SEQUENCE_TO_SEQUENCE;
    let mut enc = format!(
        "         declare\n            JSON_Items : JSON_Array;\n            procedure Add_Item (Element_Item : Model.{stem}_Item) is\n            begin\n               Append (JSON_Items, {encode});\n            end Add_Item;\n         begin\n"
    );
    if positive_unbounded {
        writeln!(enc, "            if Model.{length} (Item.{name}.Additional) > Natural'Last - {min} then raise Codec_Error with \"sequence length overflow\"; end if;\n            for Index in Item.{name}.Required'Range loop\n               Add_Item (Item.{name}.Required (Index));\n            end loop;\n            for Index in 1 .. Model.{length} (Item.{name}.Additional) loop\n               Add_Item (Model.{element} (Item.{name}.Additional, Index));\n            end loop;\n            Result.Set_Field ({key}, Create (JSON_Items));").unwrap();
    } else {
        writeln!(enc, "            for Index in 1 .. Model.{length} (Item.{name}) loop\n               Add_Item (Model.{element} (Item.{name}, Index));\n            end loop;\n            if Model.{length} (Item.{name}) > 0 then Result.Set_Field ({key}, Create (JSON_Items)); end if;").unwrap();
    }
    enc.push_str("         end;\n");
    let value_body = if decode_statement {
        decode.to_owned()
    } else {
        format!("return {decode};")
    };
    let mut dec = format!(
        "         function Sequence_{name} return Model.{stem}_Sequence is\n            JSON_Items : {array};\n            function Decode_Item (Index : Positive) return Model.{stem}_Item is\n               Child : constant {value} := {get} (JSON_Items, Index);\n            begin\n               {value_body}\n            exception\n               when Codec_Error => raise Codec_Error with {} & Index'Image;\n            end Decode_Item;\n         begin\n            if Value.Has_Field ({key}) then\n               Require_Kind (Value.Get ({key}), JSON_Array_Type);\n               JSON_Items := {get} (Value.Get ({key}));\n            end if;\n",
        literal(&format!("{} item", field.name))
    );
    writeln!(
        dec,
        "            if {json_length} (JSON_Items) < {min}{} then raise Codec_Error with {} ; end if;",
        field
            .cardinality
            .max_occurs
            .map_or(String::new(), |max| format!(
                " or else {json_length} (JSON_Items) > {max}"
            )),
        literal(&format!("{} cardinality", field.name))
    )
    .unwrap();
    if field.cardinality.max_occurs.is_some() {
        // The aggregate creates live checked elements directly; no invalid defaults.
        writeln!(dec, "            declare\n               Values : constant Model.{stem}_Values :=\n                 (for Index in 1 .. {json_length} (JSON_Items) => Decode_Item (Index));\n               function Construct return Model.{stem}_Sequence is\n               begin\n                  return Model.{constructor} (Values);\n               exception\n                  when Constraint_Error => raise Codec_Error with {} ;\n               end Construct;\n            begin\n               return Construct;\n            end;", literal(&format!("{} cardinality", field.name))).unwrap();
    } else if positive_unbounded {
        writeln!(dec, "            declare\n               Required : constant Model.{stem}_Required_Array :=\n                 (for Index in 1 .. {min} => Decode_Item (Index));\n               Additional : Model.{stem}_Additional;\n            begin\n               for Index in {} .. {json_length} (JSON_Items) loop\n                  Model.{append} (Additional, Decode_Item (Index));\n               end loop;\n               return (Required => Required, Additional => Additional);\n            end;", min + 1).unwrap();
    } else {
        writeln!(dec, "            return Result : Model.{stem}_Sequence do\n               for Index in 1 .. {json_length} (JSON_Items) loop\n                  Model.{append} (Result, Decode_Item (Index));\n               end loop;\n            end return;").unwrap();
    }
    writeln!(dec, "         end Sequence_{name};\n").unwrap();
    Ok((enc, dec))
}
