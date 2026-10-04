use super::*;
#[path = "../../../tests/task060_cases.rs"]
mod corpus;
use std::process::Command;

#[test]
fn standalone_task060_compiler_probe() {
    let dir = std::env::temp_dir().join(format!("task060-ada-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut spec = String::from("with Ada.Strings.Unbounded;\npackage Carriers is\n");
    let mut private = String::from("private\n");
    let mut body = String::from("package body Carriers is\n");
    let mut probe = String::from("with Carriers; use Carriers;\nprocedure Probe is\nbegin\n");
    for (name, profile) in corpus::profiles() {
        writeln!(spec,"type {name} is private;\nfunction Create (Value : String) return {name};\nfunction Value (Item : {name}) return String;").unwrap();
        writeln!(private,"type {name} is record Text : Ada.Strings.Unbounded.Unbounded_String := raise Program_Error; end record;").unwrap();
        let model = render_ada_alternating_ascii_body(&name, &profile);
        println!(
            "TASK060 Ada {name}: {} bytes {} lines",
            model.len(),
            model.lines().count()
        );
        body.push_str(&model);
        for (text, valid) in corpus::cases(&profile) {
            let literal = if text.is_empty() {
                "\"\"".into()
            } else {
                format!(
                    "\"\" & {}",
                    text.bytes()
                        .map(|b| format!("Character'Val ({b})"))
                        .collect::<Vec<_>>()
                        .join(" & ")
                )
            };
            if valid {
                writeln!(probe,"declare V : constant {name} := Create ({literal}); begin if Value (V) /= ({literal}) then raise Program_Error; end if; end;").unwrap();
            } else {
                writeln!(probe,"begin declare V : constant {name} := Create ({literal}); begin raise Program_Error with Value (V); end; exception when Constraint_Error => null; end;").unwrap();
            }
        }
        writeln!(probe,"begin declare V : {name}; begin raise Constraint_Error with Value (V); end; exception when Program_Error => null; end;").unwrap();
    }
    spec.push_str(&private);
    spec.push_str("end Carriers;\n");
    body.push_str("end Carriers;\n");
    probe.push_str("end Probe;\n");
    std::fs::write(dir.join("carriers.ads"), spec).unwrap();
    std::fs::write(dir.join("carriers.adb"), body).unwrap();
    let build = |source: &str| {
        std::fs::write(dir.join("probe.adb"), source).unwrap();
        let result = Command::new("gnatmake")
            .current_dir(&dir)
            .args(["-q", "-f", "probe.adb"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    for policy in ["", "pragma Assertion_Policy (Ignore);\n"] {
        build(&format!("{policy}{probe}"));
        assert!(Command::new(dir.join("probe")).status().unwrap().success());
    }
    let wrong = probe.replacen(
        "exception when Constraint_Error => null",
        "exception when Program_Error => null",
        1,
    );
    assert_ne!(wrong, probe);
    build(&wrong);
    assert!(
        !Command::new(dir.join("probe"))
            .output()
            .unwrap()
            .status
            .success()
    );
    std::fs::write(dir.join("probe.adb"), probe).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn factored_task060_source_sizes() {
    for (name, profile) in corpus::profiles() {
        let source = render_ada_alternating_ascii_body(&name, &profile);
        println!(
            "TASK060 ada {name}: {} bytes {} lines",
            source.len(),
            source.lines().count()
        );
        if profile.groups[0].len() == 625 {
            assert!(source.len() < 25_000, "IPv4 remained unfactored");
        }
    }
}
