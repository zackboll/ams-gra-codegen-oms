"use strict";
// Task-local measurements and independent navigation witnesses for real graphs.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const search = require("../../crates/schema-docs/src/explorer-search.js");
const key = JSON.stringify;
async function main() {
  const directory = path.resolve(process.argv[2]);
  for (const release of ["2.5", "2.6"]) {
    const bytes = fs.readFileSync(path.join(directory, release, "assets/explorer-data.js"), "utf8"), g = JSON.parse(bytes.slice("window.schemaExplorerData = ".length, -2));
    const types = new Map(g.types.map(t => [key(t.qname), t])), engine = search.create(g);
    console.log(release, "graph-bytes", Buffer.byteLength(bytes), "types", g.types.length, "messages", g.messages.length, "declared-members", g.types.reduce((n, t) => n + t.members.length, 0));
    for (const [kind, name] of [["message", "PositionReport"], ["message", "OrderOfBattle"], ["type", "EntityPositionType"]]) {
      const root = g[kind === "message" ? "messages" : "types"].find(t => t.qname[1] === name), ref = kind === "message" ? root.type : { named: root.qname };
      // Independent stack-based declaration closure, not the production BFS.
      const pending = [ref.named], seen = new Set(), expected = new Set();
      while (pending.length) {
        const q = pending.pop(); if (!q || seen.has(key(q))) continue;
        seen.add(key(q)); const t = types.get(key(q));
        for (const s of t.segments) for (const m of types.get(key(s.owner)).members) {
          expected.add(key([s.owner, s.compositor, m.index])); if (m.type.named) pending.push(m.type.named);
        }
        for (const r of [t.target, t.item]) if (r?.named) pending.push(r.named);
      }
      const start = performance.now(), index = await engine.index({ kind, root: root.qname });
      const indexMs = performance.now() - start;
      assert.equal(index.status, "complete"); assert.deepEqual(new Set(index.entries.map(e => e.identity)), expected);
      let maxSteps = 0, beyondDepth = 0, beyondRows = 0;
      for (const entry of index.entries) {
        let current = types.get(key(ref.named)), owner = null, member = null, rows = 1;
        const ancestors = new Set(), steps = search.path(entry.witness); maxSteps = Math.max(maxSteps, steps.length);
        if (steps.length > 24) beyondDepth++;
        for (const step of steps) {
          if (owner) {
            rows += owner.members.length; assert.equal(typeof step.member, "number"); member = owner.members[step.member]; assert(member);
            current = member.type.named ? types.get(key(member.type.named)) : null; owner = null;
          } else {
            assert(current); assert(!ancestors.has(key(current.qname)), "witness must obey path-local recursion boundary"); ancestors.add(key(current.qname));
            if (step.owner) {
              rows += current.segments.length; assert(current.segments.some(s => key(s.owner) === key(step.owner) && s.compositor === step.compositor)); owner = types.get(key(step.owner));
            } else {
              rows++; assert.equal(step.step, current.kind === "Alias" ? "alias" : "item");
              const r = current.kind === "Alias" ? current.target : current.item; current = r.named ? types.get(key(r.named)) : null;
            }
          }
        }
        assert.equal(member, entry.member); if (rows > 2000) beyondRows++;
      }
      const queryStart = performance.now(), counts = {};
      for (const input of [{ text: "Position", mode: "name" }, { text: "Type", mode: "type" }, { text: "the", mode: "documentation" }, { optional: true, repeated: true }]) {
        const result = await search.query(index, input); counts[key(input)] = result.matches.length;
      }
      const fourQueriesMs = Math.round(performance.now() - queryStart);
      const again = await engine.index({ kind, root: root.qname });
      assert.deepEqual(again.entries.map(e => [e.identity, search.path(e.witness)]), index.entries.map(e => [e.identity, search.path(e.witness)]));
      console.log(JSON.stringify({ release, kind, name, reachableTypes: seen.size, definitions: index.entries.length, work: index.work, maxSteps, beyondDepth, beyondRows, indexMs: Math.round(indexMs), fourQueriesMs, counts }));
    }
  }
}
main().catch(e => { console.error(e); process.exitCode = 1; });