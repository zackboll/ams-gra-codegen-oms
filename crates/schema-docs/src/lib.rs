//! Offline documentation over the already-normalized schema IR.

use ams_gra_oms_codegen_core::{
    GeneratedFile, StructuralSegmentContent, direct_named_dependencies, project_with_index,
};
use ams_gra_oms_ir::{
    Cardinality, ConstraintSet, FieldDecl, NumericValue, PrimitiveKind, QualifiedName, SchemaIr,
    SourceRef, TypeDecl, TypeKind, TypeRef, TypeRefTarget,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::PathBuf;

struct Index<'a> {
    schema: &'a SchemaIr,
    pages: BTreeMap<QualifiedName, String>,
    declarations: BTreeMap<&'a QualifiedName, &'a TypeDecl>,
    uses: BTreeMap<QualifiedName, BTreeSet<QualifiedName>>,
    reverse: BTreeMap<QualifiedName, BTreeSet<QualifiedName>>,
    messages: BTreeMap<QualifiedName, Vec<usize>>,
}

impl<'a> Index<'a> {
    fn new(schema: &'a SchemaIr) -> Self {
        let pages = schema
            .types
            .iter()
            .map(|t| &t.name)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), format!("types/{i:06}.html")))
            .collect();
        let declarations = schema.types.iter().map(|t| (&t.name, t)).collect();
        let mut uses = BTreeMap::new();
        let mut reverse: BTreeMap<QualifiedName, BTreeSet<QualifiedName>> = BTreeMap::new();
        for t in &schema.types {
            let deps: BTreeSet<_> = direct_named_dependencies(t).into_iter().cloned().collect();
            for dep in &deps {
                reverse
                    .entry(dep.clone())
                    .or_default()
                    .insert(t.name.clone());
            }
            uses.insert(t.name.clone(), deps);
        }
        let mut messages: BTreeMap<QualifiedName, Vec<usize>> = BTreeMap::new();
        for (i, message) in schema.messages.iter().enumerate() {
            if let TypeRefTarget::Named(name) = &message.payload_type.target {
                messages.entry(name.clone()).or_default().push(i);
            }
        }
        Self {
            schema,
            pages,
            declarations,
            uses,
            reverse,
            messages,
        }
    }

    fn link(&self, name: &QualifiedName, from_type: bool) -> String {
        let path = &self.pages[name];
        let href = if from_type {
            path.strip_prefix("types/").unwrap_or(path)
        } else {
            path
        };
        format!(
            "<a href=\"{}\">{}</a>",
            escape(href),
            escape(&canonical(name))
        )
    }

    fn reference(&self, reference: &TypeRef, from_type: bool) -> String {
        match &reference.target {
            TypeRefTarget::Named(name) => self.link(name, from_type),
            TypeRefTarget::Primitive(kind) => escape(&format!("{kind:?}")),
        }
    }

    fn primitive(&self, reference: &TypeRef) -> Option<PrimitiveKind> {
        let mut current = reference;
        let mut seen = BTreeSet::new();
        loop {
            match &current.target {
                TypeRefTarget::Primitive(p) => return Some(*p),
                TypeRefTarget::Named(name) => {
                    if !seen.insert(name) {
                        return None;
                    }
                    let decl = self.declarations.get(name)?;
                    match &decl.kind {
                        TypeKind::Primitive(p) => return Some(*p),
                        TypeKind::Alias(next) => current = next,
                        _ => current = decl.base_type.as_ref()?,
                    }
                }
            }
        }
    }
}

fn canonical(name: &QualifiedName) -> String {
    format!("{{{}}}{}", name.namespace_uri, name.local_name)
}

/// Escape untrusted schema data in both HTML text and quoted attributes.
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn source(source: &SourceRef) -> String {
    escape(&match source.line {
        Some(line) => format!("{}:{line}", source.document),
        None => source.document.clone(),
    })
}

fn documentation(text: &Option<String>) -> String {
    text.as_ref()
        .map(|s| format!("<p class=\"documentation\">{}</p>", escape(s)))
        .unwrap_or_default()
}

fn cardinality(value: Cardinality) -> String {
    match value.max_occurs {
        Some(max) if max == value.min_occurs => max.to_string(),
        Some(max) => format!("{}..{max}", value.min_occurs),
        None => format!("{}..*", value.min_occurs),
    }
}

fn number(value: NumericValue) -> String {
    match value {
        NumericValue::Integer(n) => n.to_string(),
        NumericValue::Float32(n) => n.value().to_string(),
        NumericValue::Float64(n) => n.value().to_string(),
    }
}

fn constraints(value: &ConstraintSet, primitive: Option<PrimitiveKind>) -> String {
    let mut out = String::from("<section><h3>Effective constraints</h3><ul>");
    for (label, n) in [
        ("minInclusive", value.min_inclusive),
        ("maxInclusive", value.max_inclusive),
        ("minExclusive", value.min_exclusive),
        ("maxExclusive", value.max_exclusive),
    ] {
        if let Some(n) = n {
            let _ = write!(out, "<li>{label}: {}</li>", escape(&number(n)));
        }
    }
    for (label, n) in [
        ("length", value.length),
        ("minLength", value.min_length),
        ("maxLength", value.max_length),
    ] {
        if let Some(n) = n {
            let _ = write!(out, "<li>{label}: {n}</li>");
        }
    }
    if let Some(policy) = value.lexical.white_space {
        let _ = write!(out, "<li>whiteSpace: {policy:?}</li>");
    }
    if let Some(primitive) = primitive {
        let _ = write!(
            out,
            "<li>Effective whiteSpace: {:?}</li>",
            value.lexical.effective_white_space(primitive)
        );
    }
    out.push_str("</ul>");
    for (i, group) in value.lexical.pattern_groups.iter().enumerate() {
        if i > 0 {
            out.push_str("<p>AND</p>");
        }
        let _ = write!(out, "<h4>Pattern group {} — one of:</h4><ul>", i + 1);
        for pattern in &group.alternatives {
            let _ = write!(
                out,
                "<li><code>{}</code> ({:?})</li>",
                escape(&pattern.expression),
                pattern.dialect
            );
        }
        out.push_str("</ul>");
    }
    out.push_str("</section>");
    out
}

fn members(index: &Index<'_>, fields: &[FieldDecl]) -> String {
    let mut out = String::from("<ul>");
    for field in fields {
        let _ = write!(
            out,
            "<li><strong>{}</strong> — {} — cardinality {} — nillable: {} — source: {}{}{} </li>",
            escape(&field.name),
            index.reference(&field.type_ref, true),
            cardinality(field.cardinality),
            field.nillable,
            source(&field.source),
            documentation(&field.documentation),
            constraints(&field.constraints, index.primitive(&field.type_ref))
        );
    }
    out.push_str("</ul>");
    out
}

fn kind(kind: &TypeKind) -> &'static str {
    match kind {
        TypeKind::Primitive(_) => "Primitive",
        TypeKind::Alias(_) => "Alias",
        TypeKind::Enumeration { .. } => "Enumeration",
        TypeKind::Record { .. } => "Record",
        TypeKind::Choice { .. } => "Choice",
        TypeKind::List { .. } => "List",
    }
}

fn header(title: &str, depth: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><link rel=\"stylesheet\" href=\"{depth}assets/style.css\"></head><body><nav><a href=\"{depth}index.html\">Schema index</a></nav><main><h1>{}</h1>",
        escape(title),
        escape(title)
    )
}

fn type_page(index: &Index<'_>, t: &TypeDecl) -> Result<String, String> {
    let mut out = header(&t.name.local_name, "../");
    let _ = write!(
        out,
        "<dl><dt>Qualified name</dt><dd>{}</dd><dt>Namespace URI</dt><dd>{}</dd><dt>Kind</dt><dd>{}</dd><dt>Abstract</dt><dd>{}</dd><dt>Source</dt><dd>{}</dd></dl>",
        escape(&canonical(&t.name)),
        escape(&t.name.namespace_uri),
        kind(&t.kind),
        t.is_abstract,
        source(&t.source)
    );
    if let Some(prefix) = index
        .schema
        .namespaces
        .iter()
        .find(|n| n.uri == t.name.namespace_uri)
        .and_then(|n| n.preferred_prefix.as_ref())
    {
        let _ = write!(
            out,
            "<p>Preferred spelling: {}:{}</p>",
            escape(prefix),
            escape(&t.name.local_name)
        );
    }
    out.push_str(&documentation(&t.documentation));
    if let Some(base) = &t.base_type {
        let _ = write!(out, "<p>Base type: {}</p>", index.reference(base, true));
    }
    let primitive = match t.kind {
        TypeKind::Primitive(p) => Some(p),
        _ => t.base_type.as_ref().and_then(|base| index.primitive(base)),
    };
    out.push_str(&constraints(&t.constraints, primitive));
    match &t.kind {
        TypeKind::Primitive(p) => {
            let _ = write!(out, "<h2>Primitive kind</h2><p>{p:?}</p>");
        }
        TypeKind::Alias(target) => {
            let _ = write!(
                out,
                "<h2>Alias target</h2><p>{}</p>",
                index.reference(target, true)
            );
        }
        TypeKind::Enumeration { variants } => {
            out.push_str("<h2>Variants (wire values)</h2><ul>");
            for v in variants {
                let _ = write!(
                    out,
                    "<li><code>{}</code>{}</li>",
                    escape(&v.wire_value),
                    documentation(&v.documentation)
                );
            }
            out.push_str("</ul>");
        }
        TypeKind::Record { fields }
        | TypeKind::Choice {
            alternatives: fields,
        } => {
            let label = if matches!(t.kind, TypeKind::Record { .. }) {
                "fields"
            } else {
                "alternatives"
            };
            let _ = write!(out, "<h2>Declared {label}</h2>");
            out.push_str(&members(index, fields));
            let projection =
                project_with_index(&index.declarations, &t.name).map_err(|e| e.to_string())?;
            out.push_str("<h2>Effective structural ancestry (base to derived)</h2><ol>");
            for level in &projection.ancestry {
                let _ = write!(
                    out,
                    "<li>{} — {:?} — abstract: {}</li>",
                    index.link(&level.declaration.name, true),
                    level.local_kind,
                    level.declaration.is_abstract
                );
            }
            out.push_str("</ol><h2>Effective members by owner and compositor</h2>");
            for segment in &projection.segments {
                let (label, fields) = match segment.content {
                    StructuralSegmentContent::RecordFields(fields) => ("Record fields", fields),
                    StructuralSegmentContent::ChoiceAlternatives(fields) => {
                        ("Choice alternatives", fields)
                    }
                };
                let _ = write!(
                    out,
                    "<h3>{} — {label}</h3>",
                    index.link(&segment.owner.name, true)
                );
                out.push_str(&members(index, fields));
            }
        }
        TypeKind::List {
            item_type,
            cardinality: count,
        } => {
            let _ = write!(
                out,
                "<h2>List</h2><p>Item type: {} — cardinality: {}</p>",
                index.reference(item_type, true),
                cardinality(*count)
            );
        }
    }
    out.push_str("<h2>Uses</h2><ul>");
    for name in &index.uses[&t.name] {
        let _ = write!(out, "<li>{}</li>", index.link(name, true));
    }
    out.push_str("</ul><h2>Referenced by</h2><ul>");
    if let Some(names) = index.reverse.get(&t.name) {
        for name in names {
            let _ = write!(out, "<li>{}</li>", index.link(name, true));
        }
    }
    if let Some(messages) = index.messages.get(&t.name) {
        for i in messages {
            let _ = write!(
                out,
                "<li>Message: <a href=\"../index.html#message-{i:06}\">{}</a></li>",
                escape(&canonical(&index.schema.messages[*i].name))
            );
        }
    }
    out.push_str("</ul></main></body></html>");
    Ok(out)
}

fn landing(index: &Index<'_>) -> String {
    let schema = index.schema;
    let mut out = header("Schema browser", "");
    let _ = write!(
        out,
        "<section><h2>Schema summary</h2><p>Schema version: {} · Namespaces: {} · Types: {} · Messages: {}</p>",
        escape(schema.schema_version.as_deref().unwrap_or("not specified")),
        schema.namespaces.len(),
        schema.types.len(),
        schema.messages.len()
    );
    for label in [
        "Primitive",
        "Alias",
        "Enumeration",
        "Record",
        "Choice",
        "List",
    ] {
        let count = schema
            .types
            .iter()
            .filter(|t| kind(&t.kind) == label)
            .count();
        let _ = write!(out, "<span class=\"count\">{label}: {count}</span> ");
    }
    out.push_str("</section><section><h2>Search</h2><label for=\"search\">Search types and messages</label> <input id=\"search\" type=\"search\"><p id=\"match-count\"></p><ul id=\"results\"></ul></section><section><h2>Namespaces</h2><ul>");
    for ns in &schema.namespaces {
        let count = schema
            .types
            .iter()
            .filter(|t| t.name.namespace_uri == ns.uri)
            .count();
        let _ = write!(
            out,
            "<li>{} — prefix: {} — types: {count}</li>",
            escape(&ns.uri),
            escape(ns.preferred_prefix.as_deref().unwrap_or("none"))
        );
    }
    out.push_str("</ul></section><section><h2>Browse all types</h2><ul>");
    for name in index.pages.keys() {
        let t = index.declarations[name];
        let _ = write!(
            out,
            "<li>{} — {} ({})</li>",
            index.link(name, false),
            escape(&name.local_name),
            kind(&t.kind)
        );
    }
    out.push_str("</ul></section><section><h2>Messages</h2><ul>");
    for (i, m) in schema.messages.iter().enumerate() {
        let _ = write!(
            out,
            "<li id=\"message-{i:06}\">{} — payload: {} — source: {}{}</li>",
            escape(&canonical(&m.name)),
            index.reference(&m.payload_type, false),
            source(&m.source),
            documentation(&m.documentation)
        );
    }
    out.push_str("</ul></section></main><script src=\"assets/search-index.js\"></script><script src=\"assets/search.js\"></script></body></html>");
    out
}

fn search(index: &Index<'_>) -> String {
    let mut records = Vec::new();
    for name in index.pages.keys() {
        let t = index.declarations[name];
        let prefix = index
            .schema
            .namespaces
            .iter()
            .find(|n| n.uri == name.namespace_uri)
            .and_then(|n| n.preferred_prefix.as_deref())
            .unwrap_or("");
        let mut terms = format!(
            "{} {} {} {} {} {} {}",
            name.local_name,
            canonical(name),
            name.namespace_uri,
            prefix,
            if prefix.is_empty() {
                String::new()
            } else {
                format!("{prefix}:{}", name.local_name)
            },
            kind(&t.kind),
            t.documentation.as_deref().unwrap_or("")
        );
        match &t.kind {
            TypeKind::Record { fields }
            | TypeKind::Choice {
                alternatives: fields,
            } => {
                for f in fields {
                    terms.push(' ');
                    terms.push_str(&f.name);
                }
            }
            TypeKind::Enumeration { variants } => {
                for v in variants {
                    terms.push(' ');
                    terms.push_str(&v.wire_value);
                }
            }
            _ => {}
        }
        records.push(
            serde_json::json!({"label": canonical(name), "url": index.pages[name], "terms": terms}),
        );
    }
    for (i, m) in index.schema.messages.iter().enumerate() {
        records.push(serde_json::json!({"label": canonical(&m.name), "url": format!("index.html#message-{i:06}"), "terms": format!("{} {} {} {}", m.name.local_name, canonical(&m.name), m.name.namespace_uri, m.documentation.as_deref().unwrap_or(""))}));
    }
    // JSON may contain a literal closing script tag, but this is an external
    // JavaScript file, never embedded in a <script> element as inline source.
    format!(
        "window.schemaSearchIndex = {};\n",
        serde_json::to_string(&records).expect("JSON values serialize")
    )
}

fn file(path: &str, contents: String) -> GeneratedFile {
    GeneratedFile {
        relative_path: PathBuf::from(path),
        contents,
    }
}

/// Construct the entire deterministic offline file set before any filesystem writes.
///
/// # Errors
/// Returns an IR validation or structural projection diagnostic.
pub fn generate(schema: &SchemaIr) -> Result<Vec<GeneratedFile>, String> {
    schema.validate().map_err(|e| e.to_string())?;
    let index = Index::new(schema);
    let mut files = vec![
        file("index.html", landing(&index)),
        file("assets/style.css", include_str!("style.css").to_owned()),
        file("assets/search.js", include_str!("search.js").to_owned()),
        file("assets/search-index.js", search(&index)),
    ];
    for name in index.pages.keys() {
        files.push(file(
            &index.pages[name],
            type_page(&index, index.declarations[name])?,
        ));
    }
    Ok(files)
}
