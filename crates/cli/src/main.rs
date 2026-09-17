use std::env;

const HELP: &str = r#"ams-gra-codegen-oms

Schema-driven OMS/UCI multi-language code generator.

BOOTSTRAP STATUS
    Architecture and crate boundaries are established. Full UCI XSD parsing
    and source generation are the next implementation milestones.

PLANNED COMMANDS
    parse       Parse/normalize UCI XSD and emit/debug the language-neutral IR
    generate    Generate a selected language backend
    diff        Compare two schema versions using normalized IR

Use README.md and docs/roadmap.md for the implementation plan.
"#;

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None | Some("-h" | "--help") => print!("{HELP}"),
        Some("-V" | "--version") => println!("{}", env!("CARGO_PKG_VERSION")),
        Some(command) => {
            eprintln!("command '{command}' is not implemented in the bootstrap yet\n");
            eprint!("{HELP}");
            std::process::exit(2);
        }
    }
}
