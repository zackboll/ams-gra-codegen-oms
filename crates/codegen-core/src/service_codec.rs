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
//! * **Binary.** `PrimitiveKind::Binary` is semantic octets; the IR does not
//!   retain whether the XSD primitive was `hexBinary` or `base64Binary`, whose
//!   lexical spellings differ. No encoding is chosen, so Binary fails closed.
//!
//! The backend single-namespace boundary is NOT relaxed here: normal backend
//! preflight still rejects a projection spanning several namespaces.

use crate::{
    BackendLanguage, EffectiveValueMember, GenerationWorld, ServiceApiModel, TypeEmission,
    effective_choice_alternatives, effective_record_fields, field_storage_semantics,
    plan_type_emissions, rust_model_file_name, service_api_fixed_names,
};
use ams_gra_oms_ir::{
    FieldDecl, OccurrenceShape, PrimitiveKind, SchemaIr, TypeKind, TypeRefTarget,
};
use std::fmt;

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
            Self::UnqualifiedMember { location } => write!(
                f,
                "{location} has an unqualified local element with no evidenced OMS JSON \
                 member-name mapping"
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
        // `$type` values are every concrete descendant's own type QName, which
        // is always known (`oms_json_type_name`). Each concrete descendant is
        // itself an emitted Declaration and its members are checked there.
        TypeEmission::AbstractValue(_) => return Ok(()),
    };
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
    // Task 051: the member key comes from the ELEMENT's own target namespace,
    // never the owning type's, the message's, or the member type's.
    if field.wire_name().namespace_uri.is_none() {
        return Err(ServiceCodecError::UnqualifiedMember { location });
    }
    if field.nillable {
        return Err(unsupported(&location, "nillable member".into()));
    }
    match &field.type_ref.target {
        TypeRefTarget::Primitive(kind) => primitive_support(*kind, &location),
        // A named member type is itself an emitted declaration, checked there.
        TypeRefTarget::Named(_) => Ok(()),
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

fn unsupported(location: &str, reason: String) -> ServiceCodecError {
    ServiceCodecError::UnsupportedConstruct {
        location: location.to_owned(),
        reason,
    }
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
}
