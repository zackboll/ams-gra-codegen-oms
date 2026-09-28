//! Task 054: reserved Record-field / Choice-alternative identifier remapping,
//! end to end through the real CLI and real host compilers.
//!
//! `member-keywords.xsd` has Record fields and Choice alternatives whose
//! ORDINARY generated host identifiers are target reserved words (Ada
//! `Range`/`Type`, Rust `type`/`self`/`Self`, C++ `operator`/`delete`), plus an
//! inherited, an optional and a repeated unsafe member. Every expectation is
//! derived from the shared `generated_*_name` helpers, never hard-coded per
//! backend, so a test cannot assert an escape the reserved-word table does not
//! justify.

use ams_gra_oms_codegen_core::{
    BackendLanguage, generated_choice_alternative_name, generated_record_field_name,
};
use ams_gra_oms_ir::TypeKind;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LANGUAGES: [(&str, BackendLanguage); 3] = [
    ("ada", BackendLanguage::Ada),
    ("rust", BackendLanguage::Rust),
    ("cpp", BackendLanguage::Cpp),
];

/// Record fields of `KeywordRecord` (effective, base first).
const RECORD_FIELDS: [&str; 7] = [
    "Type", "Range", "Operator", "Delete", "Self", "Ordinary", "Pick",
];
/// Choice alternatives of `KeywordChoice`.
const CHOICE_ALTERNATIVES: [&str; 5] = ["Range", "Self", "Type", "Delete", "Ordinary"];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/service-generate")
        .join(name)
}

fn output(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("ams-gra-oms-task054-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn cli(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ams-gra-codegen-oms"))
        .args(arguments)
        .output()
        .expect("CLI runs")
}

fn service(
    command: &str,
    stem: &str,
    language: &str,
    output: Option<&Path>,
    with_codec: bool,
) -> Output {
    let schema = fixture(&format!("{stem}.xsd"));
    let contract = fixture(&format!("{stem}.yaml"));
    let mut arguments = vec![
        command,
        "--schema",
        schema.to_str().unwrap(),
        "--contract",
        contract.to_str().unwrap(),
        "--language",
        language,
        "--world",
        "closed-schema",
    ];
    let output = output.map(|path| path.to_str().unwrap().to_owned());
    if let Some(output) = &output {
        arguments.extend(["--output", output]);
    }
    if with_codec {
        arguments.push("--with-codec");
    }
    cli(&arguments)
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn field(language: BackendLanguage, source: &str) -> String {
    generated_record_field_name(language, source).expect("legal field")
}

fn alternative(language: BackendLanguage, source: &str) -> String {
    generated_choice_alternative_name(language, source).expect("legal alternative")
}

fn gnat_available() -> bool {
    if Command::new("gnatmake").arg("--version").output().is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("AMS_GRA_REQUIRE_GNAT").is_none(),
        "GNAT required"
    );
    false
}

fn run_ok(command: &mut Command, label: &str) {
    let output = command.output().unwrap_or_else(|e| panic!("{label}: {e}"));
    assert!(output.status.success(), "{label}: {}", text(&output));
}

/// Schema IR keeps the XSD local names and wire names; escaping is purely a
/// backend API concern.
#[test]
fn task054_schema_ir_keeps_source_and_wire_member_names() {
    let schema = load_schema_set(&fixture("member-keywords.xsd")).unwrap();
    let names = |local: &str| -> Vec<(String, String)> {
        let declaration = schema
            .types
            .iter()
            .find(|t| t.name.local_name == local)
            .unwrap();
        let members = match &declaration.kind {
            TypeKind::Record { fields } => fields,
            TypeKind::Choice { alternatives } => alternatives,
            other => panic!("{other:?}"),
        };
        members
            .iter()
            .map(|m| (m.name.clone(), m.wire_name().local_name.to_owned()))
            .collect()
    };
    for (source, wire) in names("KeywordRecord")
        .into_iter()
        .chain(names("KeywordBase"))
        .chain(names("KeywordChoice"))
    {
        assert_eq!(source, wire);
        assert!(!source.starts_with("Field_") && !source.starts_with("field_"));
        assert!(!source.starts_with("Alternative"));
    }
}

/// Coverage / service-check / generation agree: READY in every backend, and
/// the codec is READY too.
#[test]
fn task054_keyword_service_is_ready_and_generates_in_every_backend() {
    for (language, _) in LANGUAGES {
        let checked = service("service-check", "member-keywords", language, None, false);
        assert!(
            checked.status.success() && text(&checked).contains("status: READY"),
            "{language}: {}",
            text(&checked)
        );
        let root = output(&format!("ready-{language}"));
        let generated = service(
            "service-generate",
            "member-keywords",
            language,
            Some(&root),
            false,
        );
        assert!(
            generated.status.success(),
            "{language}: {}",
            text(&generated)
        );
    }
    let codec = service("service-check", "member-keywords", "rust", None, true);
    assert!(
        text(&codec).contains("codec status: READY"),
        "{}",
        text(&codec)
    );
}

/// The Ada probe: assigns/reads every escaped Record component (including the
/// inherited `Type`, the optional `Delete` wrapper and the repeated `Range`
/// sequence) and constructs every Choice alternative, using ONLY spellings
/// taken from the shared helpers.
fn ada_probe() -> String {
    let ada = BackendLanguage::Ada;
    let f = |source: &str| field(ada, source);
    let a = |source: &str| alternative(ada, source);
    let (range_f, type_f, op_f, del_f, self_f, ord_f) = (
        f("Range"),
        f("Type"),
        f("Operator"),
        f("Delete"),
        f("Self"),
        f("Ordinary"),
    );
    let (range_a, self_a, type_a, del_a, ord_a) =
        (a("Range"), a("Self"), a("Type"), a("Delete"), a("Ordinary"));
    format!(
        "with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
 with Programs.Oam; use Programs.Oam;
 procedure Probe is
    Pick_Range : constant KeywordChoice :=
      (Kind => {range_a}_Kind,
       {range_a} => To_Sequence
         (KeywordChoice_{range_a}_Values'(1 => 1, 2 => 2)));
    Pick_Self : constant KeywordChoice := (Kind => {self_a}_Kind, {self_a} => 3);
    Pick_Type : constant KeywordChoice :=
      (Kind => {type_a}_Kind, {type_a} => To_Unbounded_String (\"t\"));
    Pick_Delete : constant KeywordChoice := (Kind => {del_a}_Kind, {del_a} => True);
    Pick_Ordinary : constant KeywordChoice := (Kind => {ord_a}_Kind, {ord_a} => 4);
    --  An explicit aggregate: the default Pick is the 1..3 {range_a}
    --  alternative, whose Task 040 storage deliberately has no default.
    Item : KeywordRecord :=
      ({type_f} => Null_Unbounded_String,
       {range_f} => To_Sequence (KeywordRecord_{range_f}_Values'(1 .. 0 => 0)),
       {op_f} => Null_Unbounded_String,
       {del_f} => (Is_Present => False),
       {self_f} => 0,
       {ord_f} => 0,
       Pick => Pick_Ordinary);
 begin
    Item.{type_f} := To_Unbounded_String (\"inherited\");
    Item.{range_f} := To_Sequence (KeywordRecord_{range_f}_Values'(1 => 10, 2 => 20));
    Item.{op_f} := To_Unbounded_String (\"op\");
    Item.{del_f} := (Is_Present => True, Value => False);
    Item.{self_f} := 5;
    Item.{ord_f} := 6;
    Item.Pick := Pick_Range;
    if To_String (Item.{type_f}) /= \"inherited\" then raise Program_Error; end if;
    if Length (Item.{range_f}) /= 2 or else Element (Item.{range_f}, 2) /= 20 then
       raise Program_Error;
    end if;
    if not Item.{del_f}.Is_Present or else Item.{del_f}.Value then raise Program_Error; end if;
    if Item.{self_f} + Item.{ord_f} /= 11 then raise Program_Error; end if;
    if Item.Pick.Kind /= {range_a}_Kind or else Length (Item.Pick.{range_a}) /= 2 then
       raise Program_Error;
    end if;
    if Pick_Self.{self_a} /= 3 or else To_String (Pick_Type.{type_a}) /= \"t\"
      or else not Pick_Delete.{del_a} or else Pick_Ordinary.{ord_a} /= 4
    then
       raise Program_Error;
    end if;
 end Probe;
"
    )
}

fn compile_and_run_ada(root: &Path, gnatmake: &Path) {
    std::fs::write(root.join("probe.adb"), ada_probe()).unwrap();
    let _ = std::fs::remove_file(root.join("probe"));
    for extension in ["o", "ali"] {
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == extension) {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    run_ok(
        Command::new(gnatmake).current_dir(root).args([
            "-q",
            "-gnat2012",
            "-gnatwa",
            "-gnata",
            "probe.adb",
        ]),
        &format!("{} probe", gnatmake.display()),
    );
    run_ok(&mut Command::new(root.join("probe")), "Ada probe run");
}

/// Ada: escaped components, Choice discriminant literals, and helper stems
/// all come from the final identifier; the probe compiles and runs under the
/// local GNAT (14) and, when present, the GNAT 13 toolchain named by
/// `AMS_GRA_GNAT13_GNATMAKE`.
#[test]
fn task054_ada_escaped_members_compile_and_run() {
    let root = output("ada");
    let generated = service(
        "service-generate",
        "member-keywords",
        "ada",
        Some(&root),
        false,
    );
    assert!(generated.status.success(), "{}", text(&generated));
    let spec = std::fs::read_to_string(root.join("programs-oam.ads")).unwrap();
    let ada = BackendLanguage::Ada;
    // Only candidates the Ada table actually reserves are escaped.
    assert_eq!(field(ada, "Range"), "Field_Range");
    assert_eq!(field(ada, "Type"), "Field_Type");
    assert_eq!(field(ada, "Operator"), "Operator");
    assert_eq!(alternative(ada, "Range"), "Alternative_Range");
    assert_eq!(alternative(ada, "Self"), "Self");
    for needle in [
        "      Field_Type : ",
        "      Field_Range : KeywordRecord_Field_Range_Sequence;",
        "      Operator : ",
        "      Delete : KeywordRecord_Delete_Optional;",
        "type KeywordRecord_Field_Range_Sequence is private;",
        "(Alternative_Range_Kind,",
        "type KeywordChoice (Kind : KeywordChoice_Kind := Alternative_Range_Kind) is record",
        "         when Alternative_Range_Kind =>\n            Alternative_Range : KeywordChoice_Alternative_Range_Sequence;",
        "         when Alternative_Type_Kind =>\n            Alternative_Type : ",
        "         when Self_Kind =>\n            Self : ",
    ] {
        assert!(spec.contains(needle), "missing {needle:?}\n{spec}");
    }
    for forbidden in [
        "      Range : ",
        "      Type : ",
        "KeywordRecord_Range_",
        " Range_Kind",
    ] {
        assert!(!spec.contains(forbidden), "unexpected {forbidden:?}");
    }
    if !gnat_available() {
        return;
    }
    compile_and_run_ada(&root, Path::new("gnatmake"));
    if let Some(gnat13) = std::env::var_os("AMS_GRA_GNAT13_GNATMAKE") {
        compile_and_run_ada(&root, Path::new(&gnat13));
        println!("TASK054 ADA GNAT13: PASSED");
    }
    println!("TASK054 ADA ESCAPED MEMBERS: PASSED");
}

/// Rust: `field_type`/`field_self` fields and the `AlternativeSelf` variant,
/// never raw identifiers; compiled with `rustc -D warnings`, and run.
#[test]
fn task054_rust_escaped_members_compile_and_run() {
    let root = output("rust");
    let generated = service(
        "service-generate",
        "member-keywords",
        "rust",
        Some(&root),
        false,
    );
    assert!(generated.status.success(), "{}", text(&generated));
    let model = std::fs::read_to_string(root.join("oam.rs")).unwrap();
    let rust = BackendLanguage::Rust;
    assert_eq!(field(rust, "Type"), "field_type");
    assert_eq!(field(rust, "Self"), "field_self");
    assert_eq!(field(rust, "Operator"), "operator");
    assert_eq!(alternative(rust, "Self"), "AlternativeSelf");
    assert_eq!(alternative(rust, "Type"), "Type");
    assert!(!model.contains("r#"), "no raw identifiers");
    let fields = RECORD_FIELDS
        .iter()
        .map(|source| format!("    pub {}: ", field(rust, source)))
        .collect::<Vec<_>>();
    for needle in &fields {
        assert!(model.contains(needle), "missing {needle:?}\n{model}");
    }
    for source in CHOICE_ALTERNATIVES {
        let needle = format!("    {}(", alternative(rust, source));
        assert!(model.contains(&needle), "missing {needle:?}");
    }
    let int = "BoundedI64::<-2147483648, 2147483647>::new";
    let main = format!(
        "#![allow(dead_code)]\n{model}\nfn main() {{
    let int = |v: i64| {int}(v).expect(\"xs:int\");
    let picks = [
        KeywordChoice::Range(BoundedVec::new(vec![int(1)]).expect(\"1..3\")),
        KeywordChoice::{self_a}(int(3)),
        KeywordChoice::{type_a}(String::from(\"t\")),
        KeywordChoice::{del_a}(true),
        KeywordChoice::{ord_a}(int(4)),
    ];
    let item = KeywordRecord {{
        {type_f}: String::from(\"inherited\"),
        {range_f}: BoundedVec::new(vec![int(10), int(20)]).expect(\"0..2\"),
        {op_f}: String::from(\"op\"),
        {del_f}: Some(false),
        {self_f}: int(5),
        {ord_f}: int(6),
        pick: picks[1].clone(),
    }};
    assert_eq!(item.{type_f}, \"inherited\");
    assert_eq!(item.{range_f}.as_slice().len(), 2);
    assert_eq!(item.{self_f}.get() + item.{ord_f}.get(), 11);
    assert!(matches!(item.pick, KeywordChoice::{self_a}(v) if v.get() == 3));
    assert_eq!(picks.len(), 5);
}}
",
        self_a = alternative(rust, "Self"),
        type_a = alternative(rust, "Type"),
        del_a = alternative(rust, "Delete"),
        ord_a = alternative(rust, "Ordinary"),
        type_f = field(rust, "Type"),
        range_f = field(rust, "Range"),
        op_f = field(rust, "Operator"),
        del_f = field(rust, "Delete"),
        self_f = field(rust, "Self"),
        ord_f = field(rust, "Ordinary"),
    );
    std::fs::write(root.join("probe.rs"), main).unwrap();
    run_ok(
        Command::new("rustc").current_dir(&root).args([
            "--edition=2024",
            "-D",
            "warnings",
            "probe.rs",
            "-o",
            "probe",
        ]),
        "rustc -D warnings",
    );
    run_ok(&mut Command::new(root.join("probe")), "Rust probe run");
    println!("TASK054 RUST ESCAPED MEMBERS: PASSED");
}

/// C++: `field_operator`/`field_delete` members; upper-camel alternatives
/// such as `Operator`-like nested types stay unchanged. Strict C++17.
#[test]
fn task054_cpp_escaped_members_compile_strictly_and_run() {
    let root = output("cpp");
    let generated = service(
        "service-generate",
        "member-keywords",
        "cpp",
        Some(&root),
        false,
    );
    assert!(generated.status.success(), "{}", text(&generated));
    let header = std::fs::read_to_string(root.join("oam.hpp")).unwrap();
    let cpp = BackendLanguage::Cpp;
    assert_eq!(field(cpp, "Operator"), "field_operator");
    assert_eq!(field(cpp, "Delete"), "field_delete");
    assert_eq!(field(cpp, "Type"), "type");
    for source in CHOICE_ALTERNATIVES {
        // C++ keywords are lowercase: every upper-camel alternative is legal.
        assert_eq!(alternative(cpp, source), source);
        assert!(
            header.contains(&format!("    struct {source} {{ ")),
            "{source}"
        );
    }
    for source in RECORD_FIELDS {
        let needle = format!(" {};\n", field(cpp, source));
        assert!(header.contains(&needle), "missing {needle:?}\n{header}");
    }
    assert!(!header.contains(" operator;") && !header.contains(" delete;"));
    let probe = format!(
        "#include \"oam.hpp\"
#include <cstdlib>
using namespace programs::oam;
int main() {{
    using Int = BoundedInteger<std::int64_t, -2147483648, 2147483647>;
    KeywordRecord item{{
        std::string(\"inherited\"),
        *BoundedVector<Int, 0, 2>::create({{*Int::create(10), *Int::create(20)}}),
        std::string(\"op\"),
        std::optional<bool>(false),
        *Int::create(5),
        *Int::create(6),
        KeywordChoice{{KeywordChoice::Self{{*Int::create(3)}}}},
    }};
    item.{op_f} = \"operator\";
    item.{del_f} = true;
    item.{type_f} = \"base\";
    if (item.{op_f} != \"operator\" || !item.{del_f}.value() || item.{type_f} != \"base\") return EXIT_FAILURE;
    if (item.{range_f}.values().size() != 2) return EXIT_FAILURE;
    if (item.{self_f}.value() + item.{ord_f}.value() != 11) return EXIT_FAILURE;
    KeywordChoice picks[] = {{
        KeywordChoice{{KeywordChoice::Range{{*BoundedVector<Int, 1, 3>::create({{*Int::create(1)}})}}}},
        KeywordChoice{{KeywordChoice::Type{{std::string(\"t\")}}}},
        KeywordChoice{{KeywordChoice::Delete{{true}}}},
        KeywordChoice{{KeywordChoice::Ordinary{{*Int::create(4)}}}},
    }};
    if (!std::holds_alternative<KeywordChoice::Self>(item.pick.value)) return EXIT_FAILURE;
    if (!std::holds_alternative<KeywordChoice::Delete>(picks[2].value)) return EXIT_FAILURE;
    return EXIT_SUCCESS;
}}
",
        op_f = field(cpp, "Operator"),
        del_f = field(cpp, "Delete"),
        type_f = field(cpp, "Type"),
        range_f = field(cpp, "Range"),
        self_f = field(cpp, "Self"),
        ord_f = field(cpp, "Ordinary"),
    );
    std::fs::write(root.join("probe.cpp"), probe).unwrap();
    run_ok(
        Command::new("g++").current_dir(&root).args([
            "-std=c++17",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic-errors",
            "probe.cpp",
            "-o",
            "probe",
        ]),
        "strict C++17",
    );
    run_ok(&mut Command::new(root.join("probe")), "C++ probe run");
    println!("TASK054 CPP ESCAPED MEMBERS: PASSED");
}

/// `(schema stem, contract stem, language, diagnostic fragments)`: every
/// selection must be NOT READY in service-check, fail in service-generate
/// with NO output, and fail in plain `generate`, with the same diagnostic.
const NEGATIVE: [(&str, &str, &str, &[&str]); 7] = [
    (
        "member-collision",
        "member-collision-local-clash",
        "rust",
        &[
            "Rust names \"Type\" and \"Field_Type\" both generate \"field_type\" in the members of LocalClash",
        ],
    ),
    (
        "member-collision",
        "member-collision-local-clash",
        "ada",
        &[
            "Ada names \"Type\" and \"Field_Type\" both generate \"Field_Type\" in the members of LocalClash",
        ],
    ),
    (
        "member-collision",
        "member-collision-inherited-clash",
        "cpp",
        &[
            "C++ names \"Operator\" and \"Field_Operator\" both generate \"field_operator\" in the members of InheritedClash",
        ],
    ),
    (
        "member-collision",
        "member-collision-inherited-clash",
        "ada",
        &[
            "Ada names \"Range\" and \"Field_Range\" both generate \"Field_Range\" in the members of InheritedClash",
        ],
    ),
    (
        "member-malformed",
        "member-malformed",
        "ada",
        &["Ada cannot form a legal identifier from \"has-dash\""],
    ),
    (
        "member-malformed",
        "member-malformed",
        "rust",
        &["Rust cannot form a legal identifier from \"has-dash\""],
    ),
    (
        "member-malformed",
        "member-malformed",
        "cpp",
        &["C++ cannot form a legal identifier from \"has-dash\""],
    ),
];

fn contract_cli(
    command: &str,
    schema: &str,
    contract: &str,
    language: &str,
    out: Option<&Path>,
) -> Output {
    let schema = fixture(&format!("{schema}.xsd"));
    let contract = fixture(&format!("{contract}.yaml"));
    let mut arguments = vec![
        command,
        "--schema",
        schema.to_str().unwrap(),
        "--contract",
        contract.to_str().unwrap(),
        "--language",
        language,
        "--world",
        "closed-schema",
    ];
    let out = out.map(|path| path.to_str().unwrap().to_owned());
    if let Some(out) = &out {
        arguments.extend(["--output", out]);
    }
    cli(&arguments)
}

/// Post-remap collisions (local and inherited) and malformed identifiers
/// fail closed with consistent diagnostics across service-check,
/// service-generate, and direct `generate`: no READY verdict can reach a
/// backend that rejects the same name, and no numeric suffix is invented.
#[test]
fn task054_collisions_and_malformed_names_fail_closed_with_parity() {
    for (schema, contract, language, fragments) in NEGATIVE {
        let label = format!("{contract}/{language}");
        let checked = contract_cli("service-check", schema, contract, language, None);
        let report = text(&checked);
        assert!(!checked.status.success(), "{label}: {report}");
        assert!(report.contains("status: NOT READY"), "{label}: {report}");
        for fragment in fragments {
            assert!(report.contains(fragment), "{label}: {report}");
        }
        let root = output(&format!("negative-{contract}-{language}"));
        let generated = contract_cli("service-generate", schema, contract, language, Some(&root));
        assert!(!generated.status.success(), "{label}");
        assert!(!root.exists(), "{label}: no files may be generated");
        let direct_root = output(&format!("negative-direct-{contract}-{language}"));
        let direct = cli(&[
            "generate",
            "--schema",
            fixture(&format!("{schema}.xsd")).to_str().unwrap(),
            "--language",
            language,
            "--world",
            "closed-schema",
            "--output",
            direct_root.to_str().unwrap(),
        ]);
        let direct_text = text(&direct);
        assert!(!direct.status.success(), "{label}");
        assert!(
            direct_text.contains("cannot form a legal identifier")
                || direct_text.contains("both generate"),
            "{label}: {direct_text}"
        );
        for suffix in ["field_type_2", "field_type2", "Field_Range_2", "escaped_"] {
            assert!(!report.contains(suffix) && !direct_text.contains(suffix));
        }
    }
}

/// Control: the collision schema's safe world. C++ has no `Type` keyword, so
/// `LocalClash` (`type` + `field_type`) is legal C++, and Rust has no
/// `operator`/`range` keyword, so `InheritedClash` is legal Rust. Escaping is
/// decided by the ACTUAL candidate, not by resemblance to a keyword.
#[test]
fn task054_collisions_are_decided_by_the_actual_candidate() {
    for (contract, language) in [
        ("member-collision-local-clash", "cpp"),
        ("member-collision-inherited-clash", "rust"),
    ] {
        let checked = contract_cli(
            "service-check",
            "member-collision",
            contract,
            language,
            None,
        );
        assert!(
            text(&checked).contains("status: READY"),
            "{contract}/{language}: {}",
            text(&checked)
        );
    }
}
