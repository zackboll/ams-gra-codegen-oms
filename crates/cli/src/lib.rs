//! Command-line orchestration for the OMS/UCI code-generation pipeline.

use ams_gra_oms_backend_ada::AdaBackend;
use ams_gra_oms_backend_cpp::CppBackend;
use ams_gra_oms_backend_rust::RustBackend;
use ams_gra_oms_codegen_core::{Backend, GeneratedFile};
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const HELP: &str = r#"ams-gra-codegen-oms

Schema-driven OMS/UCI multi-language code generator.

USAGE:
    ams-gra-codegen-oms validate --schema PATH
    ams-gra-codegen-oms generate --schema PATH --language LANGUAGE --output DIR

COMMANDS:
    validate    Load and validate an XSD schema set
    generate    Generate source files from an XSD schema set

LANGUAGES:
    ada
    rust
    cpp

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version

EXIT CODES:
    0    Success
    1    Schema, code-generation, or filesystem error
    2    Command-line usage error
"#;

const VALIDATE_HELP: &str = r#"ams-gra-codegen-oms validate

Load an XSD schema set and validate its semantic IR.

USAGE:
    ams-gra-codegen-oms validate --schema PATH

OPTIONS:
    -s, --schema PATH    Root XSD document
    -h, --help           Print help
"#;

const GENERATE_HELP: &str = r#"ams-gra-codegen-oms generate

Generate source files from an XSD schema set.

USAGE:
    ams-gra-codegen-oms generate --schema PATH --language LANGUAGE --output DIR

LANGUAGES:
    ada
    rust
    cpp

OPTIONS:
    -s, --schema PATH          Root XSD document
    -l, --language LANGUAGE    Required output language
    -o, --output DIR           Output directory
    -h, --help                 Print help
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorKind {
    Usage,
    Execution,
}

/// A command-line usage or pipeline execution failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    kind: ErrorKind,
    message: String,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Usage,
            message: message.into(),
        }
    }

    fn execution(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Execution,
            message: message.into(),
        }
    }

    /// Return the documented process exit code for this error.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self.kind {
            ErrorKind::Usage => 2,
            ErrorKind::Execution => 1,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    Ada,
    Rust,
    Cpp,
}

impl Language {
    fn parse(value: &OsStr) -> Result<Self, CliError> {
        match value.to_str() {
            Some("ada") => Ok(Self::Ada),
            Some("rust") => Ok(Self::Rust),
            Some("cpp") => Ok(Self::Cpp),
            Some(value) => Err(CliError::usage(format!(
                "unsupported language '{value}'; expected one of: ada, rust, cpp"
            ))),
            None => Err(CliError::usage(
                "language must be valid UTF-8; expected one of: ada, rust, cpp",
            )),
        }
    }

    fn backend(self) -> Box<dyn Backend> {
        match self {
            Self::Ada => Box::new(AdaBackend),
            Self::Rust => Box::new(RustBackend),
            Self::Cpp => Box::new(CppBackend),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help(&'static str),
    Version,
    Validate {
        schema: PathBuf,
    },
    Generate {
        schema: PathBuf,
        language: Language,
        output: PathBuf,
    },
}

/// Parse and execute CLI arguments, excluding the executable name.
///
/// Successful status and help text are written to `stdout`. Callers should
/// print returned errors to stderr and exit with [`CliError::exit_code`].
pub fn run<I, W>(args: I, stdout: &mut W) -> Result<(), CliError>
where
    I: IntoIterator<Item = OsString>,
    W: Write,
{
    match parse_args(args)? {
        Command::Help(help) => write_output(stdout, help),
        Command::Version => write_output(stdout, concat!(env!("CARGO_PKG_VERSION"), "\n")),
        Command::Validate { schema } => validate(&schema, stdout),
        Command::Generate {
            schema,
            language,
            output,
        } => generate(&schema, language, &output, stdout),
    }
}

fn parse_args<I>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let command = args
        .next()
        .ok_or_else(|| CliError::usage("missing command; expected 'validate' or 'generate'"))?;
    match command.to_str() {
        Some("-h" | "--help") => no_trailing_args(args, Command::Help(HELP)),
        Some("-V" | "--version") => no_trailing_args(args, Command::Version),
        Some("validate") => parse_validate(args.collect()),
        Some("generate") => parse_generate(args.collect()),
        Some(command) => Err(CliError::usage(format!(
            "unknown command '{command}'; expected 'validate' or 'generate'"
        ))),
        None => Err(CliError::usage("command must be valid UTF-8")),
    }
}

fn no_trailing_args<I>(mut args: I, command: Command) -> Result<Command, CliError>
where
    I: Iterator<Item = OsString>,
{
    if let Some(argument) = args.next() {
        return Err(unexpected_argument(&argument));
    }
    Ok(command)
}

fn parse_validate(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(VALIDATE_HELP));
    }
    let mut schema = None;
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::Validate {
        schema: required(schema, "--schema")?.into(),
    })
}

fn parse_generate(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(GENERATE_HELP));
    }
    let mut schema = None;
    let mut language = None;
    let mut output = None;
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        "-l" | "--language" => set_once(&mut language, value, "--language"),
        "-o" | "--output" => set_once(&mut output, value, "--output"),
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::Generate {
        schema: required(schema, "--schema")?.into(),
        language: Language::parse(&required(language, "--language")?)?,
        output: required(output, "--output")?.into(),
    })
}

fn is_help_request(args: &[OsString]) -> bool {
    args.len() == 1 && matches!(args[0].to_str(), Some("-h" | "--help"))
}

fn parse_options<F>(args: Vec<OsString>, mut set_option: F) -> Result<(), CliError>
where
    F: FnMut(&str, OsString) -> Result<(), CliError>,
{
    let mut args = args.into_iter();
    while let Some(option) = args.next() {
        let option_text = option
            .to_str()
            .ok_or_else(|| unexpected_argument(&option))?;
        if !option_text.starts_with('-') {
            return Err(unexpected_argument(&option));
        }
        let value = args
            .next()
            .ok_or_else(|| CliError::usage(format!("missing value for '{option_text}'")))?;
        set_option(option_text, value)?;
    }
    Ok(())
}

fn set_once(slot: &mut Option<OsString>, value: OsString, name: &str) -> Result<(), CliError> {
    if slot.replace(value).is_some() {
        return Err(CliError::usage(format!("duplicate option '{name}'")));
    }
    Ok(())
}

fn required(value: Option<OsString>, name: &str) -> Result<OsString, CliError> {
    value.ok_or_else(|| CliError::usage(format!("missing required option '{name}'")))
}

fn unexpected_argument(argument: &OsStr) -> CliError {
    CliError::usage(format!(
        "unexpected argument '{}'",
        argument.to_string_lossy()
    ))
}

fn validate<W: Write>(schema_path: &Path, stdout: &mut W) -> Result<(), CliError> {
    let schema =
        load_schema_set(schema_path).map_err(|error| CliError::execution(error.to_string()))?;
    schema
        .validate()
        .map_err(|error| CliError::execution(format!("invalid schema IR: {error}")))?;
    write_output(
        stdout,
        &format!(
            "schema valid\nnamespaces: {}\ntypes: {}\nmessages: {}\n",
            schema.namespaces.len(),
            schema.types.len(),
            schema.messages.len()
        ),
    )
}

fn generate<W: Write>(
    schema_path: &Path,
    language: Language,
    output_dir: &Path,
    stdout: &mut W,
) -> Result<(), CliError> {
    let schema =
        load_schema_set(schema_path).map_err(|error| CliError::execution(error.to_string()))?;
    schema
        .validate()
        .map_err(|error| CliError::execution(format!("invalid schema IR: {error}")))?;
    let files = language
        .backend()
        .generate(&schema)
        .map_err(|error| CliError::execution(error.to_string()))?;
    validate_generated_files(&files)?;

    fs::create_dir_all(output_dir).map_err(|error| {
        CliError::execution(format!(
            "unable to create output directory {}: {error}",
            output_dir.display()
        ))
    })?;
    for file in &files {
        write_generated_file(output_dir, file)?;
    }
    write_output(
        stdout,
        &format!(
            "generated {} file(s)\noutput: {}\n",
            files.len(),
            output_dir.display()
        ),
    )
}

fn validate_generated_files(files: &[GeneratedFile]) -> Result<(), CliError> {
    let mut paths = BTreeSet::new();
    for file in files {
        validate_relative_path(&file.relative_path)?;
        let normalized = file.relative_path.components().collect::<PathBuf>();
        if !paths.insert(normalized) {
            return Err(CliError::execution(format!(
                "backend produced duplicate generated path {}",
                file.relative_path.display()
            )));
        }
    }
    Ok(())
}

fn validate_relative_path(path: &Path) -> Result<(), CliError> {
    if path.as_os_str().is_empty()
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(CliError::execution(format!(
            "backend produced unsafe generated path {}",
            path.display()
        )));
    }
    Ok(())
}

fn write_generated_file(output_root: &Path, file: &GeneratedFile) -> Result<(), CliError> {
    let mut destination = output_root.to_path_buf();
    let mut components = file.relative_path.components().peekable();
    while let Some(component) = components.next() {
        let Component::Normal(component) = component else {
            return Err(CliError::execution(format!(
                "backend produced unsafe generated path {}",
                file.relative_path.display()
            )));
        };
        destination.push(component);
        if components.peek().is_some() {
            ensure_safe_parent(&destination)?;
        } else {
            ensure_safe_final_destination(&destination)?;
        }
    }

    fs::write(&destination, &file.contents).map_err(|error| {
        CliError::execution(format!(
            "unable to write generated file {}: {error}",
            destination.display()
        ))
    })
}

fn ensure_safe_parent(path: &Path) -> Result<(), CliError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_parent_metadata(path, &metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|error| {
                CliError::execution(format!(
                    "unable to create generated file directory {}: {error}",
                    path.display()
                ))
            })?;
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                CliError::execution(format!(
                    "unable to inspect generated file directory {}: {error}",
                    path.display()
                ))
            })?;
            validate_parent_metadata(path, &metadata)
        }
        Err(error) => Err(CliError::execution(format!(
            "unable to inspect generated file directory {}: {error}",
            path.display()
        ))),
    }
}

fn validate_parent_metadata(path: &Path, metadata: &fs::Metadata) -> Result<(), CliError> {
    if metadata.file_type().is_symlink() {
        return Err(CliError::execution(format!(
            "unsafe generated destination: parent component {} is a symbolic link",
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(CliError::execution(format!(
            "unsafe generated destination: parent component {} is not a directory",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_safe_final_destination(path: &Path) -> Result<(), CliError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CliError::execution(format!(
            "unsafe generated destination: {} is a symbolic link",
            path.display()
        ))),
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(CliError::execution(format!(
            "unsafe generated destination: {} is not a regular file",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CliError::execution(format!(
            "unable to inspect generated file destination {}: {error}",
            path.display()
        ))),
    }
}

fn write_output<W: Write>(writer: &mut W, output: &str) -> Result<(), CliError> {
    writer
        .write_all(output.as_bytes())
        .map_err(|error| CliError::execution(format!("unable to write standard output: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ams-gra-codegen-oms-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("temporary directory should be created");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary directory should be removed");
        }
    }

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn fixture(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    fn generated_file(relative_path: &str, contents: &str) -> GeneratedFile {
        GeneratedFile {
            relative_path: relative_path.into(),
            contents: contents.into(),
        }
    }

    fn parse(values: &[&str]) -> Result<Command, CliError> {
        parse_args(args(values))
    }

    fn assert_usage_error(values: &[&str], expected: &str) {
        let error = parse(values).expect_err("arguments should fail");
        assert_eq!(error.exit_code(), 2);
        assert!(error.to_string().contains(expected), "{error}");
    }

    #[test]
    fn parses_help() {
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help(HELP));
        assert_eq!(
            parse(&["generate", "--help"]).unwrap(),
            Command::Help(GENERATE_HELP)
        );
        assert_eq!(
            parse(&["validate", "-h"]).unwrap(),
            Command::Help(VALIDATE_HELP)
        );
    }

    #[test]
    fn parses_version() {
        assert_eq!(parse(&["--version"]).unwrap(), Command::Version);
    }

    #[test]
    fn parses_valid_validate() {
        assert_eq!(
            parse(&["validate", "--schema", "root.xsd"]).unwrap(),
            Command::Validate {
                schema: "root.xsd".into()
            }
        );
    }

    #[test]
    fn parses_valid_generate_languages() {
        for (name, language) in [
            ("ada", Language::Ada),
            ("rust", Language::Rust),
            ("cpp", Language::Cpp),
        ] {
            assert_eq!(
                parse(&[
                    "generate",
                    "--schema",
                    "root.xsd",
                    "--language",
                    name,
                    "--output",
                    "out",
                ])
                .unwrap(),
                Command::Generate {
                    schema: "root.xsd".into(),
                    language,
                    output: "out".into(),
                }
            );
        }
    }

    #[test]
    fn rejects_missing_and_unknown_commands() {
        assert_usage_error(&[], "missing command");
        assert_usage_error(&["diff"], "unknown command 'diff'");
        assert_usage_error(&["--unknown"], "unknown command '--unknown'");
    }

    #[test]
    fn rejects_missing_required_options() {
        assert_usage_error(&["validate"], "--schema");
        assert_usage_error(
            &["generate", "--language", "rust", "--output", "out"],
            "--schema",
        );
        assert_usage_error(
            &["generate", "--schema", "root.xsd", "--output", "out"],
            "--language",
        );
        assert_usage_error(
            &["generate", "--schema", "root.xsd", "--language", "rust"],
            "--output",
        );
    }

    #[test]
    fn rejects_unknown_language() {
        assert_usage_error(
            &[
                "generate",
                "--schema",
                "root.xsd",
                "--language",
                "python",
                "--output",
                "out",
            ],
            "unsupported language 'python'",
        );
    }

    #[test]
    fn rejects_duplicate_schema() {
        assert_usage_error(
            &["validate", "--schema", "a.xsd", "--schema", "b.xsd"],
            "duplicate option '--schema'",
        );
    }

    #[test]
    fn rejects_unexpected_positional_argument() {
        assert_usage_error(&["validate", "root.xsd"], "unexpected argument 'root.xsd'");
    }

    #[test]
    fn validates_safe_and_unsafe_generated_paths() {
        for safe in ["safe.ads", "subdir/safe.hpp"] {
            validate_relative_path(Path::new(safe)).expect(safe);
        }
        for unsafe_path in ["../escape", "subdir/../../escape", "/absolute/path"] {
            let error = validate_relative_path(Path::new(unsafe_path)).expect_err(unsafe_path);
            assert!(error.to_string().contains("unsafe generated path"));
        }
    }

    #[test]
    fn rejects_duplicate_generated_paths() {
        let files = vec![
            GeneratedFile {
                relative_path: "same.rs".into(),
                contents: "first".into(),
            },
            GeneratedFile {
                relative_path: "same.rs".into(),
                contents: "second".into(),
            },
        ];
        let error = validate_generated_files(&files).expect_err("duplicate should fail");
        assert!(
            error
                .to_string()
                .contains("duplicate generated path same.rs")
        );

        let equivalent_files = vec![
            GeneratedFile {
                relative_path: "subdir//same.rs".into(),
                contents: "first".into(),
            },
            GeneratedFile {
                relative_path: "subdir/same.rs".into(),
                contents: "second".into(),
            },
        ];
        validate_generated_files(&equivalent_files)
            .expect_err("equivalent destination paths should fail");
    }

    #[test]
    fn overwrites_existing_regular_file() {
        let output = TempDir::new();
        let destination = output.0.join("generated.rs");
        fs::write(&destination, "old contents").unwrap();

        write_generated_file(&output.0, &generated_file("generated.rs", "new contents")).unwrap();

        assert_eq!(fs::read_to_string(destination).unwrap(), "new contents");
    }

    #[test]
    fn writes_through_existing_ordinary_parent_directory() {
        let output = TempDir::new();
        fs::create_dir(output.0.join("subdir")).unwrap();

        write_generated_file(
            &output.0,
            &generated_file("subdir/generated.rs", "generated contents"),
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(output.0.join("subdir/generated.rs")).unwrap(),
            "generated contents"
        );
    }

    #[test]
    fn rejects_non_directory_parent_component() {
        let output = TempDir::new();
        fs::write(output.0.join("subdir"), "ordinary file").unwrap();

        let error = write_generated_file(
            &output.0,
            &generated_file("subdir/generated.rs", "generated contents"),
        )
        .expect_err("non-directory parent should be rejected");

        assert!(error.to_string().contains("not a directory"));
    }

    #[test]
    fn rejects_directory_as_final_destination() {
        let output = TempDir::new();
        fs::create_dir(output.0.join("generated.rs")).unwrap();

        let error = write_generated_file(
            &output.0,
            &generated_file("generated.rs", "generated contents"),
        )
        .expect_err("final directory should be rejected");

        assert!(error.to_string().contains("not a regular file"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_final_file_symlink_without_modifying_target() {
        use std::os::unix::fs::symlink;

        let workspace = TempDir::new();
        let output = workspace.0.join("output");
        fs::create_dir(&output).unwrap();
        let outside = workspace.0.join("outside.rs");
        fs::write(&outside, "outside contents").unwrap();
        symlink(&outside, output.join("generated.rs")).unwrap();

        let error = write_generated_file(
            &output,
            &generated_file("generated.rs", "generated contents"),
        )
        .expect_err("final symlink should be rejected");

        assert!(error.to_string().contains("symbolic link"));
        assert_eq!(fs::read_to_string(outside).unwrap(), "outside contents");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_intermediate_directory_symlink_without_writing_outside() {
        use std::os::unix::fs::symlink;

        let workspace = TempDir::new();
        let output = workspace.0.join("output");
        let outside = workspace.0.join("outside");
        fs::create_dir(&output).unwrap();
        fs::create_dir(&outside).unwrap();
        symlink(&outside, output.join("subdir")).unwrap();

        let error = write_generated_file(
            &output,
            &generated_file("subdir/generated.rs", "generated contents"),
        )
        .expect_err("intermediate symlink should be rejected");

        assert!(error.to_string().contains("symbolic link"));
        assert!(!outside.join("generated.rs").exists());
    }

    #[test]
    fn validation_reports_stable_counts() {
        let schema = fixture("tests/fixtures/codegen-order/root.xsd");
        let mut stdout = Vec::new();
        run(
            vec![
                OsString::from("validate"),
                OsString::from("--schema"),
                schema.into(),
            ],
            &mut stdout,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "schema valid\nnamespaces: 1\ntypes: 3\nmessages: 0\n"
        );
    }

    #[test]
    fn validation_reports_normalized_message_count() {
        let schema = fixture("crates/xsd-frontend/tests/fixtures/global-elements.xsd");
        let mut stdout = Vec::new();
        run(
            vec![
                OsString::from("validate"),
                OsString::from("--schema"),
                schema.into(),
            ],
            &mut stdout,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "schema valid\nnamespaces: 1\ntypes: 1\nmessages: 2\n"
        );
    }

    #[test]
    fn invalid_schema_returns_execution_error() {
        let schema = fixture("crates/xsd-frontend/tests/fixtures/errors/unresolved-root.xsd");
        let error = run(
            vec![
                OsString::from("validate"),
                OsString::from("--schema"),
                schema.into(),
            ],
            &mut Vec::new(),
        )
        .expect_err("invalid schema should fail");
        assert_eq!(error.exit_code(), 1);
        assert!(
            error
                .to_string()
                .contains("unresolved type reference {urn:other}Absent")
        );
    }

    #[test]
    fn generation_failure_does_not_create_output_directory() {
        let schema = fixture("crates/xsd-frontend/tests/fixtures/errors/unresolved-root.xsd");
        let parent = TempDir::new();
        let output = parent.0.join("must-not-exist");
        let error = run(
            vec![
                OsString::from("generate"),
                OsString::from("--schema"),
                schema.into(),
                OsString::from("--language"),
                OsString::from("rust"),
                OsString::from("--output"),
                output.clone().into(),
            ],
            &mut Vec::new(),
        )
        .expect_err("invalid schema should fail generation");
        assert_eq!(error.exit_code(), 1);
        assert!(!output.exists());
    }

    #[test]
    fn each_language_matches_existing_backend_golden() {
        let schema = fixture("tests/fixtures/codegen-order/root.xsd");
        for (language, generated_path, golden_path) in [
            (
                "ada",
                "example-order.ads",
                "crates/backend-ada/tests/expected/codegen_order.ads",
            ),
            (
                "rust",
                "order.rs",
                "crates/backend-rust/tests/expected/codegen_order.rs",
            ),
            (
                "cpp",
                "order.hpp",
                "crates/backend-cpp/tests/expected/codegen_order.hpp",
            ),
        ] {
            let output = TempDir::new();
            let mut stdout = Vec::new();
            run(
                vec![
                    OsString::from("generate"),
                    OsString::from("--schema"),
                    schema.clone().into(),
                    OsString::from("--language"),
                    OsString::from(language),
                    OsString::from("--output"),
                    output.0.clone().into(),
                ],
                &mut stdout,
            )
            .unwrap();
            assert!(String::from_utf8(stdout).unwrap().starts_with("generated "));
            assert_eq!(
                fs::read_to_string(output.0.join(generated_path)).unwrap(),
                fs::read_to_string(fixture(golden_path)).unwrap()
            );
        }
    }
}
