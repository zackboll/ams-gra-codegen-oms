//! Invocation-local, immutable schema snapshots for build-script generation.
use super::*;
use std::collections::BTreeMap;

/// Reuse schema loading/validation, never readiness, generated files or results.
///
/// A session is an immutable snapshot: callers must not modify schema inputs
/// during it. Drop the session before changing inputs. Nothing is cached across
/// processes, Cargo profiles, builds or source revisions. Every request still
/// loads its contract and runs the ordinary production evaluators and emitters.
#[derive(Default)]
pub struct ServiceSchemaSession {
    schemas: BTreeMap<PathBuf, SchemaIr>,
}

impl ServiceSchemaSession {
    /// Execute a standalone-equivalent service-check or service-generate request.
    /// Only Rust without extension overlays is qualified for build-script use.
    pub fn run<I, W>(&mut self, args: I, stdout: &mut W) -> Result<(), CliError>
    where
        I: IntoIterator<Item = OsString>,
        W: Write,
    {
        let (schema, contract, extensions, target, output) = match parse_args(args)? {
            Command::ServiceCheck {
                schema,
                contract,
                extensions,
                language,
                world,
                with_codec,
                ada_codec_runtime,
            } => (
                schema,
                contract,
                extensions,
                GenerationTarget {
                    language,
                    world,
                    with_codec,
                    ada_codec_runtime,
                },
                None,
            ),
            Command::ServiceGenerate {
                schema,
                contract,
                extensions,
                language,
                world,
                with_codec,
                ada_codec_runtime,
                output,
            } => (
                schema,
                contract,
                extensions,
                GenerationTarget {
                    language,
                    world,
                    with_codec,
                    ada_codec_runtime,
                },
                Some(output),
            ),
            _ => {
                return Err(CliError::usage(
                    "schema sessions require service-check or service-generate",
                ));
            }
        };
        if !extensions.is_empty() {
            return Err(CliError::usage(
                "schema sessions do not support extension overlays",
            ));
        }
        if target.language != Language::Rust {
            return Err(CliError::usage(
                "schema sessions support only Rust requests",
            ));
        }
        let contract =
            load_contract(&contract).map_err(|error| CliError::execution(error.to_string()))?;
        contract_overlay_order(&contract.standards.uci_extension_schemas, &[])?;
        let key = schema
            .canonicalize()
            .map_err(|error| CliError::execution(error.to_string()))?;
        if !self.schemas.contains_key(&key) {
            self.schemas
                .insert(key.clone(), load_validated_service_schema(&schema, &[])?);
        }
        let schema = &self.schemas[&key];
        let plan = resolve_service_plan(&contract, schema)
            .map_err(|error| CliError::execution(error.to_string()))?;
        if let Some(output) = output {
            // Move rather than clone the full schema into the existing input
            // container, and restore it even when production generation fails.
            let schema = self.schemas.remove(&key).expect("loaded schema");
            let inputs = ServiceInputs { schema, plan };
            let result = service_generate_inputs(&inputs, target, &output, stdout);
            self.schemas.insert(key, inputs.schema);
            result
        } else {
            evaluate_service_check(schema, &plan, target, stdout)
        }
    }
}
