use ams_gra_oms_codegen_core::*;
use ams_gra_oms_ir::*;
use ams_gra_oms_service_contract::parse_yaml;
use ams_gra_oms_xsd_frontend::load_schema_set;
use std::{collections::BTreeMap, path::Path, time::Instant};
fn plan(s: &SchemaIr, m: &str) -> ServicePlan {
    let c=parse_yaml(&format!("contract_version: \"0.1\"\nservice:\n  name: Audit\n  version: \"0.1\"\n  kind: service\nstandards:\n  oms_version: \"2.5\"\n  uci_schema_version: \"2.5\"\nfunctions:\n  - id: f1\n    name: Audit\n    category: specific\n    applicability: applicable\n    exchanges:\n      - id: e1\n        kind: oms_message\n        direction: input\n        mandate: mandatory\n        message: {m}\n        topic: audit\n        timing:\n          kind: asynchronous\n")).unwrap();
    resolve_service_plan(&c, s).unwrap()
}
fn q(n: &QualifiedName) -> String {
    format!("{{{}}}{}", n.namespace_uri, n.local_name)
}
fn main() {
    let start = Instant::now();
    println!(
        "release\tmessage\ttarget\tselected_or_support\towner\tmember\tcardinality\tknown_concrete_descendants\tabstract_descendants\tmaximum_depth\trecursive\tinherited\tpath"
    );
    for (rel, file) in [
        ("2.5", "/tmp/task068/manifests/uci25-root.txt"),
        ("2.6", "/tmp/task068/manifests/uci26-root.txt"),
    ] {
        let root = std::fs::read_to_string(file).unwrap();
        let s = load_schema_set(Path::new(root.trim())).unwrap();
        let index = s
            .types
            .iter()
            .map(|d| (d.name.clone(), d))
            .collect::<BTreeMap<_, _>>();
        let mut descendants: BTreeMap<QualifiedName, Vec<(&TypeDecl, usize)>> = BTreeMap::new();
        for d in &s.types {
            let mut b = d.base_type.as_ref();
            let mut depth = 0;
            while let Some(TypeRef {
                target: TypeRefTarget::Named(n),
                ..
            }) = b
            {
                depth += 1;
                descendants.entry(n.clone()).or_default().push((d, depth));
                b = index[n].base_type.as_ref();
            }
        }
        let mut edges: BTreeMap<QualifiedName, Vec<(&FieldDecl, bool)>> = BTreeMap::new();
        for d in &s.types {
            let fields = match &d.kind {
                TypeKind::Record { .. } => effective_record_fields(&s, &d.name),
                TypeKind::Choice { .. } => effective_choice_alternatives(&s, &d.name),
                _ => continue,
            };
            if let Ok(fields) = fields {
                let local = match &d.kind {
                    TypeKind::Record { fields } => fields,
                    TypeKind::Choice { alternatives } => alternatives,
                    _ => unreachable!(),
                };
                edges.insert(
                    d.name.clone(),
                    fields
                        .into_iter()
                        .map(|f| (f, !local.iter().any(|l| std::ptr::eq(l, f))))
                        .collect(),
                );
            }
        }
        let mut recursion = BTreeMap::new();
        let mut rows = Vec::new();
        let mut messages = s.messages.iter().collect::<Vec<_>>();
        messages.sort_by_key(|m| &m.name);
        for m in messages {
            let p = plan(&s, &m.name.local_name);
            let Err(ServiceGenerationError::AbstractValue(
                AbstractValueProjectionError::NotClosedUnderOpenExtensions(first),
            )) = project_service_generation_schema(&p, &s, GenerationWorld::OpenExtensions)
            else {
                continue;
            };
            let selected = p
                .selected_type_closure(&s)
                .unwrap()
                .iter()
                .map(|d| d.name.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let closed =
                project_service_generation_schema(&p, &s, GenerationWorld::ClosedSchemaSet);
            let mut required = selected.clone();
            let mut pending = selected.iter().cloned().collect::<Vec<_>>();
            while let Some(n) = pending.pop() {
                let d = index[&n];
                let mut refs = Vec::new();
                if let Some(TypeRef {
                    target: TypeRefTarget::Named(b),
                    ..
                }) = &d.base_type
                {
                    if required.insert(b.clone()) {
                        pending.push(b.clone());
                    }
                }
                match &d.kind {
                    TypeKind::Record { fields } => refs.extend(fields.iter().map(|f| &f.type_ref)),
                    TypeKind::Choice { alternatives } => {
                        refs.extend(alternatives.iter().map(|f| &f.type_ref))
                    }
                    TypeKind::Alias(r) | TypeKind::List { item_type: r, .. } => refs.push(r),
                    _ => {}
                }
                for r in refs {
                    if let TypeRefTarget::Named(t) = &r.target {
                        if required.insert(t.clone()) {
                            pending.push(t.clone());
                        }
                        if index[t].is_abstract {
                            for (desc, _) in descendants.get(t).cloned().unwrap_or_default() {
                                if !desc.is_abstract && required.insert(desc.name.clone()) {
                                    pending.push(desc.name.clone());
                                }
                            }
                        }
                    }
                }
            }
            let support = Some(required.len() - selected.len());
            if let Ok(c) = &closed {
                assert_eq!(support, Some(c.generated_support_type_names().len()));
            }
            eprintln!(
                "IMPACT\t{rel}\t{}\t{}\t{support:?}\tfirst={}",
                m.name.local_name,
                selected.len(),
                first.local_name
            );
            // Deterministic breadth-first value/support walk, one path per owner.
            let TypeRefTarget::Named(root) = &m.payload_type.target else {
                continue;
            };
            if index[root].is_abstract {
                let ds = descendants.get(root).cloned().unwrap_or_default();
                let concrete = ds.iter().filter(|(d, _)| !d.is_abstract).count();
                let rec = matches!(
                    classify_abstract_value_topology(&s, root),
                    Ok(AbstractValueTopology::RecursiveValueGraph { .. })
                );
                rows.push(format!("{rel}\t{}\t{}\tselected\t{}\t<payload>\t1..1\t{concrete}\t{}\t{}\t{rec}\tfalse\t{} -> {}",q(&m.name),q(root),q(&m.name),ds.len()-concrete,ds.iter().map(|(_,n)|*n).max().unwrap_or(0),q(&m.name),q(root)));
            }
            let mut queue = std::collections::VecDeque::from([(
                root.clone(),
                format!("{} -> {}", q(&m.name), q(root)),
            )]);
            let mut seen = std::collections::BTreeSet::new();
            let mut targets = std::collections::BTreeSet::new();
            while let Some((owner, path)) = queue.pop_front() {
                if !seen.insert(owner.clone()) {
                    continue;
                }
                if let Some(fs) = edges.get(&owner) {
                    let mut fs = fs.clone();
                    fs.sort_by_key(|(f, _)| &f.name);
                    for (f, inherited) in fs {
                        if let TypeRefTarget::Named(t) = &f.type_ref.target {
                            let d = index[t];
                            let next = format!("{path}.{} -> {}", f.name, q(t));
                            if d.is_abstract
                                && matches!(
                                    d.kind,
                                    TypeKind::Record { .. } | TypeKind::Choice { .. }
                                )
                            {
                                targets.insert(t.clone());
                                let ds = descendants.get(t).cloned().unwrap_or_default();
                                let concrete = ds.iter().filter(|(d, _)| !d.is_abstract).count();
                                let abstract_count = ds.len() - concrete;
                                let depth = ds.iter().map(|(_, n)| *n).max().unwrap_or(0);
                                let recursive = recursion.entry(t.clone()).or_insert_with(|| {
                                    matches!(
                                        classify_abstract_value_topology(&s, t),
                                        Ok(AbstractValueTopology::RecursiveValueGraph { .. })
                                    )
                                });
                                rows.push(format!("{rel}\t{}\t{}\t{}\t{}\t{}\t{}..{}\t{concrete}\t{abstract_count}\t{depth}\t{recursive}\t{inherited}\t{next}",q(&m.name),q(t),if selected.contains(&owner){"selected"}else{"support"},q(&owner),f.name,f.cardinality.min_occurs,f.cardinality.max_occurs.map_or("unbounded".to_string(),|n|n.to_string())));
                                for (desc, _) in ds {
                                    if !desc.is_abstract {
                                        queue.push_back((
                                            desc.name.clone(),
                                            format!("{next} => {}", q(&desc.name)),
                                        ));
                                    }
                                }
                            }
                            queue.push_back((t.clone(), next));
                        }
                    }
                }
                let d = index[&owner];
                if let TypeKind::Alias(r) | TypeKind::List { item_type: r, .. } = &d.kind {
                    if let TypeRefTarget::Named(t) = &r.target {
                        queue.push_back((t.clone(), format!("{path} -> {}", q(t))));
                    }
                }
            }
            eprintln!(
                "TARGETS\t{rel}\t{}\t{}",
                m.name.local_name,
                targets
                    .iter()
                    .map(|n| n.local_name.clone())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
        rows.sort_by(|a, b| {
            let a = a.split('\t').collect::<Vec<_>>();
            let b = b.split('\t').collect::<Vec<_>>();
            (&a[0], &a[1], &a[2], &a[12]).cmp(&(&b[0], &b[1], &b[2], &b[12]))
        });
        for row in rows {
            println!("{row}");
        }
        for (t, ds) in &descendants {
            if recursion.contains_key(t) {
                eprintln!(
                    "DESCENDANTS\t{rel}\t{}\t{}",
                    q(t),
                    ds.iter()
                        .filter(|(d, _)| !d.is_abstract)
                        .map(|(d, _)| q(&d.name))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
        }
    }
    eprintln!("elapsed {:?}", start.elapsed());
}
