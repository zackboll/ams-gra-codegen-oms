"use strict";
// Actual generated data, using the existing locked Playwright installation.
const { chromium, firefox } = require("playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const http = require("node:http");
const { pathToFileURL } = require("node:url");
const root = path.resolve(process.env.TASK082_PAGES);
const data = JSON.parse(fs.readFileSync(path.join(root, "compare/assets/compare-data.js"), "utf8").replace(/^window.schemaCompareData = /, "").replace(/;\n$/, ""));
const prefix = "/ams-gra-codegen-oms/";
const server = http.createServer((req, res) => {
  const target = path.resolve(root, decodeURIComponent(req.url.split("?")[0].slice(prefix.length)));
  if (!req.url.startsWith(prefix) || !target.startsWith(root + path.sep)) { res.writeHead(404).end(); return; }
  fs.readFile(target, (err, bytes) => {
    if (err) { res.writeHead(404).end(); return; }
    res.setHeader("Content-Type", target.endsWith(".js") ? "text/javascript" : target.endsWith(".css") ? "text/css" : "text/html"); res.end(bytes);
  });
});
async function main() {
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}${prefix}`;
  for (const [name, engine] of [["chromium", chromium], ["firefox", firefox]]) {
    const browser = await engine.launch();
    try {
      for (const mode of ["http", "file"]) {
        const page = await browser.newPage({ viewport: { width: 1100, height: 900 } });
        const errors = [], external = [];
        page.on("pageerror", e => errors.push(e.message));
        page.on("request", r => { if (!r.url().startsWith(base) && !r.url().startsWith("file:")) external.push(r.url()); });
        const url = mode === "http" ? base + "compare/index.html" : pathToFileURL(path.join(root, "compare/index.html")).href;
        await page.goto(url); await page.waitForSelector('body[data-ready="true"]');
        for (const category of ["Type", "Message"]) {
          await page.locator("#category").selectOption(category);
          for (const status of ["Added", "Removed", "Changed", "Unchanged"]) {
            await page.locator("#filter").selectOption(status);
            const count = data.declarations.filter(d => d.category === category && d.status === status).length;
            assert.equal(await page.locator("#results button").count(), Math.min(50, count));
            assert.match(await page.locator("#status").innerText(), new RegExp(`^${count} declarations`));
          }
        }
        const choices = [data.declarations.find(d => d.qname[1] === "ADS_B_KinematicsContributionType"),
          data.declarations.find(d => d.qname[1] === "AMTI_CapabilityType"),
          ...["Type", "Message"].flatMap(cat => ["Added", "Removed", "Changed", "Unchanged"].map(s => data.declarations.find(d => d.category === cat && d.status === s))),
          data.declarations.find(d => d.qname[1] === "Record"), data.declarations.find(d => d.qname[1] === "Pattern")].filter(Boolean);
        let previous;
        for (const d of choices) {
          await page.locator("#category").selectOption(d.category);
          await page.locator("#filter").selectOption(d.status);
          await page.locator("#search").fill(`{${d.qname[0]}}${d.qname[1]}`);
          const b = page.locator("#results button").first(); await b.focus(); await page.keyboard.press("Enter");
          await page.waitForFunction(q => document.querySelector("#details h2")?.textContent.includes(q), d.qname[1]);
          assert.equal(await page.locator("#details article").count(), d.changes.length);
          assert.deepEqual(await page.locator("#details pre").allTextContents(), d.changes.flatMap(c => [c.before, c.after].map(v => v === null ? "Absent (missing value)" : v)));
          const hrefs = await page.locator("#details a").evaluateAll(nodes => nodes.map(n => n.getAttribute("href")));
          assert.deepEqual(hrefs, [...d.destinations.filter(Boolean).flatMap(dest => [dest.reference, dest.explorer]),
            ...d.changes.flatMap(c => (c.memberDestinations || []).filter(Boolean))]);
          const share = await page.locator(".share").inputValue(); assert.equal(share, page.url());
          if (previous && previous !== share) { await page.goBack(); assert.equal(page.url(), previous); await page.goForward(); assert.equal(page.url(), share); }
          previous = share;
          await page.reload(); await page.waitForSelector("#details h2");
        }
        // Click shipped actions, verifying identity against that release's graph.
        const fieldCases = data.declarations.filter(d => d.changes.some(c => c.memberDestinations?.some(Boolean)));
        const measured = fieldCases.filter(d => d.qname[1] === "ADS_B_KinematicsContributionType" || d.qname[1] === "Record");
        if (data.declarations.length < 20) for (const d of fieldCases) if (!measured.includes(d)) measured.push(d);
        for (const kind of ["MEMBER_CHANGED", "MEMBER_ADDED", "MEMBER_REMOVED"]) {
          const d = fieldCases.find(d => d.changes.some(c => c.kind === kind && c.memberDestinations?.some(Boolean)));
          if (d && !measured.includes(d)) measured.push(d);
        }
        const graphs = ["2.5", "2.6"].map(v => JSON.parse(fs.readFileSync(path.join(root, v, "assets/explorer-data.js"), "utf8").replace(/^window.schemaExplorerData = /, "").replace(/;\n$/, "")));
        const choice = fieldCases.find(d => graphs.some(g => g.types.find(t => JSON.stringify(t.qname) === JSON.stringify(d.qname))?.kind === "Choice"));
        if (choice && !measured.includes(choice)) measured.push(choice);
        let reveals = 0;
        for (const d of measured) {
          const compareURL = url + "#compare=" + encodeURIComponent(JSON.stringify({ v: 1, category: d.category, qname: d.qname }));
          for (const c of d.changes) for (const [side, destination] of (c.memberDestinations || []).entries()) {
            if (!destination) continue;
            await page.goto(compareURL); await page.waitForSelector("#details h2");
            const exact = page.locator(`#details article a[href=${JSON.stringify(destination)}]`).first();
            await exact.focus(); await page.keyboard.press("Enter");
            await page.waitForSelector('body[data-ready="true"]');
            const state = JSON.parse(decodeURIComponent(new URL(page.url()).hash.slice(5)));
            assert.deepEqual(state.root, d.qname); assert.equal(state.path.length, 2);
            const owner = graphs[side].types.find(t => JSON.stringify(t.qname) === JSON.stringify(d.qname));
            assert.deepEqual(state.path[0], { owner: owner.qname, compositor: owner.kind });
            const member = owner.members[state.path[1].member];
            assert.equal(await page.locator('.selected > .row > .select').textContent(), member.name);
            assert((await page.locator("#detail-content").innerText()).includes(`{${owner.qname[0]}}${owner.qname[1]}`));
            const target = member.type.named ? graphs[side].types.find(t => JSON.stringify(t.qname) === JSON.stringify(member.type.named)) : owner;
            assert.equal(await page.locator("#detail-content a").first().getAttribute("href"), target.url);
            const details = await page.locator("#detail-content").innerText();
            assert(details.includes(`Local cardinality: ${member.count.min}..${member.count.max === null ? "*" : member.count.max}`));
            // Exercise normal Back before reload (Firefox reload can restore a
            // separate persisted document history entry). Then re-enter the
            // actual action and qualify reload independently.
            await page.goBack();
            await page.waitForSelector("#details h2");
            assert.deepEqual(JSON.parse(decodeURIComponent(new URL(page.url()).hash.slice(9))),
              { v: 1, category: d.category, qname: d.qname });
            assert.equal(new URL(page.url()).pathname, new URL(compareURL).pathname);
            await page.locator(`#details article a[href=${JSON.stringify(destination)}]`).first().click();
            await page.waitForSelector(".selected");
            await page.reload(); await page.waitForSelector(".selected");
            assert.equal(await page.locator("#detail-content").innerText(), details);
            await page.setViewportSize({ width: 375, height: 800 });
            assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
            await page.setViewportSize({ width: 1100, height: 900 });
            reveals++;
          }
        }
        console.log(name, mode, "exact field reveals", reveals, "Choice present:", Boolean(choice));
        const linked = fieldCases[0];
        if (linked) {
          const destination = linked.changes.flatMap(c => c.memberDestinations || []).find(Boolean);
          const resolved = new URL(destination, url);
          const state = JSON.parse(decodeURIComponent(resolved.hash.slice(5)));
          for (const bad of [{ ...state, path: [state.path[0], { member: 999999 }] },
            { ...state, path: [{ ...state.path[0], compositor: "Wrong" }, state.path[1]] },
            { ...state, path: [...state.path, { member: 0 }] }]) {
            resolved.hash = "uci=" + encodeURIComponent(JSON.stringify(bad));
            await page.goto(resolved.href); await page.waitForSelector('body[data-ready="true"]');
            assert.match(await page.locator("#status").innerText(), /Explorer state error/);
            assert.equal(await page.locator(".selected").count(), 0);
          }
        }
        for (const fragment of ["#compare=%7B", "#compare=" + "x".repeat(16385), "#compare=" + encodeURIComponent(JSON.stringify({ v: 1, category: "Type", qname: ["urn:missing", "Missing"] })), "#compare=" + encodeURIComponent(JSON.stringify({ v: 1, category: "Type", qname: ["javascript:alert(1)", "<script>bad</script>"], path: "evil" }))]) {
          await page.goto(url + fragment); await page.waitForSelector("#details p");
          assert.match(await page.locator("#details").innerText(), /Comparison state error/);
          assert.equal(await page.locator("#details h2").count(), 0);
        }
        await page.setViewportSize({ width: 375, height: 800 });
        await page.goto(previous); await page.waitForSelector("#details h2");
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
        assert.equal(await page.evaluate(() => window.hostile), undefined);
        assert.equal(await page.locator('#details script').count(), 0);
        assert.deepEqual(errors, []); assert.deepEqual(external, []);
        console.log(name, browser.version(), mode, "PASS", choices.map(d => d.qname[1]).join(", "));
        await page.close();
      }
    } finally { await browser.close(); }
  }
}
main().catch(e => { console.error(e); process.exitCode = 1; }).finally(() => server.close());