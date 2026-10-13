"use strict";
const { test } = require("node:test");
const assert = require("node:assert/strict");
const search = require("../../crates/schema-docs/src/explorer-search.js");
const q = name => ["urn:test", name];
const named = name => ({ named: q(name) });
const member = (name, target, count = { min: "1", max: "1" }) => ({ name, type: target ? named(target) : { primitive: "String" }, count, documentation: "Only field prose <img src=x>", nillable: false });
function record(name, members, owners = [name], kind = "Record") {
  return { qname: q(name), kind, members: members.map((m, index) => ({ ...m, index })), segments: owners.map(owner => ({ owner: q(owner), compositor: kind })), documentation: "unrelated type prose" };
}
const graph = () => ({ messages: [{ qname: q("Report"), type: named("Root") }], types: [
  record("Root", [member("long", "Alias"), member("short", "Leaf"), member("same", "Leaf"), member("choice", "Pick"), member("list", "Items")], ["Base", "Root"]),
  record("Base", [member("same")]),
  record("Leaf", [member("needle", null, { min: "0", max: "18446744073709551615" }), member("self", "Leaf"), member("zero", null, { min: "0", max: "0" }), member("mutual", "Root")]),
  record("Pick", [member("alternative", "Leaf")], ["Pick"], "Choice"),
  { ...record("Alias", []), kind: "Alias", segments: [], target: named("Leaf") },
  { ...record("Items", []), kind: "List", segments: [], item: named("Alias") },
  record("Unreachable", [member("needle")]), record("Descendant", [member("hidden")], ["Root", "Descendant"])
] });
const root = { kind: "message", root: q("Report") };
const instant = { schedule: async () => {} };
test("collapsed graph, cycles, owners, shortest witness and stable declaration permutations", async () => {
  const g = graph(), index = await search.create(g).index(root, instant);
  assert.equal(index.status, "complete");
  assert.equal(index.entries.length, 11);
  assert.equal(index.entries.filter(e => e.member.name === "same").length, 2);
  const needle = index.entries.find(e => e.member.name === "needle");
  assert.deepEqual(search.path(needle.witness), [{ owner: q("Root"), compositor: "Record" }, { member: 1 }, { owner: q("Leaf"), compositor: "Record" }, { member: 0 }]);
  assert(!index.entries.some(e => ["Unreachable", "Descendant"].includes(e.owner.qname[1])));
  const other = await search.create({ ...g, types: [...g.types].reverse().map(t => ({ ...t, segments: [...t.segments].reverse() })) }).index(root, instant);
  assert.deepEqual(other.entries.map(e => [e.identity, search.path(e.witness)]), index.entries.map(e => [e.identity, search.path(e.witness)]));
});
test("literal modes, exact cardinalities and independent field documentation", async () => {
  const index = await search.create(graph()).index(root, instant);
  const query = input => search.query(index, input, instant);
  assert.equal((await query({ text: "NEEDLE", mode: "name" })).matches.length, 1);
  assert.equal((await query({ text: "Alias", mode: "type" })).matches.length, 1);
  assert.equal((await query({ text: "unrelated type prose", mode: "documentation" })).matches.length, 0);
  assert.equal((await query({ text: "<img", mode: "documentation" })).matches.length, 11);
  assert.equal((await query({ text: ".*" })).matches.length, 0);
  assert.deepEqual((await query({ optional: true, repeated: true })).matches.map(m => m.entry.member.name), ["needle"]);
  assert.equal((await query({ optional: true })).matches.length, 2);
  assert.equal((await query({ text: "x".repeat(257) })).status, "invalid");
});
test("alias and list preserve structural steps", async () => {
  for (const [name, steps] of [["Alias", ["alias"]], ["Items", ["item", "alias"]]]) {
    const index = await search.create(graph()).index({ kind: "type", root: q(name) }, instant);
    const needle = index.entries.find(e => e.member.name === "needle");
    assert.deepEqual(search.path(needle.witness).slice(0, steps.length), steps.map(step => ({ step })));
  }
});
test("bounded work, genuine multiple batches and cancellation discard entries", async () => {
  const engine = search.create(graph());
  assert.equal((await engine.index(root, { ...instant, work: 3 })).status, "limited");
  let batches = 0;
  const result = await engine.index(root, { batch: 1, cancelled: () => batches >= 3, schedule: async () => { batches++; } });
  assert.equal(batches, 3); assert.equal(result.status, "cancelled"); assert.deepEqual(result.entries, []);
  const index = await engine.index(root, instant); batches = 0;
  const filtered = await search.query(index, {}, { batch: 1, cancelled: () => batches >= 2, schedule: async () => { batches++; } });
  assert.equal(filtered.status, "cancelled"); assert.deepEqual(filtered.matches, []);
});
// Exhaustive simple-path oracle on this finite fixture: independent recursive
// enumeration, deliberately not declaration-deduplicating during traversal.
function oracle(g, rootName) {
  const types = new Map(g.types.map(t => [JSON.stringify(t.qname), t])), witnesses = new Map();
  function walk(qname, path, ancestors) {
    const identity = JSON.stringify(qname);
    if (ancestors.includes(identity)) return;
    const t = types.get(identity), nextAncestors = [...ancestors, identity];
    for (const s of t.segments) {
      const owner = types.get(JSON.stringify(s.owner));
      for (const m of owner.members) {
        const p = [...path, { owner: s.owner, compositor: s.compositor }, { member: m.index }], id = JSON.stringify([owner.qname, s.compositor, m.index]);
        const previous = witnesses.get(id);
        // Independent comparator for the fixture's segment/member/edge syntax.
        const ordering = value => value.owner ? [0, ...value.owner, value.compositor] : value.member !== undefined ? [1, value.member] : [2, value.step];
        function earlier(a, b) {
          if (a.length !== b.length) return a.length < b.length;
          for (let i = 0; i < a.length; i++) {
            const x = ordering(a[i]), y = ordering(b[i]);
            for (let j = 0; j < x.length; j++) if (x[j] !== y[j]) return x[j] < y[j];
          }
          return false;
        }
        if (!previous || earlier(p, previous)) witnesses.set(id, p);
        if (m.type.named) walk(m.type.named, p, nextAncestors);
      }
    }
    const ref = t.target || t.item;
    if (ref?.named) walk(ref.named, [...path, { step: t.target ? "alias" : "item" }], nextAncestors);
  }
  walk(q(rootName), [], []); return witnesses;
}
test("all representative paths agree with independent exhaustive simple-path oracle", async () => {
  const g = graph();
  for (const name of ["Root", "Leaf", "Items", "Pick"]) {
    const expected = oracle(g, name), actual = await search.create(g).index({ kind: "type", root: q(name) }, instant);
    assert.equal(actual.entries.length, expected.size);
    for (const entry of actual.entries) assert.deepEqual(search.path(entry.witness), expected.get(entry.identity));
  }
});
test("full namespaces and numeric indices distinguish equal names; segment ties are ordered", async () => {
  const g = graph(), other = record("Leaf", [member("needle")]);
  other.qname = ["urn:other", "Leaf"]; other.segments = [{ owner: other.qname, compositor: "Record" }];
  g.types.push(other);
  const r = g.types.find(t => t.qname[1] === "Root");
  r.members.push({ ...member("same"), index: 5, type: { named: other.qname } });
  r.members.push({ ...member("same"), index: 6 });
  const index = await search.create(g).index(root, instant);
  assert.equal(index.entries.filter(e => e.member.name === "needle").length, 2);
  assert.equal(index.entries.filter(e => e.owner.qname[1] === "Root" && e.member.name === "same").length, 3);
  const expected = oracle(g, "Root");
  for (const entry of index.entries) assert.deepEqual(search.path(entry.witness), expected.get(entry.identity));
});