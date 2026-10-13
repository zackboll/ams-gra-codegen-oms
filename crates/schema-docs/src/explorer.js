"use strict";
(() => {
  const LIMIT = { depth: 24, rows: 2000, results: 50, details: 50, fragment: 16384 };
  const $ = id => document.getElementById(id);
  const status = text => { $("status").textContent = text; };
  const key = q => JSON.stringify(q);
  const validQ = q => Array.isArray(q) && q.length === 2 && q.every(v => typeof v === "string");
  const full = q => `{${q[0]}}${q[1]}`;
  const el = (tag, text, className) => {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = text;
    if (className) node.className = className;
    return node;
  };
  const button = (text, action) => { const b = el("button", text); b.type = "button"; b.addEventListener("click", action); return b; };
  const link = (text, url) => { const a = el("a", text); a.href = url; return a; };
  let data, types, messages, state, selected, rootNode, total = 0;
  const nodes = new Map();
  const decimal = v => typeof v === "string" && /^(0|[1-9][0-9]*)$/.test(v);
  const validCount = c => c && decimal(c.min) && (c.max === null || decimal(c.max));
  const refValid = r => r && (r.named ? validQ(r.named) && types.has(key(r.named)) : typeof r.primitive === "string");
  const refType = r => r && r.named ? types.get(key(r.named)) : null;
  const refLabel = r => r ? (r.named ? r.named[1] : r.primitive) : "—";
  const pathKey = path => JSON.stringify(path);
  const fields = (value, names) => value !== null && typeof value === "object" && !Array.isArray(value) && Object.keys(value).length === names.length && names.every(name => Object.hasOwn(value, name));
  function normalizeState(s) {
    if (!fields(s, ["v", "kind", "root", "path"]) || s.v !== 1 || !["message", "type"].includes(s.kind) || !validQ(s.root) || !Array.isArray(s.path) || s.path.length > LIMIT.depth) throw Error("Invalid explorer state.");
    const path = s.path.map(step => {
      if (fields(step, ["owner", "compositor"]) && validQ(step.owner) && ["Record", "Choice"].includes(step.compositor)) return { owner: [...step.owner], compositor: step.compositor };
      if (fields(step, ["member"]) && Number.isSafeInteger(step.member) && step.member >= 0) return { member: step.member };
      if (fields(step, ["step"]) && ["alias", "item"].includes(step.step)) return { step: step.step };
      throw Error("Invalid explorer path step.");
    });
    return { v: 1, kind: s.kind, root: [...s.root], path };
  }
  function resetExploration() {
    const removedFocus = $("rows").contains(document.activeElement) || $("detail-content").contains(document.activeElement);
    state = selected = rootNode = null; nodes.clear(); total = 0;
    $("rows").replaceChildren(); $("root-title").textContent = "Structure";
    $("detail-content").textContent = "Select a root or field."; $("share").value = "";
    status("Choose a message or type to begin.");
    if (removedFocus) $("root-search").focus();
  }
  function validateData() {
    data = window.schemaExplorerData;
    if (!data || data.version !== 1 || !data.schema || !Array.isArray(data.schema.roots) || !Array.isArray(data.types) || !Array.isArray(data.messages)) throw Error("Missing or invalid explorer data (version 1 required).");
    types = new Map(); messages = new Map();
    for (const [records, registry] of [[data.types, types], [data.messages, messages]]) {
      for (const r of records) {
        if (!validQ(r.qname) || registry.has(key(r.qname))) throw Error("Invalid or duplicate declaration identity.");
        registry.set(key(r.qname), r);
      }
    }
    for (const t of data.types) {
      if (!/^types\/[0-9]{6}\.html$/.test(t.url) || !Array.isArray(t.members) || !Array.isArray(t.segments) || !Array.isArray(t.ancestry) || !Array.isArray(t.enumeration) || !t.constraints || !Array.isArray(t.constraints.facets) || !Array.isArray(t.constraints.patternGroups)) throw Error("Invalid type data.");
      for (const r of [t.base, t.target, t.item].filter(Boolean)) if (!refValid(r)) throw Error("Unresolved type reference.");
      for (const s of t.segments) if (!types.has(key(s.owner)) || !["Record", "Choice"].includes(s.compositor)) throw Error("Invalid structural segment.");
      for (const [i, m] of t.members.entries()) if (m.index !== i || typeof m.name !== "string" || !(m.namespace === null || typeof m.namespace === "string") || !refValid(m.type) || !validCount(m.count)) throw Error("Invalid member data.");
      if (t.item && !validCount(t.itemCount)) throw Error("Invalid list count.");
    }
    for (const m of data.messages) if (!refValid(m.type) || !/^index\.html#message-[0-9]{6}$/.test(m.url)) throw Error("Invalid message data.");
  }
  function search() {
    const kind = $("root-filter").value;
    const query = $("root-search").value.trim().toLowerCase();
    const records = kind === "message" ? data.messages : data.types;
    const matches = records.filter(r => (full(r.qname) + " " + (r.documentation || "")).toLowerCase().includes(query));
    $("result-count").textContent = `${matches.length} results; showing first ${LIMIT.results}. Refine search for more.`;
    $("root-results").replaceChildren();
    for (const r of matches.slice(0, LIMIT.results)) {
      const li = el("li"); const b = button(r.qname[1], () => navigate({ v: 1, kind, root: r.qname, path: [] }));
      b.title = full(r.qname); li.append(b); $("root-results").append(li);
    }
  }
  function encoded(s) { return "#uci=" + encodeURIComponent(JSON.stringify(s)); }
  function navigate(s) { location.hash = encoded(s); }
  function syncSelection(node, history = true) {
    if (selected) selected.li.classList.remove("selected");
    selected = node; node.li.classList.add("selected");
    state = { ...state, path: node.path };
    if (history && location.hash !== encoded(state)) window.history.pushState(null, "", encoded(state));
    $("share").value = `${full(state.root)} / ${node.path.map(p => JSON.stringify(p)).join(" / ")}\n${location.href.split("#")[0]}${encoded(state)}`;
    details(node);
  }
  function boundedList(parent, label, values, render) {
    if (!values.length) return;
    parent.append(el("h3", label)); const list = el("ul"); parent.append(list); let offset = 0;
    const more = button("Show next 50", add);
    function add() {
      for (const value of values.slice(offset, offset + LIMIT.details)) { const li = el("li"); render(li, value); list.append(li); }
      offset += LIMIT.details;
      more.textContent = `Show next ${LIMIT.details} (${Math.max(0, values.length - offset)} remaining)`;
      if (offset >= values.length) more.remove();
    }
    parent.append(more); add();
  }
  function details(node) {
    const panel = $("detail-content"); panel.replaceChildren();
    const m = node.member, t = node.type;
    panel.append(el("h3", node.label));
    if (node.owner) panel.append(el("p", "Declaring owner: " + full(node.owner.qname)));
    if (node.group) panel.append(link("Open declaring type reference page", node.owner.url));
    if (m) {
      panel.append(el("p", `Wire member: ${m.namespace === null ? "absent namespace (unqualified) " + m.name : full([m.namespace, m.name])}`));
      panel.append(el("p", `Local cardinality: ${m.count.min}..${m.count.max === null ? "*" : m.count.max}; nillable: ${m.nillable}`));
    }
    if (node.optionalAncestor) panel.append(el("p", "Conditional presence: an ancestor is Optional. A locally required member is required only when that ancestor is present; ranges are not multiplied."));
    if (node.choice) panel.append(el("p", "Choice alternative: local occurrence is distinct from selecting an alternative. No omitted group occurrence constraints are inferred."));
    if (node.recursive) panel.append(el("p", "Recursive reference on this ancestor path. Explore as root to continue independently."));
    const doc = m ? m.documentation : (node.message ? node.message.documentation : t && t.documentation);
    if (doc) panel.append(el("p", doc, "documentation"));
    if (t) {
      panel.append(el("p", `Type QName: ${full(t.qname)} · ${t.kind}${t.abstract ? " · ABSTRACT" : ""}`));
      if (t.base) panel.append(el("p", "Immediate base: " + (t.base.named ? full(t.base.named) : t.base.primitive)));
      if (t.target) panel.append(el("p", "Alias target: " + (t.target.named ? full(t.target.named) : t.target.primitive)));
      panel.append(link("Open type reference page", t.url), button("Explore as root", () => navigate({ v: 1, kind: "type", root: t.qname, path: [] })));
      boundedList(panel, "Enumeration wire values", t.enumeration, (li, v) => li.append(el("code", v.value), el("p", v.documentation || "")));
      const derived = data.types.filter(d => d.base && d.base.named && key(d.base.named) === key(t.qname));
      boundedList(panel, "Known derived types in this schema", derived, (li, d) => li.append(button(d.qname[1], () => navigate({ v: 1, kind: "type", root: d.qname, path: [] }))));
    } else if (node.ref) {
      panel.append(el("p", "Primitive type: " + refLabel(node.ref)));
      if (node.owner) panel.append(link("Open declaring type reference page", node.owner.url));
    }
    if (node.message) panel.append(link("Open message reference", node.message.url));
    for (const [label, c] of [["Member constraints", m && m.constraints], ["Type constraints", t && t.constraints]]) {
      if (!c) continue;
      boundedList(panel, label, c.facets, (li, f) => { li.textContent = `${f.name}: ${f.value}`; });
      boundedList(panel, label + " pattern groups (AND between groups, OR within each)", c.patternGroups, (li, group) => boundedList(li, "One of", group, (item, p) => { item.textContent = `${p.expression} (${p.dialect})`; }));
    }
  }
  function row(parent, spec) {
    if (total >= LIMIT.rows) throw Error(`Materialized row limit ${LIMIT.rows} reached. Collapse exploration or explore a type as root.`);
    total++;
    const li = el("li"), line = el("div", undefined, "row"), children = el("ul"); children.hidden = true;
    const node = { ...spec, li, children, open: false, loaded: false };
    node.recursive = !!(node.type && node.ancestors.includes(key(node.type.qname)));
    const expandable = !node.recursive && (node.group || (node.type && ["Record", "Choice", "Alias", "List"].includes(node.type.kind)));
    const toggle = button(expandable ? "+" : "·", () => { try { disclose(node, !node.open); } catch (e) { status(e.message); } });
    toggle.disabled = !expandable; toggle.setAttribute("aria-label", `Expand ${spec.label}`); toggle.setAttribute("aria-expanded", "false"); node.toggle = toggle;
    const select = button(spec.label, () => syncSelection(node)); select.className = "select"; node.select = select;
    const type = el("span", node.recursive ? `${refLabel(node.ref)} — Recursive reference` : node.type ? node.type.qname[1] : refLabel(node.ref));
    if (node.type) type.title = full(node.type.qname);
    const count = spec.member ? spec.member.count : spec.count;
    const badges = el("span");
    if (count) {
      if (count.min === "0") badges.append(el("span", "Optional", "badge optional"));
      if (count.max === null || count.max.length > 1 || (count.max !== "0" && count.max !== "1")) badges.append(el("span", "Repeated", "badge repeated"));
      if (count.max === "0") badges.append(el("span", "Not occurring", "badge"));
    }
    if (spec.member && spec.member.nillable) badges.append(el("span", "Nillable", "badge"));
    if (spec.choice) badges.append(el("span", "Alternative", "badge"));
    if (node.type && node.type.abstract) badges.append(el("span", "Abstract", "badge"));
    if (node.recursive) badges.append(button("Explore as root", () => navigate({ v: 1, kind: "type", root: node.type.qname, path: [] })));
    line.append(toggle, select, type, el("span", count ? `${count.min}..${count.max === null ? "*" : count.max}` : "—"), badges);
    li.append(line, children); parent.append(li); nodes.set(pathKey(node.path), node); return node;
  }
  function specs(node) {
    const t = node.type;
    const ancestors = node.group ? node.ancestors : [...node.ancestors, key(t.qname)];
    const common = { ancestors, optionalAncestor: node.optionalAncestor || !!(node.member && node.member.count.min === "0") };
    if (node.group) return node.owner.members.map(m => ({ ...common, label: m.name, member: m, owner: node.owner, ref: m.type, type: refType(m.type), choice: node.compositor === "Choice", path: [...node.path, { member: m.index }] }));
    if (t.kind === "Record" || t.kind === "Choice") return t.segments.map(s => {
      const owner = types.get(key(s.owner));
      return { ...common, group: true, owner, compositor: s.compositor, label: `${s.compositor === "Choice" ? "Choice / Alternatives" : "Record"} — ${key(owner.qname) === key(t.qname) ? "Declared by" : "Inherited from"} ${owner.qname[1]}`, path: [...node.path, { owner: s.owner, compositor: s.compositor }] };
    });
    const r = t.kind === "Alias" ? t.target : t.item;
    return r ? [{ ...common, label: t.kind === "Alias" ? "Alias target" : "Structure of one list item", ref: r, type: refType(r), count: t.itemCount, path: [...node.path, { step: t.kind === "Alias" ? "alias" : "item" }] }] : [];
  }
  function disclose(node, open) {
    if (node.toggle.disabled) return;
    if (open && !node.loaded) {
      if (node.path.length >= LIMIT.depth) throw Error(`Expansion depth limit ${LIMIT.depth} reached. Explore as root to continue.`);
      const children = specs(node);
      if (total + children.length > LIMIT.rows) throw Error(`Materialized row limit ${LIMIT.rows} reached. Collapse exploration or explore as root.`);
      for (const child of children) row(node.children, child);
      node.loaded = true;
    }
    if (!open && selected && selected !== node && node.li.contains(selected.li)) { syncSelection(node); node.toggle.focus(); }
    node.open = open; node.children.hidden = !open;
    node.toggle.textContent = open ? "−" : "+"; node.toggle.setAttribute("aria-expanded", String(open));
    node.toggle.setAttribute("aria-label", `${open ? "Collapse" : "Expand"} ${node.label}`);
  }
  function restore() {
    try {
      if (!location.hash) { resetExploration(); return; }
      if (!location.hash.startsWith("#uci=") || location.hash.length > LIMIT.fragment) throw Error("Malformed or oversized explorer state.");
      const s = normalizeState(JSON.parse(decodeURIComponent(location.hash.slice(5))));
      const record = (s.kind === "message" ? messages : types).get(key(s.root));
      if (!record) throw Error("Unknown or stale root identity.");
      // Validate the entire path without changing the displayed exploration.
      const ref = s.kind === "message" ? record.type : { named: record.qname };
      let probe = { type: refType(ref), ancestors: [], path: [] };
      for (const step of s.path) {
        if (!probe.type && !probe.group || probe.type && probe.ancestors.includes(key(probe.type.qname))) throw Error("Path crosses a recursive or terminal reference.");
        const child = specs(probe).find(c => pathKey(c.path.at(-1)) === pathKey(step));
        if (!child) throw Error("Unknown or stale structural path; no substitute selected.");
        probe = child;
      }
      state = { ...s, path: [] }; nodes.clear(); total = 0; selected = null; $("rows").replaceChildren();
      $("root-title").textContent = `${s.kind === "message" ? "Message" : "Type"}: ${record.qname[1]}`;
      rootNode = row($("rows"), { label: record.qname[1], type: refType(ref), ref, message: s.kind === "message" ? record : null, path: [], ancestors: [] });
      let current = rootNode;
      for (const step of s.path) { disclose(current, true); current = nodes.get(pathKey([...current.path, step])); }
      syncSelection(current, false); status("Exploration restored. Local occurrence ranges are not runtime validity advice.");
    } catch (e) { status("Explorer state error: " + e.message); }
  }
  try {
    validateData();
    $("identity").textContent = `Loaded schema: ${data.schema.roots.join(", ")} · version ${data.schema.version || "not specified"} · ${data.types.length} types · ${data.messages.length} messages`;
    $("root-search").addEventListener("input", search); $("root-filter").addEventListener("change", search);
    $("collapse").addEventListener("click", () => {
      if (!rootNode) return;
      const next = { ...state, path: [] };
      if (location.hash !== encoded(next)) window.history.pushState(null, "", encoded(next));
      restore(); rootNode.toggle.focus();
    });
    $("copy").addEventListener("click", async () => {
      if (!selected) return;
      $("share").focus(); $("share").select();
      try { await navigator.clipboard.writeText($("share").value); status("Selected path and exploration link copied."); }
      catch (_) { status("Clipboard unavailable. Copy the selected text in the link fallback."); }
    });
    window.addEventListener("hashchange", restore); window.addEventListener("popstate", restore);
    search(); restore(); document.body.dataset.ready = "true";
  } catch (e) { status("Explorer data error: " + e.message + " Use the Reference browser link above."); document.body.dataset.ready = "error"; }
})();