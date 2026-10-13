"use strict";
// Task 081 opt-in browser matrix; reuses the existing locked Playwright install.
const { chromium, firefox } = require("playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const http = require("node:http");
const { pathToFileURL } = require("node:url");
const root = path.resolve(process.env.TASK081_ARTIFACT || "target/task081-fixture");
const evidence = path.resolve(process.env.TASK081_EVIDENCE || "target/task081-evidence");
const releases = process.env.TASK081_RELEASES ? ["2.5", "2.6"] : [""];
const prefix = "/ams-gra-codegen-oms/";
const server = http.createServer((req, res) => {
  const target = path.resolve(root, decodeURIComponent(req.url.split("?")[0].slice(prefix.length)));
  if (!req.url.startsWith(prefix) || !target.startsWith(root + path.sep)) { res.writeHead(404).end(); return; }
  fs.readFile(target, (error, bytes) => {
    if (error) { res.writeHead(404).end(); return; }
    res.setHeader("Content-Type", target.endsWith(".js") ? "text/javascript" : target.endsWith(".css") ? "text/css" : "text/html"); res.end(bytes);
  });
});
async function complete(page) {
  await page.waitForFunction(() => ["complete", "limited"].includes(document.querySelector("#field-status").dataset.phase), null, { timeout: 30000 });
}
// Independent closure over declarations and exported segments. This oracle does
// not call the production traversal and is not used to construct witnesses.
function reachable(graph, kind, qname) {
  const key = JSON.stringify, types = new Map(graph.types.map(t => [key(t.qname), t]));
  const visited = new Set(), fields = new Set(), pending = [kind === "type" ? qname : graph.messages.find(m => key(m.qname) === key(qname)).type.named];
  while (pending.length) {
    const q = pending.pop(); if (!q || visited.has(key(q))) continue;
    visited.add(key(q)); const t = types.get(key(q));
    for (const segment of t.segments) for (const m of types.get(key(segment.owner)).members) {
      fields.add(key([segment.owner, segment.compositor, m.index])); if (m.type.named) pending.push(m.type.named);
    }
    for (const ref of [t.target, t.item]) if (ref?.named) pending.push(ref.named);
  }
  return fields;
}
async function exercise(page, url, label, release) {
  await page.goto(url); await page.locator('body[data-ready="true"]').waitFor();
  const graph = await page.evaluate(() => window.schemaExplorerData);
  for (const [kind, name] of release ? [["message", "PositionReport"], ["message", "OrderOfBattle"], ["type", "EntityPositionType"]] : [["message", "Report"]]) {
    const record = graph[kind === "message" ? "messages" : "types"].find(t => t.qname[1] === name);
    if (!record) throw Error(`Required root missing: ${name}`);
    await page.locator("#root-filter").selectOption(kind); await page.locator("#root-search").fill(name);
    const started = performance.now();
    await page.locator("#root-results button").filter({ hasText: new RegExp(`^${name}$`) }).click(); await complete(page);
    const expected = reachable(graph, kind, record.qname);
    assert.match(await page.locator("#field-status").innerText(), new RegExp(`Complete search: ${expected.size} matching`));
    assert.equal(await page.locator("#rows li").count(), 1, "search must include collapsed structure");
    assert(await page.evaluate(() => window.task081Batches > 1), "qualification must force multiple scheduled batches");
    assert(await page.locator("#field-results li").count() <= 50);
    console.log(label, name, "definitions", expected.size, "index+query-ms", Math.round(performance.now() - started), "visible", await page.locator("#field-results li").count());
    if (expected.size > 50) {
      await page.locator("#field-more").click();
      assert(await page.locator("#field-results li").count() <= 50);
    }
    const history = await page.evaluate(() => window.history.length);
    for (const [mode, query] of release ? [["name", "SecurityInformation"], ["type", "Type"], ["documentation", "the"]] : [["name", "constructor"], ["type", "Child"], ["documentation", "<img"]]) {
      await page.locator("#field-mode").selectOption(mode); await page.locator("#field-query").fill(query); await complete(page);
      assert.match(await page.locator("#field-status").innerText(), /Complete search:/);
      if (!release) assert(await page.locator("#field-results li").count() > 0);
      if (!release && mode === "name") {
        assert.equal(await page.locator("#field-results li").count(), 2, "one definition per namespace, not one per repeated reference edge");
        assert.match(await page.locator("#field-results").innerText(), /urn:other/);
        assert.match(await page.locator("#field-results").innerText(), /Representative path:/);
      }
      assert.equal(await page.evaluate(() => window.history.length), history);
    }
    await page.locator("#field-clear").click(); await complete(page);
    if (!release) {
      await page.locator("#field-mode").selectOption("name"); await page.locator("#field-query").fill("Inherited"); await complete(page);
      assert.equal(await page.locator("#field-results li").count(), 2);
      assert.match(await page.locator("#field-results").innerText(), /Declaring owner: \{urn:explorer\}Base/);
      assert.match(await page.locator("#field-results").innerText(), /Declaring owner: \{urn:explorer\}Root/);
      await page.locator("#field-clear").click(); await complete(page);
    }
    await page.locator("#field-optional").check(); await page.locator("#field-repeated").check(); await complete(page);
    for (const text of await page.locator("#field-results li").allTextContents()) { assert.match(text, /Optional/); assert.match(text, /Repeated/); }
    console.log(label, name, "optional+repeated", await page.locator("#field-status").innerText());
    await page.locator("#field-clear").click(); await page.locator("#field-mode").selectOption("name");
    await page.locator("#field-query").fill(release ? "SecurityInformation" : "constructor"); await complete(page);
    if (await page.getByRole("button", { name: "Reveal field", exact: true }).count()) {
      const before = await page.evaluate(() => window.history.length);
      await page.getByRole("button", { name: "Reveal field", exact: true }).first().focus(); await page.keyboard.press("Enter");
      assert.equal(await page.locator(".selected > .row > .select").innerText(), release ? "SecurityInformation" : "constructor");
      assert.equal(await page.evaluate(() => window.history.length), before + 1);
      assert.match(await page.locator("#detail-content").innerText(), /Declaring owner:/);
      const selectedIdentity = await page.evaluate(() => {
        const s = JSON.parse(decodeURIComponent(location.hash.slice(5))), segment = s.path.at(-2), occurrence = s.path.at(-1);
        const owner = window.schemaExplorerData.types.find(t => JSON.stringify(t.qname) === JSON.stringify(segment.owner));
        const member = owner.members[occurrence.member];
        const target = member.type.named ? window.schemaExplorerData.types.find(t => JSON.stringify(t.qname) === JSON.stringify(member.type.named)) : owner;
        return { owner: `{${owner.qname[0]}}${owner.qname[1]}`, reference: target.url };
      });
      assert((await page.locator("#detail-content").innerText()).includes(selectedIdentity.owner));
      assert.equal(await page.locator("#detail-content a").first().getAttribute("href"), selectedIdentity.reference);
      assert.equal(await page.evaluate(() => document.activeElement.className), "select");
      await page.locator("#copy").click(); const share = await page.locator("#share").inputValue();
      await page.goBack(); await page.waitForFunction(() => document.querySelectorAll("#rows li").length === 1); await complete(page);
      await page.goForward(); await page.waitForFunction(() => JSON.parse(decodeURIComponent(location.hash.slice(5))).path.length > 0); await complete(page);
      await page.reload(); await complete(page); assert.equal(await page.locator("#share").inputValue(), share);
    }
    if (release) {
      // Clear the query and reveal an independently identified nested member.
      await page.locator("#field-clear").click(); await complete(page);
      const nested = await page.evaluate(() => {
        const g = window.schemaExplorerData, key = JSON.stringify, s = JSON.parse(decodeURIComponent(location.hash.slice(5)));
        const root = (s.kind === "message" ? g.messages : g.types).find(t => key(t.qname) === key(s.root));
        const type = g.types.find(t => key(t.qname) === key(s.kind === "message" ? root.type.named : root.qname));
        for (const segment of type.segments) for (const member of g.types.find(t => key(t.qname) === key(segment.owner)).members) {
          const child = member.type.named && g.types.find(t => key(t.qname) === key(member.type.named));
          if (child && child.segments.length) for (const nestedSegment of child.segments) {
            const owner = g.types.find(t => key(t.qname) === key(nestedSegment.owner));
            if (owner.members.length) return { name: owner.members[0].name, owner: owner.qname };
          }
        }
        return null;
      });
      assert(nested, "real root must supply a measured nested candidate");
      await page.locator("#field-mode").selectOption("name"); await page.locator("#field-query").fill(nested.name); await complete(page);
      const row = page.locator("#field-results li").filter({ has: page.locator("strong", { hasText: new RegExp(`^${nested.name}$`) }) }).filter({ hasText: `Declaring owner: {${nested.owner[0]}}${nested.owner[1]}` }).first();
      await row.getByRole("button", { name: "Reveal field", exact: true }).click();
      assert.equal(await page.locator(".selected > .row > .select").innerText(), nested.name);
      assert((await page.locator("#detail-content").innerText()).includes(`{${nested.owner[0]}}${nested.owner[1]}`));
      console.log(label, name, "nested reveal", nested.name, nested.owner);
    }
    await page.locator("#field-query").fill('<img src="https://invalid.example/" onerror="window.hostile=1">'); await complete(page);
    assert.equal(await page.locator("#field-results img").count(), 0); assert.equal(await page.evaluate(() => window.hostile), undefined);
    await page.locator("#field-clear").click(); await complete(page);
    // Start obsolete work and immediately replace its query in the same task.
    await page.evaluate(() => {
      const input = document.querySelector("#field-query");
      input.value = "obsolete"; input.dispatchEvent(new Event("input"));
      input.value = "task081-no-such-field"; input.dispatchEvent(new Event("input"));
      window.task081ExpectedGeneration = document.querySelector("#field-status").dataset.generation;
    });
    await page.waitForFunction(() => document.querySelector("#field-status").dataset.phase === "complete" && document.querySelector("#field-status").dataset.generation === window.task081ExpectedGeneration);
    assert.equal(await page.locator("#field-results li").count(), 0);
    await page.locator("#field-clear").click(); await complete(page);
    await page.screenshot({ path: path.join(evidence, `${label}-${name}-search-desktop.png`), fullPage: true });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: path.join(evidence, `${label}-${name}-search-narrow.png`), fullPage: true });
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await page.setViewportSize({ width: 1500, height: 1000 });
    await page.evaluate(() => {
      const input = document.querySelector("#field-query"); input.value = "pending"; input.dispatchEvent(new Event("input"));
      location.hash = "";
    });
    await page.waitForFunction(() => document.querySelector("#field-status").dataset.phase === "empty");
    assert.equal(await page.locator("#field-results li").count(), 0); assert.equal(await page.locator("#field-query").inputValue(), "");
    // Select a different root while its predecessor's index is still pending.
    await page.evaluate(({ kind, qname, replacement }) => {
      location.hash = "uci=" + encodeURIComponent(JSON.stringify({ v: 1, kind, root: qname, path: [] }));
      window.task081Replacement = replacement;
    }, { kind, qname: record.qname, replacement: graph.types.find(t => t.qname[1] === (release ? "EntityPositionType" : "Child")).qname });
    await page.waitForFunction(() => document.querySelector("#field-status").dataset.phase === "working");
    await page.evaluate(() => { location.hash = "uci=" + encodeURIComponent(JSON.stringify({ v: 1, kind: "type", root: window.task081Replacement, path: [] })); });
    await page.waitForFunction(() => document.querySelector("#field-scope").textContent.includes(`{${window.task081Replacement[0]}}${window.task081Replacement[1]}`) && document.querySelector("#field-status").dataset.phase === "complete");
    const replacementExpected = reachable(graph, "type", graph.types.find(t => t.qname[1] === (release ? "EntityPositionType" : "Child")).qname);
    assert.match(await page.locator("#field-status").innerText(), new RegExp(`Complete search: ${replacementExpected.size} matching`));
    await page.evaluate(() => { location.hash = ""; }); await page.waitForFunction(() => document.querySelector("#field-status").dataset.phase === "empty");
  }
}
async function revealLimits(context, url) {
  const page = await context.newPage();
  await page.addInitScript(() => {
    Object.defineProperty(window, "schemaExplorerData", { configurable: true, set(g) {
      const root = g.types.find(t => t.qname[1] === "Root"), child = g.types.find(t => t.qname[1] === "Child");
      const template = root.members[0];
      root.members = Array.from({ length: 2100 }, (_, index) => ({ ...template, index, name: index === 0 ? "wide-target" : `wide-${index}` }));
      // A deep chain remains searchable beyond the view's 24-step cap.
      for (let i = 0; i < 15; i++) {
        const qname = ["urn:limit", `Deep${i}`], next = ["urn:limit", `Deep${i + 1}`];
        g.types.push({ ...child, qname, url: child.url, ancestry: [qname], segments: [{ owner: qname, compositor: "Record" }], members: [{ ...template, index: 0, name: i === 14 ? "deep-target" : `deep-${i}`, type: i === 14 ? { primitive: "String" } : { named: next } }] });
      }
      g.messages.push({ ...g.messages[0], qname: ["urn:limit", "DeepReport"], type: { named: ["urn:limit", "Deep0"] } });
      Object.defineProperty(window, "schemaExplorerData", { value: g, configurable: true });
    } });
  });
  try {
    await page.goto(url); await page.locator('body[data-ready="true"]').waitFor();
    await page.locator("#root-search").fill("Report"); await page.locator("#root-results button").filter({ hasText: /^Report$/ }).click(); await complete(page);
    await page.locator("#field-query").fill("wide-target"); await complete(page);
    const before = await page.locator("#share").inputValue(), hash = await page.evaluate(() => location.hash);
    await page.getByRole("button", { name: "Reveal field", exact: true }).click();
    assert.match(await page.locator("#field-status").innerText(), /Materialized row limit/);
    assert.equal(await page.locator("#rows li").count(), 1); assert.equal(await page.locator("#share").inputValue(), before); assert.equal(await page.evaluate(() => location.hash), hash);
    await page.locator("#root-search").fill("DeepReport"); await page.locator("#root-results button").click(); await complete(page);
    await page.locator("#field-query").fill("deep-target"); await complete(page);
    assert.match(await page.locator("#field-status").innerText(), /Complete search: 1 matching/);
    assert.match(await page.locator("#field-results").innerText(), /Path exceeds explorer limit/);
    assert(await page.getByRole("button", { name: "Reveal field", exact: true }).isDisabled());
    assert.equal(await page.locator("#rows li").count(), 1);
    console.log("row/depth limit atomicity PASS");
  } finally { await page.close(); }
}
async function main() {
  fs.mkdirSync(evidence, { recursive: true });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}${prefix}`;
  for (const [name, engine] of [["chromium", chromium], ["firefox", firefox]]) {
    const browser = await engine.launch({ headless: true }); console.log(name, browser.version(), "Playwright", require("playwright/package.json").version);
    try { for (const release of releases) for (const mode of ["http", "file"]) {
      const context = await browser.newContext({ viewport: { width: 1500, height: 1000 } }), errors = [], requests = [];
      // Harness-only scheduling override; the production index/query code runs
      // unchanged, with batches of one and actual timer task boundaries.
      await context.addInitScript(batch => {
        window.task081Batches = 0;
        Object.defineProperty(window, "schemaExplorerSearch", { configurable: true, set(api) {
          const schedule = () => new Promise(resolve => setTimeout(() => { window.task081Batches++; resolve(); }, 0));
          const originalCreate = api.create, originalQuery = api.query;
          api.create = graph => { const engine = originalCreate(graph); return { index: (root, options) => engine.index(root, { ...options, batch: graph.types.length > 20 || graph.types.some(t => t.members.length > 100) ? 25 : batch, schedule }) }; };
          api.query = (index, input, options) => originalQuery(index, input, { ...options, batch: index.entries.length > 100 ? 25 : batch, schedule });
          Object.defineProperty(window, "schemaExplorerSearch", { value: api, configurable: true });
        } });
      }, releases[0] ? 25 : 1);
      context.on("page", p => p.on("pageerror", e => errors.push(e.message)));
      await context.route("**/*", r => { const u = r.request().url(); if (u.startsWith(base) || u.startsWith("file:")) r.continue(); else { requests.push(u); r.abort(); } });
      const page = await context.newPage(), relative = release ? `${release}/explorer.html` : "explorer.html", label = `${name}-${release || "fixture"}-${mode}`;
      try { const url = mode === "http" ? base + relative : pathToFileURL(path.join(root, relative)).href; await exercise(page, url, label, release); if (!release) await revealLimits(context, url); assert.deepEqual(errors, []); assert.deepEqual(requests, []); console.log(label, "PASS"); }
      finally { await context.close(); }
    } } finally { await browser.close(); }
  }
}
main().catch(e => { console.error(e); process.exitCode = 1; }).finally(() => server.close());