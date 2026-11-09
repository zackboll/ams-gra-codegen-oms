//! Checked closed sums use the shared projection, never a backend descendant walk.
use crate::{error, names};
use ams_gra_oms_codegen_core::{
    AbstractValueProjection, CodegenError, TypeEmission, oms_json_type_name,
};
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub(crate) fn factory(local: &str) -> String {
    format!("from_{}", names::symbol(local, false))
}

pub(crate) fn identity(projection_name: &ams_gra_oms_ir::QualifiedName) -> String {
    format!(
        "({}, {})",
        names::literal(&projection_name.namespace_uri),
        names::literal(&projection_name.local_name)
    )
}

pub(crate) fn validate(emissions: &[TypeEmission<'_>]) -> Result<(), CodegenError> {
    for emission in emissions {
        let TypeEmission::AbstractValue(projection) = emission else {
            continue;
        };
        let mut factories: BTreeSet<String> = ["alternative", "value", "self", "mro"]
            .into_iter()
            .map(str::to_owned)
            .chain(emissions.iter().map(|emission| match emission {
                TypeEmission::Declaration(decl) => names::symbol(&decl.name.local_name, false),
                TypeEmission::AbstractValue(proj) => {
                    names::symbol(&proj.declaration.name.local_name, false)
                }
            }))
            .collect();
        let mut identities = BTreeSet::new();
        let mut wires = BTreeSet::new();
        for descendant in &projection.concrete_descendants {
            if !factories.insert(factory(&descendant.name.local_name))
                || !identities.insert(descendant.name.clone())
                || !wires.insert(oms_json_type_name(
                    &descendant.name.namespace_uri,
                    &descendant.name.local_name,
                ))
            {
                return Err(error(
                    "duplicate abstract factory or ambiguous subtype identity",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn render(output: &mut String, projection: &AbstractValueProjection<'_>) {
    let name = names::symbol(&projection.declaration.name.local_name, false);
    let annotation = projection
        .concrete_descendants
        .iter()
        .map(|decl| names::symbol(&decl.name.local_name, false))
        .collect::<Vec<_>>()
        .join(" | ");
    writeln!(output, "\n\n@_dataclass(frozen=True, slots=True)\nclass {name}:\n    __schema_name__: _ClassVar[tuple[str, str]] = {}\n    alternative: tuple[str, str]\n    value: {annotation}\n\n    def __post_init__(self) -> None:\n        _check(self.alternative, tuple)\n        if len(self.alternative) != 2:\n            raise ValueError('invalid subtype QName')\n        for item in self.alternative:\n            _check(item, str)", identity(&projection.declaration.name)).unwrap();
    for descendant in &projection.concrete_descendants {
        let symbol = names::symbol(&descendant.name.local_name, false);
        writeln!(output, "        if self.alternative == {}:\n            _check(self.value, {symbol})\n            return", identity(&descendant.name)).unwrap();
    }
    writeln!(
        output,
        "        raise ValueError('unknown or abstract subtype QName')"
    )
    .unwrap();
    for descendant in &projection.concrete_descendants {
        let symbol = names::symbol(&descendant.name.local_name, false);
        writeln!(output, "\n    @staticmethod\n    def {}(value: {symbol}) -> \"{name}\":\n        return {name}({}, value)", factory(&descendant.name.local_name), identity(&descendant.name)).unwrap();
    }
}
