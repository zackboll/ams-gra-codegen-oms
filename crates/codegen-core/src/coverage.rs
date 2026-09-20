use crate::structure::{
    EffectiveStructuralType, StructuralProjectionError, StructuralSegmentContent,
    project_with_index,
};
use crate::{
    ADA_PORTABLE_POSITIVE_INDEX_MAX, AbstractValueOccurrenceRenderability,
    AbstractValueProjectionError, AbstractValueSemantics, AbstractValueTopology,
    BackendPreflightError, GenerationWorld, abstract_value_occurrence_renderable,
    abstract_value_targets, backend_preflight, classify_abstract_value_topology, floating_domain,
    inclusive_integral_domain, unsafe_named_declarations,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, MessageDecl, OccurrenceShape, PrimitiveKind,
    QualifiedName, SchemaIr, TypeDecl, TypeKind, TypeRef, TypeRefTarget,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackendLanguage {
    Ada,
    Rust,
    Cpp,
}

impl BackendLanguage {
    pub const ALL: [Self; 3] = [Self::Ada, Self::Rust, Self::Cpp];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ada => "Ada",
            Self::Rust => "Rust",
            Self::Cpp => "C++",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeatureFamily {
    PrimitiveExpansion,
    CardinalityAndNillability,
    Choice,
    StructuralInheritanceAndAbstract,
    ConstrainedSimpleTypes,
}

impl FeatureFamily {
    pub const ALL: [Self; 5] = [
        Self::PrimitiveExpansion,
        Self::CardinalityAndNillability,
        Self::Choice,
        Self::StructuralInheritanceAndAbstract,
        Self::ConstrainedSimpleTypes,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PrimitiveExpansion => "primitive refs/declarations",
            Self::CardinalityAndNillability => "cardinality/nillability",
            Self::Choice => "Choice",
            Self::StructuralInheritanceAndAbstract => "remaining structural inheritance/abstract",
            Self::ConstrainedSimpleTypes => "constrained simple types",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchemaInventory {
    /// Stable category names and counts. Categories with zero observations are retained.
    pub counts: BTreeMap<String, usize>,
    /// Stable evidence lines for named declarations and collision diagnostics.
    pub evidence: Vec<String>,
}

impl SchemaInventory {
    #[must_use]
    pub fn count(&self, category: &str) -> usize {
        self.counts.get(category).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackendCoverage {
    pub language: BackendLanguage,
    pub declarations_total: usize,
    pub declaration_kinds_renderable: usize,
    pub declarations_fully_renderable: usize,
    pub fields_total: usize,
    pub field_type_references_renderable: usize,
    pub field_occurrences_renderable: usize,
    pub messages_total: usize,
    pub message_closures_renderable: usize,
}

impl BackendCoverage {
    #[must_use]
    pub fn kind_percent(&self) -> f64 {
        percent(self.declaration_kinds_renderable, self.declarations_total)
    }

    #[must_use]
    pub fn declaration_percent(&self) -> f64 {
        percent(self.declarations_fully_renderable, self.declarations_total)
    }

    #[must_use]
    pub fn field_type_percent(&self) -> f64 {
        percent(self.field_type_references_renderable, self.fields_total)
    }

    #[must_use]
    pub fn field_occurrence_percent(&self) -> f64 {
        percent(self.field_occurrences_renderable, self.fields_total)
    }

    #[must_use]
    pub fn message_percent(&self) -> f64 {
        percent(self.message_closures_renderable, self.messages_total)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageError {
    InvalidSchema(String),
    MissingDeclaration(QualifiedName),
    DependencyCycle(Vec<QualifiedName>),
    Projection(Box<StructuralProjectionError>),
}

impl fmt::Display for CoverageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchema(error) => write!(formatter, "invalid schema IR: {error}"),
            Self::MissingDeclaration(name) => {
                write!(formatter, "missing declaration {}", format_name(name))
            }
            Self::DependencyCycle(names) => write!(
                formatter,
                "dependency cycle: {}",
                names
                    .iter()
                    .map(format_name)
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ),
            Self::Projection(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CoverageError {}

impl From<StructuralProjectionError> for CoverageError {
    fn from(error: StructuralProjectionError) -> Self {
        Self::Projection(Box::new(error))
    }
}

/// Per-declaration renderability, indexed by Schema IR declaration position.
///
/// Produced once per language/feature-set request by
/// [`CoverageAnalysis::declaration_renderability`] and then queried many times.
/// Deliberately crate-private: it is a coverage implementation detail, and the
/// public surface Task 031 exposes is the service readiness result, not this
/// vector.
pub(crate) struct DeclarationRenderability {
    full: Vec<bool>,
}

impl DeclarationRenderability {
    /// True when the declaration at Schema IR position `index` is renderable.
    pub(crate) fn is_renderable(&self, index: usize) -> bool {
        self.full[index]
    }
}

/// Indexed, reusable semantic analysis over validated Schema IR, measured
/// under one explicit [`GenerationWorld`] policy.
///
/// The world is stored once, at construction, and every policy-dependent
/// index is computed once in [`CoverageAnalysis::new`]. Coverage re-evaluates
/// every declaration for every language × feature-combination, so no
/// per-field schema scan or per-field abstract projection may happen during
/// measurement; doing so previously caused an accidental quadratic
/// regression.
pub struct CoverageAnalysis<'a> {
    schema: &'a SchemaIr,
    /// The caller's asserted world model. Stored, never inferred.
    world: GenerationWorld,
    declarations: BTreeMap<&'a QualifiedName, &'a TypeDecl>,
    indices: BTreeMap<&'a QualifiedName, usize>,
    abstract_value_topologies: BTreeMap<&'a QualifiedName, AbstractValueTopology<'a>>,
    /// Closed-world only: zero-descendant abstract targets whose every
    /// reference is a supported absent-only occurrence. Always empty under
    /// `OpenExtensions`, where zero known descendants does not imply
    /// uninhabited. Computed once, for the same performance reason as above.
    fully_elided_targets: BTreeSet<&'a QualifiedName>,
    /// Per-language global backend preflight outcome for this whole schema.
    ///
    /// # Why coverage consults preflight at all
    ///
    /// Most capability questions are per declaration, but a few are not: a
    /// generated-name collision, a reserved generated identifier, or a
    /// multi-namespace schema makes the backend reject the schema *globally*,
    /// however renderable each declaration is in isolation. Without this,
    /// coverage could report declarations fully renderable and message
    /// closures generable for a schema that generation refuses outright --
    /// exactly the coverage/generation disagreement this cleanup exists to
    /// remove.
    ///
    /// # Why it is stored
    ///
    /// The outcome depends only on the schema and the language, never on the
    /// hypothetical feature set. Coverage evaluates every language against
    /// every feature combination, so computing it once per language here
    /// keeps a whole-schema scan out of that loop. Three scans at
    /// construction replace dozens during analysis.
    ///
    /// Policy is **not** duplicated: this stores the result of the shared
    /// [`backend_preflight`] call that backend generation itself makes.
    backend_preflight: BTreeMap<BackendLanguage, Option<BackendPreflightError>>,
    /// Per-language set of declarations whose generated names are unsafe.
    ///
    /// Attributed, not global: one reserved member name makes exactly its
    /// owning declaration unrenderable, leaving the rest of the schema's
    /// metrics honest. Computed once per language for the same reason as
    /// `backend_preflight` above -- it is feature-set independent, and
    /// recomputing it inside the feature loop would rescan the whole schema
    /// repeatedly.
    unsafe_named_declarations: BTreeMap<BackendLanguage, BTreeSet<QualifiedName>>,
}

impl<'a> CoverageAnalysis<'a> {
    /// Build an index after validating the complete IR, under an explicit
    /// world policy.
    ///
    /// There is deliberately no closed-world default: saved coverage evidence
    /// must never be semantically ambiguous about which type universe it
    /// measured.
    ///
    /// # Errors
    ///
    /// Returns an error if the schema IR is invalid or if abstract value
    /// topology classification fails.
    pub fn new(schema: &'a SchemaIr, world: GenerationWorld) -> Result<Self, CoverageError> {
        schema
            .validate()
            .map_err(|error| CoverageError::InvalidSchema(error.to_string()))?;
        let abstract_value_targets = abstract_value_targets(schema)
            .into_iter()
            .map(|declaration| &declaration.name)
            .collect::<BTreeSet<_>>();
        let indices = schema
            .types
            .iter()
            .enumerate()
            .map(|(index, declaration)| (&declaration.name, index))
            .collect::<BTreeMap<_, _>>();
        let abstract_value_topologies = abstract_value_targets
            .iter()
            .map(|name| {
                classify_abstract_value_topology(schema, name)
                    .or_else(|error| match error {
                        AbstractValueProjectionError::NoConcreteDescendants(_) => Ok(
                            AbstractValueTopology::NoConcreteDescendants((*name).clone()),
                        ),
                        error => Err(error),
                    })
                    .map(|topology| (*name, topology))
                    .map_err(|error| CoverageError::InvalidSchema(error.to_string()))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let mut analysis = Self {
            schema,
            world,
            declarations: schema
                .types
                .iter()
                .map(|declaration| (&declaration.name, declaration))
                .collect(),
            indices,
            abstract_value_topologies,
            fully_elided_targets: BTreeSet::new(),
            // Computed once per language, before any per-declaration work.
            // Both are computed once per language under this analysis's own
            // world, never per declaration and never inside the feature loop:
            // the name model is world-aware, so it must be measured in exactly
            // the world this coverage run reports.
            backend_preflight: BackendLanguage::ALL
                .into_iter()
                .map(|language| (language, backend_preflight(schema, language, world).err()))
                .collect(),
            unsafe_named_declarations: BackendLanguage::ALL
                .into_iter()
                .map(|language| (language, unsafe_named_declarations(schema, language, world)))
                .collect(),
        };
        // Policy-dependent index, computed exactly once. Under
        // `OpenExtensions` no target is elided, because zero known
        // descendants does not prove zero legal descendants.
        analysis.fully_elided_targets = if world.is_closed_schema_set() {
            analysis.compute_fully_elided_targets()
        } else {
            BTreeSet::new()
        };
        Ok(analysis)
    }

    /// The world policy this analysis was constructed with.
    #[must_use]
    pub const fn world(&self) -> GenerationWorld {
        self.world
    }

    /// Return a compositor-preserving projection using the shared declaration index.
    pub fn structural_projection(
        &self,
        name: &QualifiedName,
    ) -> Result<EffectiveStructuralType<'a>, CoverageError> {
        Ok(project_with_index(&self.declarations, name)?)
    }

    /// Return the transitive named dependency closure in original declaration order.
    /// The root is included. Cycles are reported deterministically.
    pub fn dependency_closure(
        &self,
        root: &QualifiedName,
    ) -> Result<Vec<&'a TypeDecl>, CoverageError> {
        let root_index = self
            .indices
            .get(root)
            .copied()
            .ok_or_else(|| CoverageError::MissingDeclaration(root.clone()))?;
        let mut included = BTreeSet::new();
        let mut visiting = Vec::new();
        let mut visited = BTreeSet::new();
        self.visit_dependencies(root_index, &mut included, &mut visiting, &mut visited)?;
        Ok(included
            .into_iter()
            .map(|index| &self.schema.types[index])
            .collect())
    }

    /// Produce the complete backend-relevant semantic inventory.
    pub fn inventory(&self) -> Result<SchemaInventory, CoverageError> {
        let mut inventory = initialized_inventory();
        for (index, declaration) in self.schema.types.iter().enumerate() {
            increment(&mut inventory, "declarations.total");
            increment(
                &mut inventory,
                if declaration.is_abstract {
                    "declarations.abstract"
                } else {
                    "declarations.concrete"
                },
            );
            count_declaration(&mut inventory, declaration);
            if declaration.base_type.is_some() {
                increment(&mut inventory, "declarations.with_base_type");
            }
            count_base(&mut inventory, declaration, index, &self.indices);
            match &declaration.kind {
                TypeKind::Record { fields } => {
                    for field in fields {
                        count_member(&mut inventory, field, false);
                    }
                }
                TypeKind::Choice { alternatives } => {
                    count_choice(&mut inventory, declaration, alternatives);
                    for alternative in alternatives {
                        count_member(&mut inventory, alternative, true);
                    }
                }
                _ => {}
            }
        }
        self.count_structural(&mut inventory)?;
        self.count_abstract_usage(&mut inventory);
        self.count_abstract_value_topologies(&mut inventory);
        self.count_collisions(&mut inventory)?;
        inventory.evidence.sort();
        Ok(inventory)
    }

    /// The global backend precondition this schema violates for `language`,
    /// if any.
    ///
    /// Exposed so a caller can explain a zeroed declaration/message figure
    /// rather than having to guess why it is zero. The authoritative rule set
    /// is [`backend_preflight`]; this only surfaces its stored result.
    #[must_use]
    pub fn backend_preflight_error(
        &self,
        language: BackendLanguage,
    ) -> Option<&BackendPreflightError> {
        self.backend_preflight
            .get(&language)
            .and_then(Option::as_ref)
    }

    /// Whether the generated module/namespace/package itself cannot be
    /// produced for `language`.
    ///
    /// True for a multi-namespace schema, and for a namespace URI whose
    /// derived unit identifier is illegal or reserved. Both make the backend
    /// emit nothing at all, which is why they zero the whole-schema figures
    /// rather than being attributed to some declaration.
    fn namespace_unit_is_unusable(&self, language: BackendLanguage) -> bool {
        match self.backend_preflight_error(language) {
            None => false,
            Some(BackendPreflightError::MultipleNamespaces { .. }) => true,
            Some(BackendPreflightError::Name(error)) => {
                matches!(error.region(), crate::NameRegion::NamespaceUnit)
            }
        }
    }

    /// Measure current backend representability without invoking first-failure generation.
    pub fn backend_coverage(
        &self,
        language: BackendLanguage,
    ) -> Result<BackendCoverage, CoverageError> {
        self.backend_coverage_with(language, &BTreeSet::new())
    }

    /// Return message closures made renderable by each requested hypothetical feature set.
    pub fn impact(
        &self,
        language: BackendLanguage,
        enabled: &[FeatureFamily],
    ) -> Result<usize, CoverageError> {
        self.backend_coverage_with(language, &enabled.iter().copied().collect())
            .map(|coverage| coverage.message_closures_renderable)
    }

    /// Deterministic human-readable report used by the CLI and authoritative probes.
    pub fn report(&self) -> Result<String, CoverageError> {
        let inventory = self.inventory()?;
        let mut output = String::new();
        // Task 028 section 59: saved coverage evidence must state which type
        // universe it measured, or it becomes semantically ambiguous. This is
        // a report line only; the world is never part of the IR inventory.
        output.push_str(&format!("generation world: {}\n", self.world.label()));
        output.push_str("inventory\n");
        for (category, count) in inventory.counts {
            output.push_str(&format!("{category}: {count}\n"));
        }
        output.push_str("evidence\n");
        for line in inventory.evidence {
            output.push_str(&line);
            output.push('\n');
        }
        output.push_str("backend coverage\n");
        for language in BackendLanguage::ALL {
            let coverage = self.backend_coverage(language)?;
            output.push_str(&format!(
                "{}: kinds {}/{} ({:.2}%), declarations {}/{} ({:.2}%), field-types {}/{} ({:.2}%), field-occurrences {}/{} ({:.2}%), message-closures {}/{} ({:.2}%)\n",
                language.name(), coverage.declaration_kinds_renderable, coverage.declarations_total, coverage.kind_percent(),
                coverage.declarations_fully_renderable, coverage.declarations_total, coverage.declaration_percent(),
                coverage.field_type_references_renderable, coverage.fields_total, coverage.field_type_percent(),
                coverage.field_occurrences_renderable, coverage.fields_total, coverage.field_occurrence_percent(),
                coverage.message_closures_renderable, coverage.messages_total, coverage.message_percent(),
            ));
            let baseline = coverage.message_closures_renderable;
            for mask in 1..(1 << FeatureFamily::ALL.len()) {
                let features = FeatureFamily::ALL
                    .iter()
                    .enumerate()
                    .filter_map(|(index, feature)| ((mask & (1 << index)) != 0).then_some(*feature))
                    .collect::<Vec<_>>();
                let label = features
                    .iter()
                    .map(|feature| feature.name())
                    .collect::<Vec<_>>()
                    .join(" + ");
                let total = self.impact(language, &features)?;
                output.push_str(&format!(
                    "  {label}: {total} (+{})\n",
                    total.saturating_sub(baseline)
                ));
            }
        }
        Ok(output)
    }

    fn visit_dependencies(
        &self,
        index: usize,
        included: &mut BTreeSet<usize>,
        visiting: &mut Vec<usize>,
        visited: &mut BTreeSet<usize>,
    ) -> Result<(), CoverageError> {
        if let Some(position) = visiting.iter().position(|candidate| *candidate == index) {
            let mut cycle = visiting[position..]
                .iter()
                .map(|index| self.schema.types[*index].name.clone())
                .collect::<Vec<_>>();
            cycle.push(self.schema.types[index].name.clone());
            return Err(CoverageError::DependencyCycle(cycle));
        }
        if !visited.insert(index) {
            return Ok(());
        }
        visiting.push(index);
        included.insert(index);
        for name in direct_dependencies(&self.schema.types[index]) {
            let dependency = self
                .indices
                .get(name)
                .copied()
                .ok_or_else(|| CoverageError::MissingDeclaration(name.clone()))?;
            self.visit_dependencies(dependency, included, visiting, visited)?;
        }
        visiting.pop();
        Ok(())
    }

    /// The per-declaration renderability vector, indexed by Schema IR
    /// declaration position, for one language under one hypothetical feature
    /// set.
    ///
    /// This is the single semantic definition of "can this backend emit this
    /// declaration". Full-schema [`BackendCoverage`] and Task 031's
    /// contract-selected service readiness both consume it, so there is
    /// exactly one capability model rather than two drifting copies.
    ///
    /// It is deliberately computed whole-schema, once per request: closed-world
    /// Task 024 wrapper propagation and Task 026 elision are global fixpoints
    /// over the declaration vector, so there is no correct way to answer the
    /// question for one declaration in isolation.
    pub(crate) fn declaration_renderability(
        &self,
        language: BackendLanguage,
        enabled: &BTreeSet<FeatureFamily>,
    ) -> DeclarationRenderability {
        let mut full = self
            .schema
            .types
            .iter()
            .map(|declaration| self.declaration_renderable(declaration, language, enabled))
            .collect::<Vec<_>>();
        // Closed world only: a Task 024 wrapper is renderable only if every
        // variant it embeds is. Under `OpenExtensions` no wrapper is emitted,
        // so there is no such transitive edge to apply.
        if self.world.is_closed_schema_set() {
            for (name, topology) in &self.abstract_value_topologies {
                let index = self.indices[name];
                if let AbstractValueTopology::Acyclic(projection) = topology {
                    full[index] &= projection
                        .concrete_descendants
                        .iter()
                        .all(|descendant| full[self.indices[&descendant.name]]);
                }
            }
        }
        // A target whose every occurrence is absent-only is never emitted, so
        // it is vacuously renderable and must not block closures that merely
        // mention it. This is a property of the schema itself, not of any
        // hypothetical feature, so it applies under every feature set --
        // gating it would make coverage non-monotonic.
        for name in &self.fully_elided_targets {
            full[self.indices[name]] = true;
        }
        // Corrective cleanup: a declaration the backend cannot *name* safely
        // is not renderable, however well its structure is modelled. This is
        // applied last, so it overrides even the elision above: an elided
        // target still occupies its generated identifier.
        //
        // Attribution is per declaration rather than whole-schema, because a
        // reserved member name condemns that declaration and nothing else.
        // The rules come from the shared backend-name model; coverage only
        // consumes the attributed result.
        for name in &self.unsafe_named_declarations[&language] {
            if let Some(index) = self.indices.get(name) {
                full[*index] = false;
            }
        }
        DeclarationRenderability { full }
    }

    /// The renderability vector for what a backend can emit **today**.
    ///
    /// No [`FeatureFamily`] is enabled, so this answers the actual-capability
    /// question rather than "would be renderable if X were implemented".
    /// Task 031 readiness uses only this entry point.
    pub(crate) fn baseline_renderability(
        &self,
        language: BackendLanguage,
    ) -> DeclarationRenderability {
        self.declaration_renderability(language, &BTreeSet::new())
    }

    /// Schema IR declaration position of `name`, or `None` when the schema set
    /// does not declare it.
    pub(crate) fn declaration_index(&self, name: &QualifiedName) -> Option<usize> {
        self.indices.get(name).copied()
    }

    /// True when a message and its entire payload closure are renderable.
    ///
    /// `renderability` must have been produced by this same analysis under the
    /// same `enabled` set; passing it in keeps the whole-schema computation out
    /// of the per-message loop.
    pub(crate) fn message_closure_renderable(
        &self,
        message: &MessageDecl,
        language: BackendLanguage,
        enabled: &BTreeSet<FeatureFamily>,
        renderability: &DeclarationRenderability,
    ) -> Result<bool, CoverageError> {
        Ok(self.message_renderable(message, language, enabled)
            && match &message.payload_type.target {
                TypeRefTarget::Primitive(kind) => primitive_ref_renderable(*kind, enabled),
                TypeRefTarget::Named(name) => self
                    .dependency_closure(name)?
                    .iter()
                    .all(|declaration| renderability.full[self.indices[&declaration.name]]),
            })
    }

    /// True when the message's own payload reference is renderable, ignoring
    /// the transitive closure behind it.
    ///
    /// Used only to attribute a blocker: a message may have a fully renderable
    /// closure yet still be unrenderable because the payload *reference* itself
    /// is an unsupported abstract structural value position.
    pub(crate) fn message_payload_reference_renderable(
        &self,
        message: &MessageDecl,
        language: BackendLanguage,
        enabled: &BTreeSet<FeatureFamily>,
    ) -> bool {
        self.message_renderable(message, language, enabled)
    }

    fn backend_coverage_with(
        &self,
        language: BackendLanguage,
        enabled: &BTreeSet<FeatureFamily>,
    ) -> Result<BackendCoverage, CoverageError> {
        let fields = all_members(self.schema);
        let renderability = self.declaration_renderability(language, enabled);
        let full = &renderability.full;
        let mut message_closures_renderable = 0;
        for message in &self.schema.messages {
            if self.message_closure_renderable(message, language, enabled, &renderability)? {
                message_closures_renderable += 1;
            }
        }
        // A *namespace-level* precondition failure is not attributable to any
        // declaration: the generated module/namespace/package itself cannot be
        // named, so the backend emits nothing at all. Claiming declarations
        // are fully renderable, or message closures generable, would be a
        // straightforward overclaim.
        //
        // Generated-name failures are handled differently, and earlier, in
        // `declaration_renderability`: those *are* attributable, so the
        // responsible declarations are marked unrenderable individually and
        // the rest of the schema keeps its honest metrics. Condemning a whole
        // schema for one reserved member name would destroy far more evidence
        // than it corrects.
        //
        // `declaration_kinds_renderable`, `field_type_references_renderable`,
        // and `field_occurrences_renderable` are left intact even here: they
        // answer narrower questions ("does this backend model this kind, this
        // type reference, this occurrence?") that an unusable package name
        // does not change, and feature-impact analysis reads them.
        //
        // Both branches consume the authoritative shared preflight result; no
        // naming or namespace rule is reimplemented in coverage.
        let (declarations_fully_renderable, message_closures_renderable) =
            if self.namespace_unit_is_unusable(language) {
                (0, 0)
            } else {
                (
                    full.iter().filter(|renderable| **renderable).count(),
                    message_closures_renderable,
                )
            };
        Ok(BackendCoverage {
            language,
            declarations_total: self.schema.types.len(),
            declaration_kinds_renderable: self
                .schema
                .types
                .iter()
                .filter(|declaration| kind_renderable(declaration, enabled))
                .count(),
            declarations_fully_renderable,
            fields_total: fields.len(),
            field_type_references_renderable: fields
                .iter()
                .filter(|field| type_ref_renderable(&field.type_ref, enabled))
                .count(),
            field_occurrences_renderable: fields
                .iter()
                .filter(|field| occurrence_renderable(field, language, enabled))
                .count(),
            messages_total: self.schema.messages.len(),
            message_closures_renderable,
        })
    }

    fn count_structural(&self, inventory: &mut SchemaInventory) -> Result<(), CoverageError> {
        let structural = self
            .schema
            .types
            .iter()
            .filter(|declaration| is_structural(declaration))
            .collect::<Vec<_>>();
        let mut bases = BTreeSet::new();
        let mut max_depth = 0;
        for declaration in structural {
            let projection = self.structural_projection(&declaration.name)?;
            max_depth = max_depth.max(projection.ancestry.len().saturating_sub(1));
            if let Some(base) = projection.immediate_base {
                bases.insert(base.name.clone());
                let shape = format!(
                    "inheritance.{}-with-{}-base",
                    kind_name(declaration),
                    kind_name(base)
                );
                increment(inventory, &shape);
                increment(
                    inventory,
                    if base.is_abstract {
                        "inheritance.abstract_bases"
                    } else {
                        "inheritance.concrete_bases"
                    },
                );
                match &declaration.kind {
                    TypeKind::Record { fields } if fields.is_empty() => {
                        increment(inventory, "inheritance.empty_local_extensions")
                    }
                    TypeKind::Record { .. } => {
                        increment(inventory, "inheritance.local_record_extensions")
                    }
                    TypeKind::Choice { .. } => {
                        increment(inventory, "inheritance.local_choice_extensions")
                    }
                    _ => {}
                }
            } else {
                increment(inventory, "inheritance.structural_roots");
            }
        }
        inventory
            .counts
            .insert("inheritance.maximum_depth".to_owned(), max_depth);
        inventory
            .counts
            .insert("inheritance.distinct_bases".to_owned(), bases.len());
        Ok(())
    }

    fn declaration_renderable(
        &self,
        declaration: &TypeDecl,
        language: BackendLanguage,
        enabled: &BTreeSet<FeatureFamily>,
    ) -> bool {
        if !matches!(
            declaration.kind,
            TypeKind::Record { .. } | TypeKind::Choice { .. }
        ) {
            return declaration_renderable(declaration, language, enabled);
        }
        let Ok(projection) = self.structural_projection(&declaration.name) else {
            return false;
        };
        if matches!(declaration.kind, TypeKind::Record { .. })
            && projection.segments.iter().any(|segment| {
                !matches!(segment.content, StructuralSegmentContent::RecordFields(_))
            })
            && !(enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract)
                && enabled.contains(&FeatureFamily::Choice))
        {
            return false;
        }
        let supported_choice_shape = matches!(declaration.kind, TypeKind::Choice { .. })
            && projection.segments.len() == 1
            && matches!(
                projection.segments[0].content,
                StructuralSegmentContent::ChoiceAlternatives(_)
            );
        if matches!(declaration.kind, TypeKind::Choice { .. })
            && !supported_choice_shape
            && !(enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract)
                && enabled.contains(&FeatureFamily::Choice))
        {
            return false;
        }
        let is_abstract_ancestor = declaration.is_abstract
            && self
                .schema
                .types
                .iter()
                .any(|candidate| named_is(candidate.base_type.as_ref(), &declaration.name));
        // A closed sum wrapper is only a baseline capability in the closed
        // world; open-world abstract values have no generated representation.
        let abstract_value_supported = declaration.is_abstract
            && self.world.is_closed_schema_set()
            && matches!(
                self.abstract_value_topologies.get(&declaration.name),
                Some(AbstractValueTopology::Acyclic(_))
            );
        if declaration.is_abstract
            && !is_abstract_ancestor
            && !abstract_value_supported
            && !enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract)
        {
            return false;
        }
        declaration_constraints_renderable(&declaration.constraints, enabled)
            && projection
                .segments
                .iter()
                .all(|segment| match segment.content {
                    StructuralSegmentContent::RecordFields(fields) => fields.iter().all(|field| {
                        self.absent_only_occurrence(field)
                            || (field_renderable(field, language, enabled)
                                && self.abstract_reference_renderable(field, enabled))
                    }),
                    StructuralSegmentContent::ChoiceAlternatives(fields) => {
                        (supported_choice_shape || enabled.contains(&FeatureFamily::Choice))
                            && fields.iter().all(|field| {
                                field_renderable(field, language, enabled)
                                    && self.abstract_reference_renderable(field, enabled)
                            })
                    }
                })
    }

    /// True when `field` is a Task 026 supported absent-only occurrence of an
    /// uninhabited (zero-descendant) abstract structural value: the field
    /// contributes no storage, so its own type/occurrence renderability is
    /// moot and it should not block the owning declaration.
    /// This deliberately consults the pre-indexed `abstract_value_topologies`
    /// map built once in `new` instead of re-deriving inhabitance per field:
    /// coverage evaluates every declaration under every feature combination, so
    /// a per-field schema walk would make the analysis quadratic.
    /// Absent-only elision is unconditional: it reflects the schema's own
    /// inhabitance, so it must not be gated on a hypothetical feature family
    /// (gating it would let enabling a family *reduce* measured coverage).
    fn absent_only_occurrence(&self, field: &FieldDecl) -> bool {
        self.absent_only_field(field)
    }

    /// Zero-descendant abstract targets that are *entirely* elided: every
    /// reference to them anywhere in the schema (record field, choice
    /// alternative, message payload, base, alias, list item) is a supported
    /// absent-only record-field occurrence. Such a declaration generates no
    /// code at all, so it cannot block any declaration that mentions it.
    ///
    /// This mirrors `ensure_zero_descendant_target_only_used_as_absent_only`
    /// in the emission planner so coverage and generation agree.
    fn compute_fully_elided_targets(&self) -> BTreeSet<&'a QualifiedName> {
        let mut elided = BTreeSet::new();
        for (name, topology) in &self.abstract_value_topologies {
            if !matches!(topology, AbstractValueTopology::NoConcreteDescendants(_)) {
                continue;
            }
            if self.every_reference_is_absent_only(name) {
                elided.insert(*name);
            }
        }
        elided
    }

    fn every_reference_is_absent_only(&self, target: &QualifiedName) -> bool {
        if self
            .schema
            .messages
            .iter()
            .any(|message| named_is(Some(&message.payload_type), target))
        {
            return false;
        }
        for declaration in &self.schema.types {
            if named_is(declaration.base_type.as_ref(), target) {
                return false;
            }
            match &declaration.kind {
                TypeKind::Alias(reference) => {
                    if named_is(Some(reference), target) {
                        return false;
                    }
                }
                TypeKind::List { item_type, .. } => {
                    if named_is(Some(item_type), target) {
                        return false;
                    }
                }
                TypeKind::Choice { alternatives } => {
                    if alternatives
                        .iter()
                        .any(|alternative| named_is(Some(&alternative.type_ref), target))
                    {
                        return false;
                    }
                }
                TypeKind::Record { fields } => {
                    if fields.iter().any(|field| {
                        named_is(Some(&field.type_ref), target) && !self.absent_only_field(field)
                    }) {
                        return false;
                    }
                }
                TypeKind::Primitive(_) | TypeKind::Enumeration { .. } => {}
            }
        }
        true
    }

    /// Feature-independent absent-only classification, used both by
    /// `absent_only_occurrence` and by whole-target elision: an elided field
    /// generates no storage, so it creates no demand on its target.
    fn absent_only_field(&self, field: &FieldDecl) -> bool {
        // Open world: a zero-known-descendant target may still be inhabited by
        // an external derived type, so nothing is absent-only. This is the
        // central Task 026 policy correction, and it is an O(1) world check.
        if !self.world.is_closed_schema_set() {
            return false;
        }
        let TypeRefTarget::Named(name) = &field.type_ref.target else {
            return false;
        };
        let Some(AbstractValueTopology::NoConcreteDescendants(_)) =
            self.abstract_value_topologies.get(name)
        else {
            return false;
        };
        let Some(declaration) = self.declarations.get(name) else {
            return false;
        };
        // Reuse the shared occurrence rule so coverage cannot drift from the
        // backends' storage decision. The semantics value is constructed from
        // the pre-indexed topology rather than re-derived per field.
        matches!(
            abstract_value_occurrence_renderable(
                &AbstractValueSemantics::NoLegalPayload { declaration },
                field.cardinality,
                field.nillable,
                field.constraints == ConstraintSet::default(),
            ),
            AbstractValueOccurrenceRenderability::AbsentOnly
        )
    }

    fn abstract_reference_renderable(
        &self,
        field: &FieldDecl,
        enabled: &BTreeSet<FeatureFamily>,
    ) -> bool {
        let TypeRefTarget::Named(name) = &field.type_ref.target else {
            return true;
        };
        if !self
            .declarations
            .get(name)
            .is_some_and(|declaration| declaration.is_abstract)
        {
            return true;
        }
        // Closed world: a Task 024 acyclic closed sum is baseline-renderable.
        // Open world: it is not, because the known descendant set is not
        // assumed exhaustive. `StructuralInheritanceAndAbstract` remains the
        // hypothetical family that models future support; under
        // `OpenExtensions` that hypothetical explicitly means
        // runtime-polymorphic open-extension capability, which no backend
        // implements today. Keeping it available preserves monotonicity:
        // enabling a family may only ever add capability.
        (self.world.is_closed_schema_set()
            && matches!(
                self.abstract_value_topologies.get(name),
                Some(AbstractValueTopology::Acyclic(_))
            ))
            || enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract)
    }

    fn message_renderable(
        &self,
        message: &MessageDecl,
        _language: BackendLanguage,
        enabled: &BTreeSet<FeatureFamily>,
    ) -> bool {
        // Message payloads are always effectively required/non-nillable
        // occurrences of their named type, so a zero-descendant payload is
        // never a supported absent-only occurrence (Task 026 section 31).
        let payload_field = FieldDecl {
            name: message.name.local_name.clone(),
            type_ref: message.payload_type.clone(),
            cardinality: Cardinality::REQUIRED_ONE,
            nillable: false,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: message.source.clone(),
        };
        type_ref_renderable(&message.payload_type, enabled)
            && self.abstract_reference_renderable(&payload_field, enabled)
    }

    fn count_abstract_usage(&self, inventory: &mut SchemaInventory) {
        let abstract_names = self
            .schema
            .types
            .iter()
            .filter(|declaration| declaration.is_abstract && is_structural(declaration))
            .map(|declaration| declaration.name.clone())
            .collect::<BTreeSet<_>>();
        for name in abstract_names {
            let mut usages = BTreeSet::new();
            for declaration in &self.schema.types {
                if named_is(declaration.base_type.as_ref(), &name) {
                    usages.insert("base");
                }
                match &declaration.kind {
                    TypeKind::Record { fields }
                        if fields
                            .iter()
                            .any(|field| named_is(Some(&field.type_ref), &name)) =>
                    {
                        usages.insert("record-field");
                    }
                    TypeKind::Choice { alternatives }
                        if alternatives
                            .iter()
                            .any(|field| named_is(Some(&field.type_ref), &name)) =>
                    {
                        usages.insert("choice-alternative");
                    }
                    TypeKind::List { item_type, .. } if named_is(Some(item_type), &name) => {
                        usages.insert("list-item");
                    }
                    TypeKind::Alias(target) if named_is(Some(target), &name) => {
                        usages.insert("other-named-reference");
                    }
                    _ => {}
                }
            }
            if self
                .schema
                .messages
                .iter()
                .any(|message| named_is(Some(&message.payload_type), &name))
            {
                usages.insert("message-payload");
            }
            if usages == BTreeSet::from(["base"]) {
                increment(inventory, "abstract_usage.base_only");
            }
            for usage in &usages {
                increment(inventory, &format!("abstract_usage.{usage}"));
            }
            inventory.evidence.push(format!(
                "abstract {}: {}",
                name.local_name,
                usages.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
    }

    fn count_abstract_value_topologies(&self, inventory: &mut SchemaInventory) {
        for (name, topology) in &self.abstract_value_topologies {
            increment(inventory, "abstract_value.targets");
            match topology {
                AbstractValueTopology::Acyclic(projection) => {
                    increment(inventory, "abstract_value.acyclic_targets");
                    inventory.evidence.push(format!(
                        "abstract value {}: acyclic ({} concrete descendants)",
                        name.local_name,
                        projection.concrete_descendants.len()
                    ));
                }
                AbstractValueTopology::NoConcreteDescendants(_) => {
                    increment(inventory, "abstract_value.zero_descendant_targets");
                    inventory.evidence.push(format!(
                        "abstract value {}: no concrete descendants",
                        name.local_name
                    ));
                }
                AbstractValueTopology::RecursiveValueGraph { cycle } => {
                    increment(inventory, "abstract_value.recursive_targets");
                    inventory.evidence.push(format!(
                        "abstract value {}: recursive {}",
                        name.local_name,
                        cycle
                            .iter()
                            .map(|name| name.local_name.as_str())
                            .collect::<Vec<_>>()
                            .join(" -> ")
                    ));
                }
            }
        }
    }

    fn count_collisions(&self, inventory: &mut SchemaInventory) -> Result<(), CoverageError> {
        for declaration in self
            .schema
            .types
            .iter()
            .filter(|declaration| is_structural(declaration))
        {
            let projection = self.structural_projection(&declaration.name)?;
            let mut names: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
            for segment in projection.segments {
                let (kind, members) = match segment.content {
                    StructuralSegmentContent::RecordFields(fields) => ("record", fields),
                    StructuralSegmentContent::ChoiceAlternatives(alternatives) => {
                        ("choice", alternatives)
                    }
                };
                for member in members {
                    if let Some((previous_kind, previous_owner)) =
                        names.insert(&member.name, (kind, &segment.owner.name.local_name))
                    {
                        increment(inventory, "collisions.same_name_across_levels");
                        increment(
                            inventory,
                            if previous_kind == kind {
                                if kind == "record" {
                                    "collisions.inherited_field_redeclarations"
                                } else {
                                    "collisions.inherited_choice_redeclarations"
                                }
                            } else {
                                "collisions.record_choice_names"
                            },
                        );
                        inventory.evidence.push(format!(
                            "collision {}.{}: {} {} -> {} {}",
                            declaration.name.local_name,
                            member.name,
                            previous_owner,
                            previous_kind,
                            segment.owner.name.local_name,
                            kind
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn initialized_inventory() -> SchemaInventory {
    let mut inventory = SchemaInventory::default();
    for category in [
        "declarations.total",
        "declarations.abstract",
        "declarations.concrete",
        "declarations.with_base_type",
        "kind.Alias",
        "kind.Enumeration",
        "kind.Record",
        "kind.Choice",
        "kind.List",
        "base.structural_named",
        "base.simple_named",
        "base.simple_primitive",
        "inheritance.record-with-record-base",
        "inheritance.record-with-choice-base",
        "inheritance.choice-with-record-base",
        "inheritance.choice-with-choice-base",
        "inheritance.abstract_bases",
        "inheritance.concrete_bases",
        "inheritance.structural_roots",
        "inheritance.empty_local_extensions",
        "inheritance.local_record_extensions",
        "inheritance.local_choice_extensions",
        "members.record_fields",
        "members.choice_alternatives",
        "members.named_references",
        "cardinality.required_1_1",
        "cardinality.optional_0_1",
        "cardinality.finite_repeated",
        "cardinality.unbounded_repeated",
        "cardinality.min_gt_1",
        "nillable.true",
        "nillable.false",
        "constraints.present",
        "constraints.min_inclusive",
        "constraints.max_inclusive",
        "constraints.min_exclusive",
        "constraints.max_exclusive",
        "constraints.length",
        "constraints.min_length",
        "constraints.max_length",
        "constraints.pattern",
        "constraints.white_space",
        "choice.standalone",
        "choice.participating_in_inheritance",
        "choice.repeated_alternatives",
        "choice.nillable_alternatives",
        "choice.duplicate_names",
        "choice.duplicate_payload_types",
        "choice.alternatives_sharing_payload_type",
        "abstract_usage.base_only",
        "abstract_usage.base",
        "abstract_usage.record-field",
        "abstract_usage.choice-alternative",
        "abstract_usage.message-payload",
        "abstract_usage.list-item",
        "abstract_usage.other-named-reference",
        "abstract_value.targets",
        "abstract_value.acyclic_targets",
        "abstract_value.zero_descendant_targets",
        "abstract_value.recursive_targets",
        "collisions.inherited_field_redeclarations",
        "collisions.inherited_choice_redeclarations",
        "collisions.record_choice_names",
        "collisions.same_name_across_levels",
    ] {
        inventory.counts.insert(category.to_owned(), 0);
    }
    for primitive in primitive_kinds() {
        inventory
            .counts
            .insert(format!("kind.Primitive.{}", primitive_name(primitive)), 0);
        inventory.counts.insert(
            format!("members.primitive_references.{}", primitive_name(primitive)),
            0,
        );
    }
    inventory
}

fn count_declaration(inventory: &mut SchemaInventory, declaration: &TypeDecl) {
    let category = match declaration.kind {
        TypeKind::Primitive(kind) => format!("kind.Primitive.{}", primitive_name(kind)),
        TypeKind::Alias(_) => "kind.Alias".to_owned(),
        TypeKind::Enumeration { .. } => "kind.Enumeration".to_owned(),
        TypeKind::Record { .. } => "kind.Record".to_owned(),
        TypeKind::Choice { .. } => "kind.Choice".to_owned(),
        TypeKind::List { .. } => "kind.List".to_owned(),
    };
    increment(inventory, &category);
}

fn count_base(
    inventory: &mut SchemaInventory,
    declaration: &TypeDecl,
    index: usize,
    indices: &BTreeMap<&QualifiedName, usize>,
) {
    let Some(base) = &declaration.base_type else {
        return;
    };
    match &base.target {
        TypeRefTarget::Primitive(_) => increment(inventory, "base.simple_primitive"),
        TypeRefTarget::Named(name) => {
            increment(
                inventory,
                if is_structural(declaration) {
                    "base.structural_named"
                } else {
                    "base.simple_named"
                },
            );
            if let Some(base_index) = indices.get(name) {
                increment(
                    inventory,
                    if *base_index < index {
                        "base.references_backward"
                    } else {
                        "base.references_forward"
                    },
                );
            }
        }
    }
}

fn count_member(inventory: &mut SchemaInventory, field: &FieldDecl, choice: bool) {
    increment(
        inventory,
        if choice {
            "members.choice_alternatives"
        } else {
            "members.record_fields"
        },
    );
    match field.type_ref.target {
        TypeRefTarget::Primitive(kind) => increment(
            inventory,
            &format!("members.primitive_references.{}", primitive_name(kind)),
        ),
        TypeRefTarget::Named(_) => increment(inventory, "members.named_references"),
    }
    match field.cardinality {
        Cardinality::REQUIRED_ONE => increment(inventory, "cardinality.required_1_1"),
        Cardinality::OPTIONAL_ONE => increment(inventory, "cardinality.optional_0_1"),
        Cardinality {
            max_occurs: None, ..
        } => increment(inventory, "cardinality.unbounded_repeated"),
        _ => increment(inventory, "cardinality.finite_repeated"),
    }
    if field.cardinality.min_occurs > 1 {
        increment(inventory, "cardinality.min_gt_1");
    }
    increment(
        inventory,
        if field.nillable {
            "nillable.true"
        } else {
            "nillable.false"
        },
    );
    count_constraints(inventory, &field.constraints);
}

fn count_constraints(inventory: &mut SchemaInventory, constraints: &ConstraintSet) {
    if constraints == &ConstraintSet::default() {
        return;
    }
    increment(inventory, "constraints.present");
    for (present, category) in [
        (
            constraints.min_inclusive.is_some(),
            "constraints.min_inclusive",
        ),
        (
            constraints.max_inclusive.is_some(),
            "constraints.max_inclusive",
        ),
        (
            constraints.min_exclusive.is_some(),
            "constraints.min_exclusive",
        ),
        (
            constraints.max_exclusive.is_some(),
            "constraints.max_exclusive",
        ),
        (constraints.length.is_some(), "constraints.length"),
        (constraints.min_length.is_some(), "constraints.min_length"),
        (constraints.max_length.is_some(), "constraints.max_length"),
        (
            !constraints.lexical.pattern_groups.is_empty(),
            "constraints.pattern",
        ),
        (
            constraints.lexical.white_space.is_some(),
            "constraints.white_space",
        ),
    ] {
        if present {
            increment(inventory, category);
        }
    }
}

fn count_choice(
    inventory: &mut SchemaInventory,
    declaration: &TypeDecl,
    alternatives: &[FieldDecl],
) {
    increment(
        inventory,
        if declaration.base_type.is_some() {
            "choice.participating_in_inheritance"
        } else {
            "choice.standalone"
        },
    );
    inventory.counts.insert(
        format!("choice.alternative_count.{}", alternatives.len()),
        inventory
            .counts
            .get(&format!("choice.alternative_count.{}", alternatives.len()))
            .copied()
            .unwrap_or(0)
            + 1,
    );
    let mut names = BTreeSet::new();
    let mut payloads = BTreeMap::new();
    for alternative in alternatives {
        if alternative.cardinality != Cardinality::REQUIRED_ONE {
            increment(inventory, "choice.repeated_alternatives");
        }
        if alternative.nillable {
            increment(inventory, "choice.nillable_alternatives");
        }
        if !names.insert(&alternative.name) {
            increment(inventory, "choice.duplicate_names");
        }
        *payloads
            .entry(format!("{:?}", alternative.type_ref.target))
            .or_insert(0usize) += 1;
    }
    let duplicate_payloads = payloads.values().filter(|count| **count > 1).count();
    if duplicate_payloads > 0 {
        increment(inventory, "choice.duplicate_payload_types");
    }
    *inventory
        .counts
        .entry("choice.alternatives_sharing_payload_type".to_owned())
        .or_default() += payloads
        .values()
        .map(|count| count.saturating_sub(1))
        .sum::<usize>();
}

/// The canonical (sorted, deduplicated) direct dependency set of a declaration.
///
/// The dependency *semantics* are not restated here: this defers to the single
/// shared [`crate::direct_named_dependencies`] model and only imposes the
/// canonical ordering that closure analysis wants. Keeping one semantic model
/// is deliberate, so coverage closures and contract-selected closures cannot
/// disagree about which types a declaration pulls in.
fn direct_dependencies(declaration: &TypeDecl) -> Vec<&QualifiedName> {
    let mut dependencies = crate::direct_named_dependencies(declaration);
    dependencies.sort();
    dependencies.dedup();
    dependencies
}
fn all_members(schema: &SchemaIr) -> Vec<&FieldDecl> {
    schema
        .types
        .iter()
        .flat_map(|declaration| match &declaration.kind {
            TypeKind::Record { fields } => fields.iter(),
            TypeKind::Choice { alternatives } => alternatives.iter(),
            _ => [].iter(),
        })
        .collect()
}
fn kind_renderable(declaration: &TypeDecl, enabled: &BTreeSet<FeatureFamily>) -> bool {
    match declaration.kind {
        // `FeatureFamily::Choice` models only remaining unsupported Choice
        // composition; ordinary Choice declarations have backend renderers.
        TypeKind::Primitive(
            PrimitiveKind::Boolean
            | PrimitiveKind::SignedInteger
            | PrimitiveKind::UnsignedInteger
            | PrimitiveKind::Float32
            | PrimitiveKind::Float64
            | PrimitiveKind::Binary,
        )
        | TypeKind::Enumeration { .. }
        | TypeKind::Record { .. }
        | TypeKind::Choice { .. } => true,
        TypeKind::Primitive(_) => enabled.contains(&FeatureFamily::PrimitiveExpansion),
        TypeKind::Alias(_) | TypeKind::List { .. } => false,
    }
}
fn declaration_renderable(
    declaration: &TypeDecl,
    language: BackendLanguage,
    enabled: &BTreeSet<FeatureFamily>,
) -> bool {
    if !kind_renderable(declaration, enabled) {
        return false;
    }
    if declaration.is_abstract
        && !enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract)
    {
        return false;
    }
    if is_structural(declaration)
        && declaration.base_type.is_some()
        && !enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract)
    {
        return false;
    }
    match &declaration.kind {
        TypeKind::Primitive(kind) => {
            primitive_declaration_renderable(*kind, &declaration.constraints, language, enabled)
        }
        TypeKind::Enumeration { variants } => {
            !variants.is_empty()
                && declaration_constraints_renderable(&declaration.constraints, enabled)
        }
        TypeKind::Record { fields }
        | TypeKind::Choice {
            alternatives: fields,
        } => {
            declaration_constraints_renderable(&declaration.constraints, enabled)
                && fields
                    .iter()
                    .all(|field| field_renderable(field, language, enabled))
        }
        TypeKind::Alias(_) | TypeKind::List { .. } => false,
    }
}
fn primitive_declaration_renderable(
    kind: PrimitiveKind,
    constraints: &ConstraintSet,
    _language: BackendLanguage,
    enabled: &BTreeSet<FeatureFamily>,
) -> bool {
    if kind == PrimitiveKind::Boolean {
        return constraints == &ConstraintSet::default()
            || enabled.contains(&FeatureFamily::ConstrainedSimpleTypes);
    }
    // Task 033: a named floating declaration is baseline-renderable when the
    // shared helper accepts it -- unconstrained, or a bound-only numeric
    // domain. This asks exactly the question the three backends ask, so
    // coverage cannot drift from what they will actually emit. A facet shape
    // the helper rejects (lexical, length, ambiguous same-side bounds, a
    // wrong-width bound) stays non-baseline and remains attributed to
    // `ConstrainedSimpleTypes` as future hypothetical support.
    if matches!(kind, PrimitiveKind::Float32 | PrimitiveKind::Float64) {
        return floating_domain(kind, constraints).is_ok()
            || enabled.contains(&FeatureFamily::ConstrainedSimpleTypes);
    }
    if kind == PrimitiveKind::Binary {
        return constraints == &ConstraintSet::default()
            || enabled.contains(&FeatureFamily::ConstrainedSimpleTypes);
    }
    if !matches!(
        kind,
        PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger
    ) {
        return enabled.contains(&FeatureFamily::PrimitiveExpansion)
            && (constraints == &ConstraintSet::default()
                || enabled.contains(&FeatureFamily::ConstrainedSimpleTypes));
    }
    inclusive_integral_domain(kind, constraints).is_ok_and(|domain| domain.is_some())
        || enabled.contains(&FeatureFamily::ConstrainedSimpleTypes)
}
fn declaration_constraints_renderable(
    constraints: &ConstraintSet,
    enabled: &BTreeSet<FeatureFamily>,
) -> bool {
    constraints == &ConstraintSet::default()
        || enabled.contains(&FeatureFamily::ConstrainedSimpleTypes)
}
fn field_renderable(
    field: &FieldDecl,
    language: BackendLanguage,
    enabled: &BTreeSet<FeatureFamily>,
) -> bool {
    type_ref_renderable(&field.type_ref, enabled)
        && occurrence_renderable(field, language, enabled)
        && (field.constraints == ConstraintSet::default()
            || matches!(field.type_ref.target, TypeRefTarget::Primitive(kind) if inclusive_integral_domain(kind, &field.constraints).is_ok_and(|domain| domain.is_some()))
            || enabled.contains(&FeatureFamily::ConstrainedSimpleTypes))
}
fn type_ref_renderable(type_ref: &TypeRef, enabled: &BTreeSet<FeatureFamily>) -> bool {
    match type_ref.target {
        TypeRefTarget::Named(_) => true,
        TypeRefTarget::Primitive(kind) => primitive_ref_renderable(kind, enabled),
    }
}
fn primitive_ref_renderable(kind: PrimitiveKind, enabled: &BTreeSet<FeatureFamily>) -> bool {
    matches!(
        kind,
        PrimitiveKind::Boolean
            | PrimitiveKind::SignedInteger
            | PrimitiveKind::UnsignedInteger
            | PrimitiveKind::Float32
            | PrimitiveKind::Float64
            | PrimitiveKind::String
            | PrimitiveKind::Binary
    ) || enabled.contains(&FeatureFamily::PrimitiveExpansion)
}
fn occurrence_renderable(
    field: &FieldDecl,
    language: BackendLanguage,
    enabled: &BTreeSet<FeatureFamily>,
) -> bool {
    if enabled.contains(&FeatureFamily::CardinalityAndNillability) {
        return true;
    }
    if field.nillable {
        return false;
    }
    match (language, field.cardinality.shape()) {
        (_, OccurrenceShape::RequiredOne) => true,
        (BackendLanguage::Ada, OccurrenceShape::OptionalOne) => {
            field.type_ref.target == TypeRefTarget::Primitive(PrimitiveKind::String)
        }
        (BackendLanguage::Rust | BackendLanguage::Cpp, OccurrenceShape::OptionalOne) => true,
        (BackendLanguage::Ada, OccurrenceShape::Bounded { max, .. })
            if max > 1 && max <= ADA_PORTABLE_POSITIVE_INDEX_MAX =>
        {
            true
        }
        (BackendLanguage::Ada, OccurrenceShape::Unbounded { min })
            if min <= ADA_PORTABLE_POSITIVE_INDEX_MAX =>
        {
            true
        }
        (BackendLanguage::Rust | BackendLanguage::Cpp, OccurrenceShape::Bounded { max, .. })
            if max > 1 =>
        {
            true
        }
        (BackendLanguage::Rust | BackendLanguage::Cpp, OccurrenceShape::Unbounded { .. }) => true,
        _ => false,
    }
}
fn is_structural(declaration: &TypeDecl) -> bool {
    matches!(
        declaration.kind,
        TypeKind::Record { .. } | TypeKind::Choice { .. }
    )
}
fn kind_name(declaration: &TypeDecl) -> &'static str {
    match declaration.kind {
        TypeKind::Record { .. } => "record",
        TypeKind::Choice { .. } => "choice",
        _ => "non-structural",
    }
}
fn named_is(type_ref: Option<&TypeRef>, name: &QualifiedName) -> bool {
    matches!(type_ref.map(|reference| &reference.target), Some(TypeRefTarget::Named(candidate)) if candidate == name)
}
fn increment(inventory: &mut SchemaInventory, category: &str) {
    *inventory.counts.entry(category.to_owned()).or_default() += 1;
}
fn percent(value: usize, total: usize) -> f64 {
    if total == 0 {
        100.0
    } else {
        value as f64 * 100.0 / total as f64
    }
}
fn format_name(name: &QualifiedName) -> String {
    format!("{{{}}}{}", name.namespace_uri, name.local_name)
}
fn primitive_kinds() -> [PrimitiveKind; 11] {
    [
        PrimitiveKind::Boolean,
        PrimitiveKind::SignedInteger,
        PrimitiveKind::UnsignedInteger,
        PrimitiveKind::Decimal,
        PrimitiveKind::Float32,
        PrimitiveKind::Float64,
        PrimitiveKind::String,
        PrimitiveKind::Binary,
        PrimitiveKind::DateTime,
        PrimitiveKind::Time,
        PrimitiveKind::Duration,
    ]
}
fn primitive_name(kind: PrimitiveKind) -> &'static str {
    match kind {
        PrimitiveKind::Boolean => "Boolean",
        PrimitiveKind::SignedInteger => "SignedInteger",
        PrimitiveKind::UnsignedInteger => "UnsignedInteger",
        PrimitiveKind::Decimal => "Decimal",
        PrimitiveKind::Float32 => "Float32",
        PrimitiveKind::Float64 => "Float64",
        PrimitiveKind::String => "String",
        PrimitiveKind::Binary => "Binary",
        PrimitiveKind::DateTime => "DateTime",
        PrimitiveKind::Time => "Time",
        PrimitiveKind::Duration => "Duration",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::NumericValue;
    use ams_gra_oms_ir::{MessageDecl, NamespaceDecl, SourceRef};

    const NS: &str = "urn:test";
    fn source() -> SourceRef {
        SourceRef {
            document: "test.ir".to_owned(),
            line: Some(1),
        }
    }
    fn named(name: &str) -> TypeRef {
        TypeRef::named(QualifiedName::new(NS, name))
    }
    fn declaration(name: &str, kind: TypeKind) -> TypeDecl {
        TypeDecl {
            name: QualifiedName::new(NS, name),
            is_abstract: false,
            base_type: None,
            kind,
            constraints: ConstraintSet::default(),
            documentation: None,
            source: source(),
        }
    }
    fn field(name: &str, target: &str) -> FieldDecl {
        field_ref(name, named(target))
    }
    fn field_ref(name: &str, type_ref: TypeRef) -> FieldDecl {
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
    fn scalar(name: &str) -> TypeDecl {
        let mut value = declaration(name, TypeKind::Primitive(PrimitiveKind::SignedInteger));
        value.constraints.min_inclusive = Some(NumericValue::Integer(0));
        value.constraints.max_inclusive = Some(NumericValue::Integer(10));
        value
    }
    fn schema(types: Vec<TypeDecl>) -> SchemaIr {
        SchemaIr {
            schema_version: None,
            namespaces: vec![NamespaceDecl {
                uri: NS.to_owned(),
                preferred_prefix: None,
            }],
            types,
            messages: Vec::new(),
        }
    }
    fn message_schema(types: Vec<TypeDecl>, payload: &str) -> SchemaIr {
        let mut schema = schema(types);
        schema.messages.push(MessageDecl {
            name: QualifiedName::new(NS, "TestMessage"),
            payload_type: named(payload),
            documentation: None,
            source: source(),
        });
        schema
    }
    fn assert_only_family_unblocks(schema: &SchemaIr, family: FeatureFamily) {
        let analysis = CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            assert_eq!(analysis.impact(language, &[]).unwrap(), 0);
            assert_eq!(analysis.impact(language, &[family]).unwrap(), 1);
            for other in FeatureFamily::ALL {
                if other != family {
                    assert_eq!(
                        analysis.impact(language, &[other]).unwrap(),
                        0,
                        "{} unexpectedly unblocked {}",
                        other.name(),
                        family.name()
                    );
                }
            }
        }
    }

    // ---- Corrective cleanup: coverage must respect global preflight ----

    /// A record whose members are all renderable, so any zeroing observed
    /// below comes from the global precondition rather than from ordinary
    /// per-declaration capability.
    fn renderable_record(name: &str) -> TypeDecl {
        declaration(
            name,
            TypeKind::Record {
                fields: vec![field_ref(
                    "Value",
                    TypeRef::primitive(PrimitiveKind::Float64),
                )],
            },
        )
    }

    fn coverage_of(schema: &SchemaIr, language: BackendLanguage) -> BackendCoverage {
        CoverageAnalysis::new(schema, GenerationWorld::ClosedSchemaSet)
            .expect("analysis should build")
            .backend_coverage(language)
            .expect("coverage should compute")
    }

    /// The control every other case below is measured against: a schema that
    /// passes preflight reports its ordinary per-declaration metrics, which
    /// this integration must leave completely untouched.
    #[test]
    fn single_namespace_control_metrics_are_unchanged() {
        let schema = schema(vec![renderable_record("Alpha"), renderable_record("Beta")]);
        for language in BackendLanguage::ALL {
            let coverage = coverage_of(&schema, language);
            assert_eq!(
                coverage.declarations_fully_renderable, 2,
                "{language:?} must report both declarations renderable"
            );
            assert_eq!(coverage.declarations_total, 2);
        }
    }

    /// A declaration whose generated name is a reserved word makes the whole
    /// schema ungenerable, so coverage must not go on reporting it as an
    /// ordinarily fully renderable declaration.
    #[test]
    fn a_reserved_generated_name_is_not_reported_as_fully_renderable() {
        // `Range` is an Ada reserved word (case-insensitively).
        let schema = schema(vec![renderable_record("Range"), renderable_record("Beta")]);
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet)
            .expect("analysis should build");
        assert!(
            matches!(
                analysis.backend_preflight_error(BackendLanguage::Ada),
                Some(BackendPreflightError::Name(_))
            ),
            "the fixture must really fail Ada preflight"
        );
        // Exactly the offending declaration stops being renderable. The
        // unaffected one keeps its honest metric: a single reserved name is
        // not a reason to disown the rest of the schema.
        assert_eq!(
            analysis
                .backend_coverage(BackendLanguage::Ada)
                .expect("coverage should compute")
                .declarations_fully_renderable,
            1,
            "only the reserved-word declaration is unrenderable"
        );
        // The other backends are unaffected: `Range` is reserved in Ada only.
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert_eq!(
                coverage_of(&schema, language).declarations_fully_renderable,
                2,
                "{language:?} has no reserved word `Range`"
            );
        }
    }

    /// A user declaration colliding with a generated support type is a global
    /// failure for the same reason, and must be reflected honestly.
    #[test]
    fn a_generated_support_name_collision_is_reflected_in_coverage() {
        let schema = schema(vec![
            renderable_record("BoundedVec"),
            renderable_record("Beta"),
        ]);
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet)
            .expect("analysis should build");
        assert!(
            analysis
                .backend_preflight_error(BackendLanguage::Rust)
                .is_some(),
            "colliding with Rust's BoundedVec must fail preflight"
        );
        // The colliding declaration is attributed and excluded; the unrelated
        // one remains renderable.
        assert_eq!(
            analysis
                .backend_coverage(BackendLanguage::Rust)
                .expect("coverage should compute")
                .declarations_fully_renderable,
            1,
            "only the declaration shadowing BoundedVec is unrenderable"
        );
    }

    /// A top-level collision involves two declarations, and neither may be
    /// counted as an ordinary renderable declaration.
    #[test]
    fn a_top_level_normalized_name_collision_is_reflected_in_coverage() {
        let schema = schema(vec![
            renderable_record("foo_bar"),
            renderable_record("fooBar"),
        ]);
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet)
                .expect("analysis should build");
            assert!(
                analysis.backend_preflight_error(language).is_some(),
                "{language:?} must see the converging names"
            );
            assert_eq!(
                analysis
                    .backend_coverage(language)
                    .expect("coverage should compute")
                    .declarations_fully_renderable,
                0,
                "{language:?} must not count colliding declarations as renderable"
            );
        }
    }

    /// The multi-namespace case: every declaration may be individually
    /// renderable while the backend cannot emit the schema at all, so
    /// full-schema coverage must not overclaim.
    #[test]
    fn multi_namespace_coverage_does_not_overclaim_backend_capability() {
        let mut schema = schema(vec![renderable_record("Alpha")]);
        // Both namespaces are declared, so this is valid IR that the frontend
        // deliberately supports -- the backends simply remain
        // single-namespace, which is a capability boundary rather than a
        // schema defect.
        schema.namespaces.push(NamespaceDecl {
            uri: "urn:other".to_owned(),
            preferred_prefix: None,
        });
        let mut foreign = renderable_record("Beta");
        foreign.name = QualifiedName::new("urn:other", "Beta");
        schema.types.push(foreign);

        for language in BackendLanguage::ALL {
            let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet)
                .expect("analysis should build");
            assert!(
                matches!(
                    analysis.backend_preflight_error(language),
                    Some(BackendPreflightError::MultipleNamespaces { .. })
                ),
                "{language:?} must report the namespace boundary"
            );
            let coverage = analysis
                .backend_coverage(language)
                .expect("coverage should compute");
            assert_eq!(
                coverage.declarations_fully_renderable, 0,
                "{language:?} cannot generate any declaration of a two-namespace schema"
            );
            assert_eq!(coverage.message_closures_renderable, 0);
        }
    }

    /// A namespace URI whose derived package identifier is reserved makes the
    /// whole unit unnameable, so -- unlike an unsafe declaration name -- it is
    /// not attributable and does zero the whole-schema figures.
    #[test]
    fn an_unusable_namespace_unit_zeroes_whole_schema_capability() {
        let mut schema = schema(vec![renderable_record("Alpha"), renderable_record("Beta")]);
        // Derives the illegal Ada package `Backend.Record`.
        schema.namespaces[0].uri = "urn:backend:record".to_owned();
        for declaration in &mut schema.types {
            declaration.name =
                QualifiedName::new("urn:backend:record", &declaration.name.local_name);
        }
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet)
            .expect("analysis should build");
        assert!(
            analysis
                .backend_preflight_error(BackendLanguage::Ada)
                .is_some_and(|error| matches!(
                    error,
                    BackendPreflightError::Name(name) if *name.region() == crate::NameRegion::NamespaceUnit
                )),
            "the fixture must fail on the package identifier itself"
        );
        let coverage = analysis
            .backend_coverage(BackendLanguage::Ada)
            .expect("coverage should compute");
        assert_eq!(
            coverage.declarations_fully_renderable, 0,
            "nothing is generable when the package cannot be named"
        );
    }

    /// Narrower per-kind and per-member capability evidence is deliberately
    /// retained when a name is unsafe: those questions remain answerable,
    /// and feature-impact analysis reads them.
    #[test]
    fn an_unsafe_name_preserves_narrower_capability_evidence() {
        let schema = schema(vec![renderable_record("Range"), renderable_record("Beta")]);
        let coverage = coverage_of(&schema, BackendLanguage::Ada);
        assert_eq!(coverage.declarations_fully_renderable, 1);
        assert_eq!(
            coverage.declaration_kinds_renderable, 2,
            "both kinds are still modelled by the backend"
        );
        assert_eq!(
            coverage.field_type_references_renderable, coverage.fields_total,
            "member type references are still modelled"
        );
    }

    #[test]
    fn recursive_closed_values_are_unsupported_without_aborting_coverage() {
        let mut base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        base.is_abstract = true;
        let mut concrete = declaration(
            "Concrete",
            TypeKind::Record {
                fields: vec![field("again", "Base")],
            },
        );
        concrete.base_type = Some(named("Base"));
        let valid_base = declaration("ValidBase", TypeKind::Record { fields: Vec::new() });
        let mut valid_base = valid_base;
        valid_base.is_abstract = true;
        let mut valid = declaration("Valid", TypeKind::Record { fields: Vec::new() });
        valid.base_type = Some(named("ValidBase"));
        let schema = message_schema(
            vec![
                base,
                concrete,
                valid_base,
                valid,
                declaration(
                    "Holder",
                    TypeKind::Record {
                        fields: vec![field("bad", "Base"), field("good", "ValidBase")],
                    },
                ),
            ],
            "Holder",
        );
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let inventory = analysis.inventory().unwrap();
        assert_eq!(inventory.counts["abstract_value.targets"], 2);
        assert_eq!(inventory.counts["abstract_value.acyclic_targets"], 1);
        assert_eq!(inventory.counts["abstract_value.recursive_targets"], 1);
        for language in BackendLanguage::ALL {
            let coverage = analysis.backend_coverage(language).unwrap();
            assert_eq!(coverage.message_closures_renderable, 0);
            assert!(analysis.report().is_ok());
            for mask in 1..(1 << FeatureFamily::ALL.len()) {
                let features = FeatureFamily::ALL
                    .iter()
                    .enumerate()
                    .filter_map(|(index, feature)| ((mask & (1 << index)) != 0).then_some(*feature))
                    .collect::<Vec<_>>();
                assert!(analysis.impact(language, &features).is_ok());
            }
        }
    }

    #[test]
    fn dependency_closure_includes_alias_base_list_fields_and_choices_in_ir_order() {
        let base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        let alias = declaration("Alias", TypeKind::Alias(named("Base")));
        let list = declaration(
            "List",
            TypeKind::List {
                item_type: named("Alias"),
                cardinality: Cardinality::REQUIRED_ONE,
            },
        );
        let choice = declaration(
            "Choice",
            TypeKind::Choice {
                alternatives: vec![field("item", "List")],
            },
        );
        let mut leaf = declaration(
            "Leaf",
            TypeKind::Record {
                fields: vec![field("choice", "Choice")],
            },
        );
        leaf.base_type = Some(named("Base"));
        let schema = schema(vec![leaf, choice, list, alias, base]);
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        assert_eq!(
            analysis
                .dependency_closure(&QualifiedName::new(NS, "Leaf"))
                .unwrap()
                .iter()
                .map(|declaration| declaration.name.local_name.as_str())
                .collect::<Vec<_>>(),
            ["Leaf", "Choice", "List", "Alias", "Base"]
        );
    }

    #[test]
    fn raw_dependency_closure_excludes_virtual_abstract_variants() {
        let mut base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        base.is_abstract = true;
        let mut concrete = declaration("Concrete", TypeKind::Record { fields: Vec::new() });
        concrete.base_type = Some(named("Base"));
        let schema = schema(vec![
            base,
            concrete,
            declaration(
                "Holder",
                TypeKind::Record {
                    fields: vec![field("value", "Base")],
                },
            ),
        ]);
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        assert_eq!(
            analysis
                .dependency_closure(&QualifiedName::new(NS, "Holder"))
                .unwrap()
                .iter()
                .map(|declaration| declaration.name.local_name.as_str())
                .collect::<Vec<_>>(),
            ["Base", "Holder"]
        );
    }

    #[test]
    fn current_backend_metrics_and_report_are_deterministic() {
        let schema = schema(vec![
            scalar("Value"),
            declaration(
                "Record",
                TypeKind::Record {
                    fields: vec![field("value", "Value")],
                },
            ),
        ]);
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        let coverage = analysis.backend_coverage(BackendLanguage::Rust).unwrap();
        assert_eq!(coverage.declarations_fully_renderable, 2);
        let first = analysis.report().unwrap();
        assert_eq!(first, analysis.report().unwrap());
        assert_eq!(
            first.lines().filter(|line| line.starts_with("  ")).count(),
            BackendLanguage::ALL.len() * ((1 << FeatureFamily::ALL.len()) - 1)
        );
        for family in FeatureFamily::ALL {
            assert!(first.contains(family.name()));
        }
    }

    #[test]
    fn primitive_expansion_alone_unblocks_duration_field_closure() {
        let schema = message_schema(
            vec![declaration(
                "Payload",
                TypeKind::Record {
                    fields: vec![field_ref(
                        "duration",
                        TypeRef::primitive(PrimitiveKind::Duration),
                    )],
                },
            )],
            "Payload",
        );
        assert_only_family_unblocks(&schema, FeatureFamily::PrimitiveExpansion);
    }

    #[test]
    fn unconstrained_binary_field_is_baseline_renderable() {
        let schema = message_schema(
            vec![declaration(
                "Payload",
                TypeKind::Record {
                    fields: vec![field_ref(
                        "binary",
                        TypeRef::primitive(PrimitiveKind::Binary),
                    )],
                },
            )],
            "Payload",
        );
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);
        }
    }

    #[test]
    fn constrained_binary_field_requires_constrained_simple_types() {
        let mut binary = field_ref("binary", TypeRef::primitive(PrimitiveKind::Binary));
        binary.constraints.length = Some(4);
        let schema = message_schema(
            vec![declaration(
                "Payload",
                TypeKind::Record {
                    fields: vec![binary],
                },
            )],
            "Payload",
        );
        assert_only_family_unblocks(&schema, FeatureFamily::ConstrainedSimpleTypes);
    }

    #[test]
    fn cardinality_and_nillability_alone_unblocks_nillable_field_closure() {
        let mut value = field_ref("value", TypeRef::primitive(PrimitiveKind::String));
        value.nillable = true;
        let schema = message_schema(
            vec![declaration(
                "Payload",
                TypeKind::Record {
                    fields: vec![value],
                },
            )],
            "Payload",
        );
        assert_only_family_unblocks(&schema, FeatureFamily::CardinalityAndNillability);
    }

    #[test]
    fn repeated_occurrences_match_ada_portable_boundaries() {
        let mut zero = field_ref("zero", TypeRef::primitive(PrimitiveKind::String));
        zero.cardinality = Cardinality {
            min_occurs: 0,
            max_occurs: None,
        };
        let mut one = zero.clone();
        one.cardinality.min_occurs = 1;
        let mut two = zero.clone();
        two.cardinality.min_occurs = 2;
        let mut three = zero.clone();
        three.cardinality.min_occurs = 3;
        let mut over_limit = zero.clone();
        over_limit.cardinality.min_occurs = ADA_PORTABLE_POSITIVE_INDEX_MAX + 1;
        let unbounded_schema = schema(vec![declaration(
            "Payload",
            TypeKind::Record {
                fields: vec![zero, one, two, three, over_limit],
            },
        )]);
        let analysis =
            CoverageAnalysis::new(&unbounded_schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert_eq!(
                analysis
                    .backend_coverage(language)
                    .unwrap()
                    .field_occurrences_renderable,
                5
            );
        }
        assert_eq!(
            analysis
                .backend_coverage(BackendLanguage::Ada)
                .unwrap()
                .field_occurrences_renderable,
            4
        );

        let mut finite = field_ref("finite", TypeRef::primitive(PrimitiveKind::String));
        finite.cardinality = Cardinality {
            min_occurs: 2,
            max_occurs: Some(3),
        };
        let mut finite_over_limit = finite.clone();
        finite_over_limit.cardinality.max_occurs = Some(ADA_PORTABLE_POSITIVE_INDEX_MAX + 1);
        let finite_schema = schema(vec![declaration(
            "Finite",
            TypeKind::Record {
                fields: vec![finite, finite_over_limit],
            },
        )]);
        let analysis =
            CoverageAnalysis::new(&finite_schema, GenerationWorld::ClosedSchemaSet).unwrap();
        assert_eq!(
            analysis
                .backend_coverage(BackendLanguage::Ada)
                .unwrap()
                .field_occurrences_renderable,
            1
        );
    }

    #[test]
    fn supported_choice_payload_closure_is_baseline() {
        let schema = message_schema(
            vec![declaration(
                "Payload",
                TypeKind::Choice {
                    alternatives: vec![field_ref(
                        "text",
                        TypeRef::primitive(PrimitiveKind::String),
                    )],
                },
            )],
            "Payload",
        );
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);
            assert_eq!(
                analysis.impact(language, &[FeatureFamily::Choice]).unwrap(),
                1
            );
            assert_eq!(
                analysis
                    .impact(language, &[FeatureFamily::PrimitiveExpansion])
                    .unwrap(),
                1
            );
        }
    }

    #[test]
    fn supported_choice_metrics_are_baseline_and_hypothetical_features_are_additive() {
        let schema = message_schema(
            vec![declaration(
                "Payload",
                TypeKind::Choice {
                    alternatives: vec![field_ref(
                        "text",
                        TypeRef::primitive(PrimitiveKind::String),
                    )],
                },
            )],
            "Payload",
        );
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            let baseline = analysis.backend_coverage(language).unwrap();
            assert_eq!(baseline.declaration_kinds_renderable, 1);
            assert_eq!(baseline.declarations_fully_renderable, 1);
            assert_eq!(baseline.message_closures_renderable, 1);
            for mask in 1..(1 << FeatureFamily::ALL.len()) {
                let enabled = FeatureFamily::ALL
                    .iter()
                    .enumerate()
                    .filter_map(|(index, feature)| ((mask & (1 << index)) != 0).then_some(*feature))
                    .collect::<BTreeSet<_>>();
                let coverage = analysis.backend_coverage_with(language, &enabled).unwrap();
                assert!(
                    coverage.declaration_kinds_renderable >= baseline.declaration_kinds_renderable
                );
                assert!(
                    coverage.declarations_fully_renderable
                        >= baseline.declarations_fully_renderable
                );
                assert!(
                    coverage.message_closures_renderable >= baseline.message_closures_renderable
                );
            }
        }
    }

    #[test]
    fn pure_record_abstract_ancestry_is_currently_renderable() {
        let mut base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        base.is_abstract = true;
        let mut payload = declaration("Payload", TypeKind::Record { fields: Vec::new() });
        payload.base_type = Some(named("Base"));
        let schema = message_schema(vec![payload, base], "Payload");
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);
            assert_eq!(
                analysis
                    .impact(language, &[FeatureFamily::StructuralInheritanceAndAbstract])
                    .unwrap(),
                1
            );
        }
    }

    #[test]
    fn unrelated_features_preserve_pure_record_inheritance() {
        let base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        let mut derived = declaration(
            "Derived",
            TypeKind::Record {
                fields: vec![field_ref(
                    "value",
                    TypeRef::primitive(PrimitiveKind::String),
                )],
            },
        );
        derived.base_type = Some(named("Base"));
        let schema = message_schema(vec![derived, base], "Derived");
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);
            for family in [
                FeatureFamily::PrimitiveExpansion,
                FeatureFamily::CardinalityAndNillability,
                FeatureFamily::Choice,
                FeatureFamily::StructuralInheritanceAndAbstract,
                FeatureFamily::ConstrainedSimpleTypes,
            ] {
                assert_eq!(analysis.impact(language, &[family]).unwrap(), 1);
            }
        }
    }

    #[test]
    fn closed_abstract_values_are_baseline_and_empty_values_remain_hypothetical() {
        let mut base = declaration(
            "Base",
            TypeKind::Record {
                fields: vec![field_ref(
                    "value",
                    TypeRef::primitive(PrimitiveKind::String),
                )],
            },
        );
        base.is_abstract = true;
        let mut derived = declaration("Derived", TypeKind::Record { fields: Vec::new() });
        derived.base_type = Some(named("Base"));
        let holder = declaration(
            "Holder",
            TypeKind::Record {
                fields: vec![field("value", "Base")],
            },
        );
        let abstract_payload = message_schema(vec![base.clone(), derived.clone()], "Base");
        let concrete_payload = message_schema(vec![base.clone(), derived.clone()], "Derived");
        let abstract_field = message_schema(vec![base, derived, holder], "Holder");
        for language in BackendLanguage::ALL {
            let analysis =
                CoverageAnalysis::new(&abstract_payload, GenerationWorld::ClosedSchemaSet).unwrap();
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);
            for family in [
                FeatureFamily::PrimitiveExpansion,
                FeatureFamily::CardinalityAndNillability,
                FeatureFamily::Choice,
                FeatureFamily::ConstrainedSimpleTypes,
            ] {
                assert_eq!(analysis.impact(language, &[family]).unwrap(), 1);
            }
            assert_eq!(
                analysis
                    .impact(language, &[FeatureFamily::StructuralInheritanceAndAbstract])
                    .unwrap(),
                1
            );

            let analysis =
                CoverageAnalysis::new(&concrete_payload, GenerationWorld::ClosedSchemaSet).unwrap();
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);

            let analysis =
                CoverageAnalysis::new(&abstract_field, GenerationWorld::ClosedSchemaSet).unwrap();
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);
            assert_eq!(
                analysis
                    .impact(language, &[FeatureFamily::StructuralInheritanceAndAbstract])
                    .unwrap(),
                1
            );

            let mut empty = declaration("Empty", TypeKind::Record { fields: Vec::new() });
            empty.is_abstract = true;
            let empty_schema = message_schema(
                vec![
                    empty,
                    declaration(
                        "EmptyHolder",
                        TypeKind::Record {
                            fields: vec![field("value", "Empty")],
                        },
                    ),
                ],
                "EmptyHolder",
            );
            let analysis =
                CoverageAnalysis::new(&empty_schema, GenerationWorld::ClosedSchemaSet).unwrap();
            assert_eq!(analysis.impact(language, &[]).unwrap(), 0);
            assert_eq!(
                analysis
                    .impact(language, &[FeatureFamily::StructuralInheritanceAndAbstract])
                    .unwrap(),
                1
            );
        }
    }

    #[test]
    fn optional_uninhabited_abstract_field_is_baseline_renderable_in_coverage() {
        let mut empty = declaration("Empty", TypeKind::Record { fields: Vec::new() });
        empty.is_abstract = true;
        let mut optional = field("value", "Empty");
        optional.cardinality = Cardinality {
            min_occurs: 0,
            max_occurs: Some(1),
        };
        let holder = declaration(
            "Holder",
            TypeKind::Record {
                fields: vec![optional],
            },
        );
        let schema = message_schema(vec![empty, holder], "Holder");
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            // The only legal state of the field is absence, so no storage is
            // generated and the owning record is renderable with no features.
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);
            let baseline = analysis.backend_coverage(language).unwrap();
            assert_eq!(baseline.message_closures_renderable, 1);
            // Enabling the hypothetical family must never reduce coverage.
            for mask in 1..(1 << FeatureFamily::ALL.len()) {
                let enabled = FeatureFamily::ALL
                    .iter()
                    .enumerate()
                    .filter_map(|(index, feature)| ((mask & (1 << index)) != 0).then_some(*feature))
                    .collect::<BTreeSet<_>>();
                let coverage = analysis.backend_coverage_with(language, &enabled).unwrap();
                assert!(
                    coverage.declarations_fully_renderable
                        >= baseline.declarations_fully_renderable,
                    "{enabled:?} regressed declarations for {language:?}"
                );
                assert!(
                    coverage.message_closures_renderable >= baseline.message_closures_renderable,
                    "{enabled:?} regressed message closures for {language:?}"
                );
            }
        }
    }

    #[test]
    fn unsupported_uninhabited_abstract_occurrences_stay_blocked_in_coverage() {
        let mut empty = declaration("Empty", TypeKind::Record { fields: Vec::new() });
        empty.is_abstract = true;
        let repeated = Cardinality {
            min_occurs: 0,
            max_occurs: None,
        };
        let optional = Cardinality {
            min_occurs: 0,
            max_occurs: Some(1),
        };
        let mut cases: Vec<FieldDecl> = Vec::new();
        // Positive minimum: an inhabitant would be required but none exists.
        cases.push(field("required", "Empty"));
        // Repeated zero-minimum remains out of scope for absent-only elision.
        let mut repeated_field = field("repeated", "Empty");
        repeated_field.cardinality = repeated;
        cases.push(repeated_field);
        // Nillable optional still demands an explicit nil representation.
        let mut nillable_field = field("nillable", "Empty");
        nillable_field.cardinality = optional;
        nillable_field.nillable = true;
        cases.push(nillable_field);
        // Local constraints on the occurrence are not discardable.
        let mut constrained_field = field("constrained", "Empty");
        constrained_field.cardinality = optional;
        constrained_field.constraints.min_length = Some(1);
        cases.push(constrained_field);

        for case in cases {
            let name = case.name.clone();
            let holder = declaration("Holder", TypeKind::Record { fields: vec![case] });
            let schema = message_schema(vec![empty.clone(), holder], "Holder");
            let analysis =
                CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
            for language in BackendLanguage::ALL {
                // Baseline must stay closed: none of these occurrences can be
                // represented without inventing an impossible payload.
                assert_eq!(
                    analysis.impact(language, &[]).unwrap(),
                    0,
                    "{name} unexpectedly renderable for {language:?}"
                );
                // The hypothetical abstract family is necessary to unblock them.
                // Some cases additionally need their own family (nillability,
                // constrained simple types), so only require that the abstract
                // family is part of any unblocking set: without it, no feature
                // combination may unblock the case.
                for mask in 1..(1 << FeatureFamily::ALL.len()) {
                    let enabled = FeatureFamily::ALL
                        .iter()
                        .enumerate()
                        .filter_map(|(index, feature)| {
                            ((mask & (1 << index)) != 0).then_some(*feature)
                        })
                        .collect::<BTreeSet<_>>();
                    if enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract) {
                        continue;
                    }
                    assert_eq!(
                        analysis
                            .backend_coverage_with(language, &enabled)
                            .unwrap()
                            .message_closures_renderable,
                        0,
                        "{name} unblocked by {enabled:?} without the abstract family for {language:?}"
                    );
                }
                // With every family enabled the case is attributable.
                let all = FeatureFamily::ALL.iter().copied().collect::<BTreeSet<_>>();
                assert_eq!(
                    analysis
                        .backend_coverage_with(language, &all)
                        .unwrap()
                        .message_closures_renderable,
                    1,
                    "{name} should be attributable to hypothetical families for {language:?}"
                );
            }
        }
    }

    #[test]
    fn uninhabited_abstract_message_payload_stays_blocked_in_coverage() {
        let mut empty = declaration("Empty", TypeKind::Record { fields: Vec::new() });
        empty.is_abstract = true;
        let schema = message_schema(vec![empty], "Empty");
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            let baseline = analysis.backend_coverage(language).unwrap();
            assert_eq!(
                baseline.message_closures_renderable, 0,
                "an uninhabited payload can never be constructed for {language:?}"
            );
        }
    }

    #[test]
    fn constrained_simple_family_alone_unblocks_field_constraint_closure() {
        let mut text = field_ref("text", TypeRef::primitive(PrimitiveKind::String));
        text.constraints.min_length = Some(1);
        let schema = message_schema(
            vec![declaration(
                "Payload",
                TypeKind::Record { fields: vec![text] },
            )],
            "Payload",
        );
        assert_only_family_unblocks(&schema, FeatureFamily::ConstrainedSimpleTypes);
    }

    #[test]
    fn supported_choice_extends_pure_record_inheritance_baseline() {
        let base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        let choice = declaration(
            "Selection",
            TypeKind::Choice {
                alternatives: vec![field_ref("text", TypeRef::primitive(PrimitiveKind::String))],
            },
        );
        let mut payload = declaration(
            "Payload",
            TypeKind::Record {
                fields: vec![field("selection", "Selection")],
            },
        );
        payload.base_type = Some(named("Base"));
        let schema = message_schema(vec![payload, choice, base], "Payload");
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);
            assert_eq!(
                analysis.impact(language, &[FeatureFamily::Choice]).unwrap(),
                1
            );
            assert_eq!(
                analysis
                    .impact(language, &[FeatureFamily::StructuralInheritanceAndAbstract])
                    .unwrap(),
                1
            );
            assert_eq!(
                analysis
                    .impact(
                        language,
                        &[
                            FeatureFamily::Choice,
                            FeatureFamily::StructuralInheritanceAndAbstract,
                        ]
                    )
                    .unwrap(),
                1
            );
        }
    }

    #[test]
    fn message_closure_includes_unsupported_transitive_dependency() {
        let inner = declaration(
            "Inner",
            TypeKind::Record {
                fields: vec![field_ref(
                    "value",
                    TypeRef::primitive(PrimitiveKind::Duration),
                )],
            },
        );
        let outer = declaration(
            "Outer",
            TypeKind::Record {
                fields: vec![field("inner", "Inner")],
            },
        );
        let schema = message_schema(vec![outer, inner], "Outer");
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            assert_eq!(analysis.impact(language, &[]).unwrap(), 0);
            assert_eq!(
                analysis
                    .impact(language, &[FeatureFamily::PrimitiveExpansion])
                    .unwrap(),
                1
            );
        }
    }

    #[test]
    fn optional_named_field_preserves_language_specific_occurrence_support() {
        let mut value = field("value", "Value");
        value.cardinality = Cardinality::OPTIONAL_ONE;
        let schema = message_schema(
            vec![
                declaration(
                    "Payload",
                    TypeKind::Record {
                        fields: vec![value],
                    },
                ),
                scalar("Value"),
            ],
            "Payload",
        );
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        assert_eq!(analysis.impact(BackendLanguage::Ada, &[]).unwrap(), 0);
        assert_eq!(analysis.impact(BackendLanguage::Rust, &[]).unwrap(), 1);
        assert_eq!(analysis.impact(BackendLanguage::Cpp, &[]).unwrap(), 1);
        assert_eq!(
            analysis
                .backend_coverage(BackendLanguage::Ada)
                .unwrap()
                .field_occurrences_renderable,
            0
        );
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert_eq!(
                analysis
                    .backend_coverage(language)
                    .unwrap()
                    .field_occurrences_renderable,
                1
            );
        }
    }

    // ---------------------------------------------------------------
    // Task 028 -- explicit generation world policy
    // ---------------------------------------------------------------

    // ---------------------------------------------------------------
    // Task 031 -- shared renderability snapshot
    // ---------------------------------------------------------------

    /// Task 031 section 17: extracting `declaration_renderability` out of
    /// `backend_coverage_with` must not change what coverage measures. The
    /// snapshot is the SAME vector coverage counts, so the count of renderable
    /// declarations it reports and `declarations_fully_renderable` must agree
    /// for every language, feature set, and world.
    #[test]
    fn task031_snapshot_matches_full_coverage_declaration_counts() {
        let mut base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        base.is_abstract = true;
        let mut derived = declaration("Derived", TypeKind::Record { fields: Vec::new() });
        derived.base_type = Some(named("Base"));
        let holder = declaration(
            "Holder",
            TypeKind::Record {
                fields: vec![field("value", "Base")],
            },
        );
        let schema = message_schema(vec![holder, base, derived], "Holder");

        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let analysis = CoverageAnalysis::new(&schema, world).unwrap();
            for language in BackendLanguage::ALL {
                for mask in 0..(1 << FeatureFamily::ALL.len()) {
                    let enabled = FeatureFamily::ALL
                        .iter()
                        .enumerate()
                        .filter_map(|(index, feature)| {
                            ((mask & (1 << index)) != 0).then_some(*feature)
                        })
                        .collect::<BTreeSet<_>>();
                    let snapshot = analysis.declaration_renderability(language, &enabled);
                    let counted = (0..schema.types.len())
                        .filter(|index| snapshot.is_renderable(*index))
                        .count();
                    let coverage = analysis.backend_coverage_with(language, &enabled).unwrap();
                    assert_eq!(
                        counted, coverage.declarations_fully_renderable,
                        "{world} {language:?} mask {mask} must use one computation"
                    );
                }
            }
            // `baseline_renderability` is exactly the empty-feature snapshot,
            // never a separately tuned "actual capability" rule set.
            for language in BackendLanguage::ALL {
                let baseline = analysis.baseline_renderability(language);
                let explicit = analysis.declaration_renderability(language, &BTreeSet::new());
                for index in 0..schema.types.len() {
                    assert_eq!(
                        baseline.is_renderable(index),
                        explicit.is_renderable(index),
                        "{world} {language:?} declaration {index}"
                    );
                }
            }
        }
    }

    /// All 31 non-empty feature combinations must complete and stay monotonic
    /// under BOTH worlds (sections 31/32). No feature may reduce capability,
    /// and no combination may panic or hit a recursion failure.
    #[test]
    fn task028_all_feature_combinations_are_monotonic_in_both_worlds() {
        let mut base = declaration("Base", TypeKind::Record { fields: Vec::new() });
        base.is_abstract = true;
        let mut derived = declaration("Derived", TypeKind::Record { fields: Vec::new() });
        derived.base_type = Some(named("Base"));
        let mut empty = declaration("EmptyBase", TypeKind::Record { fields: Vec::new() });
        empty.is_abstract = true;
        let mut holder = declaration(
            "Holder",
            TypeKind::Record {
                fields: vec![field("value", "Base"), field("absent", "EmptyBase")],
            },
        );
        let TypeKind::Record { fields } = &mut holder.kind else {
            unreachable!();
        };
        fields[1].cardinality = Cardinality::OPTIONAL_ONE;
        let schema = message_schema(vec![holder, base, derived, empty], "Holder");

        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let analysis = CoverageAnalysis::new(&schema, world).unwrap();
            assert_eq!(analysis.world(), world);
            assert!(analysis.report().is_ok(), "{world} report must complete");
            for language in BackendLanguage::ALL {
                let baseline = analysis
                    .backend_coverage(language)
                    .unwrap()
                    .message_closures_renderable;
                let mut combinations = 0;
                for mask in 1..(1 << FeatureFamily::ALL.len()) {
                    let features = FeatureFamily::ALL
                        .iter()
                        .enumerate()
                        .filter_map(|(index, feature)| {
                            ((mask & (1 << index)) != 0).then_some(*feature)
                        })
                        .collect::<Vec<_>>();
                    let total = analysis
                        .impact(language, &features)
                        .unwrap_or_else(|error| panic!("{world}/{language:?}: {error}"));
                    assert!(
                        total >= baseline,
                        "{world}/{language:?}: feature set {features:?} reduced coverage \
                         from {baseline} to {total}"
                    );
                    combinations += 1;
                }
                assert_eq!(combinations, 31, "all 31 combinations must run");
            }
        }
    }

    fn float_bound(value: f64) -> NumericValue {
        NumericValue::Float64(ams_gra_oms_ir::Float64Value::from_value(value))
    }

    fn bounded_float(name: &str, constraints: ConstraintSet) -> TypeDecl {
        let mut value = declaration(name, TypeKind::Primitive(PrimitiveKind::Float64));
        value.constraints = constraints;
        value
    }

    /// Task 033 sections 38/40: a named bound-only floating declaration is
    /// baseline-renderable, while the same bound applied *field-locally* to a
    /// direct primitive is not -- there is no checked wrapper for the latter,
    /// so it stays attributed to `ConstrainedSimpleTypes`.
    #[test]
    fn task033_named_float_ranges_are_baseline_but_field_local_ones_are_not() {
        for constraints in [
            ConstraintSet {
                min_inclusive: Some(float_bound(0.0)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_exclusive: Some(float_bound(0.0)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_inclusive: Some(float_bound(1.0)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                max_exclusive: Some(float_bound(1.0)),
                ..ConstraintSet::default()
            },
            ConstraintSet {
                min_inclusive: Some(float_bound(0.0)),
                max_exclusive: Some(float_bound(1.0)),
                ..ConstraintSet::default()
            },
        ] {
            let holder = declaration(
                "Holder",
                TypeKind::Record {
                    fields: vec![field("value", "Bounded")],
                },
            );
            let schema = message_schema(
                vec![holder, bounded_float("Bounded", constraints)],
                "Holder",
            );
            let analysis =
                CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
            for language in BackendLanguage::ALL {
                let coverage = analysis.backend_coverage(language).unwrap();
                assert_eq!(
                    coverage.declarations_fully_renderable, 2,
                    "{language:?}: a bound-only named float needs no hypothetical feature"
                );
                assert_eq!(coverage.message_closures_renderable, 1);
                // No feature family can add anything: it is already baseline.
                assert_eq!(
                    analysis
                        .impact(language, &[FeatureFamily::ConstrainedSimpleTypes])
                        .unwrap(),
                    1
                );
            }
        }

        // The same numeric bound on a direct primitive *field* is not baseline.
        let mut holder = declaration(
            "Holder",
            TypeKind::Record {
                fields: vec![field_ref(
                    "value",
                    TypeRef::primitive(PrimitiveKind::Float64),
                )],
            },
        );
        let TypeKind::Record { fields } = &mut holder.kind else {
            unreachable!();
        };
        fields[0].constraints = ConstraintSet {
            min_inclusive: Some(float_bound(0.0)),
            ..ConstraintSet::default()
        };
        let schema = message_schema(vec![holder], "Holder");
        let analysis = CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
        for language in BackendLanguage::ALL {
            assert_eq!(
                analysis
                    .backend_coverage(language)
                    .unwrap()
                    .declarations_fully_renderable,
                0,
                "{language:?}: a field-local floating bound must not be baseline"
            );
        }
        assert_only_family_unblocks(&schema, FeatureFamily::ConstrainedSimpleTypes);
    }

    /// Task 033 section 39: a facet shape the helper rejects stays
    /// non-baseline and remains modelled as future `ConstrainedSimpleTypes`
    /// support, exactly as before Task 033.
    #[test]
    fn task033_unsupported_float_constraint_shapes_remain_hypothetical() {
        for constraints in [
            // Lexical alongside a numeric bound: not partially enforced.
            ConstraintSet {
                min_inclusive: Some(float_bound(0.0)),
                lexical: ams_gra_oms_ir::LexicalConstraintSet {
                    pattern_groups: vec![ams_gra_oms_ir::PatternGroup {
                        alternatives: vec![ams_gra_oms_ir::PatternExpression::xml_schema("[0-9]+")],
                    }],
                    white_space: None,
                },
                ..ConstraintSet::default()
            },
            // Length facets, which are meaningless on a float.
            ConstraintSet {
                length: Some(4),
                ..ConstraintSet::default()
            },
            // Ambiguous same-side bounds.
            ConstraintSet {
                min_inclusive: Some(float_bound(0.0)),
                min_exclusive: Some(float_bound(0.0)),
                ..ConstraintSet::default()
            },
            // A bound in the wrong width domain.
            ConstraintSet {
                min_inclusive: Some(NumericValue::Float32(
                    ams_gra_oms_ir::Float32Value::from_value(0.0),
                )),
                ..ConstraintSet::default()
            },
        ] {
            let holder = declaration(
                "Holder",
                TypeKind::Record {
                    fields: vec![field("value", "Bounded")],
                },
            );
            let schema = message_schema(
                vec![holder, bounded_float("Bounded", constraints)],
                "Holder",
            );
            let analysis =
                CoverageAnalysis::new(&schema, GenerationWorld::ClosedSchemaSet).unwrap();
            for language in BackendLanguage::ALL {
                assert_eq!(
                    analysis
                        .backend_coverage(language)
                        .unwrap()
                        .declarations_fully_renderable,
                    1,
                    "{language:?}: only the Holder renders; the bad float does not"
                );
            }
            assert_only_family_unblocks(&schema, FeatureFamily::ConstrainedSimpleTypes);
        }
    }

    /// Task 033 section 42: supported named floating ranges stay baseline in
    /// every one of the 31 feature combinations, under both worlds, and no
    /// feature ever reduces capability.
    #[test]
    fn task033_float_ranges_hold_across_all_feature_combinations_and_worlds() {
        let holder = declaration(
            "Holder",
            TypeKind::Record {
                fields: vec![field("value", "Bounded"), field("ratio", "Unit")],
            },
        );
        let schema = message_schema(
            vec![
                holder,
                bounded_float(
                    "Bounded",
                    ConstraintSet {
                        min_exclusive: Some(float_bound(0.0)),
                        ..ConstraintSet::default()
                    },
                ),
                bounded_float(
                    "Unit",
                    ConstraintSet {
                        min_inclusive: Some(float_bound(0.0)),
                        max_inclusive: Some(float_bound(1.0)),
                        ..ConstraintSet::default()
                    },
                ),
            ],
            "Holder",
        );

        for world in [
            GenerationWorld::ClosedSchemaSet,
            GenerationWorld::OpenExtensions,
        ] {
            let analysis = CoverageAnalysis::new(&schema, world).unwrap();
            assert!(analysis.report().is_ok(), "{world} report must complete");
            for language in BackendLanguage::ALL {
                let baseline = analysis
                    .backend_coverage(language)
                    .unwrap()
                    .message_closures_renderable;
                assert_eq!(
                    baseline, 1,
                    "{world}/{language:?}: bounded floats are baseline in both worlds"
                );
                let mut combinations = 0;
                for mask in 1..(1 << FeatureFamily::ALL.len()) {
                    let features = FeatureFamily::ALL
                        .iter()
                        .enumerate()
                        .filter_map(|(index, feature)| {
                            ((mask & (1 << index)) != 0).then_some(*feature)
                        })
                        .collect::<Vec<_>>();
                    let total = analysis
                        .impact(language, &features)
                        .unwrap_or_else(|error| panic!("{world}/{language:?}: {error}"));
                    assert!(
                        total >= baseline,
                        "{world}/{language:?}: {features:?} reduced coverage"
                    );
                    combinations += 1;
                }
                assert_eq!(combinations, 31, "all 31 combinations must run");
            }
        }
    }
}
