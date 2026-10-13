"use strict";
const { chromium, firefox } = require("playwright");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const http = require("node:http");
const { pathToFileURL } = require("node:url");
const root = path.resolve(process.env.TASK080_ARTIFACT || "../../target/task080-fixture");
const evidence = path.resolve(process.env.TASK080_EVIDENCE || "evidence");
fs.mkdirSync(evidence, { recursive: true });
const prefix = "/ams-gra-codegen-oms/";
const server = http.createServer((req, res) => {
  const target = path.resolve(root, decodeURIComponent(req.url.split("?")[0].slice(prefix.length)));
  if (!req.url.startsWith(prefix) || !target.startsWith(root + path.sep)) { res.writeHead(404).end(); return; }
  fs.readFile(target, (error, bytes) => {
    if (error) { res.writeHead(404).end(); return; }
    res.setHeader("Content-Type", target.endsWith(".js") ? "text/javascript" : target.endsWith(".css") ? "text/css" : "text/html"); res.end(bytes);
  });
});
const state = (kind, name, steps = []) => "#uci=" + encodeURIComponent(JSON.stringify({ v: 1, kind, root: ["urn:explorer", name], path: steps }));
async function ready(page) { await page.waitForSelector('body[data-ready="true"]', { timeout: 15000 }); }
async function select(page, name) { await page.locator(".select").filter({ hasText: new RegExp("^" + name + "$") }).first().click(); }
async function expand(page, name) { await page.getByRole("button", { name: "Expand " + name, exact: true }).first().click(); }
async function fixture(page, url, label) {
  await page.goto(url); await ready(page);
  await page.locator("#root-search").fill("Report"); await page.locator("#root-results button").click();
  await page.locator(".selected").waitFor();
  await expand(page, "Report");
  await expand(page, "Record — Declared by Root");
  await expand(page, "Record — Inherited from Base");
  assert.match(await page.locator("#rows").innerText(), /Inherited/);
  const first = page.locator("li").filter({ has: page.locator(".row > button.select", { hasText: /^First$/ }) }).last();
  await expand(page, "First"); await expand(page, "Record — Declared by Child");
  await select(page, "constructor");
  assert.match(await page.locator("#detail-content").innerText(), /Conditional presence/);
  assert.match(await page.locator("#detail-content").innerText(), /<img/);
  assert.equal(await page.evaluate(() => window.hostile), undefined);
  assert.equal(await page.locator("#detail-content img").count(), 0);
  assert.match(await page.locator("#detail-content a").first().getAttribute("href"), /^types\/[0-9]{6}\.html$/);
  await expand(page, "Second");
  assert.equal(await page.getByRole("button", { name: "Expand Record — Declared by Child", exact: true }).count(), 1);
  await expand(page, "Record — Declared by Child");
  assert.equal(await page.locator(".select").filter({ hasText: /^constructor$/ }).count(), 2);
  assert.match(await page.locator("#rows").innerText(), /Recursive reference/);
  assert.match(await page.locator("#rows").innerText(), /9007199254740993\.\.18446744073709551615/);
  assert.match(await page.locator("#rows").innerText(), /2\.\.8/);
  assert.match(await page.locator("#rows").innerText(), /Not occurring/);
  assert.match(await page.locator("#rows").innerText(), /Optional/);
  assert.match(await page.locator("#rows").innerText(), /Repeated/);
  await expand(page, "Pick"); await expand(page, "Choice / Alternatives — Declared by Choice");
  await select(page, "Right");
  assert.match(await page.locator("#detail-content").innerText(), /Choice alternative/);
  const deep = page.url();
  await page.reload(); await ready(page); await page.locator(".selected .select").filter({ hasText: /^Right$/ }).waitFor();
  await page.locator("#copy").click(); assert.match(await page.locator("#share").inputValue(), /#uci=/);
  await page.locator("#root-filter").selectOption("type"); await page.locator("#root-search").fill("Enum"); await page.locator("#root-results button").click();
  await page.locator(".selected .select").filter({ hasText: /^Enum$/ }).waitFor();
  await page.goBack(); await page.locator(".selected .select").filter({ hasText: /^Right$/ }).waitFor();
  await page.goForward(); await page.locator(".selected .select").filter({ hasText: /^Enum$/ }).waitFor();
  await page.goto(url + state("type", "Root")); await ready(page);
  const disclosure = page.locator("#rows > li > .row > button").first(); await disclosure.focus(); await page.keyboard.press("Enter");
  assert.equal(await disclosure.getAttribute("aria-expanded"), "true");
  await page.keyboard.press("Enter"); assert.equal(await disclosure.getAttribute("aria-expanded"), "false");
  await page.goto(deep); await ready(page);
  await page.screenshot({ path: path.join(evidence, label + "-desktop.png"), fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.evaluate(() => { document.documentElement.style.fontSize = "200%"; });
  await page.screenshot({ path: path.join(evidence, label + "-narrow.png"), fullPage: true });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.goto(url + "#uci=%7B"); await ready(page); assert.match(await page.locator("#status").innerText(), /error/i);
  await page.goto(url + "#uci=" + "x".repeat(17000)); await ready(page); assert.match(await page.locator("#status").innerText(), /oversized/);
  await page.goto(deep); await ready(page);
  await page.getByRole("button", { name: "Collapse Record — Declared by Root", exact: true }).click();
  assert.match(await page.locator(".selected > .row > .select").innerText(), /Declared by Root/);
  assert.equal(await page.evaluate(() => document.activeElement.getAttribute("aria-expanded")), "false");
  await page.locator("#collapse").click();
  assert.equal(await page.locator("#rows li").count(), 1);
  assert.equal(await page.locator("#rows > li > .row > button").first().getAttribute("aria-expanded"), "false");
}
async function main() {
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}${prefix}`;
  for (const [name, engine] of [["chromium", chromium], ["firefox", firefox]]) {
    const browser = await engine.launch({ headless: true });
    console.log(name, browser.version(), "Playwright", require("playwright/package.json").version);
    try {
      for (const mode of ["http", "file"]) {
        const context = await browser.newContext({ viewport: { width: 1500, height: 1000 } });
        const unexpected = [];
        await context.route("**/*", route => { const u = route.request().url(); if (u.startsWith(base) || u.startsWith("file:")) route.continue(); else { unexpected.push(u); route.abort(); } });
        const page = await context.newPage();
        const url = mode === "http" ? base + "explorer.html" : pathToFileURL(path.join(root, "explorer.html")).href;
        await fixture(page, url, name + "-" + mode);
        assert.deepEqual(unexpected, []);
        await context.close(); console.log(name, mode, "PASS");
      }
      const nojs = await browser.newContext({ javaScriptEnabled: false }); const p = await nojs.newPage();
      await p.goto(base + "explorer.html"); assert.match(await p.locator("noscript").innerText(), /reference browser/); await p.getByRole("link", { name: "Reference browser", exact: true }).first().click(); await p.waitForURL("**/index.html"); await nojs.close();
      for (const invalid of [null, "window.schemaExplorerData = {version:999};"]) {
        const c = await browser.newContext(); await c.route("**/explorer-data.js", r => invalid === null ? r.abort() : r.fulfill({ contentType: "text/javascript", body: invalid }));
        const p = await c.newPage(); await p.goto(base + "explorer.html"); await p.waitForSelector('body[data-ready="error"]'); assert.match(await p.locator("#status").innerText(), /data error/); await c.close();
      }
    } finally { await browser.close(); }
  }
}
main().catch(e => { console.error(e); process.exitCode = 1; }).finally(() => server.close());