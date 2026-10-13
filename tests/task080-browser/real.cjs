"use strict";
// Opt-in qualification of a complete production Pages artifact.
const { chromium, firefox } = require("playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const http = require("node:http");
const { pathToFileURL } = require("node:url");
const { historyChecks, equivalentChecks } = require("./navigation.cjs");
const root = path.resolve(process.env.TASK080_PAGES || "../../target/task080-pages-final-a");
const evidence = path.resolve(process.env.TASK080_EVIDENCE || "evidence");
fs.mkdirSync(evidence, { recursive: true });
const prefix = "/ams-gra-codegen-oms/";
const server = http.createServer((req, res) => {
  const target = path.resolve(root, decodeURIComponent(req.url.split("?")[0].slice(prefix.length)));
  if (!req.url.startsWith(prefix) || !target.startsWith(root + path.sep)) { res.writeHead(404).end(); return; }
  fs.readFile(target, (error, bytes) => { if (error) { res.writeHead(404).end(); return; }
    res.setHeader("Content-Type", target.endsWith(".js") ? "text/javascript" : target.endsWith(".css") ? "text/css" : "text/html"); res.end(bytes); });
});
async function main() {
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}${prefix}`;
  for (const [name, engine] of [["chromium", chromium], ["firefox", firefox]]) {
    const browser = await engine.launch();
    try {
      for (const version of ["2.5", "2.6"]) for (const mode of ["http", "file"]) {
        const context = await browser.newContext({ viewport: { width: 1600, height: 1100 } });
        const unexpected = [];
        const errors = []; context.on("page", p => p.on("pageerror", e => errors.push(e.message)));
        try {
        await context.route("**/*", r => { const url = r.request().url(); if (url.startsWith(base) || url.startsWith("file:")) r.continue(); else { unexpected.push(url); r.abort(); } });
          const page = await context.newPage();
          const url = mode === "http" ? base + version + "/explorer.html" : pathToFileURL(path.join(root, version, "explorer.html")).href;
          const start = performance.now(); await page.goto(url); await page.waitForSelector('body[data-ready="true"]', { timeout: 30000 });
          assert.match(await page.locator("#identity").innerText(), new RegExp("v2_" + version.split(".")[1] + "_0"));
          assert.equal(await page.locator("#rows li").count(), 0);
          console.log(name, browser.version(), version, mode, "ready-ms", Math.round(performance.now() - start));
          const nested = await page.evaluate(() => {
            const g = window.schemaExplorerData;
            const m = g.messages.find(m => m.qname[1] === "PositionReport");
            const t = g.types.find(t => JSON.stringify(t.qname) === JSON.stringify(m.type.named));
            const segment = t.segments.find(s => g.types.find(t => JSON.stringify(t.qname) === JSON.stringify(s.owner)).members.some(m => m.name === "SecurityInformation"));
            const owner = g.types.find(t => JSON.stringify(t.qname) === JSON.stringify(segment.owner));
            const member = owner.members.find(m => m.name === "SecurityInformation");
            return { state: { v: 1, kind: "message", root: m.qname, path: [{ owner: segment.owner, compositor: segment.compositor }, { member: member.index }] }, selected: member.name };
          });
          const label = `${name}-${version}-${mode}`;
          await historyChecks(page, url, "PositionReport", nested, evidence, label);
          const reversed = { ...nested, state: { path: nested.state.path.map(p => p.owner ? { compositor: p.compositor, owner: p.owner } : p), root: nested.state.root, kind: "message", v: 1 } };
          await equivalentChecks(page, url, [{ ...nested, equivalent: "field" }, { ...reversed, equivalent: "field" }], label);
          for (const [kind, query] of [["message", "PositionReport"], ["message", "OrderOfBattle"], ["type", "EntityPositionType"]]) {
            const qname = await page.evaluate(({ kind, query }) => {
              const records = kind === "message" ? window.schemaExplorerData.messages : window.schemaExplorerData.types;
              const r = records.find(r => r.qname[1] === query); return r && r.qname;
            }, { kind, query });
            assert(qname, `${version}: missing ${query}`);
            await page.locator("#root-filter").selectOption(kind); await page.locator("#root-search").fill(query);
            await page.locator("#root-results button").filter({ hasText: new RegExp("^" + query + "$") }).click();
            await page.locator(".selected .select").filter({ hasText: new RegExp("^" + query + "$") }).waitFor();
            let expansions = 0;
            while (expansions < 14) {
              const buttons = page.locator('#rows button[aria-expanded="false"]:not([disabled])');
              if (!await buttons.count()) break;
              await buttons.first().click(); expansions++;
            }
            const candidates = page.locator("#rows .row");
            for (let i = 0; i < await candidates.count(); i++) {
              if (await candidates.nth(i).locator("span").nth(1).innerText() !== "—") {
                await candidates.nth(i).locator(".select").click(); break;
              }
            }
            assert(await page.locator("#detail-content a").count() > 0);
            // Compare selected field presentation against the reference renderer and exported IR graph.
            const selected = await page.locator(".selected > .row > .select").innerText();
            const href = await page.locator("#detail-content a").first().getAttribute("href");
            assert(/^types\/[0-9]{6}\.html$/.test(href));
            const reference = fs.readFileSync(path.join(root, version, href), "utf8");
            const field = await page.evaluate(() => {
              const selected = document.querySelector(".selected > .row");
              return selected.children[3].textContent;
            });
            if (field !== "—") assert(reference.includes(selected), `reference missing ${selected}`);
            const expected = await page.evaluate(() => {
              const s = JSON.parse(decodeURIComponent(location.hash.slice(5)));
              const member = s.path.at(-1), segment = s.path.at(-2);
              if (!member || !segment || member.member === undefined) return null;
              const t = window.schemaExplorerData.types.find(t => JSON.stringify(t.qname) === JSON.stringify(segment.owner));
              const m = t.members[member.member];
              return { name: m.name, range: `${m.count.min}..${m.count.max === null ? "*" : m.count.max}`, documentation: m.documentation, type: m.type };
            });
            if (expected) {
              assert.equal(selected, expected.name); assert.equal(field, expected.range);
              if (expected.documentation) assert((await page.locator("#detail-content").innerText()).includes(expected.documentation));
            }
            const refPage = await context.newPage(); await refPage.goto(new URL(href, page.url()).href);
            await refPage.getByRole("link", { name: "Explore this type", exact: true }).waitFor(); await refPage.close();
            console.log(version, query, "rows", await page.locator("#rows li").count(), "expansions", expansions, "selected", selected, "range", field);
            if (query === "PositionReport" && mode === "http") {
              await page.screenshot({ path: path.join(evidence, `${name}-${version}-real-desktop.png`), fullPage: true });
              await page.setViewportSize({ width: 390, height: 844 });
              await page.screenshot({ path: path.join(evidence, `${name}-${version}-real-narrow.png`), fullPage: true });
              assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
              await page.setViewportSize({ width: 1600, height: 1100 });
            }
          }
          assert.deepEqual(unexpected, []); assert.deepEqual(errors, []);
        } finally { await context.close(); }
      }
    } finally { await browser.close(); }
  }
}
main().catch(e => { console.error(e); process.exitCode = 1; }).finally(() => server.close());