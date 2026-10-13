"use strict";
// Classic local asset, also loaded directly by Node tests. No DOM dependencies.
((host) => {
  const key = q => JSON.stringify(q);
  const full = q => `{${q[0]}}${q[1]}`;
  const compare = (a, b) => a < b ? -1 : a > b ? 1 : 0;
  const limits = Object.freeze({ query: 256, work: 100000, batch: 250, page: 50 });
  const pause = () => new Promise(resolve => setTimeout(resolve, 0));
  const repeated = c => c.max === null || c.max.length > 1 || (c.max !== "0" && c.max !== "1");
  function path(witness) {
    const steps = [];
    while (witness && witness.step) { steps.push(witness.step); witness = witness.parent; }
    return steps.reverse();
  }
  // Step order: segment (namespace, local name, compositor), member (numeric
  // index), reference (alias before item). UTF-16 lexical, never locale order.
  function stepCompare(a, b) {
    const tag = s => s.owner ? 0 : s.member !== undefined ? 1 : 2;
    const order = tag(a) - tag(b);
    if (order) return order;
    if (a.owner) return compare(a.owner[0], b.owner[0]) || compare(a.owner[1], b.owner[1]) || compare(a.compositor, b.compositor);
    return a.member !== undefined ? a.member - b.member : compare(a.step, b.step);
  }
  function create(graph) {
    const types = new Map(graph.types.map(t => [key(t.qname), t]));
    const messages = new Map(graph.messages.map(m => [key(m.qname), m]));
    const segments = new Map(graph.types.map(t => [key(t.qname), [...t.segments].sort(stepCompare)]));
    async function index(root, options = {}) {
      const cap = options.work ?? limits.work, batch = options.batch ?? limits.batch;
      if (!Number.isSafeInteger(cap) || cap < 1 || !Number.isSafeInteger(batch) || batch < 1) throw Error("Invalid work limits.");
      const cancelled = options.cancelled || (() => false), schedule = options.schedule || pause;
      const record = (root.kind === "message" ? messages : types).get(key(root.root));
      if (!record) return { status: "invalid", entries: [], work: 0 };
      const reference = root.kind === "message" ? record.type : { named: record.qname };
      const queue = [{ ref: reference, parent: null }], entries = [], seenTypes = new Set(), seenGroups = new Set(), seenFields = new Set();
      let offset = 0, limited = false, edgeWork = 0;
      async function yieldEdges() {
        if (++edgeWork % batch === 0) await schedule();
        return cancelled();
      }
      function enqueue(parent, step, spec) {
        if (queue.length >= cap) { limited = true; return; }
        queue.push({ ...spec, parent, step });
      }
      while (offset < queue.length) {
        if (cancelled()) return { status: "cancelled", entries: [], work: offset };
        const end = Math.min(queue.length, offset + batch);
        for (; offset < end; offset++) {
          const node = queue[offset];
          if (node.member) {
            const identity = JSON.stringify([node.owner.qname, node.compositor, node.member.index]);
            if (!seenFields.has(identity)) {
              seenFields.add(identity); entries.push({ identity, owner: node.owner, compositor: node.compositor, member: node.member, witness: node });
            }
          }
          if (node.group) {
            const identity = JSON.stringify([node.owner.qname, node.compositor]);
            if (seenGroups.has(identity)) continue;
            seenGroups.add(identity);
            // Valid exported members are in numeric occurrence-index order.
            for (const member of node.owner.members) {
              if (queue.length >= cap) { limited = true; break; }
              enqueue(node, { member: member.index }, { member, owner: node.owner, compositor: node.compositor, ref: member.type });
              if (await yieldEdges()) return { status: "cancelled", entries: [], work: offset };
            }
          } else if (node.ref?.named) {
            const identity = key(node.ref.named), t = types.get(identity);
            if (!t || seenTypes.has(identity)) continue;
            seenTypes.add(identity);
            if (t.kind === "Record" || t.kind === "Choice") {
              for (const segment of segments.get(identity)) {
                if (queue.length >= cap) { limited = true; break; }
                enqueue(node, { owner: segment.owner, compositor: segment.compositor }, { group: true, owner: types.get(key(segment.owner)), compositor: segment.compositor });
                if (await yieldEdges()) return { status: "cancelled", entries: [], work: offset };
              }
            } else {
              const ref = t.kind === "Alias" ? t.target : t.item;
              if (ref) enqueue(node, { step: t.kind === "Alias" ? "alias" : "item" }, { ref });
            }
          }
        }
        options.progress?.(offset);
        // A real browser task boundary, including between growing BFS levels.
        await schedule();
      }
      return cancelled() ? { status: "cancelled", entries: [], work: offset } : { status: limited ? "limited" : "complete", entries, work: offset };
    }
    return { index };
  }
  async function query(index, input, options = {}) {
    const text = input.text || "", mode = input.mode || "all";
    if (text.length > limits.query || !["all", "name", "type", "documentation"].includes(mode) || !["complete", "limited"].includes(index.status)) return { status: "invalid", matches: [] };
    const needle = text.trim().toLowerCase(), matches = [], batch = options.batch ?? limits.batch;
    if (!Number.isSafeInteger(batch) || batch < 1) throw Error("Invalid batch limit.");
    const cancelled = options.cancelled || (() => false), schedule = options.schedule || pause;
    for (let offset = 0; offset < index.entries.length; offset += batch) {
      if (cancelled()) return { status: "cancelled", matches: [] };
      for (const entry of index.entries.slice(offset, offset + batch)) {
        const m = entry.member;
        if (input.optional && m.count.min !== "0" || input.repeated && !repeated(m.count)) continue;
        const values = { name: m.name, type: m.type.named ? full(m.type.named) : m.type.primitive, documentation: m.documentation || "" };
        const reasons = Object.keys(values).filter(k => (mode === "all" || mode === k) && values[k].toLowerCase().includes(needle));
        if (reasons.length) matches.push({ entry, reasons });
      }
      await schedule();
    }
    return cancelled() ? { status: "cancelled", matches: [] } : { status: index.status, matches };
  }
  const api = { create, query, path, stepCompare, repeated, limits };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else host.schemaExplorerSearch = api;
})(globalThis);