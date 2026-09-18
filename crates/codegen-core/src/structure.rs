use ams_gra_oms_ir::{FieldDecl, QualifiedName, SchemaIr, TypeDecl, TypeKind, TypeRefTarget};
use std::collections::BTreeMap;
use std::fmt;

/// The local compositor represented by one structural declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralKind {
    Record,
    Choice,
}

/// One named declaration in a base-to-derived structural ancestry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralLevel<'a> {
    pub declaration: &'a TypeDecl,
    pub local_kind: StructuralKind,
}

/// Borrowed local content for one non-empty structural segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralSegmentContent<'a> {
    RecordFields(&'a [FieldDecl]),
    ChoiceAlternatives(&'a [FieldDecl]),
}

/// A compositor-preserving segment and the declaration that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralSegment<'a> {
    pub owner: &'a TypeDecl,
    pub content: StructuralSegmentContent<'a>,
}

/// Effective structural content, ordered from the oldest base to the target.
///
/// Empty local segments are omitted, but their declarations remain in
/// `ancestry`, preserving identity, local kind, and abstractness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveStructuralType<'a> {
    pub declaration: &'a TypeDecl,
    pub immediate_base: Option<&'a TypeDecl>,
    pub ancestry: Vec<StructuralLevel<'a>>,
    pub segments: Vec<StructuralSegment<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralProjectionError {
    MissingDeclaration(QualifiedName),
    NonStructuralTarget(QualifiedName),
    PrimitiveBase {
        owner: QualifiedName,
    },
    MissingBase {
        owner: QualifiedName,
        base: QualifiedName,
    },
    NonStructuralBase {
        owner: QualifiedName,
        base: QualifiedName,
    },
    InheritedMemberNameCollision {
        target: Box<QualifiedName>,
        member: String,
        first_owner: Box<QualifiedName>,
        second_owner: Box<QualifiedName>,
    },
    Cycle(Vec<QualifiedName>),
}

impl fmt::Display for StructuralProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDeclaration(name) => {
                write!(formatter, "missing declaration {}", name.local_name)
            }
            Self::NonStructuralTarget(name) => write!(
                formatter,
                "declaration {} is not structural",
                name.local_name
            ),
            Self::PrimitiveBase { owner } => write!(
                formatter,
                "structural declaration {} has a primitive base",
                owner.local_name
            ),
            Self::MissingBase { owner, base } => write!(
                formatter,
                "structural declaration {} has missing base {}",
                owner.local_name, base.local_name
            ),
            Self::NonStructuralBase { owner, base } => write!(
                formatter,
                "structural declaration {} has non-structural base {}",
                owner.local_name, base.local_name
            ),
            Self::InheritedMemberNameCollision {
                target,
                member,
                first_owner,
                second_owner,
            } => write!(
                formatter,
                "structural declaration {} inherits duplicate member {member} from {} and {}",
                target.local_name, first_owner.local_name, second_owner.local_name
            ),
            Self::Cycle(names) => write!(
                formatter,
                "structural inheritance cycle: {}",
                names
                    .iter()
                    .map(|name| name.local_name.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ),
        }
    }
}

impl std::error::Error for StructuralProjectionError {}

/// Project one Record or Choice while retaining named ancestry and compositor boundaries.
///
/// The function is defensive for externally constructed IR and does not require callers
/// to validate first.
pub fn project_structural_type<'a>(
    schema: &'a SchemaIr,
    name: &QualifiedName,
) -> Result<EffectiveStructuralType<'a>, StructuralProjectionError> {
    let declarations = schema
        .types
        .iter()
        .map(|declaration| (&declaration.name, declaration))
        .collect::<BTreeMap<_, _>>();
    project_with_index(&declarations, name)
}

pub(crate) fn project_with_index<'a>(
    declarations: &BTreeMap<&'a QualifiedName, &'a TypeDecl>,
    name: &QualifiedName,
) -> Result<EffectiveStructuralType<'a>, StructuralProjectionError> {
    let target = declarations
        .get(name)
        .copied()
        .ok_or_else(|| StructuralProjectionError::MissingDeclaration(name.clone()))?;
    structural_kind(target)
        .ok_or_else(|| StructuralProjectionError::NonStructuralTarget(name.clone()))?;

    let immediate_base = resolve_base(declarations, target)?;
    let mut derived_to_base = Vec::new();
    let mut positions = BTreeMap::new();
    let mut current = target;
    loop {
        if let Some(&position) = positions.get(&current.name) {
            let mut cycle = derived_to_base[position..]
                .iter()
                .map(|declaration: &&TypeDecl| declaration.name.clone())
                .collect::<Vec<_>>();
            cycle.push(current.name.clone());
            return Err(StructuralProjectionError::Cycle(cycle));
        }
        positions.insert(current.name.clone(), derived_to_base.len());
        derived_to_base.push(current);
        let Some(base) = resolve_base(declarations, current)? else {
            break;
        };
        current = base;
    }
    derived_to_base.reverse();

    let ancestry = derived_to_base
        .iter()
        .map(|declaration| StructuralLevel {
            declaration,
            local_kind: structural_kind(declaration).expect("checked while resolving ancestry"),
        })
        .collect();
    let segments = derived_to_base
        .iter()
        .filter_map(|declaration| segment(declaration))
        .collect::<Vec<_>>();
    let mut names = BTreeMap::new();
    for projected in &segments {
        let members = match projected.content {
            StructuralSegmentContent::RecordFields(fields) => fields,
            StructuralSegmentContent::ChoiceAlternatives(alternatives) => alternatives,
        };
        for member in members {
            if let Some(first_owner) = names.insert(&member.name, &projected.owner.name) {
                return Err(StructuralProjectionError::InheritedMemberNameCollision {
                    target: Box::new(target.name.clone()),
                    member: member.name.clone(),
                    first_owner: Box::new(first_owner.clone()),
                    second_owner: Box::new(projected.owner.name.clone()),
                });
            }
        }
    }
    Ok(EffectiveStructuralType {
        declaration: target,
        immediate_base,
        ancestry,
        segments,
    })
}

fn resolve_base<'a>(
    declarations: &BTreeMap<&'a QualifiedName, &'a TypeDecl>,
    owner: &'a TypeDecl,
) -> Result<Option<&'a TypeDecl>, StructuralProjectionError> {
    let Some(base) = &owner.base_type else {
        return Ok(None);
    };
    let TypeRefTarget::Named(base_name) = &base.target else {
        return Err(StructuralProjectionError::PrimitiveBase {
            owner: owner.name.clone(),
        });
    };
    let base = declarations.get(base_name).copied().ok_or_else(|| {
        StructuralProjectionError::MissingBase {
            owner: owner.name.clone(),
            base: base_name.clone(),
        }
    })?;
    if structural_kind(base).is_none() {
        return Err(StructuralProjectionError::NonStructuralBase {
            owner: owner.name.clone(),
            base: base.name.clone(),
        });
    }
    Ok(Some(base))
}

fn structural_kind(declaration: &TypeDecl) -> Option<StructuralKind> {
    match declaration.kind {
        TypeKind::Record { .. } => Some(StructuralKind::Record),
        TypeKind::Choice { .. } => Some(StructuralKind::Choice),
        _ => None,
    }
}

fn segment(declaration: &TypeDecl) -> Option<StructuralSegment<'_>> {
    let content = match &declaration.kind {
        TypeKind::Record { fields } if !fields.is_empty() => {
            StructuralSegmentContent::RecordFields(fields)
        }
        TypeKind::Choice { alternatives } if !alternatives.is_empty() => {
            StructuralSegmentContent::ChoiceAlternatives(alternatives)
        }
        TypeKind::Record { .. } | TypeKind::Choice { .. } => return None,
        _ => return None,
    };
    Some(StructuralSegment {
        owner: declaration,
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ams_gra_oms_ir::{Cardinality, ConstraintSet, NamespaceDecl, SourceRef, TypeRef};

    const NS: &str = "urn:test";

    fn field(name: &str) -> FieldDecl {
        FieldDecl {
            name: name.to_owned(),
            type_ref: TypeRef::primitive(ams_gra_oms_ir::PrimitiveKind::String),
            cardinality: Cardinality::REQUIRED_ONE,
            nillable: false,
            constraints: ConstraintSet::default(),
            documentation: Some(format!("{name} docs")),
            source: SourceRef {
                document: "test.ir".to_owned(),
                line: Some(1),
            },
        }
    }

    fn declaration(name: &str, choice: bool, members: &[&str]) -> TypeDecl {
        let members = members.iter().map(|name| field(name)).collect();
        TypeDecl {
            name: QualifiedName::new(NS, name),
            is_abstract: false,
            base_type: None,
            kind: if choice {
                TypeKind::Choice {
                    alternatives: members,
                }
            } else {
                TypeKind::Record { fields: members }
            },
            constraints: ConstraintSet::default(),
            documentation: None,
            source: SourceRef {
                document: "test.ir".to_owned(),
                line: Some(1),
            },
        }
    }

    fn derived(mut declaration: TypeDecl, base: &str) -> TypeDecl {
        declaration.base_type = Some(TypeRef::named(QualifiedName::new(NS, base)));
        declaration
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

    fn project<'a>(schema: &'a SchemaIr, name: &str) -> EffectiveStructuralType<'a> {
        project_structural_type(schema, &QualifiedName::new(NS, name)).unwrap()
    }

    fn shape(projection: &EffectiveStructuralType<'_>) -> Vec<(StructuralKind, Vec<String>)> {
        projection
            .segments
            .iter()
            .map(|segment| match segment.content {
                StructuralSegmentContent::RecordFields(fields) => (
                    StructuralKind::Record,
                    fields.iter().map(|field| field.name.clone()).collect(),
                ),
                StructuralSegmentContent::ChoiceAlternatives(alternatives) => (
                    StructuralKind::Choice,
                    alternatives
                        .iter()
                        .map(|field| field.name.clone())
                        .collect(),
                ),
            })
            .collect()
    }

    #[test]
    fn standalone_record_and_choice_project_as_one_segment() {
        let schema = schema(vec![
            declaration("R", false, &["A", "B"]),
            declaration("C", true, &["X", "Y"]),
        ]);
        assert_eq!(
            shape(&project(&schema, "R")),
            [(StructuralKind::Record, vec!["A".into(), "B".into()])]
        );
        assert_eq!(
            shape(&project(&schema, "C")),
            [(StructuralKind::Choice, vec!["X".into(), "Y".into()])]
        );
    }

    #[test]
    fn multilevel_records_are_base_to_derived() {
        let schema = schema(vec![
            declaration("Base", false, &["A"]),
            derived(declaration("Middle", false, &["B"]), "Base"),
            derived(declaration("Leaf", false, &["C"]), "Middle"),
        ]);
        let projection = project(&schema, "Leaf");
        assert_eq!(
            projection
                .ancestry
                .iter()
                .map(|level| level.declaration.name.local_name.as_str())
                .collect::<Vec<_>>(),
            ["Base", "Middle", "Leaf"]
        );
        assert_eq!(
            shape(&projection),
            [
                (StructuralKind::Record, vec!["A".into()]),
                (StructuralKind::Record, vec!["B".into()]),
                (StructuralKind::Record, vec!["C".into()])
            ]
        );
    }

    #[test]
    fn mixed_compositors_remain_distinct() {
        let record_choice = schema(vec![
            declaration("Base", false, &["A"]),
            derived(declaration("Derived", true, &["X", "Y"]), "Base"),
        ]);
        assert_eq!(
            shape(&project(&record_choice, "Derived")),
            [
                (StructuralKind::Record, vec!["A".into()]),
                (StructuralKind::Choice, vec!["X".into(), "Y".into()])
            ]
        );
        let choice_record = schema(vec![
            declaration("Base", true, &["X", "Y"]),
            derived(declaration("Derived", false, &["A"]), "Base"),
        ]);
        assert_eq!(
            shape(&project(&choice_record, "Derived")),
            [
                (StructuralKind::Choice, vec!["X".into(), "Y".into()]),
                (StructuralKind::Record, vec!["A".into()])
            ]
        );
    }

    #[test]
    fn empty_extension_is_omitted_without_losing_ancestry() {
        let schema = schema(vec![
            declaration("Base", false, &["A"]),
            derived(declaration("Derived", false, &[]), "Base"),
        ]);
        let projection = project(&schema, "Derived");
        assert_eq!(projection.ancestry.len(), 2);
        assert_eq!(
            shape(&projection),
            [(StructuralKind::Record, vec!["A".into()])]
        );
    }

    #[test]
    fn forward_base_and_abstract_metadata_are_preserved() {
        let derived = derived(declaration("Derived", false, &["B"]), "Base");
        let mut base = declaration("Base", false, &["A"]);
        base.is_abstract = true;
        let schema = schema(vec![derived, base]);
        let projection = project(&schema, "Derived");
        assert!(projection.ancestry[0].declaration.is_abstract);
        assert!(!projection.declaration.is_abstract);
        assert_eq!(projection.immediate_base.unwrap().name.local_name, "Base");
    }
    #[test]
    fn malformed_ir_returns_errors_instead_of_panicking() {
        let missing_schema = schema(vec![derived(declaration("Derived", false, &[]), "Absent")]);
        assert!(matches!(
            project_structural_type(&missing_schema, &QualifiedName::new(NS, "Derived")),
            Err(StructuralProjectionError::MissingBase { .. })
        ));
        let cyclic = schema(vec![
            derived(declaration("A", false, &[]), "B"),
            derived(declaration("B", true, &[]), "A"),
        ]);
        assert!(matches!(
            project_structural_type(&cyclic, &QualifiedName::new(NS, "A")),
            Err(StructuralProjectionError::Cycle(_))
        ));
    }

    #[test]
    fn inherited_member_name_collisions_fail_closed() {
        let schema = schema(vec![
            declaration("Base", false, &["same"]),
            derived(declaration("Derived", true, &["same"]), "Base"),
        ]);
        assert!(matches!(
            project_structural_type(&schema, &QualifiedName::new(NS, "Derived")),
            Err(StructuralProjectionError::InheritedMemberNameCollision { .. })
        ));
    }
}
