//! Static named-reference exposure, not generated-support or compatibility analysis.
use crate::direct_named_dependencies;
use ams_gra_oms_ir::{QualifiedName, SchemaIr, TypeRefTarget};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyImpactError {
    InvalidSchema(ams_gra_oms_ir::ValidationError),
    UnknownSeed(QualifiedName),
}

impl std::fmt::Display for DependencyImpactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSchema(e) => write!(f, "{e}"),
            Self::UnknownSeed(name) => write!(f, "unknown impact seed: {name:?}"),
        }
    }
}
impl std::error::Error for DependencyImpactError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyExposure {
    pub message: QualifiedName,
    pub seed: QualifiedName,
    /// Named payload first, seed last. The message-to-payload edge is separate.
    pub witness: Vec<QualifiedName>,
}

/// Query a validated snapshot. Validation is enforced here; unknown seeds fail.
/// No reverse inheritance/descendant or backend-storage edges are introduced.
///
/// Index construction costs O((V+E+M) log V); each unique seed costs
/// O((V+E) log V) for reverse BFS, plus witness reconstruction/output size.
/// Memory is O(V+E+M) working storage plus returned witness size. Traversal is
/// iterative and cycle-safe. Equal-length paths use lexicographic full QNames.
pub fn static_dependency_exposures(
    schema: &SchemaIr,
    seeds: &BTreeSet<QualifiedName>,
) -> Result<Vec<DependencyExposure>, DependencyImpactError> {
    schema
        .validate()
        .map_err(DependencyImpactError::InvalidSchema)?;
    let edges: BTreeMap<_, BTreeSet<_>> = schema
        .types
        .iter()
        .map(|t| {
            (
                t.name.clone(),
                direct_named_dependencies(t).into_iter().cloned().collect(),
            )
        })
        .collect();
    let mut reverse: BTreeMap<QualifiedName, BTreeSet<QualifiedName>> = edges
        .keys()
        .cloned()
        .map(|n| (n, BTreeSet::new()))
        .collect();
    for (owner, targets) in &edges {
        for target in targets {
            reverse
                .get_mut(target)
                .expect("validated reference")
                .insert(owner.clone());
        }
    }
    let mut roots: BTreeMap<QualifiedName, BTreeSet<QualifiedName>> = BTreeMap::new();
    for message in &schema.messages {
        if let TypeRefTarget::Named(root) = &message.payload_type.target {
            roots
                .entry(root.clone())
                .or_default()
                .insert(message.name.clone());
        }
    }
    let mut result = Vec::new();
    for seed in seeds {
        if !edges.contains_key(seed) {
            return Err(DependencyImpactError::UnknownSeed(seed.clone()));
        }
        let distances = reverse_distances(&reverse, seed);
        for (root, messages) in &roots {
            if !distances.contains_key(root) {
                continue;
            }
            let mut witness = vec![root.clone()];
            let mut current = root;
            while current != seed {
                current = edges[current]
                    .iter()
                    .find(|next| {
                        distances
                            .get(*next)
                            .is_some_and(|d| *d + 1 == distances[current])
                    })
                    .expect("BFS predecessor");
                witness.push(current.clone());
            }
            for message in messages {
                result.push(DependencyExposure {
                    message: message.clone(),
                    seed: seed.clone(),
                    witness: witness.clone(),
                });
            }
        }
    }
    result.sort();
    Ok(result)
}

fn reverse_distances(
    reverse: &BTreeMap<QualifiedName, BTreeSet<QualifiedName>>,
    seed: &QualifiedName,
) -> BTreeMap<QualifiedName, usize> {
    let mut distances = BTreeMap::from([(seed.clone(), 0)]);
    let mut queue = VecDeque::from([seed.clone()]);
    while let Some(node) = queue.pop_front() {
        let distance = distances[&node] + 1;
        for parent in &reverse[&node] {
            if !distances.contains_key(parent) {
                distances.insert(parent.clone(), distance);
                queue.push_back(parent.clone());
            }
        }
    }
    distances
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn internal_cycle_distances_terminate() {
        let a = QualifiedName::new("n", "a");
        let b = QualifiedName::new("n", "b");
        let graph = BTreeMap::from([
            (a.clone(), BTreeSet::from([b.clone()])),
            (b.clone(), BTreeSet::from([a.clone()])),
        ]);
        assert_eq!(
            reverse_distances(&graph, &a),
            BTreeMap::from([(a, 0), (b, 1)])
        );
    }
}
