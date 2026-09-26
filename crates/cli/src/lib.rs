//! Command-line orchestration for the OMS/UCI code-generation pipeline.

use ams_gra_oms_backend_ada::AdaBackend;
use ams_gra_oms_backend_cpp::CppBackend;
use ams_gra_oms_backend_rust::RustBackend;
use ams_gra_oms_codegen_core::{
    Backend, BackendLanguage, CoverageAnalysis, GeneratedFile, GenerationWorld, ResolvedExchange,
    ServiceBackendReadiness, ServiceGenerationProjection, ServicePlan, analyze_service_readiness,
    project_service_generation_schema, resolve_service_plan, service_api_preflight,
};
// Only the CLI's own loading path touches contract files; no backend parses
// YAML, and the resolution itself lives in codegen-core.
use ams_gra_oms_ir::SchemaIr;
use ams_gra_oms_schema_docs::generate as generate_docs;
use ams_gra_oms_service_contract::load_contract;
use ams_gra_oms_xsd_frontend::load_schema_set_with_overlays;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const HELP: &str = r#"ams-gra-codegen-oms

Schema-driven OMS/UCI multi-language code generator.

USAGE:
    ams-gra-codegen-oms validate --schema PATH [--overlay PATH]...
    ams-gra-codegen-oms coverage --schema PATH [--overlay PATH]... --world WORLD
    ams-gra-codegen-oms generate --schema PATH [--overlay PATH]... --language LANGUAGE --output DIR --world WORLD
    ams-gra-codegen-oms docs --schema PATH [--overlay PATH]... --output DIR
    ams-gra-codegen-oms service-plan --schema PATH --contract PATH [--extension ID=PATH]...
    ams-gra-codegen-oms service-check --schema PATH --contract PATH [--extension ID=PATH]... --language LANGUAGE --world WORLD
    ams-gra-codegen-oms service-generate --schema PATH --contract PATH [--extension ID=PATH]... --language LANGUAGE --world WORLD --output DIR

COMMANDS:
    validate           Load and validate an XSD schema set
    coverage           Report deterministic IR and backend coverage counts
    generate           Generate source files from an XSD schema set
    docs               Generate an offline HTML browser for a normalized XSD schema set
    service-plan       Resolve a portable Service Contract against a schema set
    service-check      Report backend/world readiness for a contract's selection
    service-generate   Generate a contract's selected UCI type model and typed
                       service API wrapper

SERVICE CONTRACT PLANNING:
    'service-plan' joins a portable AMS GRA Service Contract (v0.1 YAML or
    JSON) to a normalized UCI schema set and reports the resolved plan. It
    performs NO filesystem writes and takes no --language, --output, or
    --world: contract validity and message/type resolution are independent of
    backend rendering policy.

    A contract's standards.uci_extension_schemas entries are logical extension
    IDENTIFIERS, not paths. Supply each one explicitly as
    --extension ID=PATH. The set must match the contract exactly: a declared
    extension with no mapping, an undeclared extension, and a duplicate
    extension ID all fail. Overlay composition order follows the CONTRACT's
    declaration order, not the order the options appear on the command line.

SERVICE CONTRACT BACKEND READINESS:
    'service-check' answers a different question than 'service-plan'.
    'service-plan' answers WHAT a contract selects; 'service-check' answers
    WHETHER one backend, under one generation world, can render that selected
    UCI type model TODAY. It therefore requires --language and --world, which
    'service-plan' deliberately refuses.

    Only the contract-selected OMS Message closures are measured. An
    unrenderable UCI declaration that the contract does not select does NOT
    make the service unready, so 'service-check' can report READY for a schema
    set whose full-schema 'generate' fails. Neither command writes any file:
    READY does not mean service source has been generated.

SERVICE CONTRACT SELECTED GENERATION:
    'service-generate' emits the UCI TYPE MODEL a Service Contract selects,
    and nothing unrelated, plus one typed SERVICE API wrapper entrypoint
    (service_api.rs / service_api.hpp / service_api.ads) describing the
    contract's functions and exchanges. It checks 'service-check' readiness
    first and writes no file at all unless the selection is READY, the
    projection succeeds, the backend generates the model AND the wrapper, and
    every generated path validates.

    The wrapper is compile-time endpoint metadata only. No CAL facade,
    publisher/subscriber API, codec, or runtime code is generated yet.

SCHEMA OVERLAYS:
    'validate', 'coverage', and 'generate' accept a repeatable --overlay PATH:
    an additional top-level schema document loaded into the same normalized
    schema set as --schema. Overlays are additive and are applied in the order
    given; each must declare the same targetNamespace as the root. Overlays
    never replace, override, or remove a root declaration, so a duplicate
    qualified name is still an error. Use this to supply known private derived
    types without editing a pinned authoritative root document. It is
    build-time schema composition, not a runtime extension registry.

LANGUAGES:
    ada
    rust
    cpp

GENERATION WORLDS:
    'generate' and 'coverage' require --world. There is no default: the
    semantic assumption about which types may legally exist must be stated by
    you, because a complete schema *file* set does not prove a complete
    *type* universe.

    closed-schema       You assert the supplied schema set is the complete
                        value-type universe. Known concrete descendants of an
                        abstract value are exhaustive, so abstract values
                        lower to closed sums, and an abstract value with zero
                        known descendants is genuinely uninhabited.

    open-extensions     External or private derived types may exist outside
                        the supplied schema set. Known descendants are never
                        assumed exhaustive, so every abstract structural
                        value fails closed; no placeholder is invented.
                        Abstract types used only as inheritance ancestry are
                        unaffected.

    'validate' takes no --world: schema validity is independent of generation
    policy. 'service-plan' takes no --world either, for the same reason: which
    UCI messages and types a contract selects is a fact about the contract and
    the schema, not about a backend's rendering policy.

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
    ams-gra-codegen-oms validate --schema PATH [--overlay PATH]...

OPTIONS:
    -s, --schema PATH    Root XSD document
        --overlay PATH   Repeatable same-target-namespace schema overlay,
                         additively composed into the validated schema set in
                         the order given; never overrides root declarations
    -h, --help           Print help
"#;

const DOCS_HELP: &str = r#"ams-gra-codegen-oms docs

Generate an offline HTML browser for a normalized XSD schema set.

USAGE:
    ams-gra-codegen-oms docs --schema PATH [--overlay PATH]... --output DIR

OPTIONS:
    -s, --schema PATH    Root schema
        --overlay PATH   Additional schema (repeatable)
    -o, --output DIR     Documentation output directory
    -h, --help           Print help
"#;

const GENERATE_HELP: &str = r#"ams-gra-codegen-oms generate

Generate source files from an XSD schema set.

USAGE:
    ams-gra-codegen-oms generate --schema PATH [--overlay PATH]... --language LANGUAGE --output DIR --world WORLD

LANGUAGES:
    ada
    rust
    cpp

WORLDS:
    closed-schema       You assert the supplied schema set is the complete
                        value-type universe: known concrete descendants of an
                        abstract value are exhaustive (closed-sum lowering),
                        and zero known descendants means uninhabited.

    open-extensions     External derived abstract-value types may exist
                        outside the supplied schema set; unsupported abstract
                        values fail closed rather than being represented by a
                        non-exhaustive closed sum.

OPTIONS:
    -s, --schema PATH          Root XSD document
        --overlay PATH         Repeatable same-target-namespace schema overlay,
                               additively composed into the generated schema
                               set in the order given; never overrides root
                               declarations and never selects a world
    -l, --language LANGUAGE    Required output language
    -o, --output DIR           Output directory
    -w, --world WORLD          Required generation world policy
    -h, --help                 Print help
"#;

const COVERAGE_HELP: &str = r#"ams-gra-codegen-oms coverage

Report deterministic semantic IR inventory and backend coverage.

USAGE:
    ams-gra-codegen-oms coverage --schema PATH [--overlay PATH]... --world WORLD

WORLDS:
    closed-schema       Measure capability assuming the supplied schema set is
                        the complete value-type universe.

    open-extensions     Measure capability assuming external derived
                        abstract-value types may exist; abstract structural
                        values are not baseline-renderable.

    The report states which world produced it, so saved evidence stays
    unambiguous.

OPTIONS:
    -s, --schema PATH    Root XSD document
        --overlay PATH   Repeatable same-target-namespace schema overlay,
                         additively composed into the analyzed schema set in
                         the order given; never overrides root declarations
                         and never implies a world
    -w, --world WORLD    Required generation world policy
    -h, --help           Print help
"#;

const SERVICE_PLAN_HELP: &str = r#"ams-gra-codegen-oms service-plan

Resolve a portable AMS GRA Service Contract against an XSD schema set and
print the resolved Service Plan. Writes no files.

USAGE:
    ams-gra-codegen-oms service-plan --schema PATH --contract PATH [--extension ID=PATH]...

OPTIONS:
    -s, --schema PATH           Root XSD document
    -c, --contract PATH         Portable Service Contract (.yaml, .yml, .json)
        --extension ID=PATH     Repeatable explicit mapping from a contract
                                standards.uci_extension_schemas identifier to a
                                local overlay schema root. The supplied set must
                                match the contract's declared set exactly;
                                overlays are composed in CONTRACT declaration
                                order, not command-line order
    -h, --help                  Print help

NOTES:
    Contract extension entries are logical identifiers, not paths, so they are
    never guessed at as filesystem locations. This command takes no --world:
    contract validity and UCI message/type resolution do not depend on the
    closed-schema/open-extensions generation policy.
"#;

const SERVICE_CHECK_HELP: &str = r#"ams-gra-codegen-oms service-check

Report whether one backend can render a Service Contract's selected UCI type
model under one generation world.

USAGE:
    ams-gra-codegen-oms service-check --schema PATH --contract PATH [--extension ID=PATH]... --language LANGUAGE --world WORLD

LANGUAGES:
    ada
    rust
    cpp

WORLDS:
    closed-schema       Measure readiness assuming the supplied schema set is
                        the complete value-type universe.

    open-extensions     Measure readiness assuming external derived
                        abstract-value types may exist; abstract structural
                        VALUE positions fail closed.

OPTIONS:
    -s, --schema PATH          Root XSD document
    -c, --contract PATH        Portable Service Contract v0.1 (YAML or JSON)
        --extension ID=PATH    Repeatable mapping from a contract
                               standards.uci_extension_schemas IDENTIFIER to a
                               local schema document
    -l, --language LANGUAGE    Required backend to measure
    -w, --world WORLD          Required generation world policy
    -h, --help                 Print help

NO OUTPUT FILES:
    'service-check' writes no generated source. There is no --output option;
    supplying one is a usage error. This command is analysis only, and a READY
    verdict does not mean any service source has been generated.

EXTENSIONS USE EXACT CONTRACT MAPPING:
    A contract's standards.uci_extension_schemas entries are logical extension
    IDENTIFIERS, not paths, so --extension is the ONLY way to supply schema
    content here; this command accepts no raw path-only overlay option. The
    supplied --extension set must match the contract exactly: a declared
    extension with no mapping, an undeclared extension, and a duplicate
    extension ID all fail, and no filename is ever guessed. Composition order
    follows the CONTRACT's declaration order.

READINESS IS SELECTED-CLOSURE CAPABILITY ONLY:
    Only the transitive type closures of contract-selected OMS Messages are
    measured, using current backend capability with no hypothetical feature
    enabled. Unselected unrenderable declarations are not reported and do not
    block readiness. Non-UCI exchanges (data transfer, special signal,
    security exchange, non-OMS message) require no UCI type model, so a
    contract with zero OMS Message exchanges is vacuously ready.

READY INCLUDES THE SERVICE API WRAPPER:
    READY also means 'service-generate' can emit the typed service API
    wrapper. A wrapper name that is unsafe in the requested language -- for
    example two distinct contract IDs such as 'foo-bar' and 'foo_bar' that
    normalize to one identifier -- makes the service NOT READY and is
    reported on its own 'service api boundary:' line. So is a wrapper that
    cannot be emitted beside its model: a shared output path, or a wrapper
    name that conflicts with the model in a shared host-language scope. It is
    never reported as an unsupported UCI type, and selected-type counts are
    unaffected.

EXIT CODES:
    0    READY
    1    NOT READY (the full report is still written to stdout)
    2    Command-line usage error
"#;

const SERVICE_GENERATE_HELP: &str = r#"ams-gra-codegen-oms service-generate

Generate the UCI type model a Service Contract selects, and its typed service
API wrapper, after backend/world readiness succeeds.

USAGE:
    ams-gra-codegen-oms service-generate --schema PATH --contract PATH [--extension ID=PATH]... --language LANGUAGE --world WORLD --output DIR

LANGUAGES:
    ada
    rust
    cpp

WORLDS:
    closed-schema       You assert the supplied schema set is the complete
                        value-type universe: a selected abstract structural
                        value lowers to a closed sum over every concrete
                        transitive descendant.

    open-extensions     External derived types may exist, so a selected
                        abstract structural VALUE has no closed representation
                        and generation fails closed rather than emitting a
                        partial sum.

OPTIONS:
    -s, --schema PATH          Root XSD document
    -c, --contract PATH        Portable Service Contract v0.1 (YAML or JSON)
        --extension ID=PATH    Repeatable mapping from a contract
                               standards.uci_extension_schemas IDENTIFIER to a
                               local schema document
    -l, --language LANGUAGE    Required output language
    -w, --world WORLD          Required generation world policy
    -o, --output DIR           Required output directory
    -h, --help                 Print help

EXTENSIONS USE EXACT CONTRACT MAPPING:
    Exactly the rules 'service-plan' and 'service-check' use. A contract's
    standards.uci_extension_schemas entries are logical extension IDENTIFIERS,
    not paths, so --extension is the ONLY way to supply schema content; there
    is no raw --overlay PATH option here, and supplying one is a usage error.
    The supplied set must match the contract exactly: a declared extension
    with no mapping, an undeclared extension, and a duplicate extension ID all
    fail, and no filename is ever guessed. Composition order follows the
    CONTRACT's declaration order, so reordering --extension on the command
    line cannot change generated output.

READINESS IS CHECKED FIRST:
    Task 031 readiness is the authoritative capability gate. If the selected
    model is NOT READY for the requested language and world, the same
    readiness report 'service-check' prints is written to stdout, no backend
    is invoked, no output directory is created, no file is written, and the
    command exits 1.

SELECTED TYPES ONLY:
    Only the contract-selected type closure, plus the generated support types
    its representation requires (Task 024 closed-sum concrete descendants and
    their dependencies), are emitted. Unrelated schema declarations are
    absent, so unselected declarations contribute no helper, import, or
    static assertion.

TYPED SERVICE API WRAPPER:
    One wrapper entrypoint is always emitted beside the model:

      rust  service_api.rs   (compile as the crate root; mounts the model)
      cpp   service_api.hpp  (includes the model header)
      ada   service_api.ads  (a body-less package that 'with's the model)

    It has one scope per contract function and one per exchange
    OCCURRENCE, in contract order, with ID/KIND/DIRECTION/MANDATE constants.
    Only OMS Message exchanges add TOPIC and a Payload type bound to the
    generated UCI payload type. Scope names come from contract IDs behind a
    fixed function_/exchange_ prefix, never from human-readable names; IDs
    that normalize to the same identifier fail closed.

    A contract with zero OMS Message exchanges produces zero model files and
    exactly one wrapper file.

    The wrapper sends, receives, encodes, decodes, subscribes, publishes,
    dispatches, and connects to nothing. No CAL facade, publisher/subscriber
    API, codec, or runtime source is generated.

EXIT CODES:
    0    Selected UCI type source and service API wrapper generated
    1    NOT READY, projection failure, or filesystem error
    2    Command-line usage error
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

    /// The language-neutral codegen-core identity for this CLI spelling.
    ///
    /// There is deliberately one spelling table (`Language::parse`) and one
    /// mapping into the core enum, shared by `generate` and `service-check`.
    const fn backend_language(self) -> BackendLanguage {
        match self {
            Self::Ada => BackendLanguage::Ada,
            Self::Rust => BackendLanguage::Rust,
            Self::Cpp => BackendLanguage::Cpp,
        }
    }

    /// The stable public spelling, as accepted by `--language`.
    const fn label(self) -> &'static str {
        language_label(self.backend_language())
    }
}

/// Parse the required `--world` value into the shared codegen-core policy.
///
/// The CLI owns the surface strings; the backends share one language-neutral
/// [`GenerationWorld`]. Abbreviations such as `closed` or `open` are rejected
/// deliberately: an ambiguous world assertion is exactly the failure mode this
/// option exists to prevent.
fn parse_world(value: &OsStr) -> Result<GenerationWorld, CliError> {
    match value.to_str() {
        Some("closed-schema") => Ok(GenerationWorld::ClosedSchemaSet),
        Some("open-extensions") => Ok(GenerationWorld::OpenExtensions),
        Some(value) => Err(CliError::usage(format!(
            "unsupported generation world '{value}'; expected one of: closed-schema, open-extensions"
        ))),
        None => Err(CliError::usage(
            "generation world must be valid UTF-8; expected one of: closed-schema, open-extensions",
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help(&'static str),
    Version,
    Validate {
        schema: PathBuf,
        overlays: Vec<PathBuf>,
    },
    Coverage {
        schema: PathBuf,
        overlays: Vec<PathBuf>,
        world: GenerationWorld,
    },
    Generate {
        schema: PathBuf,
        overlays: Vec<PathBuf>,
        language: Language,
        output: PathBuf,
        world: GenerationWorld,
    },
    Docs {
        schema: PathBuf,
        overlays: Vec<PathBuf>,
        output: PathBuf,
    },
    ServicePlan {
        schema: PathBuf,
        contract: PathBuf,
        /// Explicit contract-extension-identifier to overlay-path mappings, in
        /// command-line order. Command-line order is retained only for
        /// diagnostics; semantic composition order comes from the contract.
        extensions: Vec<(String, PathBuf)>,
    },
    ServiceCheck {
        schema: PathBuf,
        contract: PathBuf,
        extensions: Vec<(String, PathBuf)>,
        /// Required: readiness is a question about one specific backend.
        language: Language,
        /// Required: readiness is a question under one specific type universe.
        world: GenerationWorld,
    },
    ServiceGenerate {
        schema: PathBuf,
        contract: PathBuf,
        extensions: Vec<(String, PathBuf)>,
        language: Language,
        world: GenerationWorld,
        /// Required: unlike `service-plan`/`service-check`, this command does
        /// write files, so the destination must be stated explicitly.
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
        Command::Validate { schema, overlays } => validate(&schema, &overlays, stdout),
        Command::Coverage {
            schema,
            overlays,
            world,
        } => coverage(&schema, &overlays, world, stdout),
        Command::Generate {
            schema,
            overlays,
            language,
            output,
            world,
        } => generate(&schema, &overlays, language, &output, world, stdout),
        Command::Docs {
            schema,
            overlays,
            output,
        } => docs(&schema, &overlays, &output, stdout),
        Command::ServicePlan {
            schema,
            contract,
            extensions,
        } => service_plan(&schema, &contract, &extensions, stdout),
        Command::ServiceCheck {
            schema,
            contract,
            extensions,
            language,
            world,
        } => service_check(&schema, &contract, &extensions, language, world, stdout),
        Command::ServiceGenerate {
            schema,
            contract,
            extensions,
            language,
            world,
            output,
        } => service_generate(
            &schema,
            &contract,
            &extensions,
            language,
            world,
            &output,
            stdout,
        ),
    }
}

fn parse_args<I>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let command = args.next().ok_or_else(|| {
        CliError::usage("missing command; expected 'validate', 'coverage', 'generate', 'docs', 'service-plan', 'service-check', or 'service-generate'")
    })?;
    match command.to_str() {
        Some("-h" | "--help") => no_trailing_args(args, Command::Help(HELP)),
        Some("-V" | "--version") => no_trailing_args(args, Command::Version),
        Some("validate") => parse_validate(args.collect()),
        Some("coverage") => parse_coverage(args.collect()),
        Some("generate") => parse_generate(args.collect()),
        Some("docs") => parse_docs(args.collect()),
        Some("service-plan") => parse_service_plan(args.collect()),
        Some("service-check") => parse_service_check(args.collect()),
        Some("service-generate") => parse_service_generate(args.collect()),
        Some(command) => Err(CliError::usage(format!(
            "unknown command '{command}'; expected 'validate', 'coverage', 'generate', \
             'docs', 'service-plan', 'service-check', or 'service-generate'"
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
    let mut overlays = Vec::new();
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        "--overlay" => push_overlay(&mut overlays, value),
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::Validate {
        schema: required(schema, "--schema")?.into(),
        overlays,
    })
}

fn parse_coverage(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(COVERAGE_HELP));
    }
    let mut schema = None;
    let mut overlays = Vec::new();
    let mut world = None;
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        "--overlay" => push_overlay(&mut overlays, value),
        "-w" | "--world" => set_once(&mut world, value, "--world"),
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::Coverage {
        schema: required(schema, "--schema")?.into(),
        // Additive only: an overlay never infers or relaxes the world below.
        overlays,
        // Required, with no fallback: coverage numbers are meaningless
        // without knowing which type universe they measured.
        world: parse_world(&required(world, "--world")?)?,
    })
}

fn parse_generate(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(GENERATE_HELP));
    }
    let mut schema = None;
    let mut overlays = Vec::new();
    let mut language = None;
    let mut output = None;
    let mut world = None;
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        "--overlay" => push_overlay(&mut overlays, value),
        "-l" | "--language" => set_once(&mut language, value, "--language"),
        "-o" | "--output" => set_once(&mut output, value, "--output"),
        "-w" | "--world" => set_once(&mut world, value, "--world"),
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::Generate {
        schema: required(schema, "--schema")?.into(),
        // Additive only: an overlay never infers or relaxes the world below.
        overlays,
        language: Language::parse(&required(language, "--language")?)?,
        output: required(output, "--output")?.into(),
        // Required, with no fallback: the caller must state the semantic
        // assumption rather than inherit a silent closed-schema default.
        world: parse_world(&required(world, "--world")?)?,
    })
}

fn parse_docs(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(DOCS_HELP));
    }
    let mut schema = None;
    let mut overlays = Vec::new();
    let mut output = None;
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        "--overlay" => push_overlay(&mut overlays, value),
        "-o" | "--output" => set_once(&mut output, value, "--output"),
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::Docs {
        schema: required(schema, "--schema")?.into(),
        overlays,
        output: required(output, "--output")?.into(),
    })
}

fn parse_service_plan(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(SERVICE_PLAN_HELP));
    }
    let mut schema = None;
    let mut contract = None;
    let mut extensions = Vec::new();
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        "-c" | "--contract" => set_once(&mut contract, value, "--contract"),
        "--extension" => push_extension(&mut extensions, value),
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::ServicePlan {
        schema: required(schema, "--schema")?.into(),
        contract: required(contract, "--contract")?.into(),
        // Deliberately no --world and no --overlay here: the contract names
        // its extensions logically, and the mapping option is the only way to
        // bind those names to local schema documents.
        extensions,
    })
}

fn parse_service_check(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(SERVICE_CHECK_HELP));
    }
    let mut schema = None;
    let mut contract = None;
    let mut extensions = Vec::new();
    let mut language = None;
    let mut world = None;
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        "-c" | "--contract" => set_once(&mut contract, value, "--contract"),
        "--extension" => push_extension(&mut extensions, value),
        "-l" | "--language" => set_once(&mut language, value, "--language"),
        "-w" | "--world" => set_once(&mut world, value, "--world"),
        // `--output` and `--overlay` are rejected by this arm on purpose.
        // `service-check` generates nothing, and a contract's extensions are
        // logical identifiers whose set must stay exactly matchable.
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::ServiceCheck {
        schema: required(schema, "--schema")?.into(),
        contract: required(contract, "--contract")?.into(),
        extensions,
        // Both required, with no fallback: a readiness verdict that did not
        // state its backend and type universe would be meaningless evidence.
        language: Language::parse(&required(language, "--language")?)?,
        world: parse_world(&required(world, "--world")?)?,
    })
}

fn parse_service_generate(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(SERVICE_GENERATE_HELP));
    }
    let mut schema = None;
    let mut contract = None;
    let mut extensions = Vec::new();
    let mut language = None;
    let mut world = None;
    let mut output = None;
    parse_options(args, |option, value| match option {
        "-s" | "--schema" => set_once(&mut schema, value, "--schema"),
        "-c" | "--contract" => set_once(&mut contract, value, "--contract"),
        "--extension" => push_extension(&mut extensions, value),
        "-l" | "--language" => set_once(&mut language, value, "--language"),
        "-w" | "--world" => set_once(&mut world, value, "--world"),
        "-o" | "--output" => set_once(&mut output, value, "--output"),
        // `--overlay` is rejected here exactly as it is for `service-plan`
        // and `service-check`: the contract's declared extension identifiers
        // are the authoritative statement of what composes the schema set.
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::ServiceGenerate {
        schema: required(schema, "--schema")?.into(),
        contract: required(contract, "--contract")?.into(),
        extensions,
        language: Language::parse(&required(language, "--language")?)?,
        // Required, with no fallback, for the same reason ordinary 'generate'
        // requires it: the type-universe assumption must be stated.
        world: parse_world(&required(world, "--world")?)?,
        output: required(output, "--output")?.into(),
    })
}

/// Accumulate one `--extension ID=PATH` mapping.
///
/// The value must be an explicit `identifier=path` pair. There is intentionally
/// no path-only spelling: a contract's `uci_extension_schemas` entries are
/// logical identifiers, so inferring an ID from a filename would be a guess.
/// The pair is split at the FIRST `=` so overlay paths may themselves contain
/// `=`; identifiers never do.
fn push_extension(
    extensions: &mut Vec<(String, PathBuf)>,
    value: OsString,
) -> Result<(), CliError> {
    let text = value.to_str().ok_or_else(|| {
        CliError::usage("'--extension' value must be valid UTF-8 in the form ID=PATH")
    })?;
    let Some((id, path)) = text.split_once('=') else {
        return Err(CliError::usage(format!(
            "invalid '--extension' value '{text}'; expected ID=PATH mapping a contract \
             uci_extension_schemas identifier to a local schema path"
        )));
    };
    if id.is_empty() || path.is_empty() {
        return Err(CliError::usage(format!(
            "invalid '--extension' value '{text}'; both the contract extension identifier and \
             the schema path must be non-empty"
        )));
    }
    extensions.push((id.to_owned(), PathBuf::from(path)));
    Ok(())
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

/// Accumulate one `--overlay` value.
///
/// Deliberately not [`set_once`]: unlike the singular options, repeating
/// `--overlay` is the documented way to compose several overlays, and command
/// line order is the deterministic ordering source. Repeating the same path is
/// accepted here too; the frontend's canonical-path identity tracking loads the
/// physical document once.
fn push_overlay(overlays: &mut Vec<PathBuf>, value: OsString) -> Result<(), CliError> {
    overlays.push(value.into());
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

fn validate<W: Write>(
    schema_path: &Path,
    overlays: &[PathBuf],
    stdout: &mut W,
) -> Result<(), CliError> {
    let schema = load_schema_set_with_overlays(schema_path, overlays)
        .map_err(|error| CliError::execution(error.to_string()))?;
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

fn coverage<W: Write>(
    schema_path: &Path,
    overlays: &[PathBuf],
    world: GenerationWorld,
    stdout: &mut W,
) -> Result<(), CliError> {
    let schema = load_schema_set_with_overlays(schema_path, overlays)
        .map_err(|error| CliError::execution(error.to_string()))?;
    let analysis = CoverageAnalysis::new(&schema, world)
        .map_err(|error| CliError::execution(error.to_string()))?;
    let report = analysis
        .report()
        .map_err(|error| CliError::execution(error.to_string()))?;
    write_output(stdout, &report)
}

fn docs<W: Write>(
    schema_path: &Path,
    overlays: &[PathBuf],
    output_dir: &Path,
    stdout: &mut W,
) -> Result<(), CliError> {
    let schema = load_schema_set_with_overlays(schema_path, overlays)
        .map_err(|error| CliError::execution(error.to_string()))?;
    schema
        .validate()
        .map_err(|error| CliError::execution(format!("invalid schema IR: {error}")))?;
    let files = generate_docs(&schema).map_err(CliError::execution)?;
    write_generated_files(&files, output_dir)?;
    write_output(
        stdout,
        &format!(
            "generated {} documentation file(s)\noutput: {}\n",
            files.len(),
            output_dir.display()
        ),
    )
}

fn generate<W: Write>(
    schema_path: &Path,
    overlays: &[PathBuf],
    language: Language,
    output_dir: &Path,
    world: GenerationWorld,
    stdout: &mut W,
) -> Result<(), CliError> {
    let schema = load_schema_set_with_overlays(schema_path, overlays)
        .map_err(|error| CliError::execution(error.to_string()))?;
    schema
        .validate()
        .map_err(|error| CliError::execution(format!("invalid schema IR: {error}")))?;
    let files = language
        .backend()
        .generate(&schema, world)
        .map_err(|error| CliError::execution(error.to_string()))?;
    write_generated_files(&files, output_dir)?;
    write_output(
        stdout,
        &format!(
            "generated {} file(s)\noutput: {}\n",
            files.len(),
            output_dir.display()
        ),
    )
}

/// Resolve a portable Service Contract against a schema set and report it.
///
/// This command writes nothing to the filesystem: it loads, validates,
/// resolves, and prints. That is deliberate, because Task 030 delivers the
/// library-level plan rather than contract-driven backend generation.
fn service_plan<W: Write>(
    schema_path: &Path,
    contract_path: &Path,
    extensions: &[(String, PathBuf)],
    stdout: &mut W,
) -> Result<(), CliError> {
    let inputs = load_service_inputs(schema_path, contract_path, extensions)?;
    let closure = inputs
        .plan
        .selected_type_closure(&inputs.schema)
        .map_err(|error| CliError::execution(error.to_string()))?;
    write_output(stdout, &render_service_plan(&inputs.plan, closure.len()))
}

/// A loaded schema set and the Resolved Service Plan over it.
struct ServiceInputs {
    schema: SchemaIr,
    plan: ServicePlan,
}

/// Load a portable contract, bind its declared extensions to overlay paths,
/// compose the schema set, and resolve the plan.
///
/// Both `service-plan` and `service-check` go through this one function, so
/// there is exactly one implementation of exact extension-set matching and
/// contract-ordered overlay composition. A second copy would be free to drift
/// into accepting a mapping the other rejects.
fn load_service_inputs(
    schema_path: &Path,
    contract_path: &Path,
    extensions: &[(String, PathBuf)],
) -> Result<ServiceInputs, CliError> {
    let contract =
        load_contract(contract_path).map_err(|error| CliError::execution(error.to_string()))?;
    // Contract order is authoritative for overlay composition, so the loader
    // argument list is built from the contract's declared extension order and
    // never from the command line's option order.
    let overlays = contract_overlay_order(&contract.standards.uci_extension_schemas, extensions)?;
    let schema = load_schema_set_with_overlays(schema_path, &overlays)
        .map_err(|error| CliError::execution(error.to_string()))?;
    schema
        .validate()
        .map_err(|error| CliError::execution(format!("invalid schema IR: {error}")))?;
    let plan = resolve_service_plan(&contract, &schema)
        .map_err(|error| CliError::execution(error.to_string()))?;
    Ok(ServiceInputs { schema, plan })
}

/// Report whether one backend can render a contract's selected UCI type model
/// under one asserted generation world.
///
/// This writes no generated files. It answers a capability question about the
/// contract-selected closure only; unselected unrenderable declarations in the
/// same schema set are deliberately not consulted.
fn service_check<W: Write>(
    schema_path: &Path,
    contract_path: &Path,
    extensions: &[(String, PathBuf)],
    language: Language,
    world: GenerationWorld,
    stdout: &mut W,
) -> Result<(), CliError> {
    let inputs = load_service_inputs(schema_path, contract_path, extensions)?;
    let readiness = analyze_service_readiness(
        &inputs.plan,
        &inputs.schema,
        language.backend_language(),
        world,
    )
    .map_err(|error| CliError::execution(error.to_string()))?;
    // The full deterministic report goes to stdout even when the verdict is
    // NOT READY: a CI failure that hides the blockers is useless.
    write_output(stdout, &render_service_check(&inputs.plan, &readiness))?;
    if readiness.is_ready() {
        return Ok(());
    }
    Err(CliError::execution(format!(
        "service selection is not renderable for {} under {}",
        language.label(),
        world.label()
    )))
}

/// Generate the UCI type model a contract selects, after readiness succeeds.
///
/// Ordering is the whole point of this function. Readiness is consulted
/// before any projection, projection before any backend call, backend
/// generation before any path validation, and path validation before the
/// first byte touches the filesystem. A NOT READY or otherwise failing
/// selection therefore cannot leave a half-written output tree behind, and
/// the output directory is not even created.
fn service_generate<W: Write>(
    schema_path: &Path,
    contract_path: &Path,
    extensions: &[(String, PathBuf)],
    language: Language,
    world: GenerationWorld,
    output_dir: &Path,
    stdout: &mut W,
) -> Result<(), CliError> {
    let inputs = load_service_inputs(schema_path, contract_path, extensions)?;
    let readiness = analyze_service_readiness(
        &inputs.plan,
        &inputs.schema,
        language.backend_language(),
        world,
    )
    .map_err(|error| CliError::execution(error.to_string()))?;
    if !readiness.is_ready() {
        // Exactly the 'service-check' report, from the same helper: an
        // operator must not have to run a second command, and a second
        // formatter would be free to disagree about the blockers.
        write_output(stdout, &render_service_check(&inputs.plan, &readiness))?;
        return Err(CliError::execution(format!(
            "service selection is not renderable for {} under {}; no files were generated",
            language.label(),
            world.label()
        )));
    }

    let projection = project_service_generation_schema(&inputs.plan, &inputs.schema, world)
        .map_err(|error| CliError::execution(error.to_string()))?;
    // Task 047: lower the plan ONCE into the language-neutral service API
    // model, with the same shared preflight readiness just passed. The CLI
    // orchestrates; the backend renders the model and never sees the plan.
    //
    // That preflight includes the model/wrapper artifact boundary (shared
    // output paths, shared-scope name conflicts), so a predictable collision
    // already stopped at readiness above, before any backend call or
    // directory creation. `write_generated_files` still validates the combined
    // file list as defence in depth.
    let api_model = service_api_preflight(
        &inputs.plan,
        projection.schema(),
        language.backend_language(),
        world,
    )
    .map_err(|error| CliError::execution(error.to_string()))?;
    let backend = language.backend();
    // A contract may legitimately select zero OMS messages (a service whose
    // exchanges are all data transfer, special signal, security, or non-OMS).
    // There is no UCI type model to emit in that case, and fabricating a
    // schema type just to give the backend something to render would invent
    // meaning the contract never stated. Zero MODEL files is the honest
    // answer; the service API wrapper below is still real and still emitted.
    let model_files = if api_model.emits_type_model() {
        // The ordinary backend API, on an ordinary schema. No backend learns
        // what a Service Contract is.
        backend
            .generate(projection.schema(), world)
            .map_err(|error| CliError::execution(error.to_string()))?
    } else {
        Vec::new()
    };
    let api_files = backend
        .generate_service_api(&api_model, projection.schema())
        .map_err(|error| CliError::execution(error.to_string()))?;
    // Only now, with EVERY file rendered in memory, is the combined set
    // validated (including duplicate paths across model and wrapper) and the
    // output directory touched. A wrapper failure above leaves nothing behind.
    let mut files = model_files;
    let model_file_count = files.len();
    files.extend(api_files);
    write_generated_files(&files, output_dir)?;
    write_output(
        stdout,
        &render_service_generation(
            &inputs.plan,
            &projection,
            language,
            world,
            model_file_count,
            files.len() - model_file_count,
            output_dir,
        ),
    )
}

/// Render the deterministic selected-generation summary.
///
/// Contract-selected types and generated support types are reported on
/// separate lines on purpose: the contract selected the former, while the
/// latter exist only because generated representation (Task 024 closed sums
/// and their dependencies) needs them.
///
/// Since Task 047 generated files are likewise split: UCI model files, service
/// API wrapper files, and their total. The wrapper is never counted as a model
/// file.
fn render_service_generation(
    plan: &ServicePlan,
    projection: &ServiceGenerationProjection,
    language: Language,
    world: GenerationWorld,
    model_files: usize,
    service_api_files: usize,
    output_dir: &Path,
) -> String {
    let mut report = String::new();
    report.push_str("service contract valid\n");
    report.push_str(&format!("service: {}\n", plan.service.name));
    report.push_str(&format!("language: {}\n", language.label()));
    report.push_str(&format!("generation world: {}\n", world.label()));
    report.push('\n');
    report.push_str(&format!(
        "selected oms messages: {}\n",
        projection.selected_message_names().len()
    ));
    report.push_str(&format!(
        "contract-selected types: {}\n",
        projection.selected_type_names().len()
    ));
    report.push_str(&format!(
        "generated support types: {}\n",
        projection.generated_support_type_names().len()
    ));
    report.push_str(&format!(
        "projected schema types: {}\n",
        projection.schema().types.len()
    ));
    report.push_str(&format!("generated model files: {model_files}\n"));
    report.push_str(&format!(
        "generated service api files: {service_api_files}\n"
    ));
    report.push_str(&format!(
        "generated {} file(s)\n",
        model_files + service_api_files
    ));
    report.push_str(&format!("output: {}\n", output_dir.display()));
    report
}

/// Render the deterministic readiness report.
///
/// Every line derives from plan order or schema order, so repeated runs on
/// identical inputs are byte-identical.
fn render_service_check(plan: &ServicePlan, readiness: &ServiceBackendReadiness) -> String {
    let mut report = String::new();
    report.push_str("service contract valid\n");
    report.push_str(&format!("service: {}\n", plan.service.name));
    // Stable public spellings, never Rust enum Debug text.
    report.push_str(&format!(
        "language: {}\n",
        language_label(readiness.language)
    ));
    report.push_str(&format!("generation world: {}\n", readiness.world.label()));
    report.push('\n');
    report.push_str(&format!(
        "selected oms messages: {}\n",
        readiness.selected_messages_total
    ));
    report.push_str(&format!(
        "renderable selected oms messages: {}\n",
        readiness.selected_messages_renderable
    ));
    report.push_str(&format!(
        "selected type closure: {}\n",
        readiness.selected_types_total
    ));
    report.push_str(&format!(
        "renderable selected types: {}\n",
        readiness.selected_types_renderable
    ));
    report.push_str(&format!(
        "status: {}\n",
        if readiness.is_ready() {
            "READY"
        } else {
            "NOT READY"
        }
    ));
    if readiness.is_ready() {
        return report;
    }
    if !readiness.unsupported_types.is_empty() {
        report.push('\n');
        report.push_str("unsupported selected types:\n");
        // Schema declaration order, qualified to avoid local-name ambiguity.
        for name in &readiness.unsupported_types {
            report.push_str(&format!(
                "  {{{}}}{}\n",
                name.namespace_uri, name.local_name
            ));
        }
    }
    if let Some(blocker) = &readiness.backend_blocker {
        // A global backend precondition the projected selected schema
        // violates. Reported separately from per-message blockers because it
        // is a property of the selection as a whole, not of one message.
        report.push('\n');
        report.push_str(&format!("backend boundary: {blocker}\n"));
    }
    if let Some(blocker) = &readiness.service_api_blocker {
        // Task 047: the service API wrapper cannot be generated. This is a
        // statement about wrapper names or payload bindings, never a fake
        // unsupported UCI declaration, so it gets its own line.
        report.push('\n');
        report.push_str(&format!("service api boundary: {blocker}\n"));
    }
    // Before Task 047 every NOT READY report ended with this section, even
    // when only a backend boundary applied. That output is kept byte-for-byte;
    // the section is omitted only when the service API is the SOLE blocker,
    // since no selected message is blocked at all then.
    if readiness.blocked_messages.is_empty() && readiness.backend_blocker.is_none() {
        return report;
    }
    report.push('\n');
    report.push_str("blocked selected messages:\n");
    // Contract first-occurrence order, matching the plan's selection order.
    for blocked in &readiness.blocked_messages {
        report.push_str(&format!("  {}\n", blocked.contract_message));
        report.push_str(&format!(
            "    resolved: {{{}}}{}\n",
            blocked.message_name.namespace_uri, blocked.message_name.local_name
        ));
        report.push_str(&format!("    blocker: {}\n", blocked.blocker));
    }
    report
}

/// The stable CLI spelling of a backend language.
const fn language_label(language: BackendLanguage) -> &'static str {
    match language {
        BackendLanguage::Ada => "ada",
        BackendLanguage::Rust => "rust",
        BackendLanguage::Cpp => "cpp",
    }
}

/// Map the contract's declared extension IDs onto supplied overlay paths.
///
/// The match must be exact in both directions. A declared extension with no
/// mapping means the operator did not supply schema content the contract says
/// it needs; an undeclared mapping means the operator supplied content the
/// contract never asked for. Both silently change the resolved type universe,
/// so both fail rather than being tolerated. Duplicate IDs fail too, because
/// the second path would otherwise silently win.
///
/// The returned order is CONTRACT order. Task 029 gives overlay caller order
/// semantic significance, and the contract -- not the shell history that
/// produced this command line -- is the reproducible authority for how a
/// service composes its extensions. Task 029's rule that ordering never
/// implies duplicate-declaration precedence is untouched: duplicates still
/// fail in the frontend.
fn contract_overlay_order(
    declared: &[String],
    supplied: &[(String, PathBuf)],
) -> Result<Vec<PathBuf>, CliError> {
    let mut seen = BTreeSet::new();
    for (id, _) in supplied {
        if !seen.insert(id.as_str()) {
            return Err(CliError::usage(format!(
                "duplicate '--extension' identifier '{id}'"
            )));
        }
    }
    let declared_set = declared.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for (id, _) in supplied {
        if !declared_set.contains(id.as_str()) {
            return Err(CliError::usage(format!(
                "'--extension {id}=...' was supplied, but the service contract does not declare \
                 extension '{id}' in standards.uci_extension_schemas"
            )));
        }
    }
    let mut overlays = Vec::with_capacity(declared.len());
    for id in declared {
        let path = supplied
            .iter()
            .find(|(supplied_id, _)| supplied_id == id)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| {
                CliError::usage(format!(
                    "the service contract declares extension '{id}', but no \
                     '--extension {id}=PATH' mapping was supplied"
                ))
            })?;
        overlays.push(path);
    }
    Ok(overlays)
}

/// Render a deterministic human-readable summary of a resolved plan.
///
/// The order is the plan's order, which is the contract's order, so repeated
/// runs on the same inputs produce byte-identical output.
fn render_service_plan(plan: &ServicePlan, closure_size: usize) -> String {
    let mut report = String::new();
    report.push_str("service contract valid\n");
    report.push_str(&format!("contract version: {}\n", plan.contract_version));
    report.push_str(&format!("service: {}\n", plan.service.name));
    report.push_str(&format!("kind: {}\n", plan.service.kind));
    // Both version strings are reported side by side and never compared: no
    // documented mapping exists between the contract's logical UCI version
    // ("2.5") and the XSD root's release string (UCI 2.5 declares "002.5.0").
    report.push_str(&format!(
        "contract uci schema version: {}\n",
        plan.standards.uci_schema_version
    ));
    report.push_str(&format!(
        "schema root version: {}\n",
        plan.standards
            .schema_root_version
            .as_deref()
            .unwrap_or("(none declared)")
    ));
    // Omitted and explicitly-empty are reported differently, because the
    // contract said different things.
    match &plan.capabilities {
        None => report.push_str("capabilities: (omitted by contract)\n"),
        Some(capabilities) if capabilities.is_empty() => {
            report.push_str("capabilities: (explicitly empty)\n");
        }
        Some(capabilities) => report.push_str(&format!("capabilities: {}\n", capabilities.len())),
    }
    report.push('\n');
    report.push_str(&format!("functions: {}\n", plan.functions.len()));
    report.push_str(&format!(
        "exchange occurrences: {}\n",
        plan.exchange_occurrence_count()
    ));
    report.push_str(&format!(
        "oms message exchanges: {}\n",
        plan.oms_message_exchange_count()
    ));
    report.push_str(&format!(
        "unique uci messages: {}\n",
        plan.selected_messages().len()
    ));
    report.push_str(&format!("selected type closure: {closure_size}\n"));

    for function in &plan.functions {
        for exchange in &function.exchanges {
            report.push('\n');
            report.push_str(&format!("{} / {}\n", function.id, exchange.id()));
            match exchange {
                ResolvedExchange::OmsMessage(oms) => {
                    report.push_str(&format!("  {} {}\n", oms.direction, oms.contract_message));
                    report.push_str(&format!("  topic: {}\n", oms.topic));
                    report.push_str(&format!(
                        "  resolved: {{{}}}{}\n",
                        oms.message_name.namespace_uri, oms.message_name.local_name
                    ));
                }
                other => {
                    // Non-UCI exchanges are reported, never resolved.
                    report.push_str(&format!(
                        "  {} {} (not a uci message; preserved unresolved)\n",
                        other.direction(),
                        other.kind_str()
                    ));
                }
            }
        }
    }
    report
}

/// Validate then write a complete generated file set.
///
/// This is the single write orchestration for both `generate` and
/// `service-generate`, so there is exactly one implementation of relative-path
/// validation, duplicate-path rejection, and symlink/traversal protection. A
/// second writer would be free to be the unsafe one.
///
/// Every path is validated before the output directory is created, so a
/// backend that produced an unsafe or duplicated path leaves no directory
/// behind. An empty file set creates nothing at all.
fn write_generated_files(files: &[GeneratedFile], output_dir: &Path) -> Result<(), CliError> {
    validate_generated_files(files)?;
    if files.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(output_dir).map_err(|error| {
        CliError::execution(format!(
            "unable to create output directory {}: {error}",
            output_dir.display()
        ))
    })?;
    for file in files {
        write_generated_file(output_dir, file)?;
    }
    Ok(())
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
        assert_eq!(
            parse(&["coverage", "--help"]).unwrap(),
            Command::Help(COVERAGE_HELP)
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
                schema: "root.xsd".into(),
                overlays: Vec::new(),
            }
        );
    }

    #[test]
    fn parses_valid_coverage() {
        for (text, world) in [
            ("closed-schema", GenerationWorld::ClosedSchemaSet),
            ("open-extensions", GenerationWorld::OpenExtensions),
        ] {
            assert_eq!(
                parse(&["coverage", "--schema", "root.xsd", "--world", text]).unwrap(),
                Command::Coverage {
                    schema: "root.xsd".into(),
                    overlays: Vec::new(),
                    world,
                }
            );
        }
    }

    #[test]
    fn parses_valid_generate_languages() {
        for (name, language) in [
            ("ada", Language::Ada),
            ("rust", Language::Rust),
            ("cpp", Language::Cpp),
        ] {
            for (text, world) in [
                ("closed-schema", GenerationWorld::ClosedSchemaSet),
                ("open-extensions", GenerationWorld::OpenExtensions),
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
                        "--world",
                        text,
                    ])
                    .unwrap(),
                    Command::Generate {
                        schema: "root.xsd".into(),
                        overlays: Vec::new(),
                        language,
                        output: "out".into(),
                        world,
                    }
                );
            }
        }
    }

    // ---------------------------------------------------------------
    // Task 029 -- additive same-namespace schema overlays
    // ---------------------------------------------------------------

    fn overlay_fixture(name: &str) -> PathBuf {
        fixture("crates/xsd-frontend/tests/fixtures/schema-overlay").join(name)
    }

    /// Task 029 section 20: repeated `--overlay` accumulates in the exact order
    /// given on the command line, for all three commands. That order is the
    /// deterministic composition input, so it must never be reordered.
    #[test]
    fn parses_repeated_overlays_in_command_line_order() {
        assert_eq!(
            parse(&[
                "validate",
                "--schema",
                "root.xsd",
                "--overlay",
                "a.xsd",
                "--overlay",
                "b.xsd",
            ])
            .unwrap(),
            Command::Validate {
                schema: "root.xsd".into(),
                overlays: vec!["a.xsd".into(), "b.xsd".into()],
            }
        );
        assert_eq!(
            parse(&[
                "coverage",
                "--schema",
                "root.xsd",
                "--overlay",
                "a.xsd",
                "--overlay",
                "b.xsd",
                "--world",
                "closed-schema",
            ])
            .unwrap(),
            Command::Coverage {
                schema: "root.xsd".into(),
                overlays: vec!["a.xsd".into(), "b.xsd".into()],
                world: GenerationWorld::ClosedSchemaSet,
            }
        );
        assert_eq!(
            parse(&[
                "generate",
                "--schema",
                "root.xsd",
                "--overlay",
                "b.xsd",
                "--overlay",
                "a.xsd",
                "--language",
                "rust",
                "--output",
                "out",
                "--world",
                "open-extensions",
            ])
            .unwrap(),
            Command::Generate {
                schema: "root.xsd".into(),
                overlays: vec!["b.xsd".into(), "a.xsd".into()],
                language: Language::Rust,
                output: "out".into(),
                world: GenerationWorld::OpenExtensions,
            }
        );
    }

    /// Task 029 section 21: unlike the singular options, a repeated
    /// `--overlay` is intentional, so argument parsing must accept even the
    /// same path twice. The frontend's canonical dedupe then loads it once, so
    /// the composed schema set gains exactly one PrivateA.
    #[test]
    fn accepts_the_same_overlay_path_twice() {
        assert_eq!(
            parse(&[
                "validate",
                "--schema",
                "root.xsd",
                "--overlay",
                "a.xsd",
                "--overlay",
                "a.xsd",
            ])
            .unwrap(),
            Command::Validate {
                schema: "root.xsd".into(),
                overlays: vec!["a.xsd".into(), "a.xsd".into()],
            }
        );

        let mut stdout = Vec::new();
        run(
            vec![
                OsString::from("validate"),
                OsString::from("--schema"),
                overlay_fixture("public.xsd").into(),
                OsString::from("--overlay"),
                overlay_fixture("private-a.xsd").into(),
                OsString::from("--overlay"),
                overlay_fixture("private-a.xsd").into(),
            ],
            &mut stdout,
        )
        .expect("a repeated overlay path must not duplicate declarations");
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "schema valid\nnamespaces: 1\ntypes: 3\nmessages: 0\n"
        );
    }

    /// Task 029 section 22: `validate` composes overlays without `--world`,
    /// and its counts reflect the whole composed schema set.
    #[test]
    fn validate_with_overlay_counts_the_combined_schema_set() {
        let mut root_only = Vec::new();
        run(
            vec![
                OsString::from("validate"),
                OsString::from("--schema"),
                overlay_fixture("public.xsd").into(),
            ],
            &mut root_only,
        )
        .expect("root-only validation should succeed");
        assert_eq!(
            String::from_utf8(root_only).unwrap(),
            "schema valid\nnamespaces: 1\ntypes: 2\nmessages: 0\n"
        );

        let mut combined = Vec::new();
        run(
            vec![
                OsString::from("validate"),
                OsString::from("--schema"),
                overlay_fixture("public.xsd").into(),
                OsString::from("--overlay"),
                overlay_fixture("private-a.xsd").into(),
                OsString::from("--overlay"),
                overlay_fixture("private-b.xsd").into(),
            ],
            &mut combined,
        )
        .expect("overlay validation should succeed without --world");
        assert_eq!(
            String::from_utf8(combined).unwrap(),
            "schema valid\nnamespaces: 1\ntypes: 4\nmessages: 0\n"
        );
    }

    /// Task 029 sections 23/25: `coverage` analyzes the combined IR under both
    /// worlds, and supplying an overlay never infers a world.
    #[test]
    fn coverage_with_overlay_analyzes_the_combined_ir_in_both_worlds() {
        for world in ["closed-schema", "open-extensions"] {
            let mut stdout = Vec::new();
            run(
                vec![
                    OsString::from("coverage"),
                    OsString::from("--schema"),
                    overlay_fixture("public.xsd").into(),
                    OsString::from("--overlay"),
                    overlay_fixture("private-a.xsd").into(),
                    OsString::from("--world"),
                    OsString::from(world),
                ],
                &mut stdout,
            )
            .expect("overlay coverage should succeed");
            let report = String::from_utf8(stdout).unwrap();
            assert!(
                report.contains(world),
                "report must state which world produced it: {report}"
            );
            assert!(
                report.contains("declarations.total: 3"),
                "combined IR must be analyzed under {world}: {report}"
            );
        }

        assert_usage_error(
            &[
                "coverage",
                "--schema",
                "root.xsd",
                "--overlay",
                "private.xsd",
            ],
            "--world",
        );
    }

    /// Task 029 sections 24/25: `generate` builds from the combined IR under
    /// `closed-schema`, still fails closed under `open-extensions`, and never
    /// modifies either input document.
    #[test]
    fn generate_with_overlay_uses_the_combined_ir_without_mutating_inputs() {
        let before = [
            fs::read_to_string(overlay_fixture("public.xsd")).unwrap(),
            fs::read_to_string(overlay_fixture("private-a.xsd")).unwrap(),
        ];

        let output = TempDir::new();
        let mut stdout = Vec::new();
        run(
            vec![
                OsString::from("generate"),
                OsString::from("--schema"),
                overlay_fixture("public.xsd").into(),
                OsString::from("--overlay"),
                overlay_fixture("private-a.xsd").into(),
                OsString::from("--language"),
                OsString::from("rust"),
                OsString::from("--output"),
                output.0.clone().into(),
                OsString::from("--world"),
                OsString::from("closed-schema"),
            ],
            &mut stdout,
        )
        .expect("closed generation with an overlay should succeed");
        assert!(String::from_utf8(stdout).unwrap().starts_with("generated "));
        assert!(
            fs::read_to_string(output.0.join("overlay.rs"))
                .unwrap()
                .contains("PrivateA(PrivateA)")
        );

        let open_output = TempDir::new();
        let error = run(
            vec![
                OsString::from("generate"),
                OsString::from("--schema"),
                overlay_fixture("public.xsd").into(),
                OsString::from("--overlay"),
                overlay_fixture("private-a.xsd").into(),
                OsString::from("--language"),
                OsString::from("rust"),
                OsString::from("--output"),
                open_output.0.clone().into(),
                OsString::from("--world"),
                OsString::from("open-extensions"),
            ],
            &mut Vec::new(),
        )
        .expect_err("an overlay must not relax open-extensions");
        assert_eq!(error.exit_code(), 1);
        assert!(error.to_string().contains("open-extensions"), "{error}");

        assert_eq!(
            before,
            [
                fs::read_to_string(overlay_fixture("public.xsd")).unwrap(),
                fs::read_to_string(overlay_fixture("private-a.xsd")).unwrap(),
            ],
            "generation must not mutate either input document"
        );
    }

    /// Task 029 section 17: a missing overlay is an execution error naming the
    /// overlay path, never a silent skip.
    #[test]
    fn missing_overlay_is_an_execution_error_naming_the_path() {
        let error = run(
            vec![
                OsString::from("validate"),
                OsString::from("--schema"),
                overlay_fixture("public.xsd").into(),
                OsString::from("--overlay"),
                overlay_fixture("absent-overlay.xsd").into(),
            ],
            &mut Vec::new(),
        )
        .expect_err("a missing overlay must fail");
        assert_eq!(error.exit_code(), 1);
        assert!(error.to_string().contains("absent-overlay.xsd"), "{error}");
    }

    /// Task 028 section 34: after this task there is no implicit world, so
    /// omitting `--world` is a usage error (exit 2), not a silent
    /// closed-schema run.
    #[test]
    fn requires_explicit_world_for_generate_and_coverage() {
        assert_usage_error(&["coverage", "--schema", "root.xsd"], "--world");
        assert_usage_error(
            &[
                "generate",
                "--schema",
                "root.xsd",
                "--language",
                "rust",
                "--output",
                "out",
            ],
            "--world",
        );
    }

    /// Section 35: an abbreviated or unknown world must name the accepted
    /// values rather than guessing which one the caller meant.
    #[test]
    fn rejects_invalid_world_values() {
        for text in ["closed", "open", "foo", "ClosedSchemaSet", ""] {
            let error = parse(&["coverage", "--schema", "root.xsd", "--world", text])
                .expect_err("invalid world must be a usage error");
            assert_eq!(error.exit_code(), 2, "world '{text}' must be usage error");
            assert!(
                error.to_string().contains("closed-schema")
                    && error.to_string().contains("open-extensions"),
                "world '{text}' error must list accepted values: {error}"
            );
        }
    }

    /// Section 36: duplicate `--world` flows through the existing
    /// duplicate-option semantics rather than silently taking the last value.
    #[test]
    fn rejects_duplicate_world() {
        assert_usage_error(
            &[
                "coverage",
                "--schema",
                "root.xsd",
                "--world",
                "closed-schema",
                "--world",
                "closed-schema",
            ],
            "duplicate option '--world'",
        );
        assert_usage_error(
            &[
                "generate",
                "--schema",
                "root.xsd",
                "--language",
                "rust",
                "--output",
                "out",
                "--world",
                "closed-schema",
                "--world",
                "open-extensions",
            ],
            "duplicate option '--world'",
        );
    }

    /// Section 38: schema validity is independent of generation policy, so
    /// `validate` neither requires nor accepts `--world`.
    #[test]
    fn validate_is_world_independent() {
        assert_eq!(
            parse(&["validate", "--schema", "root.xsd"]).unwrap(),
            Command::Validate {
                schema: "root.xsd".into(),
                overlays: Vec::new(),
            }
        );
        assert_usage_error(
            &[
                "validate",
                "--schema",
                "root.xsd",
                "--world",
                "closed-schema",
            ],
            "unknown option '--world'",
        );
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
        assert_usage_error(&["coverage"], "--schema");
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
    fn coverage_report_is_deterministic() {
        let schema = fixture("tests/fixtures/codegen-order/root.xsd");
        let invoke = || {
            let mut stdout = Vec::new();
            run(
                vec![
                    OsString::from("coverage"),
                    OsString::from("--schema"),
                    schema.clone().into(),
                    OsString::from("--world"),
                    OsString::from("closed-schema"),
                ],
                &mut stdout,
            )
            .unwrap();
            String::from_utf8(stdout).unwrap()
        };
        let first = invoke();
        assert_eq!(first, invoke());
        assert!(first.contains("declarations.total: 3"));
        assert!(first.contains("backend coverage\nAda:"));
        // Section 59: the report names the world that produced it.
        assert!(first.starts_with("generation world: closed-schema\n"));
    }

    /// Section 59, open variant: the same schema reported under the other
    /// world must be self-describing too, so saved evidence files can never be
    /// confused with each other.
    #[test]
    fn coverage_report_states_open_world() {
        let schema = fixture("tests/fixtures/codegen-order/root.xsd");
        let mut stdout = Vec::new();
        run(
            vec![
                OsString::from("coverage"),
                OsString::from("--schema"),
                schema.into(),
                OsString::from("--world"),
                OsString::from("open-extensions"),
            ],
            &mut stdout,
        )
        .unwrap();
        assert!(
            String::from_utf8(stdout)
                .unwrap()
                .starts_with("generation world: open-extensions\n")
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
                OsString::from("--world"),
                OsString::from("closed-schema"),
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
                    OsString::from("--world"),
                    OsString::from("closed-schema"),
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

    /// Task 028 section 44: the world policy must not churn unrelated
    /// generated code. This fixture has no abstract value reference, so both
    /// worlds must produce byte-identical output for every backend.
    #[test]
    fn concrete_only_schema_output_is_world_independent() {
        let schema = fixture("tests/fixtures/codegen-order/root.xsd");
        for (language, generated_path) in [
            ("ada", "example-order.ads"),
            ("rust", "order.rs"),
            ("cpp", "order.hpp"),
        ] {
            let rendered = ["closed-schema", "open-extensions"].map(|world| {
                let output = TempDir::new();
                run(
                    vec![
                        OsString::from("generate"),
                        OsString::from("--schema"),
                        schema.clone().into(),
                        OsString::from("--language"),
                        OsString::from(language),
                        OsString::from("--output"),
                        output.0.clone().into(),
                        OsString::from("--world"),
                        OsString::from(world),
                    ],
                    &mut Vec::new(),
                )
                .unwrap_or_else(|error| {
                    panic!("{language} generation under {world} must succeed: {error}")
                });
                fs::read_to_string(output.0.join(generated_path)).unwrap()
            });
            assert_eq!(
                rendered[0], rendered[1],
                "{language} output must be byte-identical across worlds"
            );
        }
    }
}
