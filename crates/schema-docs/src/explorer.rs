//! Versioned graph export. All semantics come from validated normalized IR.
use super::*;
use serde_json::{Value, json};

fn qname(name: &QualifiedName) -> Value {
    json!([name.namespace_uri, name.local_name])
}

fn reference(value: &TypeRef) -> Value {
    match &value.target {
        TypeRefTarget::Named(name) => json!({"named": qname(name)}),
        TypeRefTarget::Primitive(p) => json!({"primitive": format!("{p:?}")}),
    }
}

fn count(value: Cardinality) -> Value {
    json!({"min": value.min_occurs.to_string(), "max": value.max_occurs.map(|n| n.to_string())})
}

fn facets(value: &ConstraintSet, primitive: Option<PrimitiveKind>) -> Value {
    let mut items = Vec::new();
    for (label, n) in [
        ("minInclusive", value.min_inclusive),
        ("maxInclusive", value.max_inclusive),
        ("minExclusive", value.min_exclusive),
        ("maxExclusive", value.max_exclusive),
    ] {
        if let Some(n) = n {
            items.push(json!({"name": label, "value": number(n)}));
        }
    }
    for (label, n) in [
        ("length", value.length),
        ("minLength", value.min_length),
        ("maxLength", value.max_length),
    ] {
        if let Some(n) = n {
            items.push(json!({"name": label, "value": n.to_string()}));
        }
    }
    if let Some(policy) = value.lexical.white_space {
        items.push(json!({"name": "whiteSpace", "value": format!("{policy:?}")}));
    }
    if let Some(p) = primitive {
        items.push(json!({"name": "effectiveWhiteSpace", "value": format!("{:?}", value.lexical.effective_white_space(p))}));
    }
    json!({"facets": items, "patternGroups": value.lexical.pattern_groups.iter().map(|g| g.alternatives.iter().map(|p| json!({"dialect": format!("{:?}", p.dialect), "expression": p.expression})).collect::<Vec<_>>()).collect::<Vec<_>>()})
}

pub(super) fn data(index: &Index<'_>) -> Result<String, String> {
    let mut types = Vec::new();
    for name in index.pages.keys() {
        let t = index.declarations[name];
        let mut members = Vec::new();
        let mut segments = Vec::new();
        let mut ancestry = Vec::new();
        if matches!(t.kind, TypeKind::Record { .. } | TypeKind::Choice { .. }) {
            let structure = lossless_structural_with_index(&index.declarations, name)
                .map_err(|e| e.to_string())?;
            ancestry = structure
                .ancestry
                .iter()
                .map(|l| qname(&l.declaration.name))
                .collect();
            for segment in structure.segments {
                let compositor = match segment.content {
                    StructuralSegmentContent::RecordFields(_) => "Record",
                    StructuralSegmentContent::ChoiceAlternatives(_) => "Choice",
                };
                segments
                    .push(json!({"owner": qname(&segment.owner.name), "compositor": compositor}));
            }
            let fields = match &t.kind {
                TypeKind::Record { fields } => fields,
                TypeKind::Choice { alternatives } => alternatives,
                _ => unreachable!(),
            };
            for (i, f) in fields.iter().enumerate() {
                members.push(json!({"index": i, "name": f.name, "namespace": f.wire_namespace_uri,
                    "type": reference(&f.type_ref), "count": count(f.cardinality), "nillable": f.nillable,
                    "documentation": f.documentation, "constraints": facets(&f.constraints, index.primitive(&f.type_ref))}));
            }
        }
        let (target, item, item_count, enumeration) = match &t.kind {
            TypeKind::Alias(r) => (Some(reference(r)), None, None, Vec::new()),
            TypeKind::List {
                item_type,
                cardinality,
            } => (
                None,
                Some(reference(item_type)),
                Some(count(*cardinality)),
                Vec::new(),
            ),
            TypeKind::Enumeration { variants } => (
                None,
                None,
                None,
                variants
                    .iter()
                    .map(|v| json!({"value": v.wire_value, "documentation": v.documentation}))
                    .collect(),
            ),
            _ => (None, None, None, Vec::new()),
        };
        let primitive = index.primitive(&TypeRef::named(name.clone()));
        types.push(
            json!({"qname": qname(name), "kind": kind(&t.kind), "abstract": t.is_abstract,
            "base": t.base_type.as_ref().map(reference), "url": index.pages[name],
            "documentation": t.documentation, "constraints": facets(&t.constraints, primitive),
            "members": members, "segments": segments, "ancestry": ancestry,
            "target": target, "item": item, "itemCount": item_count, "enumeration": enumeration,
            "primitive": primitive.map(|p| format!("{p:?}"))}),
        );
    }
    let messages = index.schema.messages.iter().enumerate().map(|(i, m)| json!({
        "qname": qname(&m.name), "type": reference(&m.payload_type), "documentation": m.documentation,
        "url": format!("index.html#message-{i:06}")
    })).collect::<Vec<_>>();
    let roots = index
        .schema
        .messages
        .iter()
        .map(|m| &m.source.document)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let graph = json!({"version": 1, "schema": {"version": index.schema.schema_version,
        "roots": roots, "namespaces": index.schema.namespaces.iter().map(|n| json!({"uri": n.uri, "prefix": n.preferred_prefix})).collect::<Vec<_>>()},
        "types": types, "messages": messages});
    Ok(format!(
        "window.schemaExplorerData = {};\n",
        serde_json::to_string(&graph).expect("JSON values serialize")
    ))
}

pub(super) fn link(kind: &str, name: &QualifiedName, prefix: &str, label: &str) -> String {
    let state = json!({"v": 1, "kind": kind, "root": qname(name), "path": []});
    let encoded = serde_json::to_string(&state)
        .expect("JSON values serialize")
        .bytes()
        .map(|b| format!("%{b:02X}"))
        .collect::<String>();
    format!("<a href=\"{prefix}explorer.html#uci={encoded}\">{label}</a>")
}
