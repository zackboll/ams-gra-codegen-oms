//! Global backend generation preconditions, shared by generation validation
//! and by readiness analysis.
//!
//! # Why this exists
//!
//! Most capability questions are *per declaration*: can this backend render
//! this Record, this cardinality, this constrained float? A few are not. Two
//! declarations can each be individually renderable while the schema as a
//! whole is not, either because their generated names collide or because they
//! live in different namespaces.
//!
//! Readiness historically only asked the per-declaration questions, so a
//! selected service closure spanning two namespaces could be reported READY
//! even though all three backends reject the projected schema globally, and a
//! generated-name collision was invisible to readiness entirely. That is a
//! readiness/generation disagreement, and this module removes it by giving
//! both callers one shared preflight.
//!
//! # Boundaries
//!
//! A violated precondition is a **backend capability** statement, not a claim
//! that the Schema IR is malformed. Multi-namespace schema input is perfectly
//! valid IR that the frontend deliberately supports; the language backends
//! simply remain single-namespace today. Nothing here implements
//! multi-namespace generation, and nothing here weakens the backends' existing
//! single-namespace rejection.

use crate::backend_names::{BackendNameError, validate_backend_names};
use crate::coverage::BackendLanguage;
use ams_gra_oms_ir::SchemaIr;
use std::collections::BTreeSet;
use std::fmt;

/// A global backend precondition the supplied schema violates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendPreflightError {
    /// The backend emits into exactly one module/namespace/package, but the
    /// schema declares or uses more than one namespace.
    ///
    /// This is a current backend boundary, not a schema defect.
    MultipleNamespaces {
        language: BackendLanguage,
        /// Every distinct namespace URI the schema uses, sorted.
        namespaces: Vec<String>,
    },
    /// A generated host-language name cannot be emitted safely.
    ///
    /// Boxed so the whole error stays small: it is returned by value from
    /// every preflight call, including ones that succeed.
    Name(Box<BackendNameError>),
}

impl fmt::Display for BackendPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MultipleNamespaces {
                language,
                namespaces,
            } => write!(
                formatter,
                "{} generation requires exactly one namespace, but the selected schema uses {}: {}",
                language.name(),
                namespaces.len(),
                namespaces.join(", ")
            ),
            Self::Name(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BackendPreflightError {}

impl From<BackendNameError> for BackendPreflightError {
    fn from(error: BackendNameError) -> Self {
        Self::Name(Box::new(error))
    }
}

/// Every distinct namespace URI the schema actually uses, sorted.
///
/// Declared namespaces and the namespaces of declared identities are both
/// considered: a schema that declares one namespace but contains a type from
/// another is still multi-namespace input for the backends.
fn used_namespaces(schema: &SchemaIr) -> Vec<String> {
    let mut namespaces = schema
        .namespaces
        .iter()
        .map(|namespace| namespace.uri.clone())
        .collect::<BTreeSet<_>>();
    for declaration in &schema.types {
        namespaces.insert(declaration.name.namespace_uri.clone());
    }
    for message in &schema.messages {
        namespaces.insert(message.name.namespace_uri.clone());
    }
    namespaces.into_iter().collect()
}

/// Check every global precondition one backend imposes on a whole schema.
///
/// This is the single shared preflight. Backend `generate` entry points and
/// capability/readiness analysis both call it, so a READY verdict can never
/// disagree with what generation would do.
///
/// It deliberately answers only *global* questions. Per-declaration capability
/// remains `CoverageAnalysis`'s responsibility and is not duplicated here.
///
/// # Errors
///
/// Returns the first violated precondition: namespace boundary first, then
/// generated-name safety in schema order.
pub fn backend_preflight(
    schema: &SchemaIr,
    language: BackendLanguage,
) -> Result<(), BackendPreflightError> {
    // An empty schema is vacuously generable: there is nothing to emit, so
    // there is no namespace to need. This is the legitimate shape a Service
    // Contract with zero OMS Message exchanges projects to, and demanding a
    // namespace for it would turn "nothing to generate" into a failure.
    if schema.types.is_empty() && schema.messages.is_empty() {
        return Ok(());
    }
    // A non-empty schema always uses at least one namespace: every declared
    // identity carries one, even when the schema declares none explicitly.
    let namespaces = used_namespaces(schema);
    if namespaces.len() > 1 {
        return Err(BackendPreflightError::MultipleNamespaces {
            language,
            namespaces,
        });
    }
    validate_backend_names(schema, language)?;
    Ok(())
}

/// Whether one backend's global preconditions hold for `schema`.
#[must_use]
pub fn backend_preflight_passes(schema: &SchemaIr, language: BackendLanguage) -> bool {
    backend_preflight(schema, language).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{
        ConstraintSet, NamespaceDecl, PrimitiveKind, QualifiedName, SourceRef, TypeDecl, TypeKind,
    };

    fn declaration(namespace: &str, name: &str) -> TypeDecl {
        TypeDecl {
            name: QualifiedName::new(namespace, name),
            is_abstract: false,
            base_type: None,
            kind: TypeKind::Primitive(PrimitiveKind::String),
            constraints: ConstraintSet::default(),
            documentation: None,
            source: SourceRef {
                document: "test.ir".to_owned(),
                line: Some(1),
            },
        }
    }

    fn schema(namespaces: &[&str], types: Vec<TypeDecl>) -> SchemaIr {
        SchemaIr {
            schema_version: None,
            namespaces: namespaces
                .iter()
                .map(|uri| NamespaceDecl {
                    uri: (*uri).to_owned(),
                    preferred_prefix: None,
                })
                .collect(),
            types,
            messages: Vec::new(),
        }
    }

    #[test]
    fn a_single_namespace_schema_passes_for_every_backend() {
        let schema = schema(&["urn:a"], vec![declaration("urn:a", "Track")]);
        for language in BackendLanguage::ALL {
            assert!(backend_preflight(&schema, language).is_ok());
        }
    }

    #[test]
    fn two_namespaces_are_a_backend_boundary_for_every_backend() {
        let schema = schema(
            &["urn:a", "urn:b"],
            vec![declaration("urn:a", "Track"), declaration("urn:b", "Site")],
        );
        for language in BackendLanguage::ALL {
            let error = backend_preflight(&schema, language)
                .expect_err("multi-namespace input is a backend boundary");
            let BackendPreflightError::MultipleNamespaces { namespaces, .. } = error else {
                panic!("expected MultipleNamespaces, got {error:?}");
            };
            assert_eq!(namespaces, ["urn:a", "urn:b"]);
        }
    }

    /// A declaration from an undeclared second namespace is multi-namespace
    /// input too, even though only one namespace is declared.
    #[test]
    fn a_foreign_declaration_namespace_also_counts() {
        let schema = schema(
            &["urn:a"],
            vec![declaration("urn:a", "Track"), declaration("urn:b", "Site")],
        );
        assert!(matches!(
            backend_preflight(&schema, BackendLanguage::Rust),
            Err(BackendPreflightError::MultipleNamespaces { .. })
        ));
    }

    /// A contract with zero OMS Message exchanges projects to an empty
    /// schema. There is nothing to emit, so there is nothing to reject.
    #[test]
    fn an_empty_schema_is_vacuously_generable() {
        let schema = schema(&[], Vec::new());
        for language in BackendLanguage::ALL {
            assert!(backend_preflight(&schema, language).is_ok());
        }
    }

    /// An undeclared namespace is still the declaration's namespace, so a
    /// schema whose types all share it is single-namespace input rather than a
    /// missing-namespace failure. The backends derive their package/namespace
    /// name from the URI and reject an unusable one themselves.
    #[test]
    fn an_undeclared_but_consistent_namespace_is_single_namespace_input() {
        let schema = schema(&[], vec![declaration("urn:a", "Track")]);
        assert!(backend_preflight(&schema, BackendLanguage::Ada).is_ok());
    }

    /// Generated-name safety is part of the same preflight, so readiness and
    /// generation consult one mechanism rather than two.
    #[test]
    fn colliding_generated_names_fail_preflight() {
        let schema = schema(
            &["urn:a"],
            vec![
                declaration("urn:a", "foo_bar"),
                declaration("urn:a", "fooBar"),
            ],
        );
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert!(
                matches!(
                    backend_preflight(&schema, language),
                    Err(BackendPreflightError::Name(error))
                        if matches!(*error, BackendNameError::Collision { .. })
                ),
                "{language:?} must reject converging declaration names"
            );
        }
    }
}
