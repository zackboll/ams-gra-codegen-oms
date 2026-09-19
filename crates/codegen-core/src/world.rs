//! Language-neutral generation world policy.
//!
//! Task 027 established that the loaded XSD dependency graph is a closed
//! *file* set, but that the legal *type universe* is not necessarily closed:
//! UCI documents several abstract types as open extension points whose
//! concrete derived types are supplied by schemas outside the open,
//! unclassified schema set. Schema-set closure therefore does not imply
//! type-universe closure, and no reliable machine-readable per-type
//! discriminator distinguishes an open extension point from an ordinary
//! abstract declaration.
//!
//! The generator cannot decide that question from schema content, so the
//! caller must state it. [`GenerationWorld`] is that statement. It is a
//! generator *policy*, not a property of the source schema: it is never
//! stored in `SchemaIr`, never inferred from imports, namespace counts, UCI
//! version metadata, type names, or documentation text.

/// The caller's asserted world model for generated values.
///
/// This is language-neutral on purpose. Ada, Rust, and C++ must not make
/// independent world-model decisions; they consume the single policy chosen
/// here so that all three agree about which values can legally exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationWorld {
    /// The supplied `SchemaIr` is asserted by the caller to contain **every**
    /// concrete type that may legally inhabit an abstract value.
    ///
    /// Under this assertion the known concrete descendants of an abstract
    /// structural declaration are exhaustive, so Task 024 closed-sum lowering
    /// is valid; and an abstract structural declaration with zero known
    /// concrete descendants is genuinely uninhabited, so Task 026 absent-only
    /// storage elision is valid.
    ///
    /// This is a semantic assertion by the caller, not a schema-validation
    /// property: loading a complete, self-consistent schema set does **not**
    /// by itself prove it.
    ClosedSchemaSet,
    /// Additional concrete derived types may exist outside the supplied
    /// `SchemaIr`.
    ///
    /// The known descendant set of an abstract structural declaration is not
    /// assumed exhaustive — including when it is empty, when it has exactly
    /// one member, and when it has many. Consequently no abstract structural
    /// *value* can be represented from schema knowledge alone, and every such
    /// value position fails closed until a runtime-polymorphic/extension
    /// representation exists.
    ///
    /// Abstract declarations used only as inheritance ancestry are unaffected:
    /// a concrete descendant still has a known effective field layout.
    OpenExtensions,
}

impl GenerationWorld {
    /// True when the caller asserted the supplied schema set is the complete
    /// legal type universe.
    #[must_use]
    pub const fn is_closed_schema_set(self) -> bool {
        matches!(self, Self::ClosedSchemaSet)
    }

    /// Stable lower-kebab label used by the CLI and by saved evidence so that
    /// a stored report can never become semantically ambiguous.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClosedSchemaSet => "closed-schema",
            Self::OpenExtensions => "open-extensions",
        }
    }
}

impl std::fmt::Display for GenerationWorld {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::GenerationWorld;

    #[test]
    fn labels_are_stable() {
        assert_eq!(GenerationWorld::ClosedSchemaSet.label(), "closed-schema");
        assert_eq!(GenerationWorld::OpenExtensions.label(), "open-extensions");
        assert_eq!(
            GenerationWorld::OpenExtensions.to_string(),
            "open-extensions"
        );
    }

    #[test]
    fn closed_predicate_is_exact() {
        assert!(GenerationWorld::ClosedSchemaSet.is_closed_schema_set());
        assert!(!GenerationWorld::OpenExtensions.is_closed_schema_set());
    }

    #[test]
    fn world_has_no_implicit_default() {
        // A `Default` impl would silently reintroduce an implicit world at
        // every call site that forgot to state one; this test documents the
        // deliberate absence. (Compile-time enforcement: the type has no
        // `Default` derive, so `GenerationWorld::default()` does not exist.)
        let worlds = [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ];
        assert_eq!(worlds.len(), 2);
    }
}
