//! Additive, render-backed readiness for backends outside the qualified coverage
//! inventory. The actual Backend identity and renderers are authoritative; no
//! language is substituted and no parallel capability/naming table is needed.
use crate::{
    Backend, CodegenError, GeneratedFile, GenerationWorld, ServiceApiModel, ServiceGenerationError,
    ServiceGenerationProjection, ServicePlan, build_service_api_model,
    project_service_generation_schema,
};
use ams_gra_oms_ir::SchemaIr;

/// An in-memory qualification of the exact files selected generation will write.
/// Model, API and projection boundaries remain distinct. Files are returned only
/// when every boundary succeeds. Optional codec qualification uses the actual
/// renderer and the very same once-lowered ServiceApiModel.
#[derive(Debug)]
pub struct RenderedServiceReadiness {
    pub language: &'static str,
    pub projection: Option<ServiceGenerationProjection>,
    pub model_blocker: Option<String>,
    pub service_api_blocker: Option<String>,
    pub codec_blocker: Option<String>,
    pub codec_measured: bool,
    pub codec_file_count: usize,
    pub model_file_count: usize,
    pub files: Vec<GeneratedFile>,
}

impl RenderedServiceReadiness {
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.projection.is_some()
            && self.model_blocker.is_none()
            && self.service_api_blocker.is_none()
            && self.codec_blocker.is_none()
    }
}

/// Project and lower through the shared pipeline, then exercise the actual
/// backend. No filesystem access and no contract reinterpretation occur here.
///
/// # Errors
/// Returns integrity/binding defects rather than disguising them as ordinary
/// backend capability blockers. Unsupported world/emission shapes are NOT READY.
pub fn analyze_rendered_service_readiness(
    plan: &ServicePlan,
    schema: &SchemaIr,
    world: GenerationWorld,
    backend: &dyn Backend,
) -> Result<RenderedServiceReadiness, ServiceGenerationError> {
    analyze(plan, schema, world, backend, None)
}

/// Codec renderer signature: syntax rendering of already authoritative inputs.
pub type ServiceCodecRenderer =
    fn(&ServiceApiModel, &SchemaIr) -> Result<Vec<GeneratedFile>, CodegenError>;

/// The same selected-service pipeline with opt-in actual codec qualification.
/// Model/API failures prevent codec measurement; no partial files are returned.
///
/// # Errors
/// Returns the same integrity/binding defects as model/API-only readiness.
pub fn analyze_rendered_service_with_codec(
    plan: &ServicePlan,
    schema: &SchemaIr,
    world: GenerationWorld,
    backend: &dyn Backend,
    codec: ServiceCodecRenderer,
) -> Result<RenderedServiceReadiness, ServiceGenerationError> {
    analyze(plan, schema, world, backend, Some(codec))
}

fn analyze(
    plan: &ServicePlan,
    schema: &SchemaIr,
    world: GenerationWorld,
    backend: &dyn Backend,
    codec: Option<ServiceCodecRenderer>,
) -> Result<RenderedServiceReadiness, ServiceGenerationError> {
    let mut result = RenderedServiceReadiness {
        language: backend.name(),
        projection: None,
        model_blocker: None,
        service_api_blocker: None,
        codec_blocker: None,
        codec_measured: false,
        codec_file_count: 0,
        model_file_count: 0,
        files: Vec::new(),
    };
    let projection = match project_service_generation_schema(plan, schema, world) {
        Ok(projection) => projection,
        Err(
            error @ (ServiceGenerationError::AbstractValue(_)
            | ServiceGenerationError::ProjectedEmissionPlan(_)),
        ) => {
            result.model_blocker = Some(error.to_string());
            return Ok(result);
        }
        Err(error) => return Err(error),
    };
    // A primitive OMS payload can require no declarations but still fails the
    // shared API binding. Zero OMS exchanges must not invent a model instead.
    let model_files = if projection.schema().types.is_empty() {
        Vec::new()
    } else {
        match backend.generate(projection.schema(), world) {
            Ok(files) => files,
            Err(error) => {
                result.model_blocker = Some(error.to_string());
                Vec::new()
            }
        }
    };
    let api_model = build_service_api_model(plan, projection.schema(), world);
    let api_files = api_model
        .as_ref()
        .map_err(|error| error.to_string())
        .and_then(|model| {
            backend
                .generate_service_api(model, projection.schema())
                .map_err(|error| error.to_string())
        });
    match api_files {
        Ok(files) if result.model_blocker.is_none() => {
            result.model_file_count = model_files.len();
            result.files = model_files;
            result.files.extend(files);
        }
        Err(error) => result.service_api_blocker = Some(error),
        Ok(_) => {}
    }
    if result.model_blocker.is_none() && result.service_api_blocker.is_none() {
        if let Some(codec) = codec {
            result.codec_measured = true;
            match codec(
                api_model.as_ref().expect("qualified API"),
                projection.schema(),
            ) {
                Ok(files) => {
                    result.codec_file_count = files.len();
                    result.files.extend(files);
                }
                Err(error) => {
                    result.codec_blocker = Some(error.to_string());
                    result.files.clear();
                }
            }
        }
    }
    result.projection = Some(projection);
    Ok(result)
}
