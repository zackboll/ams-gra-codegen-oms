use crate::inclusive_integral_domain;
use crate::structure::{
    EffectiveStructuralType, StructuralProjectionError, StructuralSegmentContent,
    project_with_index,
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

/// Indexed, reusable semantic analysis over validated Schema IR.
pub struct CoverageAnalysis<'a> {
    schema: &'a SchemaIr,
    declarations: BTreeMap<&'a QualifiedName, &'a TypeDecl>,
    indices: BTreeMap<&'a QualifiedName, usize>,
}

impl<'a> CoverageAnalysis<'a> {
    /// Build an index after validating the complete IR.
    pub fn new(schema: &'a SchemaIr) -> Result<Self, CoverageError> {
        schema
            .validate()
            .map_err(|error| CoverageError::InvalidSchema(error.to_string()))?;
        Ok(Self {
            schema,
            declarations: schema
                .types
                .iter()
                .map(|declaration| (&declaration.name, declaration))
                .collect(),
            indices: schema
                .types
                .iter()
                .enumerate()
                .map(|(index, declaration)| (&declaration.name, index))
                .collect(),
        })
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
        self.count_collisions(&mut inventory)?;
        inventory.evidence.sort();
        Ok(inventory)
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

    fn backend_coverage_with(
        &self,
        language: BackendLanguage,
        enabled: &BTreeSet<FeatureFamily>,
    ) -> Result<BackendCoverage, CoverageError> {
        let fields = all_members(self.schema);
        let full = self
            .schema
            .types
            .iter()
            .map(|declaration| self.declaration_renderable(declaration, language, enabled))
            .collect::<Vec<_>>();
        let mut message_closures_renderable = 0;
        for message in &self.schema.messages {
            if self.message_renderable(message, language, enabled)
                && match &message.payload_type.target {
                    TypeRefTarget::Primitive(kind) => primitive_ref_renderable(*kind, enabled),
                    TypeRefTarget::Named(name) => self
                        .dependency_closure(name)?
                        .iter()
                        .all(|declaration| full[self.indices[&declaration.name]]),
                }
            {
                message_closures_renderable += 1;
            }
        }
        Ok(BackendCoverage {
            language,
            declarations_total: self.schema.types.len(),
            declaration_kinds_renderable: self
                .schema
                .types
                .iter()
                .filter(|declaration| kind_renderable(declaration, enabled))
                .count(),
            declarations_fully_renderable: full.iter().filter(|renderable| **renderable).count(),
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
        if declaration.is_abstract
            && !is_abstract_ancestor
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
                        field_renderable(field, language, enabled)
                            && (!matches!(&field.type_ref.target, TypeRefTarget::Named(name)
                            if self.declarations.get(name).is_some_and(|target| target.is_abstract))
                                || enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract))
                    }),
                    StructuralSegmentContent::ChoiceAlternatives(fields) => {
                        (supported_choice_shape || enabled.contains(&FeatureFamily::Choice))
                            && fields.iter().all(|field| {
                                field_renderable(field, language, enabled)
                                    && (!matches!(&field.type_ref.target, TypeRefTarget::Named(name)
                                    if self.declarations.get(name).is_some_and(|target| target.is_abstract))
                                        || enabled.contains(
                                            &FeatureFamily::StructuralInheritanceAndAbstract,
                                        ))
                            })
                    }
                })
    }

    fn message_renderable(
        &self,
        message: &MessageDecl,
        _language: BackendLanguage,
        enabled: &BTreeSet<FeatureFamily>,
    ) -> bool {
        type_ref_renderable(&message.payload_type, enabled)
            && (!matches!(&message.payload_type.target, TypeRefTarget::Named(name)
                if self.declarations.get(name).is_some_and(|target| target.is_abstract))
                || enabled.contains(&FeatureFamily::StructuralInheritanceAndAbstract))
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

fn direct_dependencies(declaration: &TypeDecl) -> Vec<&QualifiedName> {
    let mut dependencies = Vec::new();
    if let Some(base) = &declaration.base_type {
        push_named(&mut dependencies, base);
    }
    match &declaration.kind {
        TypeKind::Primitive(_) | TypeKind::Enumeration { .. } => {}
        TypeKind::Alias(target) => push_named(&mut dependencies, target),
        TypeKind::Record { fields } => {
            for field in fields {
                push_named(&mut dependencies, &field.type_ref);
            }
        }
        TypeKind::Choice { alternatives } => {
            for alternative in alternatives {
                push_named(&mut dependencies, &alternative.type_ref);
            }
        }
        TypeKind::List { item_type, .. } => push_named(&mut dependencies, item_type),
    }
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

fn push_named<'a>(dependencies: &mut Vec<&'a QualifiedName>, type_ref: &'a TypeRef) {
    if let TypeRefTarget::Named(name) = &type_ref.target {
        dependencies.push(name);
    }
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
            PrimitiveKind::Boolean | PrimitiveKind::SignedInteger | PrimitiveKind::UnsignedInteger,
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
            | PrimitiveKind::String
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
        (BackendLanguage::Ada, OccurrenceShape::Bounded { min: 0, max }) if max > 1 => true,
        (BackendLanguage::Ada, OccurrenceShape::Unbounded { min: 0 }) => true,
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
        let analysis = CoverageAnalysis::new(schema).unwrap();
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
    fn primitive_expansion_alone_unblocks_float_field_closure() {
        let schema = message_schema(
            vec![declaration(
                "Payload",
                TypeKind::Record {
                    fields: vec![field_ref(
                        "value",
                        TypeRef::primitive(PrimitiveKind::Float32),
                    )],
                },
            )],
            "Payload",
        );
        assert_only_family_unblocks(&schema, FeatureFamily::PrimitiveExpansion);
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
    fn unbounded_occurrences_are_baseline_only_for_implemented_languages() {
        let mut zero = field_ref("zero", TypeRef::primitive(PrimitiveKind::String));
        zero.cardinality = Cardinality {
            min_occurs: 0,
            max_occurs: None,
        };
        let mut one = zero.clone();
        one.cardinality.min_occurs = 1;
        let mut two = zero.clone();
        two.cardinality.min_occurs = 2;
        let schema = schema(vec![declaration(
            "Payload",
            TypeKind::Record {
                fields: vec![zero, one, two],
            },
        )]);
        let analysis = CoverageAnalysis::new(&schema).unwrap();
        for language in [BackendLanguage::Rust, BackendLanguage::Cpp] {
            assert_eq!(
                analysis
                    .backend_coverage(language)
                    .unwrap()
                    .field_occurrences_renderable,
                3
            );
        }
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
    fn abstract_values_require_remaining_structural_abstract_support() {
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
            let analysis = CoverageAnalysis::new(&abstract_payload).unwrap();
            assert_eq!(analysis.impact(language, &[]).unwrap(), 0);
            for family in [
                FeatureFamily::PrimitiveExpansion,
                FeatureFamily::CardinalityAndNillability,
                FeatureFamily::Choice,
                FeatureFamily::ConstrainedSimpleTypes,
            ] {
                assert_eq!(analysis.impact(language, &[family]).unwrap(), 0);
            }
            assert_eq!(
                analysis
                    .impact(language, &[FeatureFamily::StructuralInheritanceAndAbstract])
                    .unwrap(),
                1
            );

            let analysis = CoverageAnalysis::new(&concrete_payload).unwrap();
            assert_eq!(analysis.impact(language, &[]).unwrap(), 1);

            let analysis = CoverageAnalysis::new(&abstract_field).unwrap();
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
                    TypeRef::primitive(PrimitiveKind::Float32),
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
        let analysis = CoverageAnalysis::new(&schema).unwrap();
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
}
