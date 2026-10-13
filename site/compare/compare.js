"use strict";
(() => {
  const $ = id => document.getElementById(id);
  const el = (tag, text) => { const n = document.createElement(tag); if (text !== undefined) n.textContent = text; return n; };
  const full = q => `{${q[0]}}${q[1]}`;
  const key = d => JSON.stringify([d.category, d.qname]);
  const data = window.schemaCompareData;
  const registry = new Map(data.declarations.map(d => [key(d), d]));
  let offset = 0, matches = [];
  function render() {
    const query = $("search").value.trim().toLowerCase();
    matches = data.declarations.filter(d => d.category === $("category").value &&
      (!$("filter").value || d.status === $("filter").value) && full(d.qname).toLowerCase().includes(query));
    $("results").replaceChildren();
    for (const d of matches.slice(offset, offset + 50)) {
      const li = el("li"), b = el("button", `${d.status}: ${full(d.qname)}`);
      b.type = "button"; b.className = d.status;
      b.addEventListener("click", () => { location.hash = "compare=" + encodeURIComponent(JSON.stringify({ v: 1, category: d.category, qname: d.qname })); });
      li.append(b); $("results").append(li);
    }
    $("previous").disabled = offset === 0; $("next").disabled = offset + 50 >= matches.length;
    $("status").textContent = `${matches.length} declarations; showing ${matches.length ? offset + 1 : 0}–${Math.min(offset + 50, matches.length)}.`;
  }
  function restore() {
    $("details").replaceChildren();
    if (!location.hash) { $("details").append(el("p", "Choose a declaration.")); return; }
    try {
      if (location.hash.length > 16384 || !location.hash.startsWith("#compare=")) throw Error("Malformed or oversized comparison fragment.");
      const s = JSON.parse(decodeURIComponent(location.hash.slice(9)));
      if (!s || Object.keys(s).sort().join(",") !== "category,qname,v" || s.v !== 1 ||
          !["Type", "Message"].includes(s.category) || !Array.isArray(s.qname) || s.qname.length !== 2 ||
          !s.qname.every(x => typeof x === "string" && x.length <= 4096)) throw Error("Invalid comparison identity.");
      const d = registry.get(key(s));
      if (!d) throw Error("Unknown or stale declaration identity; no substitute selected.");
      $("details").append(el("h2", `${d.category}: ${full(d.qname)}`), el("p", d.status));
      const actions = el("p");
      d.destinations.forEach((dest, i) => {
        if (!dest) return;
        for (const [label, url] of [["Open UCI", dest.reference], ["Explore in UCI", dest.explorer]]) {
          const a = el("a", `${label} ${data.releases[i].release}${label === "Open UCI" ? " reference" : ""}`); a.href = url; actions.append(a);
        }
      });
      $("details").append(actions);
      const label = el("label", "Share this comparison"), share = el("input");
      share.readOnly = true; share.className = "share"; share.value = location.href; label.append(share); $("details").append(label);
      if (!d.changes.length) $("details").append(el("p", "No direct normalized semantic property changes reported."));
      for (const c of d.changes) {
        const article = el("article");
        const split = c.property.lastIndexOf("/");
        if (split >= 0) article.append(el("p", `Member / enumeration value: ${c.property.slice(0, split)}`));
        article.append(el("h3", `Property: ${c.property.slice(split + 1)}`), el("p", `Task 069 property label: ${c.property}`), el("p", `Task 069 change: ${c.kind}`));
        const fieldActions = el("p");
        (c.memberDestinations || []).forEach((url, i) => {
          if (!url) return;
          const a = el("a", `Reveal field in UCI ${data.releases[i].release}`);
          a.href = url; fieldActions.append(a);
        });
        if (fieldActions.childElementCount) article.append(fieldActions);
        const table = el("table"), head = el("tr"), row = el("tr");
        for (const release of data.releases) { const th = el("th", `UCI ${release.release}`); th.scope = "col"; head.append(th); }
        for (const value of [c.before, c.after]) { const td = el("td"), pre = el("pre", value === null ? "Absent (missing value)" : value); pre.tabIndex = 0; td.append(pre); row.append(td); }
        const thead = el("thead"), tbody = el("tbody"); thead.append(head); tbody.append(row); table.append(thead, tbody); article.append(table); $("details").append(article);
      }
    } catch (e) { $("details").append(el("p", "Comparison state error: " + e.message)); }
  }
  $("summary").textContent = Object.entries(data.summaries).map(([cat, s]) => `${cat}s: ${s.added} Added · ${s.removed} Removed · ${s.changed} Changed · ${s.unchanged} Unchanged`).join(" | ");
  for (const id of ["category", "filter", "search"]) $(id).addEventListener(id === "search" ? "input" : "change", () => { offset = 0; render(); });
  for (const [id, step] of [["previous", -50], ["next", 50]]) $(id).addEventListener("click", () => { offset += step; render(); $("results").querySelector("button")?.focus(); });
  window.addEventListener("hashchange", restore); window.addEventListener("popstate", restore);
  render(); restore(); document.body.dataset.ready = "true";
})();