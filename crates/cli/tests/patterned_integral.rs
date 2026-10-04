//! Task 063 shared named capability and fail-closed direct-field controls.
use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, CoverageAnalysis, GenerationWorld, PatternedIntegralProfile,
    analyze_service_readiness, inclusive_integral_domain, patterned_integral_profile,
    project_service_generation_schema, resolve_service_plan,
};
use ams_gra_oms_ir::{
    ConstraintSet, NumericValue, PatternExpression, PrimitiveKind, TypeKind, TypeRef,
    WhiteSpacePolicy,
};
use std::path::Path;

const CLOSED: GenerationWorld = GenerationWorld::ClosedSchemaSet;

fn fixture() -> (
    ams_gra_oms_ir::SchemaIr,
    ams_gra_oms_service_contract::Contract,
) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/integral/patterned.xsd");
    (
        ams_gra_oms_xsd_frontend::load_schema_document(&path).unwrap(),
        ams_gra_oms_service_contract::parse_yaml(
            &std::fs::read_to_string(path.with_extension("yaml")).unwrap(),
        )
        .unwrap(),
    )
}

fn backend(language: BackendLanguage) -> Box<dyn Backend> {
    match language {
        BackendLanguage::Ada => Box::new(ams_gra_oms_backend_ada::AdaBackend),
        BackendLanguage::Rust => Box::new(ams_gra_oms_backend_rust::RustBackend),
        BackendLanguage::Cpp => Box::new(ams_gra_oms_backend_cpp::CppBackend),
    }
}

#[test]
fn task063_exact_profile_and_fail_closed_neighbors() {
    let (schema, contract) = fixture();
    let index = schema
        .types
        .iter()
        .position(|d| d.name.local_name == "Serial")
        .unwrap();
    let exact = schema.types[index].constraints.clone();
    let mut cases = vec![(PrimitiveKind::SignedInteger, exact.clone())];
    for (min, max) in [(0, 999), (1, 1000)] {
        cases.push((
            PrimitiveKind::SignedInteger,
            ConstraintSet {
                min_inclusive: Some(NumericValue::Integer(min)),
                max_inclusive: Some(NumericValue::Integer(max)),
                ..exact.clone()
            },
        ));
    }
    for expression in ["[0-9]{1,4}", "[1-9][0-9]{0,2}", "[0-9]+"] {
        let mut c = exact.clone();
        c.lexical.pattern_groups[0].alternatives[0].expression = expression.into();
        cases.push((PrimitiveKind::SignedInteger, c));
    }
    let mut c = exact.clone();
    c.lexical.pattern_groups[0]
        .alternatives
        .push(PatternExpression::xml_schema("[0-9]{1,3}"));
    cases.push((PrimitiveKind::SignedInteger, c));
    let mut c = exact.clone();
    c.lexical
        .pattern_groups
        .push(c.lexical.pattern_groups[0].clone());
    cases.push((PrimitiveKind::SignedInteger, c));
    for policy in [
        WhiteSpacePolicy::Collapse,
        WhiteSpacePolicy::Replace,
        WhiteSpacePolicy::Preserve,
    ] {
        let mut c = exact.clone();
        c.lexical.white_space = Some(policy);
        cases.push((PrimitiveKind::SignedInteger, c));
    }
    for c in [
        ConstraintSet {
            min_exclusive: Some(NumericValue::Integer(0)),
            ..exact.clone()
        },
        ConstraintSet {
            max_exclusive: Some(NumericValue::Integer(1000)),
            ..exact.clone()
        },
        ConstraintSet {
            length: Some(3),
            ..exact.clone()
        },
        ConstraintSet {
            min_length: Some(1),
            ..exact.clone()
        },
        ConstraintSet {
            max_length: Some(3),
            ..exact.clone()
        },
    ] {
        cases.push((PrimitiveKind::SignedInteger, c));
    }
    cases.push((PrimitiveKind::UnsignedInteger, exact));
    for (i, (kind, constraints)) in cases.into_iter().enumerate() {
        let supported = i == 0;
        assert_eq!(
            patterned_integral_profile(kind, &constraints),
            supported.then_some(PatternedIntegralProfile::DecimalDigits1To3Range1To999)
        );
        assert!(inclusive_integral_domain(kind, &constraints).is_err());
        let mut changed = schema.clone();
        changed.types[index].kind = TypeKind::Primitive(kind);
        changed.types[index].constraints = constraints;
        // Retaining the real name cannot rescue a changed profile.
        if !supported {
            let old = changed.types[index].name.clone();
            changed.types[index].name.local_name = "USMTF_SerialNumberOfQualifierType".into();
            let new = changed.types[index].name.clone();
            for d in &mut changed.types {
                let fields = match &mut d.kind {
                    TypeKind::Record { fields }
                    | TypeKind::Choice {
                        alternatives: fields,
                    } => fields,
                    _ => continue,
                };
                for f in fields {
                    if f.type_ref == TypeRef::named(old.clone()) {
                        f.type_ref = TypeRef::named(new.clone());
                    }
                }
            }
        }
        if changed.validate().is_err() {
            assert!(!supported);
            for lang in BackendLanguage::ALL {
                assert!(backend(lang).generate(&changed, CLOSED).is_err());
            }
            continue;
        }
        let plan = resolve_service_plan(&contract, &changed).unwrap();
        let coverage = CoverageAnalysis::new(&changed, CLOSED).unwrap();
        for lang in BackendLanguage::ALL {
            let r = analyze_service_readiness(&plan, &changed, lang, CLOSED).unwrap();
            assert_eq!(r.is_ready(), supported, "case {i} {lang:?}: {r:?}");
            assert_eq!(
                coverage
                    .backend_coverage(lang)
                    .unwrap()
                    .declarations_fully_renderable
                    == changed.types.len(),
                supported
            );
            assert_eq!(backend(lang).generate(&changed, CLOSED).is_ok(), supported);
        }
    }
    println!("TASK063 EXACT AND NEIGHBOR GATES: PASSED");
}

#[test]
fn task063_generated_support_only_readiness_control() {
    let (mut schema, contract) = fixture();
    let mut support = schema
        .types
        .iter()
        .find(|d| d.name.local_name == "Serial")
        .unwrap()
        .clone();
    support.name.local_name = "SupportSerial".into();
    schema.types.push(support.clone());
    let concrete = schema
        .types
        .iter_mut()
        .find(|d| d.name.local_name == "ConcreteValue")
        .unwrap();
    let TypeKind::Record { fields } = &mut concrete.kind else {
        panic!()
    };
    fields[0].type_ref = TypeRef::named(support.name.clone());
    for exact in [true, false] {
        if !exact {
            schema
                .types
                .iter_mut()
                .find(|d| d.name == support.name)
                .unwrap()
                .constraints
                .min_inclusive = Some(NumericValue::Integer(0));
        }
        let plan = resolve_service_plan(&contract, &schema).unwrap();
        let projection = project_service_generation_schema(&plan, &schema, CLOSED).unwrap();
        assert!(
            projection
                .generated_support_type_names()
                .contains(&support.name)
        );
        assert!(!projection.selected_type_names().contains(&support.name));
        for lang in BackendLanguage::ALL {
            let r = analyze_service_readiness(&plan, &schema, lang, CLOSED).unwrap();
            assert_eq!(r.selected_types_renderable, r.selected_types_total);
            assert_eq!(r.is_ready(), exact, "{lang:?}: {r:?}");
            assert_eq!(
                backend(lang).generate(projection.schema(), CLOSED).is_ok(),
                exact
            );
        }
    }
    println!("TASK063 GENERATED SUPPORT GATE: PASSED");
}

#[test]
fn task063_direct_patterned_integer_remains_unsupported() {
    let (schema, contract) = fixture();
    let constraints = schema
        .types
        .iter()
        .find(|d| d.name.local_name == "Serial")
        .unwrap()
        .constraints
        .clone();
    for owner in ["Payload", "Pick"] {
        let mut changed = schema.clone();
        let d = changed
            .types
            .iter_mut()
            .find(|d| d.name.local_name == owner)
            .unwrap();
        let fields = match &mut d.kind {
            TypeKind::Record { fields }
            | TypeKind::Choice {
                alternatives: fields,
            } => fields,
            _ => panic!(),
        };
        fields[0].type_ref = TypeRef::primitive(PrimitiveKind::SignedInteger);
        fields[0].constraints = constraints.clone();
        let plan = resolve_service_plan(&contract, &changed).unwrap();
        for lang in BackendLanguage::ALL {
            assert!(
                !analyze_service_readiness(&plan, &changed, lang, CLOSED)
                    .unwrap()
                    .is_ready()
            );
            assert!(backend(lang).generate(&changed, CLOSED).is_err());
        }
    }
    println!("TASK063 DIRECT FIELD GATE: PASSED");
}

#[test]
fn task063_numeric_rendering_matches_ordinary_bounded_profile() {
    let (schema, _) = fixture();
    let mut ordinary = schema.clone();
    for d in &mut ordinary.types {
        if matches!(d.kind, TypeKind::Primitive(PrimitiveKind::SignedInteger)) {
            d.constraints.lexical = Default::default();
        }
    }
    for lang in BackendLanguage::ALL {
        let exact = backend(lang).generate(&schema, CLOSED).unwrap();
        let control = backend(lang).generate(&ordinary, CLOSED).unwrap();
        assert_eq!(exact, control, "{lang:?}");
    }
    println!("TASK063 NUMERIC RENDERING EQUIVALENCE: PASSED");
}

#[test]
fn task063_generated_numeric_models_compile_and_execute() {
    use std::process::Command;
    fn success(output: std::process::Output) {
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let (schema, _) = fixture();
    for lang in BackendLanguage::ALL {
        let dir =
            std::env::temp_dir().join(format!("task063-compiler-{lang:?}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in backend(lang).generate(&schema, CLOSED).unwrap() {
            std::fs::write(dir.join(f.relative_path), f.contents).unwrap();
        }
        let compile = match lang {
            BackendLanguage::Rust => {
                std::fs::write(
                    dir.join("probe.rs"),
                    r#"
include!("integral.rs");
fn traits<T: Copy + Eq + Ord + std::hash::Hash>() {}
fn main() {
 traits::<Serial>(); traits::<Renamed>();
 for n in [i64::MIN,-1,0,1,9,10,99,100,999,1000,i64::MAX] {
  let ok = (1..=999).contains(&n);
  assert_eq!(Serial::new(n).is_some(),ok);
  assert_eq!(Renamed::new(n).is_some(),ok);
  if ok { assert_eq!(Serial::new(n).unwrap().get(),n); }
 }
 let v=Serial::new(1).unwrap(); let copied=v; assert_eq!(v,copied);
 assert!(v<Serial::new(999).unwrap());
 if std::env::args().len()>1 { assert!(Serial::new(0).is_some()); }
}
"#,
                )
                .unwrap();
                Command::new("rustc")
                    .current_dir(&dir)
                    .args(["--edition=2024", "probe.rs", "-o", "probe"])
                    .output()
                    .unwrap()
            }
            BackendLanguage::Cpp => {
                std::fs::write(dir.join("probe.cpp"), r#"
#include "integral.hpp"
#include <cassert>
#include <limits>
#include <type_traits>
using namespace test::integral;
int main(int argc,char**) {
 static_assert(std::is_copy_constructible_v<Serial>);
 for (std::int64_t n: {std::numeric_limits<std::int64_t>::min(),std::int64_t(-1),std::int64_t(0),std::int64_t(1),std::int64_t(9),std::int64_t(10),std::int64_t(99),std::int64_t(100),std::int64_t(999),std::int64_t(1000),std::numeric_limits<std::int64_t>::max()}) {
  bool ok=n>=1 && n<=999;
  assert(Serial::create(n).has_value()==ok);
  assert(Renamed::create(n).has_value()==ok);
  if(ok) { auto v=Serial::create(n).value(); auto copied=v; assert(copied.value()==n); }
 }
 if(argc>1) { assert(Serial::create(0)); }
}
"#).unwrap();
                Command::new("c++")
                    .current_dir(&dir)
                    .args([
                        "-std=c++17",
                        "-Wall",
                        "-Wextra",
                        "-Werror",
                        "-pedantic-errors",
                        "probe.cpp",
                        "-o",
                        "probe",
                    ])
                    .output()
                    .unwrap()
            }
            BackendLanguage::Ada => {
                std::fs::write(dir.join("probe.adb"), r#"
with Test.Integral; use Test.Integral;
with Ada.Command_Line;
procedure Probe is
 type Values is array (Positive range <>) of Long_Long_Integer;
 Data : constant Values := (Long_Long_Integer'First,-1,0,1,9,10,99,100,999,1000,Long_Long_Integer'Last);
 procedure Check (N : Long_Long_Integer) is
  Expected : constant Boolean := N >= 1 and N <= 999;
 begin
  begin
   declare V : constant Serial := Serial(N); R : constant Renamed := Renamed(N); begin
    if not Expected or else Long_Long_Integer(V)/=N or else Long_Long_Integer(R)/=N then raise Program_Error; end if;
   end;
  exception when Constraint_Error => if Expected then raise Program_Error; end if;
  end;
 end Check;
begin
 for N of Data loop Check(N); end loop;
 if Ada.Command_Line.Argument_Count > 0 then
  declare N : Long_Long_Integer := Long_Long_Integer'Value(Ada.Command_Line.Argument(1)); V : Serial := Serial(N); begin
   if V = 0 then null; end if;
  end;
 end if;
end Probe;
"#).unwrap();
                Command::new("gnatmake")
                    .current_dir(&dir)
                    .args(["-gnat2022", "-gnata", "-gnato", "probe.adb"])
                    .output()
                    .unwrap()
            }
        };
        success(compile);
        success(Command::new(dir.join("probe")).output().unwrap());
        let negative = Command::new(dir.join("probe")).arg("0").output().unwrap();
        assert!(
            !negative.status.success(),
            "planted wrong expectation must fail: {lang:?}"
        );
        println!(
            "{lang:?} planted failure: {}",
            String::from_utf8_lossy(&negative.stderr)
        );
        success(Command::new(dir.join("probe")).output().unwrap());
        println!("TASK063 {lang:?} NUMERIC COMPILER: PASSED");
    }
}
