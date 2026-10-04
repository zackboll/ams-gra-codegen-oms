use super::*;
#[path = "../../../tests/task060_cases.rs"]
mod corpus;
use std::process::Command;

#[test]
fn standalone_task060_compiler_probe() {
    let dir = std::env::temp_dir().join(format!("task060-cpp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut source = String::from(
        "#include <string>\n#include <string_view>\n#include <optional>\n#include <utility>\n",
    );
    let mut probe = String::from("int main() {\n");
    for (name, profile) in corpus::profiles() {
        let model = render_cpp_alternating_ascii(&name, &profile);
        println!(
            "TASK060 C++ {name}: {} bytes {} lines",
            model.len(),
            model.lines().count()
        );
        source.push_str(&model);
        for (text, valid) in corpus::cases(&profile) {
            let bytes = text
                .bytes()
                .map(|b| format!("\\{b:03o}"))
                .collect::<String>();
            writeln!(probe,"{{ const std::string text(\"{bytes}\", {}); auto v={name}::create(text); if (v.has_value() != {valid}) return 1; if (v && v->value() != text) return 2; }}",text.len()).unwrap();
        }
    }
    probe.push_str("return 0;\n}\n");
    source.push_str(&probe);
    let build = |source: &str| {
        std::fs::write(dir.join("probe.cpp"), source).unwrap();
        let result = Command::new("c++")
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
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    build(&source);
    assert!(Command::new(dir.join("probe")).status().unwrap().success());
    let wrong = source.replacen("has_value() != false", "has_value() != true", 1);
    assert_ne!(wrong, source);
    build(&wrong);
    assert!(!Command::new(dir.join("probe")).status().unwrap().success());
    std::fs::write(dir.join("probe.cpp"), source).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn factored_task060_source_sizes() {
    for (name, profile) in corpus::profiles() {
        let source = render_cpp_alternating_ascii(&name, &profile);
        println!(
            "TASK060 cpp {name}: {} bytes {} lines",
            source.len(),
            source.lines().count()
        );
        if profile.groups[0].len() == 625 {
            assert!(source.len() < 25_000, "IPv4 remained unfactored");
        }
    }
}
