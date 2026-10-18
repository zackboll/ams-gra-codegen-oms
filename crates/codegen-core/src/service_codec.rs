//! Task 050: generated OMS JSON payload-codec readiness.
//!
//! A separate, opt-in readiness dimension. It never changes model readiness,
//! a selected-type count, or any default report line. It answers: can
//! `service-generate --with-codec` emit a codec for this projected selected
//! schema without guessing a wire rule?
//!
//! The wire rules are OMSC-SPC-013 Rev B section 6.1 (see
//! `docs/task-050-rust-oms-json-codecs.md` and
//! `docs/task-051-member-qname-provenance.md`). Two boundaries are deliberate:
//!
//! * **Member names (Task 051).** A particle member is keyed by its ELEMENT
//!   DECLARATION's target namespace (`FieldDecl::wire_name`): bare local name
//!   for the OAM namespace, `{namespace}local` otherwise ([`oms_json_member_name`]).
//!   `$type` values use the concrete complexType's QName the same way
//!   ([`oms_json_type_name`]). A local element whose target namespace is
//!   ABSENT (unqualified) has no evidenced OMS JSON spelling, so it fails
//!   closed ([`ServiceCodecError::UnqualifiedMember`]).
//! * **Binary (Task 052).** `PrimitiveKind::Binary` is semantic octets; its
//!   OMS JSON string spelling depends on which XSD primitive it came from.
//!   The encoding is read ONLY from IR lexical provenance
//!   (`ams_gra_oms_ir::resolve_binary_encoding` /
//!   `declaration_binary_encoding`), never assumed from the value kind:
//!   `HexBinary` is supported; UNKNOWN provenance fails closed
//!   ([`ServiceCodecError::UnknownBinaryEncoding`]); `Base64Binary` fails
//!   closed as unimplemented ([`ServiceCodecError::UnsupportedBinaryEncoding`]).
//!
//! The backend single-namespace boundary is NOT relaxed here: normal backend
//! preflight still rejects a projection spanning several namespaces.

use crate::{
    BackendLanguage, EffectiveValueMember, GenerationWorld, ServiceApiModel, TypeEmission,
    effective_choice_alternatives, effective_record_fields, field_storage_semantics,
    plan_type_emissions, rust_model_file_name, service_api_fixed_names,
};
use ams_gra_oms_ir::{
    BinaryLexicalEncoding, FieldDecl, OccurrenceShape, PrimitiveKind, SchemaIr, TypeKind,
    TypeRefTarget, declaration_binary_encoding, resolve_binary_encoding,
};
use std::fmt;

const ADA_CODEC_FIXED_NAMES: &[&str] = &[
    "Model",
    "Codec_Error",
    "Require_Kind",
    "Scalar_Boolean",
    "Scalar_String",
    "Scalar_Integer",
    "Member",
    "Scalar_Date_Time",
    "Scalar_Duration",
];

/// Explicit deployment contract for Ada codecs. The default never admits
/// Float32; the qualified profile must also be enforced by the target runtime.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AdaCodecRuntime {
    #[default]
    Portable,
    LinuxX86_64Glibc241,
}

/// The OAM namespace (OMSC-SPC-013 Rev B section 6.1): members of element
/// declarations in this namespace are keyed by their bare local name.
pub const OAM_NAMESPACE: &str = "https://www.vdl.afrl.af.mil/programs/oam";

/// The OMS JSON member name of a QUALIFIED element particle
/// (OMSC-SPC-013 Rev B section 6.1.2): `local_name` when `namespace_uri` is
/// the OAM namespace, otherwise `{namespace_uri}local_name`.
///
/// The one shared formatter for Record member keys and Choice alternative
/// keys. An unqualified (absent-namespace) element never reaches it: codec
/// readiness rejects it first.
#[must_use]
pub fn oms_json_member_name(namespace_uri: &str, local_name: &str) -> String {
    oms_json_qname(namespace_uri, local_name)
}

/// The OMS JSON `"$type"` value naming a concrete complexType
/// (OMSC-SPC-013 Rev B section 6.1.2): `{complexType name}` in the OAM
/// namespace, otherwise `{{complexType target namespace}}{complexType name}`.
/// Always derived from the concrete type declaration's own QName.
#[must_use]
pub fn oms_json_type_name(namespace_uri: &str, local_name: &str) -> String {
    oms_json_qname(namespace_uri, local_name)
}

fn oms_json_qname(namespace_uri: &str, local_name: &str) -> String {
    if namespace_uri == OAM_NAMESPACE {
        local_name.to_owned()
    } else {
        format!("{{{namespace_uri}}}{local_name}")
    }
}

/// The Rust codec file written beside the model and `service_api.rs`.
pub const RUST_SERVICE_CODEC_FILE: &str = "service_codec.rs";

/// The Rust module `service_api.rs` mounts the codec file as.
pub const RUST_SERVICE_CODEC_MODULE: &str = "service_codec";

/// Why a codec cannot be generated for a projected selected schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceCodecError {
    /// No generated OMS JSON codec exists for this language.
    LanguageNotImplemented(BackendLanguage),
    /// A stored member is an unqualified local element (absent target
    /// namespace). OMSC-SPC-013 Rev B gives no unambiguous member name for
    /// it, so no spelling is guessed (Task 051).
    UnqualifiedMember { location: String },
    /// Task 052: a Binary value whose XSD lexical primitive is unknown in the
    /// IR (synthetic/manual Binary). No lexical family is assumed.
    UnknownBinaryEncoding { location: String },
    /// Task 052: a Binary value whose XSD lexical primitive is known but has
    /// no generated codec mapping (`base64Binary`).
    UnsupportedBinaryEncoding {
        location: String,
        encoding: BinaryLexicalEncoding,
    },
    /// A primitive with no evidenced codec mapping in this task.
    UnsupportedPrimitive {
        location: String,
        kind: PrimitiveKind,
    },
    /// Any other construct the codec does not represent.
    UnsupportedConstruct { location: String, reason: String },
    /// The codec file or module would collide with another generated artifact.
    ArtifactCollision { artifact: String },
    /// Type emission planning failed (an integrity defect; projection already
    /// planned the same schema).
    EmissionPlan(String),
}

impl fmt::Display for ServiceCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LanguageNotImplemented(language) => write!(
                f,
                "generated OMS JSON codec is not implemented for {}",
                language.name()
            ),
            Self::UnqualifiedMember { location } => write!(
                f,
                "{location} has an unqualified local element with no evidenced OMS JSON \
                 member-name mapping"
            ),
            Self::UnknownBinaryEncoding { location } => write!(
                f,
                "{location} is Binary but its XSD lexical encoding provenance is unknown"
            ),
            Self::UnsupportedBinaryEncoding { location, encoding } => write!(
                f,
                "{location} is Binary with XSD lexical encoding {}; the {} codec \
                 mapping is not implemented",
                encoding.xsd_name(),
                encoding.xsd_name()
            ),
            Self::UnsupportedPrimitive { location, kind } => write!(
                f,
                "{location} is {kind:?}, which has no generated OMS JSON codec mapping"
            ),
            Self::UnsupportedConstruct { location, reason } => write!(f, "{location}: {reason}"),
            Self::ArtifactCollision { artifact } => write!(
                f,
                "codec file '{RUST_SERVICE_CODEC_FILE}' (module '{RUST_SERVICE_CODEC_MODULE}') \
                 collides with generated artifact '{artifact}'"
            ),
            Self::EmissionPlan(message) => {
                write!(
                    f,
                    "codec preflight could not plan the projected model: {message}"
                )
            }
        }
    }
}

impl std::error::Error for ServiceCodecError {}

/// Codec readiness for one projected selected schema in one language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCodecReadiness {
    pub language: BackendLanguage,
    /// Whether any OMS exchange exists; without one the codec is vacuous and
    /// no codec file is written.
    pub has_oms_exchanges: bool,
    /// Planned emitted declarations the codec must cover: every
    /// `TypeEmission` that owns a generated top-level type (abstract Records
    /// kept only as inheritance metadata are excluded), INCLUDING generated
    /// support declarations. This is deliberately not the selected-type
    /// closure used by model readiness.
    pub emitted_declarations: usize,
    /// How many of those the codec can encode/decode.
    pub renderable_emitted_declarations: usize,
    /// The first blocker, in emission order, if any.
    pub blocker: Option<ServiceCodecError>,
}

impl ServiceCodecReadiness {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.blocker.is_none()
    }

    /// Whether `service-generate --with-codec` writes a codec file.
    #[must_use]
    pub const fn emits_codec_file(&self) -> bool {
        self.has_oms_exchanges && self.blocker.is_none()
    }
}

/// Measure codec readiness for an already-lowered service API model over the
/// same projected schema `service-generate` hands to the backend.
///
/// # Errors
///
/// [`ServiceCodecError::EmissionPlan`] when the projection cannot be planned
/// (an integrity defect, never an ordinary NOT READY).
pub fn analyze_service_codec(
    model: &ServiceApiModel,
    schema: &SchemaIr,
    language: BackendLanguage,
    world: GenerationWorld,
) -> Result<ServiceCodecReadiness, ServiceCodecError> {
    analyze_service_codec_with_runtime(model, schema, language, world, AdaCodecRuntime::Portable)
}

/// Analyze codecs against an explicitly selected deployment runtime.
///
/// # Errors
/// Returns emission-plan errors exactly as the portable analyzer does.
pub fn analyze_service_codec_with_runtime(
    model: &ServiceApiModel,
    schema: &SchemaIr,
    language: BackendLanguage,
    world: GenerationWorld,
    runtime: AdaCodecRuntime,
) -> Result<ServiceCodecReadiness, ServiceCodecError> {
    let emissions = if schema.types.is_empty() {
        Vec::new()
    } else {
        plan_type_emissions(schema, world)
            .map_err(|error| ServiceCodecError::EmissionPlan(error.message))?
    };
    let emitted: Vec<&TypeEmission<'_>> = emissions
        .iter()
        .filter(|emission| emission.emits_own_top_level_name())
        .collect();
    let has_oms_exchanges = model.has_oms_exchanges();
    let mut readiness = ServiceCodecReadiness {
        language,
        has_oms_exchanges,
        emitted_declarations: emitted.len(),
        renderable_emitted_declarations: 0,
        blocker: None,
    };
    if !has_oms_exchanges {
        return Ok(readiness);
    }
    if language == BackendLanguage::Ada {
        let package = crate::ada_model_package(schema)
            .map_err(|error| ServiceCodecError::EmissionPlan(error.message))?;
        if runtime != AdaCodecRuntime::Portable
            && package.iter().any(|part| {
                ["AMS_GRA_OMS_JSON_Float32", "AMS_GRA_OMS_JSON_Occurrences"]
                    .iter()
                    .any(|fixed| part.eq_ignore_ascii_case(fixed))
            })
        {
            readiness.blocker = Some(unsupported(
                "Service_Codec",
                "model package collision".into(),
            ));
            return Ok(readiness);
        }
        if package.iter().any(|part| {
            [
                "Service_Codec",
                "AMS_GRA_OMS_JSON_Guard",
                "GNATCOLL",
                "Interfaces",
                "Ada",
            ]
            .iter()
            .any(|fixed| part.eq_ignore_ascii_case(fixed))
        }) {
            readiness.blocker = Some(unsupported(
                "Service_Codec",
                "model package collision".into(),
            ));
            return Ok(readiness);
        }
        let files = crate::ada_model_file_names(&package);
        for artifact in [files.parent_spec, files.spec, files.body] {
            if ["service_codec.ads", "service_codec.adb"]
                .iter()
                .any(|codec| artifact.eq_ignore_ascii_case(codec))
            {
                readiness.blocker = Some(unsupported(
                    "Service_Codec",
                    format!("artifact collision: {artifact}"),
                ));
                return Ok(readiness);
            }
        }
        // Names in the codec live in a separate library package, not the model
        // or wrapper scope. Reserve the fixed local names and both public/private
        // operation families case-insensitively before rendering any artifact.
        let mut names = std::collections::BTreeSet::new();
        for fixed in ADA_CODEC_FIXED_NAMES {
            names.insert(fixed.to_ascii_lowercase());
        }
        if runtime != AdaCodecRuntime::Portable {
            for fixed in ["Scalar_Float32", "Ingress"] {
                names.insert(fixed.to_ascii_lowercase());
            }
        }
        for emission in &emitted {
            if let TypeEmission::Declaration(declaration) = emission {
                for prefix in ["To_JSON_", "From_JSON_", "Encode_", "Decode_"] {
                    let name = format!("{prefix}{}", declaration.name.local_name);
                    if !names.insert(name.to_ascii_lowercase()) {
                        readiness.blocker =
                            Some(unsupported(&name, "codec identifier collision".into()));
                        return Ok(readiness);
                    }
                }
            }
        }
        let mut analysis = AdaCodecAnalysis::new(&emissions);
        analysis.runtime = runtime;
        for emission in emitted {
            match analysis.support(schema, emission, world) {
                Ok(()) => readiness.renderable_emitted_declarations += 1,
                Err(error) if readiness.blocker.is_none() => readiness.blocker = Some(error),
                Err(_) => {}
            }
        }
        return Ok(readiness);
    }
    if language != BackendLanguage::Rust {
        readiness.blocker = Some(ServiceCodecError::LanguageNotImplemented(language));
        return Ok(readiness);
    }
    if let Err(error) = check_rust_codec_artifact(schema) {
        readiness.blocker = Some(error);
        return Ok(readiness);
    }
    for emission in emitted {
        match emission_codec_support(schema, emission, world) {
            Ok(()) => readiness.renderable_emitted_declarations += 1,
            Err(error) => {
                if readiness.blocker.is_none() {
                    readiness.blocker = Some(error);
                }
            }
        }
    }
    Ok(readiness)
}

/// The codec file must be neither the model file nor the wrapper (compared
/// case-insensitively), and its module must not reuse `model`/`service_api`.
fn check_rust_codec_artifact(schema: &SchemaIr) -> Result<(), ServiceCodecError> {
    let fixed = service_api_fixed_names(BackendLanguage::Rust);
    if !schema.types.is_empty() {
        let model = rust_model_file_name(schema)
            .map_err(|error| ServiceCodecError::EmissionPlan(error.message))?;
        if model.eq_ignore_ascii_case(RUST_SERVICE_CODEC_FILE) {
            return Err(ServiceCodecError::ArtifactCollision { artifact: model });
        }
    }
    if fixed.file.eq_ignore_ascii_case(RUST_SERVICE_CODEC_FILE) {
        return Err(ServiceCodecError::ArtifactCollision {
            artifact: fixed.file.to_owned(),
        });
    }
    for module in [fixed.model_module.unwrap_or_default(), fixed.root] {
        if module == RUST_SERVICE_CODEC_MODULE {
            return Err(ServiceCodecError::ArtifactCollision {
                artifact: module.to_owned(),
            });
        }
    }
    Ok(())
}

fn emission_codec_support(
    schema: &SchemaIr,
    emission: &TypeEmission<'_>,
    world: GenerationWorld,
) -> Result<(), ServiceCodecError> {
    let declaration = match emission {
        TypeEmission::Declaration(declaration) => *declaration,
        // `$type` values are every concrete descendant's own type QName, which
        // is always known (`oms_json_type_name`). Each concrete descendant is
        // itself an emitted Declaration and its members are checked there.
        TypeEmission::AbstractValue(_) => return Ok(()),
    };
    let owner = &declaration.name.local_name;
    match &declaration.kind {
        // Task 052: a named Binary's lexical family is its declaration's
        // restriction ancestry, resolved by the one shared IR query.
        TypeKind::Primitive(PrimitiveKind::Binary) => {
            binary_support(declaration_binary_encoding(schema, declaration), owner)
        }
        TypeKind::Primitive(PrimitiveKind::Time) => {
            match crate::temporal_profile(PrimitiveKind::Time, &declaration.constraints) {
                Ok(Some(crate::TemporalProfile::TimeZulu)) => Ok(()),
                _ => Err(unsupported(
                    owner,
                    "unsupported named Time constraints".into(),
                )),
            }
        }
        TypeKind::Primitive(PrimitiveKind::String) => {
            crate::string_profile(PrimitiveKind::String, &declaration.constraints)
                .map(|_| ())
                .map_err(|reason| unsupported(owner, reason.to_string()))
        }
        TypeKind::Primitive(kind) => primitive_support(*kind, owner),
        TypeKind::Enumeration { .. } => Ok(()),
        TypeKind::Record { .. } => {
            let fields = effective_record_fields(schema, &declaration.name).map_err(|error| {
                unsupported(owner, format!("record projection failed: {error:?}"))
            })?;
            for field in fields {
                let stored = field_storage_semantics(schema, field, world)
                    .map_err(|error| unsupported(owner, error.to_string()))?;
                if matches!(stored, EffectiveValueMember::AbsentOnly(_)) {
                    // Task 026: encoded as nothing, decoded as rejected.
                    continue;
                }
                member_support(schema, field, owner)?;
            }
            Ok(())
        }
        TypeKind::Choice { .. } => {
            let alternatives =
                effective_choice_alternatives(schema, &declaration.name).map_err(|error| {
                    unsupported(owner, format!("choice projection failed: {error:?}"))
                })?;
            for alternative in alternatives {
                if alternative.cardinality.shape() == OccurrenceShape::OptionalOne {
                    return Err(unsupported(
                        &format!("{owner}.{}", alternative.name),
                        "optional Choice alternative has no unambiguous OMS JSON form".into(),
                    ));
                }
                member_support(schema, alternative, owner)?;
            }
            Ok(())
        }
        TypeKind::Alias(_) | TypeKind::List { .. } => Err(unsupported(
            owner,
            "alias/list declarations have no generated codec".into(),
        )),
    }
}

fn member_support(
    schema: &SchemaIr,
    field: &FieldDecl,
    owner: &str,
) -> Result<(), ServiceCodecError> {
    let location = format!("{owner}.{}", field.name);
    // Task 051: the member key comes from the ELEMENT's own target namespace,
    // never the owning type's, the message's, or the member type's.
    if field.wire_name().namespace_uri.is_none() {
        return Err(ServiceCodecError::UnqualifiedMember { location });
    }
    if field.nillable {
        return Err(unsupported(&location, "nillable member".into()));
    }
    match &field.type_ref.target {
        // Task 052: a direct Binary reference carries its own provenance.
        TypeRefTarget::Primitive(PrimitiveKind::Binary) => {
            binary_support(resolve_binary_encoding(schema, &field.type_ref), &location)
        }
        TypeRefTarget::Primitive(kind) => primitive_support(*kind, &location),
        // A named member type is itself an emitted declaration, checked there.
        TypeRefTarget::Named(_) => Ok(()),
    }
}

/// Task 052: the codec decision for one Binary value, from its RESOLVED
/// lexical provenance only. There is deliberately no path from
/// `PrimitiveKind::Binary` alone to a hex encoding.
fn binary_support(
    encoding: Option<BinaryLexicalEncoding>,
    location: &str,
) -> Result<(), ServiceCodecError> {
    match encoding {
        Some(BinaryLexicalEncoding::HexBinary) => Ok(()),
        Some(encoding @ BinaryLexicalEncoding::Base64Binary) => {
            Err(ServiceCodecError::UnsupportedBinaryEncoding {
                location: location.to_owned(),
                encoding,
            })
        }
        None => Err(ServiceCodecError::UnknownBinaryEncoding {
            location: location.to_owned(),
        }),
    }
}

/// Non-Binary primitives. Binary never reaches here: it is decided by
/// [`binary_support`] from its provenance.
fn primitive_support(kind: PrimitiveKind, location: &str) -> Result<(), ServiceCodecError> {
    match kind {
        PrimitiveKind::Boolean
        | PrimitiveKind::SignedInteger
        | PrimitiveKind::UnsignedInteger
        | PrimitiveKind::Float32
        | PrimitiveKind::Float64
        | PrimitiveKind::String
        | PrimitiveKind::DateTime
        // Task 057: OMSC-SPC-013 Rev B section 6.1.4 maps a simple type by
        // its {primitive type definition} only; xs:duration matches none of
        // cases 1-4, so case 5 ("Otherwise string") applies, and section
        // 6.1.5.4 makes that string's characters the XML lexical value.
        | PrimitiveKind::Duration => Ok(()),
        PrimitiveKind::Binary => Err(ServiceCodecError::UnknownBinaryEncoding {
            location: location.to_owned(),
        }),
        PrimitiveKind::Decimal | PrimitiveKind::Time => {
            Err(ServiceCodecError::UnsupportedPrimitive {
                location: location.to_owned(),
                kind,
            })
        }
    }
}

fn unsupported(location: &str, reason: String) -> ServiceCodecError {
    ServiceCodecError::UnsupportedConstruct {
        location: location.to_owned(),
        reason,
    }
}

fn ada_scalar_support(
    kind: PrimitiveKind,
    constraints: &ams_gra_oms_ir::ConstraintSet,
    named: bool,
    location: &str,
) -> Result<(), ServiceCodecError> {
    use crate::InclusiveIntegralDomain;
    match kind {
        PrimitiveKind::Duration if named => match crate::temporal_profile(kind, constraints) {
            Ok(Some(crate::TemporalProfile::Duration)) => Ok(()),
            Ok(_) => Err(unsupported(
                location,
                "unsupported named Duration profile".into(),
            )),
            Err(reason) => Err(unsupported(location, reason.to_string())),
        },
        PrimitiveKind::Duration => match crate::direct_temporal_profile(kind, constraints) {
            Ok(Some(crate::DirectTemporalProfile::Duration)) => Ok(()),
            Ok(_) => Err(unsupported(
                location,
                "unsupported direct Duration profile".into(),
            )),
            Err(reason) => Err(unsupported(location, format!("{reason:?}"))),
        },
        PrimitiveKind::DateTime if named => match crate::temporal_profile(kind, constraints) {
            Ok(Some(crate::TemporalProfile::DateTimeZulu)) => Ok(()),
            Err(reason) => Err(unsupported(location, reason.to_string())),
            _ => Err(unsupported(
                location,
                "unsupported named DateTime profile".into(),
            )),
        },
        PrimitiveKind::DateTime => match crate::direct_temporal_profile(kind, constraints) {
            Ok(Some(crate::DirectTemporalProfile::DateTime)) => Ok(()),
            Err(reason) => Err(unsupported(location, format!("{reason:?}"))),
            _ => Err(unsupported(
                location,
                "unsupported direct DateTime profile".into(),
            )),
        },
        PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger => {
            let domain = if named {
                crate::named_integral_domain(kind, constraints)
            } else {
                crate::inclusive_integral_domain(kind, constraints)
            }
            .map_err(|reason| unsupported(location, reason.into()))?;
            match (kind, domain) {
                (
                    PrimitiveKind::SignedInteger,
                    None | Some(InclusiveIntegralDomain::Signed { .. }),
                ) => Ok(()),
                (_, Some(InclusiveIntegralDomain::Unsigned { max, .. }))
                    if max <= i64::MAX as u64 =>
                {
                    Ok(())
                }
                _ => Err(unsupported(
                    location,
                    "integer domain exceeds signed-64 JSON boundary".into(),
                )),
            }
        }
        PrimitiveKind::String if named => crate::string_profile(kind, constraints)
            .map(|_| ())
            .map_err(|reason| unsupported(location, reason.to_string())),
        PrimitiveKind::String | PrimitiveKind::Boolean if *constraints == Default::default() => {
            Ok(())
        }
        _ => Err(unsupported(
            location,
            format!("unsupported Ada codec primitive or facets: {kind:?}"),
        )),
    }
}

fn ada_runtime_scalar_support(
    kind: PrimitiveKind,
    constraints: &ams_gra_oms_ir::ConstraintSet,
    named: bool,
    location: &str,
    runtime: AdaCodecRuntime,
) -> Result<(), ServiceCodecError> {
    if kind != PrimitiveKind::Float32 || runtime == AdaCodecRuntime::Portable {
        return ada_scalar_support(kind, constraints, named, location);
    }
    if !named && *constraints != Default::default() {
        return Err(unsupported(location, "field-local Float32 facets".into()));
    }
    crate::floating_domain(kind, constraints)
        .map(|_| ())
        .map_err(|reason| unsupported(location, reason.to_string()))
}

#[cfg(test)]
fn ada_codec_support(
    schema: &SchemaIr,
    emission: &TypeEmission<'_>,
    world: GenerationWorld,
) -> Result<(), ServiceCodecError> {
    let emissions = plan_type_emissions(schema, world)
        .map_err(|error| ServiceCodecError::EmissionPlan(error.message))?;
    AdaCodecAnalysis::new(&emissions).support(schema, emission, world)
}

struct AdaCodecAnalysis<'a, 's> {
    runtime: AdaCodecRuntime,
    emitted: std::collections::BTreeMap<ams_gra_oms_ir::QualifiedName, &'a TypeEmission<'s>>,
    capability:
        std::collections::BTreeMap<ams_gra_oms_ir::QualifiedName, Result<(), ServiceCodecError>>,
    record_values:
        std::collections::BTreeMap<ams_gra_oms_ir::QualifiedName, Result<(), ServiceCodecError>>,
}

impl<'a, 's> AdaCodecAnalysis<'a, 's> {
    fn new(emissions: &'a [TypeEmission<'s>]) -> Self {
        Self {
            runtime: AdaCodecRuntime::Portable,
            emitted: emissions
                .iter()
                .filter(|item| item.emits_own_top_level_name())
                .map(|item| (item.name().clone(), item))
                .collect(),
            capability: Default::default(),
            record_values: Default::default(),
        }
    }

    fn support(
        &mut self,
        schema: &SchemaIr,
        emission: &TypeEmission<'_>,
        world: GenerationWorld,
    ) -> Result<(), ServiceCodecError> {
        if let Some(result) = self.capability.get(emission.name()) {
            return result.clone();
        }
        // The planner rejects cycles. Keep a fail-closed in-progress sentinel
        // as well, rather than allowing an accidental re-entry to recurse.
        self.capability.insert(
            emission.name().clone(),
            Err(unsupported(
                &emission.name().local_name,
                "cyclic codec dependency".into(),
            )),
        );
        #[cfg(test)]
        ADA_CAPABILITY_VISITS.with(|visits| {
            *visits
                .borrow_mut()
                .entry(emission.name().clone())
                .or_default() += 1;
        });
        let result = self.support_uncached(schema, emission, world);
        self.capability
            .insert(emission.name().clone(), result.clone());
        result
    }

    fn support_uncached(
        &mut self,
        schema: &SchemaIr,
        emission: &TypeEmission<'_>,
        world: GenerationWorld,
    ) -> Result<(), ServiceCodecError> {
        let TypeEmission::Declaration(declaration) = emission else {
            return Err(unsupported(
                "abstract value",
                "polymorphic Ada codec is unsupported".into(),
            ));
        };
        let owner = &declaration.name.local_name;
        if declaration.is_abstract {
            return Err(unsupported(
                owner,
                "abstract Ada codec is unsupported".into(),
            ));
        }
        match &declaration.kind {
            TypeKind::Primitive(kind) => ada_runtime_scalar_support(
                *kind,
                &declaration.constraints,
                true,
                owner,
                self.runtime,
            ),
            TypeKind::Enumeration { .. } => Ok(()),
            TypeKind::Record { .. } => {
                let fields = effective_record_fields(schema, &declaration.name)
                    .map_err(|error| unsupported(owner, format!("{error:?}")))?;
                for field in fields {
                    let location = format!("{owner}.{}", field.name);
                    if field.wire_name().namespace_uri.is_none() {
                        return Err(ServiceCodecError::UnqualifiedMember { location });
                    }
                    if field.nillable {
                        return Err(unsupported(&location, "nillable member".into()));
                    }
                    let repeated = match field.cardinality.shape() {
                        OccurrenceShape::RequiredOne | OccurrenceShape::OptionalOne => false,
                        OccurrenceShape::Bounded { min, max }
                            if max > 1
                                && min <= max
                                && max <= crate::ADA_PORTABLE_POSITIVE_INDEX_MAX =>
                        {
                            true
                        }
                        OccurrenceShape::Unbounded { min }
                            if min <= crate::ADA_PORTABLE_POSITIVE_INDEX_MAX =>
                        {
                            true
                        }
                        _ => {
                            return Err(unsupported(
                                &location,
                                "unsupported repeated cardinality".into(),
                            ));
                        }
                    };
                    if !matches!(
                        field_storage_semantics(schema, field, world)
                            .map_err(|error| unsupported(&location, error.to_string()))?,
                        EffectiveValueMember::Stored(_)
                    ) {
                        return Err(unsupported(&location, "abstract member".into()));
                    }
                    match field.type_ref.target {
                        TypeRefTarget::Primitive(kind) => {
                            // Repeated temporal storage uses the same checked carrier as
                            // single values; retain the full direct-profile facet check.
                            ada_runtime_scalar_support(
                                kind,
                                &field.constraints,
                                false,
                                &location,
                                self.runtime,
                            )?
                        }
                        TypeRefTarget::Named(_) if field.constraints != Default::default() => {
                            return Err(unsupported(&location, "field-local named facets".into()));
                        }
                        TypeRefTarget::Named(ref name) if repeated => {
                            let referenced = self.emitted.get(name).copied().ok_or_else(|| {
                                unsupported(&location, "named repeated value is not emitted".into())
                            })?;
                            if !matches!(referenced, TypeEmission::Declaration(d) if matches!(d.kind,
                                TypeKind::Primitive(_) | TypeKind::Enumeration { .. } | TypeKind::Record { .. } | TypeKind::Choice { .. }))
                            {
                                return Err(unsupported(
                                    &location,
                                    "alias/list repeated element is unsupported".into(),
                                ));
                            }
                            // The emitted declaration must pass its existing named
                            // scalar classifier, including temporal profile constraints.
                            self.support(schema, referenced, world)?;
                            self.record_support(schema, referenced, world)?;
                        }
                        TypeRefTarget::Named(_) => {}
                    }
                }
                Ok(())
            }
            TypeKind::Choice { .. } => {
                let alternatives = effective_choice_alternatives(schema, &declaration.name)
                    .map_err(|error| unsupported(owner, error.to_string()))?;
                let mut keys = std::collections::BTreeSet::new();
                for field in alternatives {
                    let location = format!("{owner}.{}", field.name);
                    if field.cardinality.shape() != OccurrenceShape::RequiredOne {
                        return Err(unsupported(
                            &location,
                            "Choice alternative must be required-single".into(),
                        ));
                    }
                    let Some(namespace) = field.wire_name().namespace_uri else {
                        return Err(ServiceCodecError::UnqualifiedMember { location });
                    };
                    if !keys.insert(oms_json_member_name(namespace, &field.name)) {
                        return Err(unsupported(
                            &location,
                            "duplicate effective Choice JSON key".into(),
                        ));
                    }
                    if field.nillable {
                        return Err(unsupported(&location, "nillable Choice alternative".into()));
                    }
                    if !matches!(
                        field_storage_semantics(schema, field, world)
                            .map_err(|error| unsupported(&location, error.to_string()))?,
                        EffectiveValueMember::Stored(_)
                    ) {
                        return Err(unsupported(&location, "abstract Choice alternative".into()));
                    }
                    match &field.type_ref.target {
                        TypeRefTarget::Primitive(PrimitiveKind::Float32)
                            if self.runtime != AdaCodecRuntime::Portable =>
                        {
                            return Err(unsupported(
                                &location,
                                "direct Float32 Choice alternative is not qualified".into(),
                            ));
                        }
                        TypeRefTarget::Primitive(PrimitiveKind::DateTime) => {
                            return Err(unsupported(
                                &location,
                                "direct DateTime Choice alternative is unsupported".into(),
                            ));
                        }
                        TypeRefTarget::Primitive(PrimitiveKind::Duration) => {
                            return Err(unsupported(
                                &location,
                                "direct Duration Choice alternative is not qualified".into(),
                            ));
                        }
                        TypeRefTarget::Primitive(kind) => {
                            ada_scalar_support(*kind, &field.constraints, false, &location)?
                        }
                        TypeRefTarget::Named(_) if field.constraints != Default::default() => {
                            return Err(unsupported(&location, "field-local named facets".into()));
                        }
                        TypeRefTarget::Named(name) => {
                            let referenced = self.emitted.get(name).copied().ok_or_else(|| {
                                unsupported(&location, "named Choice value is not emitted".into())
                            })?;
                            if self.runtime != AdaCodecRuntime::Portable
                                && matches!(referenced, TypeEmission::Declaration(d) if matches!(d.kind, TypeKind::Primitive(PrimitiveKind::Float32)))
                            {
                                return Err(unsupported(
                                    &location,
                                    "named Float32 Choice alternative is not qualified".into(),
                                ));
                            }
                            if matches!(referenced, TypeEmission::Declaration(d) if matches!(d.kind, TypeKind::Primitive(PrimitiveKind::DateTime)))
                            {
                                return Err(unsupported(
                                    &location,
                                    "named DateTime Choice alternative is not qualified".into(),
                                ));
                            }
                            if matches!(referenced, TypeEmission::Declaration(d) if matches!(d.kind, TypeKind::Primitive(PrimitiveKind::Duration)))
                            {
                                return Err(unsupported(
                                    &location,
                                    "named Duration Choice alternative is not qualified".into(),
                                ));
                            }
                            self.support(schema, referenced, world)
                                .map_err(|error| unsupported(&location, error.to_string()))?;
                            // Record analysis historically counts local capability.
                            // A Choice must additionally cover every reachable value
                            // without changing Choice-free readiness counts.
                            self.record_support(schema, referenced, world)
                                .map_err(|error| unsupported(&location, error.to_string()))?;
                        }
                    }
                }
                Ok(())
            }
            _ => Err(unsupported(
                owner,
                "alias/list Ada codec is unsupported".into(),
            )),
        }
    }

    fn record_support(
        &mut self,
        schema: &SchemaIr,
        emission: &TypeEmission<'_>,
        world: GenerationWorld,
    ) -> Result<(), ServiceCodecError> {
        if let Some(result) = self.record_values.get(emission.name()) {
            return result.clone();
        }
        self.record_values.insert(
            emission.name().clone(),
            Err(unsupported(
                &emission.name().local_name,
                "cyclic codec dependency".into(),
            )),
        );
        #[cfg(test)]
        ADA_RECORD_VISITS.with(|visits| {
            *visits
                .borrow_mut()
                .entry(emission.name().clone())
                .or_default() += 1;
        });
        let result = self.record_support_uncached(schema, emission, world);
        self.record_values
            .insert(emission.name().clone(), result.clone());
        result
    }

    fn record_support_uncached(
        &mut self,
        schema: &SchemaIr,
        emission: &TypeEmission<'_>,
        world: GenerationWorld,
    ) -> Result<(), ServiceCodecError> {
        let TypeEmission::Declaration(declaration) = emission else {
            return Ok(()); // Already rejected by ada_codec_support.
        };
        if !matches!(declaration.kind, TypeKind::Record { .. }) {
            return Ok(());
        }
        let fields = effective_record_fields(schema, &declaration.name)
            .map_err(|error| unsupported(&declaration.name.local_name, error.to_string()))?;
        for field in fields {
            if let TypeRefTarget::Named(name) = &field.type_ref.target {
                let location = format!("{}.{}", declaration.name.local_name, field.name);
                let referenced = self.emitted.get(name).copied().ok_or_else(|| {
                    unsupported(&location, "named Choice value is not emitted".into())
                })?;
                self.support(schema, referenced, world)?;
                self.record_support(schema, referenced, world)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
thread_local! {
    static ADA_CAPABILITY_VISITS: std::cell::RefCell<std::collections::BTreeMap<ams_gra_oms_ir::QualifiedName, usize>> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
    static ADA_RECORD_VISITS: std::cell::RefCell<std::collections::BTreeMap<ams_gra_oms_ir::QualifiedName, usize>> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{Cardinality, ConstraintSet, QualifiedName, SourceRef, TypeDecl, TypeRef};

    fn source() -> SourceRef {
        SourceRef {
            document: "t.xsd".to_owned(),
            line: None,
        }
    }

    fn field(name: &str, type_ref: TypeRef) -> FieldDecl {
        FieldDecl {
            name: name.to_owned(),
            // Every record() below uses one namespace; tests override this.
            wire_namespace_uri: Some(OAM_NAMESPACE.to_owned()),
            type_ref,
            cardinality: Cardinality::REQUIRED_ONE,
            nillable: false,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }
    }

    fn record(namespace: &str, fields: Vec<FieldDecl>) -> SchemaIr {
        let mut schema = SchemaIr::empty();
        schema.namespaces.push(ams_gra_oms_ir::NamespaceDecl {
            uri: namespace.to_owned(),
            preferred_prefix: None,
        });
        schema.types.push(TypeDecl {
            name: QualifiedName::new(namespace, "P"),
            is_abstract: false,
            base_type: None,
            kind: TypeKind::Record { fields },
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        });
        schema
    }

    fn check(schema: &SchemaIr) -> Result<(), ServiceCodecError> {
        let emissions =
            plan_type_emissions(schema, GenerationWorld::ClosedSchemaSet).expect("plan");
        emissions.iter().try_for_each(|emission| {
            emission_codec_support(schema, emission, GenerationWorld::ClosedSchemaSet)
        })
    }

    #[test]
    fn every_primitive_has_an_explicit_codec_decision() {
        for kind in [
            PrimitiveKind::Boolean,
            PrimitiveKind::SignedInteger,
            PrimitiveKind::UnsignedInteger,
            PrimitiveKind::Float32,
            PrimitiveKind::Float64,
            PrimitiveKind::String,
            PrimitiveKind::DateTime,
            PrimitiveKind::Duration,
        ] {
            let schema = record(OAM_NAMESPACE, vec![field("F", TypeRef::primitive(kind))]);
            assert_eq!(check(&schema), Ok(()), "{kind:?}");
        }
        // Task 052: Binary is decided by provenance, never by the kind.
        let hex = record(
            OAM_NAMESPACE,
            vec![field(
                "F",
                TypeRef::binary(BinaryLexicalEncoding::HexBinary),
            )],
        );
        assert_eq!(check(&hex), Ok(()));
        let binary = record(
            OAM_NAMESPACE,
            vec![field("F", TypeRef::primitive(PrimitiveKind::Binary))],
        );
        assert!(matches!(
            check(&binary),
            Err(ServiceCodecError::UnknownBinaryEncoding { .. })
        ));
        for kind in [PrimitiveKind::Decimal, PrimitiveKind::Time] {
            let schema = record(OAM_NAMESPACE, vec![field("F", TypeRef::primitive(kind))]);
            assert!(
                matches!(check(&schema), Err(ServiceCodecError::UnsupportedPrimitive { kind: k, .. }) if k == kind),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn task095_float32_requires_explicit_runtime_and_rejects_facets() {
        let empty = ConstraintSet::default();
        assert!(
            ada_runtime_scalar_support(
                PrimitiveKind::Float32,
                &empty,
                false,
                "P.F",
                AdaCodecRuntime::Portable
            )
            .is_err()
        );
        for named in [false, true] {
            assert!(
                ada_runtime_scalar_support(
                    PrimitiveKind::Float32,
                    &empty,
                    named,
                    "P.F",
                    AdaCodecRuntime::LinuxX86_64Glibc241
                )
                .is_ok()
            );
            for kind in [PrimitiveKind::Float64, PrimitiveKind::Decimal] {
                assert!(
                    ada_runtime_scalar_support(
                        kind,
                        &empty,
                        named,
                        "P.F",
                        AdaCodecRuntime::LinuxX86_64Glibc241
                    )
                    .is_err()
                );
            }
        }
        let mut restricted = empty;
        restricted.length = Some(1);
        for named in [false, true] {
            assert!(
                ada_runtime_scalar_support(
                    PrimitiveKind::Float32,
                    &restricted,
                    named,
                    "P.F",
                    AdaCodecRuntime::LinuxX86_64Glibc241
                )
                .is_err()
            );
        }
    }

    #[test]
    fn task095_qualified_record_admits_float32_but_direct_choice_does_not() {
        let mut f = field("F", TypeRef::primitive(PrimitiveKind::Float32));
        f.cardinality = Cardinality {
            min_occurs: 0,
            max_occurs: None,
        };
        let schema = record(OAM_NAMESPACE, vec![f]);
        let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let mut analysis = AdaCodecAnalysis::new(&emissions);
        analysis.runtime = AdaCodecRuntime::LinuxX86_64Glibc241;
        assert!(
            analysis
                .support(&schema, &emissions[0], GenerationWorld::ClosedSchemaSet)
                .is_ok()
        );
        assert!(
            ada_codec_support(&schema, &emissions[0], GenerationWorld::ClosedSchemaSet).is_err()
        );
        let mut choice = record(OAM_NAMESPACE, vec![]);
        choice.types[0].kind = TypeKind::Choice {
            alternatives: vec![field("F", TypeRef::primitive(PrimitiveKind::Float32))],
        };
        let emissions = plan_type_emissions(&choice, GenerationWorld::ClosedSchemaSet).unwrap();
        let mut analysis = AdaCodecAnalysis::new(&emissions);
        analysis.runtime = AdaCodecRuntime::LinuxX86_64Glibc241;
        let error = analysis
            .support(&choice, &emissions[0], GenerationWorld::ClosedSchemaSet)
            .unwrap_err();
        assert_eq!(
            error,
            unsupported(
                "P.F",
                "direct Float32 Choice alternative is not qualified".into()
            )
        );
    }

    /// Task 051: one semantic formatter for member keys, one for `$type`;
    /// OAM is bare, anything else is Clark notation, applied literally.
    #[test]
    fn oms_json_names_follow_omsc_spc_013_section_6_1_2() {
        assert_eq!(oms_json_member_name(OAM_NAMESPACE, "Count"), "Count");
        assert_eq!(oms_json_member_name("urn:test", "Count"), "{urn:test}Count");
        assert_eq!(
            oms_json_member_name("https://www.vdl.afrl.af.mil/programs/oam/", "Count"),
            "{https://www.vdl.afrl.af.mil/programs/oam/}Count",
            "only the exact OAM URI is special"
        );
        assert_eq!(oms_json_type_name(OAM_NAMESPACE, "BoxShape"), "BoxShape");
        assert_eq!(
            oms_json_type_name("urn:shape", "BoxShape"),
            "{urn:shape}BoxShape"
        );
    }

    /// Task 051: the blanket non-OAM rejection is gone. A qualified
    /// single-namespace non-OAM Record is ready; an UNQUALIFIED member is not,
    /// whatever its owner's namespace is.
    #[test]
    fn qualified_non_oam_is_ready_and_unqualified_members_fail_closed() {
        let mut member = field("F", TypeRef::primitive(PrimitiveKind::Boolean));
        member.wire_namespace_uri = Some("urn:test".to_owned());
        assert_eq!(check(&record("urn:test", vec![member.clone()])), Ok(()));

        for owner in ["urn:test", OAM_NAMESPACE] {
            member.wire_namespace_uri = None;
            let error = check(&record(owner, vec![member.clone()])).expect_err("unqualified");
            assert_eq!(
                error,
                ServiceCodecError::UnqualifiedMember {
                    location: "P.F".to_owned()
                }
            );
            assert_eq!(
                error.to_string(),
                "P.F has an unqualified local element with no evidenced OMS JSON \
                 member-name mapping"
            );
        }
    }

    /// Choice alternatives use the same member rule.
    #[test]
    fn unqualified_choice_alternative_fails_closed() {
        let mut schema = record("urn:test", Vec::new());
        let mut alternative = field("A", TypeRef::primitive(PrimitiveKind::Boolean));
        alternative.wire_namespace_uri = None;
        schema.types[0].kind = TypeKind::Choice {
            alternatives: vec![alternative],
        };
        assert_eq!(
            check(&schema),
            Err(ServiceCodecError::UnqualifiedMember {
                location: "P.A".to_owned()
            })
        );
    }

    /// Task 052 negative control: semantic Binary WITHOUT lexical provenance
    /// (hand-constructed IR) is valid model IR but the codec fails closed with
    /// a deterministic diagnostic. This is what proves no code path assumes
    /// "Binary means hexBinary".
    #[test]
    fn task052_unknown_binary_provenance_fails_closed() {
        let mut schema = record(
            OAM_NAMESPACE,
            vec![field("Data", TypeRef::primitive(PrimitiveKind::Binary))],
        );
        schema.types[0].name.local_name = "Payload".to_owned();
        assert_eq!(schema.validate(), Ok(()), "legal model IR");
        let error = check(&schema).expect_err("unknown provenance");
        assert_eq!(
            error,
            ServiceCodecError::UnknownBinaryEncoding {
                location: "Payload.Data".to_owned()
            }
        );
        assert_eq!(
            error.to_string(),
            "Payload.Data is Binary but its XSD lexical encoding provenance is unknown"
        );
    }

    /// Task 052: Base64Binary is representable in IR but has no codec
    /// mapping; it must never be routed through hex.
    #[test]
    fn task052_base64_binary_is_not_ready() {
        let schema = record(
            OAM_NAMESPACE,
            vec![field(
                "F",
                TypeRef::binary(BinaryLexicalEncoding::Base64Binary),
            )],
        );
        let error = check(&schema).expect_err("base64");
        assert_eq!(
            error.to_string(),
            "P.F is Binary with XSD lexical encoding base64Binary; the base64Binary \
             codec mapping is not implemented"
        );
    }

    /// Task 052: a named Binary declaration is decided by its own
    /// restriction ancestry: `Hex1 -> Hex0 -> xs:hexBinary` is ready, a
    /// named Binary with no provenance is not.
    #[test]
    fn task052_named_binary_uses_declaration_ancestry() {
        let named_binary = |name: &str, base: Option<TypeRef>| TypeDecl {
            name: QualifiedName::new(OAM_NAMESPACE, name),
            is_abstract: false,
            base_type: base,
            kind: TypeKind::Primitive(PrimitiveKind::Binary),
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        };
        let mut schema = record(OAM_NAMESPACE, Vec::new());
        schema.types.push(named_binary(
            "Hex0",
            Some(TypeRef::binary(BinaryLexicalEncoding::HexBinary)),
        ));
        schema.types.push(named_binary(
            "Hex1",
            Some(TypeRef::named(QualifiedName::new(OAM_NAMESPACE, "Hex0"))),
        ));
        assert_eq!(check(&schema), Ok(()));
        schema.types.push(named_binary("Opaque", None));
        assert_eq!(
            check(&schema),
            Err(ServiceCodecError::UnknownBinaryEncoding {
                location: "Opaque".to_owned()
            })
        );
    }

    #[test]
    fn nillable_members_fail_closed() {
        let mut nillable = field("F", TypeRef::primitive(PrimitiveKind::Boolean));
        nillable.nillable = true;
        let schema = record(OAM_NAMESPACE, vec![nillable]);
        assert_eq!(
            check(&schema).map_err(|error| error.to_string()),
            Err("P.F: nillable member".to_owned())
        );
    }

    #[test]
    fn task091_duration_uses_only_zero_facet_profiles() {
        use ams_gra_oms_ir::{
            LexicalConstraintSet, NumericValue, PatternExpression, PatternGroup, WhiteSpacePolicy,
        };
        for named in [false, true] {
            assert!(
                ada_scalar_support(
                    PrimitiveKind::Duration,
                    &ConstraintSet::default(),
                    named,
                    "Renamed.Span"
                )
                .is_ok()
            );
            for constraints in [
                ConstraintSet {
                    min_inclusive: Some(NumericValue::Integer(0)),
                    ..Default::default()
                },
                ConstraintSet {
                    max_length: Some(20),
                    ..Default::default()
                },
                ConstraintSet {
                    lexical: LexicalConstraintSet {
                        white_space: Some(WhiteSpacePolicy::Collapse),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ConstraintSet {
                    lexical: LexicalConstraintSet {
                        pattern_groups: vec![PatternGroup {
                            alternatives: vec![PatternExpression::xml_schema("P.*")],
                        }],
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ] {
                assert!(
                    ada_scalar_support(
                        PrimitiveKind::Duration,
                        &constraints,
                        named,
                        "Renamed.Span"
                    )
                    .is_err()
                );
            }
            for kind in [
                PrimitiveKind::Time,
                PrimitiveKind::Binary,
                PrimitiveKind::Float64,
            ] {
                assert!(ada_scalar_support(kind, &ConstraintSet::default(), named, "P.F").is_err());
            }
        }
    }

    #[test]
    fn task090_ada_datetime_domains_remain_distinct() {
        use ams_gra_oms_ir::{PatternExpression, PatternGroup};
        let empty = ConstraintSet::default();
        let mut zulu = empty.clone();
        zulu.lexical.pattern_groups.push(PatternGroup {
            alternatives: vec![PatternExpression::xml_schema(".+Z")],
        });
        assert!(ada_scalar_support(PrimitiveKind::DateTime, &zulu, true, "Instant").is_ok());
        assert!(ada_scalar_support(PrimitiveKind::DateTime, &empty, false, "P.F").is_ok());
        assert!(ada_scalar_support(PrimitiveKind::DateTime, &empty, true, "Instant").is_err());
        assert!(ada_scalar_support(PrimitiveKind::DateTime, &zulu, false, "P.F").is_err());
        for kind in [
            PrimitiveKind::Time,
            PrimitiveKind::Decimal,
            PrimitiveKind::Float32,
            PrimitiveKind::Float64,
            PrimitiveKind::Binary,
        ] {
            for named in [false, true] {
                assert!(ada_scalar_support(kind, &empty, named, "P.F").is_err());
                assert!(ada_scalar_support(kind, &zulu, named, "P.F").is_err());
            }
        }
        zulu.max_length = Some(100);
        assert!(ada_scalar_support(PrimitiveKind::DateTime, &zulu, true, "Instant").is_err());
    }

    #[test]
    fn task090r_direct_datetime_contexts_fail_closed() {
        for max_occurs in [Some(3), None] {
            let mut member = field("Dates", TypeRef::primitive(PrimitiveKind::DateTime));
            member.cardinality = Cardinality {
                min_occurs: 0,
                max_occurs,
            };
            let schema = record(OAM_NAMESPACE, vec![member.clone()]);
            assert!(
                ada_codec_support(
                    &schema,
                    &TypeEmission::Declaration(&schema.types[0]),
                    GenerationWorld::ClosedSchemaSet,
                )
                .is_ok()
            );
            // Preserve the rejection purpose with an unsupported neighboring
            // direct temporal profile, not the now-qualified repeated shape.
            member.constraints.max_length = Some(100);
            let schema = record(OAM_NAMESPACE, vec![member]);
            assert!(
                ada_codec_support(
                    &schema,
                    &TypeEmission::Declaration(&schema.types[0]),
                    GenerationWorld::ClosedSchemaSet,
                )
                .is_err()
            );
        }
        let mut schema = record(OAM_NAMESPACE, Vec::new());
        schema.types[0].kind = TypeKind::Choice {
            alternatives: vec![field("Date", TypeRef::primitive(PrimitiveKind::DateTime))],
        };
        assert_eq!(
            ada_codec_support(
                &schema,
                &TypeEmission::Declaration(&schema.types[0]),
                GenerationWorld::ClosedSchemaSet,
            )
            .unwrap_err()
            .to_string(),
            "P.Date: direct DateTime Choice alternative is unsupported"
        );
    }

    #[test]
    fn task090r_datetime_helper_uses_case_insensitive_fixed_name_reservation() {
        let names: std::collections::BTreeSet<_> = ADA_CODEC_FIXED_NAMES
            .iter()
            .map(|name| name.to_ascii_lowercase())
            .collect();
        for spelling in ["Scalar_Date_Time", "SCALAR_DATE_TIME", "scalar_date_time"] {
            assert!(names.contains(&spelling.to_ascii_lowercase()));
        }
        assert_eq!(names.len(), ADA_CODEC_FIXED_NAMES.len());
    }

    #[test]
    fn task088_shared_dependency_dag_has_bounded_capability_visits() {
        let mut schema = record(
            OAM_NAMESPACE,
            vec![field("Text", TypeRef::primitive(PrimitiveKind::String))],
        );
        schema.types[0].name = QualifiedName::new(OAM_NAMESPACE, "Layer0");
        for layer in 1..=8 {
            let mut declaration = schema.types[0].clone();
            declaration.name = QualifiedName::new(OAM_NAMESPACE, format!("Layer{layer}"));
            declaration.kind = TypeKind::Record {
                fields: vec![
                    field(
                        "Left",
                        TypeRef::named(QualifiedName::new(
                            OAM_NAMESPACE,
                            format!("Layer{}", layer - 1),
                        )),
                    ),
                    field(
                        "Right",
                        TypeRef::named(QualifiedName::new(
                            OAM_NAMESPACE,
                            format!("Layer{}", layer - 1),
                        )),
                    ),
                ],
            };
            schema.types.push(declaration);
        }
        let mut choice = schema.types[0].clone();
        choice.name = QualifiedName::new(OAM_NAMESPACE, "Selection");
        choice.kind = TypeKind::Choice {
            alternatives: vec![field(
                "Value",
                TypeRef::named(QualifiedName::new(OAM_NAMESPACE, "Layer8")),
            )],
        };
        schema.types.push(choice);
        let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet)
            .expect("valid acyclic shared DAG");
        assert_eq!(emissions.len(), 10);
        ADA_CAPABILITY_VISITS.with(|visits| visits.borrow_mut().clear());
        ADA_RECORD_VISITS.with(|visits| visits.borrow_mut().clear());
        assert!(
            ada_codec_support(
                &schema,
                emissions.last().unwrap(),
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok()
        );
        ADA_CAPABILITY_VISITS.with(|visits| {
            let visits = visits.borrow();
            println!(
                "TASK088_DAG declarations={} evaluations={} leaf_evaluations={}",
                emissions.len(),
                visits.values().sum::<usize>(),
                visits[&schema.types[0].name]
            );
            assert_eq!(visits.values().sum::<usize>(), 10);
            assert_eq!(visits[&schema.types[0].name], 1);
        });
        ADA_RECORD_VISITS.with(|visits| {
            assert_eq!(visits.borrow().values().sum::<usize>(), 9);
            assert!(visits.borrow().values().all(|count| *count == 1));
        });
        // Exercise the shared context used by the production top-level scan,
        // including a cache primed by the Choice before scanning its children.
        let mut analysis = AdaCodecAnalysis::new(&emissions);
        ADA_CAPABILITY_VISITS.with(|visits| visits.borrow_mut().clear());
        for emission in emissions.iter().rev().chain(emissions.iter()) {
            assert!(
                analysis
                    .support(&schema, emission, GenerationWorld::ClosedSchemaSet)
                    .is_ok()
            );
        }
        ADA_CAPABILITY_VISITS.with(|visits| {
            assert_eq!(visits.borrow().values().sum::<usize>(), emissions.len());
        });
    }

    #[test]
    fn task088_shared_nested_record_choice_dependencies_fail_closed() {
        let mut schema = record(
            OAM_NAMESPACE,
            vec![field("Bad", TypeRef::primitive(PrimitiveKind::Float64))],
        );
        schema.types[0].name = QualifiedName::new(OAM_NAMESPACE, "Leaf");
        let mut nested = schema.types[0].clone();
        nested.name = QualifiedName::new(OAM_NAMESPACE, "NestedChoice");
        nested.kind = TypeKind::Choice {
            alternatives: vec![field(
                "Selected",
                TypeRef::named(schema.types[0].name.clone()),
            )],
        };
        schema.types.push(nested);
        let mut parent = schema.types[0].clone();
        parent.name = QualifiedName::new(OAM_NAMESPACE, "Parent");
        parent.kind = TypeKind::Record {
            fields: vec![field("Pick", TypeRef::named(schema.types[1].name.clone()))],
        };
        schema.types.push(parent);
        for name in ["FirstChoice", "SecondChoice"] {
            let mut choice = schema.types[1].clone();
            choice.name = QualifiedName::new(OAM_NAMESPACE, name);
            choice.kind = TypeKind::Choice {
                alternatives: vec![field("Value", TypeRef::named(schema.types[2].name.clone()))],
            };
            schema.types.push(choice);
        }
        let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet)
            .expect("nested acyclic DAG");
        let mut analysis = AdaCodecAnalysis::new(&emissions);
        ADA_CAPABILITY_VISITS.with(|visits| visits.borrow_mut().clear());
        for name in ["FirstChoice", "SecondChoice"] {
            let emission = emissions
                .iter()
                .find(|item| item.name().local_name == name)
                .unwrap();
            assert_eq!(
                analysis
                    .support(&schema, emission, GenerationWorld::ClosedSchemaSet)
                    .unwrap_err()
                    .to_string(),
                format!(
                    "{name}.Value: NestedChoice.Selected: Leaf.Bad: unsupported Ada codec primitive or facets: Float64"
                )
            );
        }
        let mut first = None;
        for emission in &emissions {
            if let Err(error) =
                analysis.support(&schema, emission, GenerationWorld::ClosedSchemaSet)
            {
                first.get_or_insert(error);
            }
        }
        assert_eq!(
            first.unwrap().to_string(),
            "Leaf.Bad: unsupported Ada codec primitive or facets: Float64"
        );
        ADA_CAPABILITY_VISITS.with(|visits| {
            assert_eq!(visits.borrow().len(), emissions.len());
            assert!(visits.borrow().values().all(|count| *count == 1));
        });
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            unreachable!()
        };
        fields[0].type_ref = TypeRef::primitive(PrimitiveKind::String);
        let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let mut analysis = AdaCodecAnalysis::new(&emissions);
        assert!(emissions.iter().all(|item| {
            analysis
                .support(&schema, item, GenerationWorld::ClosedSchemaSet)
                .is_ok()
        }));
        // Missing and cyclic named dependencies remain planner errors.
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            unreachable!()
        };
        fields[0].type_ref = TypeRef::named(QualifiedName::new(OAM_NAMESPACE, "Missing"));
        assert!(plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).is_err());
        fields_retarget_cycle(&mut schema);
        assert!(plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).is_err());
    }

    fn fields_retarget_cycle(schema: &mut SchemaIr) {
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            unreachable!()
        };
        fields[0].type_ref = TypeRef::named(QualifiedName::new(OAM_NAMESPACE, "Parent"));
    }

    #[test]
    fn task088_ada_choice_checks_all_alternatives_and_wire_identity() {
        let mut schema = record(OAM_NAMESPACE, Vec::new());
        let mut alternatives = vec![
            field("Flag", TypeRef::primitive(PrimitiveKind::Boolean)),
            field("Text", TypeRef::primitive(PrimitiveKind::String)),
        ];
        schema.types[0].kind = TypeKind::Choice {
            alternatives: alternatives.clone(),
        };
        let check = |schema: &SchemaIr| {
            ada_codec_support(
                schema,
                &TypeEmission::Declaration(&schema.types[0]),
                GenerationWorld::ClosedSchemaSet,
            )
        };
        assert!(check(&schema).is_ok());
        for cardinality in [
            Cardinality {
                min_occurs: 0,
                max_occurs: Some(1),
            },
            Cardinality {
                min_occurs: 1,
                max_occurs: Some(2),
            },
            Cardinality {
                min_occurs: 1,
                max_occurs: None,
            },
        ] {
            alternatives[1].cardinality = cardinality;
            schema.types[0].kind = TypeKind::Choice {
                alternatives: alternatives.clone(),
            };
            assert_eq!(
                check(&schema).unwrap_err().to_string(),
                "P.Text: Choice alternative must be required-single"
            );
        }
        alternatives[1].cardinality = Cardinality::REQUIRED_ONE;
        alternatives[1].wire_namespace_uri = None;
        schema.types[0].kind = TypeKind::Choice {
            alternatives: alternatives.clone(),
        };
        assert!(matches!(
            check(&schema),
            Err(ServiceCodecError::UnqualifiedMember { .. })
        ));
        alternatives[1].wire_namespace_uri = Some(OAM_NAMESPACE.into());
        alternatives[1].nillable = true;
        schema.types[0].kind = TypeKind::Choice {
            alternatives: alternatives.clone(),
        };
        assert_eq!(
            check(&schema).unwrap_err().to_string(),
            "P.Text: nillable Choice alternative"
        );
        alternatives[1].nillable = false;
        alternatives[1].type_ref = TypeRef::primitive(PrimitiveKind::Float64);
        schema.types[0].kind = TypeKind::Choice {
            alternatives: alternatives.clone(),
        };
        assert!(
            check(&schema)
                .unwrap_err()
                .to_string()
                .starts_with("P.Text:")
        );
        alternatives[1].type_ref = TypeRef::primitive(PrimitiveKind::String);
        alternatives[1].name = "Flag".into();
        schema.types[0].kind = TypeKind::Choice {
            alternatives: alternatives.clone(),
        };
        // Existing structural/model validation may reject duplicate source
        // identifiers before codec wire-key validation; do not loosen it.
        assert!(check(&schema).is_err());
        alternatives[1].wire_namespace_uri = Some("urn:other".into());
        schema.types[0].kind = TypeKind::Choice { alternatives };
        assert!(
            check(&schema).is_err(),
            "model name collision remains unsupported"
        );
        assert_ne!(
            oms_json_member_name(OAM_NAMESPACE, "Flag"),
            oms_json_member_name("urn:other", "Flag")
        );
    }

    #[test]
    fn task088_ada_choice_named_values_require_renderable_closure() {
        let mut schema = record(OAM_NAMESPACE, Vec::new());
        schema.types[0].kind = TypeKind::Choice {
            alternatives: vec![field(
                "Value",
                TypeRef::named(QualifiedName::new(OAM_NAMESPACE, "Missing")),
            )],
        };
        assert!(
            ada_codec_support(
                &schema,
                &TypeEmission::Declaration(&schema.types[0]),
                GenerationWorld::ClosedSchemaSet
            )
            .is_err()
        );
        let mut value = schema.types[0].clone();
        value.name = QualifiedName::new(OAM_NAMESPACE, "ValueType");
        value.kind = TypeKind::Primitive(PrimitiveKind::Float64);
        schema.types.push(value);
        schema.types[0].kind = TypeKind::Choice {
            alternatives: vec![field(
                "Value",
                TypeRef::named(QualifiedName::new(OAM_NAMESPACE, "ValueType")),
            )],
        };
        assert!(
            ada_codec_support(
                &schema,
                &TypeEmission::Declaration(&schema.types[0]),
                GenerationWorld::ClosedSchemaSet
            )
            .unwrap_err()
            .to_string()
            .starts_with("P.Value:")
        );
        schema.types[1].kind = TypeKind::Record { fields: Vec::new() };
        schema.types[1].is_abstract = true;
        assert!(
            ada_codec_support(
                &schema,
                &TypeEmission::Declaration(&schema.types[0]),
                GenerationWorld::ClosedSchemaSet
            )
            .is_err()
        );
        schema.types[1].is_abstract = false;
        assert!(
            ada_codec_support(
                &schema,
                &TypeEmission::Declaration(&schema.types[0]),
                GenerationWorld::ClosedSchemaSet
            )
            .is_ok()
        );
    }

    #[test]
    fn task087b_ada_scalar_domains_and_feature_boundaries() {
        use ams_gra_oms_ir::NumericValue;
        let empty = ConstraintSet::default();
        for kind in [
            PrimitiveKind::Boolean,
            PrimitiveKind::String,
            PrimitiveKind::SignedInteger,
            PrimitiveKind::Duration,
        ] {
            assert!(ada_scalar_support(kind, &empty, false, "P.F").is_ok());
        }
        for kind in [
            PrimitiveKind::UnsignedInteger,
            PrimitiveKind::Float32,
            PrimitiveKind::Float64,
            PrimitiveKind::Decimal,
            PrimitiveKind::Binary,
            PrimitiveKind::Time,
        ] {
            assert!(
                ada_scalar_support(kind, &empty, false, "P.F").is_err(),
                "{kind:?}"
            );
        }
        let mut bounded = ConstraintSet {
            min_inclusive: Some(NumericValue::Integer(0)),
            max_inclusive: Some(NumericValue::Integer(i128::from(i64::MAX))),
            ..Default::default()
        };
        assert!(ada_scalar_support(PrimitiveKind::UnsignedInteger, &bounded, false, "P.F").is_ok());
        bounded.max_inclusive = Some(NumericValue::Integer(i128::from(i64::MAX) + 1));
        assert!(
            ada_scalar_support(PrimitiveKind::UnsignedInteger, &bounded, false, "P.F")
                .unwrap_err()
                .to_string()
                .contains("signed-64")
        );
        bounded.max_inclusive = Some(NumericValue::Integer(i128::from(i64::MAX) + 1));
        assert!(ada_scalar_support(PrimitiveKind::SignedInteger, &bounded, true, "P.F").is_err());
    }

    #[test]
    fn task087b_ada_checks_every_member_not_only_authored_json() {
        let mut schema = record(
            OAM_NAMESPACE,
            vec![
                field("First", TypeRef::primitive(PrimitiveKind::Boolean)),
                field("Second", TypeRef::primitive(PrimitiveKind::Float64)),
            ],
        );
        let emission = TypeEmission::Declaration(&schema.types[0]);
        let error =
            ada_codec_support(&schema, &emission, GenerationWorld::ClosedSchemaSet).unwrap_err();
        assert!(error.to_string().starts_with("P.Second:"));
        let TypeKind::Record { fields } = &mut schema.types[0].kind else {
            unreachable!()
        };
        fields[1].type_ref = TypeRef::primitive(PrimitiveKind::String);
        fields[1].cardinality = Cardinality {
            min_occurs: 0,
            max_occurs: Some(3),
        };
        assert!(
            ada_codec_support(
                &schema,
                &TypeEmission::Declaration(&schema.types[0]),
                GenerationWorld::ClosedSchemaSet,
            )
            .is_ok()
        );
    }

    #[test]
    fn task089_repeated_cardinalities_and_element_boundaries() {
        for cardinality in [
            Cardinality {
                min_occurs: 0,
                max_occurs: Some(3),
            },
            Cardinality {
                min_occurs: 2,
                max_occurs: Some(3),
            },
            Cardinality {
                min_occurs: 2,
                max_occurs: Some(2),
            },
            Cardinality {
                min_occurs: 0,
                max_occurs: None,
            },
            Cardinality {
                min_occurs: 2,
                max_occurs: None,
            },
        ] {
            for kind in [
                PrimitiveKind::String,
                PrimitiveKind::Boolean,
                PrimitiveKind::SignedInteger,
                PrimitiveKind::DateTime,
                PrimitiveKind::Duration,
            ] {
                let mut member = field("Repeated", TypeRef::primitive(kind));
                member.cardinality = cardinality;
                let schema = record(OAM_NAMESPACE, vec![member]);
                assert!(
                    ada_codec_support(
                        &schema,
                        &TypeEmission::Declaration(&schema.types[0]),
                        GenerationWorld::ClosedSchemaSet
                    )
                    .is_ok()
                );
                if matches!(kind, PrimitiveKind::DateTime | PrimitiveKind::Duration) {
                    let mut constrained = field("Repeated", TypeRef::primitive(kind));
                    constrained.cardinality = cardinality;
                    constrained.constraints.max_length = Some(100);
                    let schema = record(OAM_NAMESPACE, vec![constrained]);
                    assert!(
                        ada_codec_support(
                            &schema,
                            &TypeEmission::Declaration(&schema.types[0]),
                            GenerationWorld::ClosedSchemaSet
                        )
                        .is_err(),
                        "{kind:?} must reject local length facets at {cardinality:?}"
                    );
                }
            }
        }
        for cardinality in [
            Cardinality {
                min_occurs: 4,
                max_occurs: Some(3),
            },
            Cardinality {
                min_occurs: 0,
                max_occurs: Some(32_768),
            },
            Cardinality {
                min_occurs: 32_768,
                max_occurs: None,
            },
        ] {
            for kind in [
                PrimitiveKind::String,
                PrimitiveKind::DateTime,
                PrimitiveKind::Duration,
            ] {
                let mut member = field("Repeated", TypeRef::primitive(kind));
                member.cardinality = cardinality;
                let schema = record(OAM_NAMESPACE, vec![member]);
                assert!(
                    ada_codec_support(
                        &schema,
                        &TypeEmission::Declaration(&schema.types[0]),
                        GenerationWorld::ClosedSchemaSet
                    )
                    .is_err(),
                    "{kind:?} must reject invalid cardinality {cardinality:?}"
                );
            }
        }
        for kind in [
            PrimitiveKind::Float32,
            PrimitiveKind::Float64,
            PrimitiveKind::Decimal,
            PrimitiveKind::Time,
            PrimitiveKind::Binary,
            PrimitiveKind::UnsignedInteger,
        ] {
            let mut member = field("Repeated", TypeRef::primitive(kind));
            member.cardinality = Cardinality {
                min_occurs: 0,
                max_occurs: None,
            };
            let schema = record(OAM_NAMESPACE, vec![member]);
            assert!(
                ada_codec_support(
                    &schema,
                    &TypeEmission::Declaration(&schema.types[0]),
                    GenerationWorld::ClosedSchemaSet
                )
                .is_err()
            );
        }
    }

    #[test]
    fn task089r_shared_repeated_record_dependencies_are_cached() {
        for malformed in [false, true] {
            let mut schema = record(
                OAM_NAMESPACE,
                vec![field(
                    "Value",
                    TypeRef::primitive(if malformed {
                        PrimitiveKind::Float64
                    } else {
                        PrimitiveKind::String
                    }),
                )],
            );
            schema.types[0].name = QualifiedName::new(OAM_NAMESPACE, "Leaf");
            for level in 0..18 {
                let previous = schema.types.last().unwrap().name.clone();
                let mut members = Vec::new();
                for name in ["Left", "Right"] {
                    let mut member = field(name, TypeRef::named(previous.clone()));
                    member.cardinality = Cardinality {
                        min_occurs: 0,
                        max_occurs: Some(3),
                    };
                    members.push(member);
                }
                let mut parent = record(OAM_NAMESPACE, members).types.remove(0);
                parent.name = QualifiedName::new(OAM_NAMESPACE, format!("Level{level}"));
                schema.types.push(parent);
            }
            let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
            let mut analysis = AdaCodecAnalysis::new(&emissions);
            ADA_CAPABILITY_VISITS.with(|visits| visits.borrow_mut().clear());
            ADA_RECORD_VISITS.with(|visits| visits.borrow_mut().clear());
            for emission in emissions.iter().rev().chain(emissions.iter()) {
                let result = analysis.support(&schema, emission, GenerationWorld::ClosedSchemaSet);
                assert_eq!(result.is_err(), malformed);
                if let Err(error) = result {
                    assert!(error.to_string().contains("Leaf.Value"));
                }
            }
            ADA_CAPABILITY_VISITS.with(|visits| {
                assert_eq!(visits.borrow().len(), emissions.len());
                assert!(visits.borrow().values().all(|count| *count == 1));
            });
            ADA_RECORD_VISITS.with(|visits| {
                assert!(visits.borrow().len() <= emissions.len());
                assert!(visits.borrow().values().all(|count| *count == 1));
            });
        }
    }

    #[test]
    fn task092_missing_and_cyclic_repeated_choice_dependencies_fail_closed() {
        let mut schema = record(OAM_NAMESPACE, vec![]);
        let parent = schema.types[0].name.clone();
        let choice_name = QualifiedName::new(OAM_NAMESPACE, "Selection");
        let mut member = field("Items", TypeRef::named(choice_name.clone()));
        member.cardinality.max_occurs = None;
        schema.types[0].kind = TypeKind::Record {
            fields: vec![member],
        };
        let mut choice = schema.types[0].clone();
        choice.name = choice_name;
        choice.kind = TypeKind::Choice {
            alternatives: vec![field("Back", TypeRef::named(parent))],
        };
        schema.types.push(choice);
        assert!(plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).is_err());
        // Exercise the analyzer's fail-closed sentinels even if a future caller
        // supplies a hand-built emission list instead of the rejecting planner.
        let emissions: Vec<_> = schema.types.iter().map(TypeEmission::Declaration).collect();
        let mut analysis = AdaCodecAnalysis::new(&emissions);
        let error = analysis
            .support(&schema, &emissions[0], GenerationWorld::ClosedSchemaSet)
            .unwrap_err();
        assert!(error.to_string().contains("cyclic codec dependency"));
        let projected = vec![TypeEmission::Declaration(&schema.types[0])];
        let mut analysis = AdaCodecAnalysis::new(&projected);
        let error = analysis
            .support(&schema, &projected[0], GenerationWorld::ClosedSchemaSet)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("named repeated value is not emitted")
        );
    }

    #[test]
    fn task092_shared_repeated_choice_closure_is_cached() {
        for malformed in [false, true] {
            let mut schema = record(
                OAM_NAMESPACE,
                vec![field(
                    "Value",
                    TypeRef::primitive(if malformed {
                        PrimitiveKind::Float64
                    } else {
                        PrimitiveKind::Boolean
                    }),
                )],
            );
            schema.types[0].name = QualifiedName::new(OAM_NAMESPACE, "Leaf");
            for level in 0..12 {
                let previous = schema.types.last().unwrap().name.clone();
                let mut choice = record(OAM_NAMESPACE, vec![]).types.remove(0);
                choice.name = QualifiedName::new(OAM_NAMESPACE, format!("Choice{level}"));
                choice.kind = TypeKind::Choice {
                    alternatives: vec![field("Selected", TypeRef::named(previous))],
                };
                schema.types.push(choice);
                let name = schema.types.last().unwrap().name.clone();
                let mut members = Vec::new();
                for member_name in ["First", "Second"] {
                    let mut member = field(member_name, TypeRef::named(name.clone()));
                    member.cardinality.max_occurs = None;
                    members.push(member);
                }
                let mut parent = record(OAM_NAMESPACE, members).types.remove(0);
                parent.name = QualifiedName::new(OAM_NAMESPACE, format!("Parent{level}"));
                schema.types.push(parent);
            }
            let emissions = plan_type_emissions(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
            let mut analysis = AdaCodecAnalysis::new(&emissions);
            ADA_CAPABILITY_VISITS.with(|visits| visits.borrow_mut().clear());
            ADA_RECORD_VISITS.with(|visits| visits.borrow_mut().clear());
            for emission in emissions.iter().rev().chain(emissions.iter()) {
                let result = analysis.support(&schema, emission, GenerationWorld::ClosedSchemaSet);
                assert_eq!(result.is_err(), malformed);
                if let Err(error) = result {
                    assert!(error.to_string().contains("Leaf.Value"));
                    assert!(!error.to_string().contains("Second"));
                }
            }
            ADA_CAPABILITY_VISITS.with(|visits| {
                assert_eq!(visits.borrow().len(), emissions.len());
                assert!(visits.borrow().values().all(|count| *count == 1));
            });
            ADA_RECORD_VISITS.with(|visits| {
                assert!(visits.borrow().values().all(|count| *count == 1));
            });
        }
    }

    #[test]
    fn task089_repeated_named_record_checks_complete_closure() {
        let mut schema = record(
            OAM_NAMESPACE,
            vec![field("Bad", TypeRef::primitive(PrimitiveKind::Float64))],
        );
        schema.types[0].name = QualifiedName::new(OAM_NAMESPACE, "Child");
        let mut repeated = field("Children", TypeRef::named(schema.types[0].name.clone()));
        repeated.cardinality = Cardinality {
            min_occurs: 0,
            max_occurs: None,
        };
        let parent = record(OAM_NAMESPACE, vec![repeated]).types.remove(0);
        schema.types.push(parent);
        let error = ada_codec_support(
            &schema,
            &TypeEmission::Declaration(&schema.types[1]),
            GenerationWorld::ClosedSchemaSet,
        )
        .unwrap_err();
        assert!(error.to_string().contains("Child.Bad"));
        schema.types[0].kind = TypeKind::Choice {
            alternatives: vec![field("Value", TypeRef::primitive(PrimitiveKind::String))],
        };
        // Task 092 qualifies repeated named Choices through the same closure.
        ada_codec_support(
            &schema,
            &TypeEmission::Declaration(&schema.types[1]),
            GenerationWorld::ClosedSchemaSet,
        )
        .unwrap();
        // Preserve the historical fail-closed purpose with an unsupported
        // repeated alternative, rather than rejecting the now-qualified member.
        let TypeKind::Choice { alternatives } = &mut schema.types[0].kind else {
            unreachable!();
        };
        alternatives[0].cardinality.max_occurs = Some(2);
        assert!(
            ada_codec_support(
                &schema,
                &TypeEmission::Declaration(&schema.types[1]),
                GenerationWorld::ClosedSchemaSet
            )
            .unwrap_err()
            .to_string()
            .contains("Choice alternative must be required-single")
        );
    }
}
