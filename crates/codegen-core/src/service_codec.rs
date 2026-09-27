//! Task 050: generated OMS JSON payload-codec readiness.
//!
//! A separate, opt-in readiness dimension. It never changes model readiness,
//! a selected-type count, or any default report line. It answers: can
//! `service-generate --with-codec` emit a codec for this projected selected
//! schema without guessing a wire rule?
//!
//! The wire rules are OMSC-SPC-013 Rev B section 6.1 (see
//! `docs/task-050-rust-oms-json-codecs.md`). Two boundaries are deliberate:
//!
//! * **Member namespaces.** OMS JSON member keys are bare only for element
//!   declarations whose target namespace is the OAM namespace; any other
//!   namespace needs `{namespace}local`. Schema IR keeps `FieldDecl.name` but
//!   not the local element's namespace/form, so only an all-OAM projection is
//!   accepted. Every other namespace fails closed.
//! * **Binary.** `PrimitiveKind::Binary` is semantic octets; the IR does not
//!   retain whether the XSD primitive was `hexBinary` or `base64Binary`, whose
//!   lexical spellings differ. No encoding is chosen, so Binary fails closed.

use crate::{
    BackendLanguage, EffectiveValueMember, GenerationWorld, ServiceApiModel, TypeEmission,
    effective_choice_alternatives, effective_record_fields, field_storage_semantics,
    plan_type_emissions, rust_model_file_name, service_api_fixed_names,
};
use ams_gra_oms_ir::{
    FieldDecl, OccurrenceShape, PrimitiveKind, QualifiedName, SchemaIr, TypeKind, TypeRefTarget,
};
use std::fmt;

/// The OAM namespace (OMSC-SPC-013 Rev B section 6.1): members of element
/// declarations in this namespace are keyed by their bare local name.
pub const OAM_NAMESPACE: &str = "https://www.vdl.afrl.af.mil/programs/oam";

/// The Rust codec file written beside the model and `service_api.rs`.
pub const RUST_SERVICE_CODEC_FILE: &str = "service_codec.rs";

/// The Rust module `service_api.rs` mounts the codec file as.
pub const RUST_SERVICE_CODEC_MODULE: &str = "service_codec";

/// Why a codec cannot be generated for a projected selected schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceCodecError {
    /// No generated OMS JSON codec exists for this language.
    LanguageNotImplemented(BackendLanguage),
    /// A codec-emitted declaration is outside the OAM namespace, so member
    /// keys and `$type` values cannot be proven from the current IR.
    NonOamNamespace(QualifiedName),
    /// A Binary value: hexBinary vs base64Binary provenance is not retained.
    BinaryProvenance { location: String },
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
            Self::NonOamNamespace(name) => write!(
                f,
                "declaration {{{}}}{} is outside the OAM namespace; OMS JSON member \
                 names and $type values for other namespaces need element QName/form \
                 semantics that Schema IR does not retain",
                name.namespace_uri, name.local_name
            ),
            Self::BinaryProvenance { location } => write!(
                f,
                "{location} is Binary; Schema IR does not retain whether the XSD \
                 primitive was hexBinary or base64Binary, so its OMS JSON string \
                 encoding cannot be chosen"
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
        TypeEmission::AbstractValue(projection) => {
            // `$type` values are concrete XSD type names; bare only in OAM.
            // Each concrete descendant is itself an emitted Declaration and
            // is checked on its own.
            for descendant in std::iter::once(projection.declaration)
                .chain(projection.concrete_descendants.iter().copied())
            {
                require_oam(&descendant.name)?;
            }
            return Ok(());
        }
    };
    require_oam(&declaration.name)?;
    let owner = &declaration.name.local_name;
    match &declaration.kind {
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
                member_support(field, owner)?;
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
                member_support(alternative, owner)?;
            }
            Ok(())
        }
        TypeKind::Alias(_) | TypeKind::List { .. } => Err(unsupported(
            owner,
            "alias/list declarations have no generated codec".into(),
        )),
    }
}

fn member_support(field: &FieldDecl, owner: &str) -> Result<(), ServiceCodecError> {
    let location = format!("{owner}.{}", field.name);
    if field.nillable {
        return Err(unsupported(&location, "nillable member".into()));
    }
    match &field.type_ref.target {
        TypeRefTarget::Primitive(kind) => primitive_support(*kind, &location),
        TypeRefTarget::Named(name) => require_oam(name),
    }
}

fn primitive_support(kind: PrimitiveKind, location: &str) -> Result<(), ServiceCodecError> {
    match kind {
        PrimitiveKind::Boolean
        | PrimitiveKind::SignedInteger
        | PrimitiveKind::UnsignedInteger
        | PrimitiveKind::Float32
        | PrimitiveKind::Float64
        | PrimitiveKind::String
        | PrimitiveKind::DateTime => Ok(()),
        PrimitiveKind::Binary => Err(ServiceCodecError::BinaryProvenance {
            location: location.to_owned(),
        }),
        PrimitiveKind::Decimal | PrimitiveKind::Time | PrimitiveKind::Duration => {
            Err(ServiceCodecError::UnsupportedPrimitive {
                location: location.to_owned(),
                kind,
            })
        }
    }
}

fn require_oam(name: &QualifiedName) -> Result<(), ServiceCodecError> {
    if name.namespace_uri == OAM_NAMESPACE {
        Ok(())
    } else {
        Err(ServiceCodecError::NonOamNamespace(name.clone()))
    }
}

fn unsupported(location: &str, reason: String) -> ServiceCodecError {
    ServiceCodecError::UnsupportedConstruct {
        location: location.to_owned(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{Cardinality, ConstraintSet, SourceRef, TypeDecl, TypeRef};

    fn source() -> SourceRef {
        SourceRef {
            document: "t.xsd".to_owned(),
            line: None,
        }
    }

    fn field(name: &str, type_ref: TypeRef) -> FieldDecl {
        FieldDecl {
            name: name.to_owned(),
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
        ] {
            let schema = record(OAM_NAMESPACE, vec![field("F", TypeRef::primitive(kind))]);
            assert_eq!(check(&schema), Ok(()), "{kind:?}");
        }
        let binary = record(
            OAM_NAMESPACE,
            vec![field("F", TypeRef::primitive(PrimitiveKind::Binary))],
        );
        assert!(matches!(
            check(&binary),
            Err(ServiceCodecError::BinaryProvenance { .. })
        ));
        for kind in [
            PrimitiveKind::Decimal,
            PrimitiveKind::Time,
            PrimitiveKind::Duration,
        ] {
            let schema = record(OAM_NAMESPACE, vec![field("F", TypeRef::primitive(kind))]);
            assert!(
                matches!(check(&schema), Err(ServiceCodecError::UnsupportedPrimitive { kind: k, .. }) if k == kind),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn non_oam_declarations_and_nillable_members_fail_closed() {
        let foreign = record(
            "urn:test",
            vec![field("F", TypeRef::primitive(PrimitiveKind::Boolean))],
        );
        assert!(matches!(
            check(&foreign),
            Err(ServiceCodecError::NonOamNamespace(_))
        ));
        let mut nillable = field("F", TypeRef::primitive(PrimitiveKind::Boolean));
        nillable.nillable = true;
        let schema = record(OAM_NAMESPACE, vec![nillable]);
        assert_eq!(
            check(&schema).map_err(|error| error.to_string()),
            Err("P.F: nillable member".to_owned())
        );
    }
}
