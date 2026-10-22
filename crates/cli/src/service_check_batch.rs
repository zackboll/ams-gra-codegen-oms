//! Schema ownership and transport only; readiness stays in the production evaluator.
use super::*;
use serde::{Deserialize, Serialize};

const HELP: &str = r#"ams-gra-codegen-oms service-check-batch

USAGE:
    ams-gra-codegen-oms service-check-batch --schema PATH --manifest PATH

Manifest JSON (unknown fields rejected):
    {"format":"service-check-batch-v1","schema_version":"002.5.0","requests":[
      {"id":"unique-id","contract":"/absolute/service.yaml",
       "language":"rust","world":"closed-schema","with_codec":false}
    ]}

Each request optionally accepts ada_codec_runtime as in service-check.
Contract paths must be absolute. Empty batches and duplicate IDs are errors.
This interface accepts a single schema set without extension overlays; contracts
declaring extensions fail exact mapping validation, never silently ignore them.
Use independent batch processes for bounded parallelism and isolated schemas.
JSON output contains ordered per-request exit_code, stdout and stderr, exactly
as standalone service-check, plus schema identity and an explicit completion count.
Every request is evaluated even after another fails. Any failure exits nonzero.
An absent/truncated output or missing completion count is NOT a passing batch.
"#;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    schema_version: String,
    requests: Vec<Request>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: String,
    contract: PathBuf,
    language: String,
    world: String,
    #[serde(default)]
    with_codec: bool,
    #[serde(default)]
    ada_codec_runtime: Option<String>,
}

#[derive(Serialize)]
struct ResultRecord<'a> {
    request: &'a Request,
    exit_code: i32,
    stdout: String,
    stderr: String,
}

#[derive(Serialize)]
struct Report<'a> {
    format: &'static str,
    schema: &'a Path,
    schema_version: &'a str,
    schema_loads: usize,
    completed: usize,
    results: Vec<ResultRecord<'a>>,
}

pub(super) fn parse(args: Vec<OsString>) -> Result<Command, CliError> {
    if is_help_request(&args) {
        return Ok(Command::Help(HELP));
    }
    let mut schema = None;
    let mut manifest = None;
    parse_options(args, |option, value| match option {
        "--schema" | "-s" => set_once(&mut schema, value, "--schema"),
        "--manifest" => set_once(&mut manifest, value, "--manifest"),
        _ => Err(CliError::usage(format!("unknown option '{option}'"))),
    })?;
    Ok(Command::ServiceCheckBatch {
        schema: required(schema, "--schema")?.into(),
        manifest: required(manifest, "--manifest")?.into(),
    })
}

fn target(request: &Request, schema: &Path) -> Result<GenerationTarget, CliError> {
    // Reuse standalone option validation, including codec/runtime restrictions.
    let mut args = vec![
        "--schema".into(),
        schema.as_os_str().to_owned(),
        "--contract".into(),
        request.contract.as_os_str().to_owned(),
        "--language".into(),
        request.language.clone().into(),
        "--world".into(),
        request.world.clone().into(),
    ];
    if request.with_codec {
        args.push("--with-codec".into());
    }
    if let Some(runtime) = &request.ada_codec_runtime {
        args.extend(["--ada-codec-runtime".into(), runtime.into()]);
    }
    let Command::ServiceCheck {
        language,
        world,
        with_codec,
        ada_codec_runtime,
        ..
    } = parse_service_check(args)?
    else {
        unreachable!("not a help request")
    };
    Ok(GenerationTarget {
        language,
        world,
        with_codec,
        ada_codec_runtime,
    })
}

pub(super) fn execute<W: Write>(
    schema_path: &Path,
    manifest_path: &Path,
    stdout: &mut W,
) -> Result<(), CliError> {
    let text = fs::read_to_string(manifest_path)
        .map_err(|e| CliError::execution(format!("batch manifest: {e}")))?;
    let manifest: Manifest =
        serde_json::from_str(&text).map_err(|e| CliError::usage(format!("batch manifest: {e}")))?;
    if manifest.format != "service-check-batch-v1"
        || manifest.schema_version.is_empty()
        || manifest.requests.is_empty()
    {
        return Err(CliError::usage(
            "invalid batch format, schema_version or empty requests",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut targets = Vec::new();
    for request in &manifest.requests {
        if request.id.trim().is_empty() || !ids.insert(&request.id) {
            return Err(CliError::usage("empty or duplicate batch request ID"));
        }
        if !request.contract.is_absolute() {
            return Err(CliError::usage("batch contract paths must be absolute"));
        }
        targets.push(target(request, schema_path)?);
    }
    let schema = load_validated_service_schema(schema_path, &[])?;
    if schema.schema_version.as_deref() != Some(&manifest.schema_version) {
        return Err(CliError::execution(
            "batch schema identity/version mismatch",
        ));
    }
    let mut results = Vec::new();
    for (request, target) in manifest.requests.iter().zip(targets) {
        let mut output = Vec::new();
        let outcome = (|| {
            let contract =
                load_contract(&request.contract).map_err(|e| CliError::execution(e.to_string()))?;
            contract_overlay_order(&contract.standards.uci_extension_schemas, &[])?;
            let plan = resolve_service_plan(&contract, &schema)
                .map_err(|e| CliError::execution(e.to_string()))?;
            evaluate_service_check(&schema, &plan, target, &mut output)
        })();
        let (exit_code, stderr) = match outcome {
            Ok(()) => (0, String::new()),
            Err(error) => (error.exit_code(), format!("error: {error}\n")),
        };
        results.push(ResultRecord {
            request,
            exit_code,
            stdout: String::from_utf8(output).map_err(|e| CliError::execution(e.to_string()))?,
            stderr,
        });
    }
    let failed = results.iter().any(|result| result.exit_code != 0);
    let report = Report {
        format: "service-check-batch-results-v1",
        schema: schema_path,
        schema_version: &manifest.schema_version,
        schema_loads: 1,
        completed: results.len(),
        results,
    };
    let json = serde_json::to_string(&report).map_err(|e| CliError::execution(e.to_string()))?;
    write_output(stdout, &(json + "\n"))?;
    if failed {
        return Err(CliError::execution(
            "one or more batch service checks failed",
        ));
    }
    Ok(())
}
