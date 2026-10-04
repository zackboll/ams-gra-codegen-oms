//! Shared standalone compiler campaign; expected values come only from corpus.
#[path = "task062_cases.rs"]
mod corpus;
use std::{fmt::Write as _, path::Path, process::Command};

fn execute(dir: &Path, program: &str, args: &[&str], succeeds: bool) -> String {
    let result = Command::new(program)
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    println!(
        "{program} {args:?} exit {:?}\n{diagnostics}",
        result.status.code()
    );
    assert_eq!(
        result.status.success(),
        succeeds,
        "artifacts retained at {}",
        dir.display()
    );
    diagnostics
}

pub fn run(language: &str, model: &str) {
    let dir = std::env::temp_dir().join(format!("task062-{language}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cases = corpus::cases();
    let mut data = String::new();
    for (text, expected) in &cases {
        assert_eq!(corpus::independent_accepts(text), *expected);
        write!(data, "{} ", u8::from(*expected)).unwrap();
        for byte in text.bytes() {
            write!(data, "{byte:02X}").unwrap();
        }
        data.push('\n');
    }
    std::fs::write(dir.join("corpus.txt"), &data).unwrap();
    assert!(
        model.len() < 12_000 && model.lines().count() < 250,
        "pathological IPv6 source"
    );
    println!(
        "TASK062 {language} SOURCE: {} bytes {} lines",
        model.len(),
        model.lines().count()
    );
    let binary = dir.join("probe").to_string_lossy().into_owned();
    match language {
        "rust" => {
            std::fs::write(dir.join("probe.rs"), format!("{model}\n{RUST_PROBE}")).unwrap();
            execute(
                &dir,
                "rustc",
                &["--edition=2024", "-Dwarnings", "probe.rs", "-o", "probe"],
                true,
            );
            let diagnostics = execute(&dir, &binary, &[], true);
            assert!(diagnostics.contains("TASK062 RUST CORPUS LIFECYCLE: PASSED"));
            std::fs::write(
                dir.join("unchecked.rs"),
                format!("{model}\nfn main() {{ let _: Address = Default::default(); }}"),
            )
            .unwrap();
            execute(&dir, "rustc", &["--edition=2024", "unchecked.rs"], false);
        }
        "cpp" => {
            std::fs::write(
                dir.join("probe.cpp"),
                format!("{CPP_HEADERS}\n{model}\n{CPP_PROBE}"),
            )
            .unwrap();
            execute(
                &dir,
                "g++",
                &[
                    "-std=c++17",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-pedantic",
                    "probe.cpp",
                    "-o",
                    "probe",
                ],
                true,
            );
            let diagnostics = execute(&dir, &binary, &[], true);
            assert!(diagnostics.contains("TASK062 CPP CORPUS LIFECYCLE: PASSED"));
        }
        "ada" => {
            std::fs::write(dir.join("carriers.ads"), ADA_SPEC).unwrap();
            std::fs::write(
                dir.join("carriers.adb"),
                format!("package body Carriers is\n{model}\nend Carriers;\n"),
            )
            .unwrap();
            for (policy, flags, prefix) in [
                ("default", &[][..], ""),
                ("enabled", &["-gnata"][..], ""),
                (
                    "ignored",
                    &["-gnata"][..],
                    "pragma Assertion_Policy (Ignore);\n",
                ),
            ] {
                std::fs::write(dir.join("probe.adb"), format!("{prefix}{ADA_PROBE}")).unwrap();
                let mut args = vec!["-q", "-f", "-gnat2022", "-gnatwe", "-O0", "-gnato"];
                args.extend(flags);
                args.push("probe.adb");
                execute(&dir, "gnatmake", &args, true);
                let diagnostics = execute(&dir, &binary, &[], true);
                assert!(diagnostics.contains("TASK062 ADA CORPUS LIFECYCLE: PASSED"));
                println!("TASK062 ADA {policy} BOUNDS ASSERTIONS: PASSED");
            }
        }
        _ => unreachable!(),
    }
    // Plant precisely one false expectation; require each executable to fail.
    let first = data.find('0').unwrap();
    let mut wrong = data.clone();
    wrong.replace_range(first..first + 1, "1");
    std::fs::write(dir.join("corpus.txt"), wrong).unwrap();
    execute(&dir, &binary, &[], false);
    std::fs::write(dir.join("corpus.txt"), data).unwrap();
    execute(&dir, &binary, &[], true);
    println!(
        "TASK062 {language} WRONG EXPECTATION DETECTED AND RESTORED: PASSED ({} cases)",
        cases.len()
    );
}

const RUST_PROBE: &str = r#"
fn main() {
    let corpus = std::fs::read_to_string("corpus.txt").unwrap();
    for line in corpus.lines() {
        let (expected, encoded) = line.split_once(' ').unwrap();
        let bytes: Vec<u8> = encoded.as_bytes().chunks_exact(2).map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(),16).unwrap()).collect();
        let text = String::from_utf8(bytes).unwrap();
        let item = Address::new(&text);
        assert_eq!(item.is_some(), expected == "1", "{text:?}");
        if let Some(item) = item {
            assert_eq!(item.as_str(), text);
            let copy = item.clone();
            let values = vec![item, copy];
            for value in values { assert_eq!(value.as_str(), text); }
        }
    }
    println!("TASK062 RUST CORPUS LIFECYCLE: PASSED");
}
"#;
const CPP_HEADERS: &str = r#"#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <type_traits>
"#;
const CPP_PROBE: &str = r#"
static_assert(!std::is_default_constructible_v<Address>);
static_assert(!std::is_constructible_v<Address, std::string>);
int main() {
    std::ifstream input("corpus.txt");
    if (!input) throw std::runtime_error("missing corpus");
    std::string line;
    while (std::getline(input, line)) {
        const bool expected = line[0] == '1';
        std::string text;
        for (std::size_t i = 2; i < line.size(); i += 2)
            text.push_back(static_cast<char>(std::stoul(line.substr(i, 2), nullptr, 16)));
        const auto item = Address::create(text);
        if (item.has_value() != expected) throw std::runtime_error("unexpected: " + line);
        if (item) {
            Address copy = *item;
            Address moved = std::move(copy);
            if (copy.value() != text || moved.value() != text) throw std::runtime_error("destructive move");
            copy = std::move(moved);
            std::vector<Address> values{copy, moved};
            for (const auto& value : values) if (value.value() != text) throw std::runtime_error("changed spelling");
        }
    }
    std::cout << "TASK062 CPP CORPUS LIFECYCLE: PASSED\n";
}
"#;
const ADA_SPEC: &str = r#"with Ada.Strings.Unbounded;
package Carriers is
   type Address is private;
   function Create (Value : String) return Address;
   function Value (Item : Address) return String;
private
   type Address is record
      Text : Ada.Strings.Unbounded.Unbounded_String :=
        raise Program_Error with "Address requires initialization from Create";
   end record;
end Carriers;
"#;
const ADA_PROBE: &str = r#"with Ada.Text_IO;
with Ada.Exceptions;
with Carriers; use Carriers;
procedure Probe is
   Input : Ada.Text_IO.File_Type;
   function Digit (B : Character) return Natural is
   begin
      if B in '0' .. '9' then return Character'Pos (B) - Character'Pos ('0'); end if;
      return Character'Pos (B) - Character'Pos ('A') + 10;
   end Digit;
   procedure Check (Text : String; Expected : Boolean) is
   begin
      declare
         Item : constant Address := Create (Text);
         Copy : Address := Item;
         type Items is array (Positive range <>) of Address;
         Stored : Items (1 .. 2) := (others => Item);
      begin
         if not Expected then raise Program_Error with "accepted invalid"; end if;
         Copy := Stored (1); Stored (2) := Copy;
         if Value (Copy) /= Text or else Value (Stored (2)) /= Text then
            raise Program_Error with "changed spelling";
         end if;
      end;
   exception
      when Error : Constraint_Error =>
         if Expected or else Ada.Exceptions.Exception_Message (Error) /=
           "invalid IPv6 profile string"
         then
            raise Program_Error with Ada.Exceptions.Exception_Information (Error);
         end if;
   end Check;
   procedure Placements (Text : String; Expected : Boolean) is
      One : String (1 .. Text'Length) := Text;
      Shifted : String (37 .. 37 + (Text'Length - 1)) := Text;
   begin
      Check (One, Expected); Check (Shifted, Expected);
      if Text'Length = 0 then
         declare
            High_Null : String (Positive'Last .. Positive'Last - 1) := "";
         begin Check (High_Null, Expected); end;
      else
         declare
            High : String (Positive'Last - (Text'Length - 1) .. Positive'Last) := Text;
         begin Check (High, Expected); end;
      end if;
   end Placements;
begin
   begin
      declare Unchecked : Address;
      begin raise Constraint_Error with Value (Unchecked); end;
   exception
      when Program_Error => null;
   end;
   Ada.Text_IO.Open (Input, Ada.Text_IO.In_File, "corpus.txt");
   while not Ada.Text_IO.End_Of_File (Input) loop
      declare
         Line : constant String := Ada.Text_IO.Get_Line (Input);
         Text : String (1 .. (Line'Length - 2) / 2);
      begin
         for I in Text'Range loop
            Text (I) := Character'Val (16 * Digit (Line (2 * I + 1)) + Digit (Line (2 * I + 2)));
         end loop;
         Placements (Text, Line (1) = '1');
      end;
   end loop;
   Ada.Text_IO.Close (Input);
   Ada.Text_IO.Put_Line ("TASK062 ADA CORPUS LIFECYCLE: PASSED");
end Probe;
"#;
