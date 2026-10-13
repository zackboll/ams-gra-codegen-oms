"use strict";
const assert = require("node:assert/strict");
const path = require("node:path");
const corpus = require("./state-cases.json");
const fragment = s => "uci=" + encodeURIComponent(JSON.stringify(s));
function rejectedFragment(c) {
  if (c.fragment) return c.fragment;
  if (c.size) return "uci=" + "x".repeat(c.size);
  const s = c.state || { v: 1, kind: "message", root: ["urn:explorer", "Report"], path: c.path ||
    (c.depth ? Array(c.depth).fill({ member: 0 }) : [{ owner: ["urn:explorer", "Root"], compositor: "Record" }, { member: c.member }]) };
  return fragment(s);
}
async function snapshot(page) {
  return page.evaluate(() => ({
    title: document.querySelector("#root-title").textContent,
    rows: document.querySelectorAll("#rows li").length,
    selected: document.querySelector(".selected > .row > .select")?.textContent || null,
    details: document.querySelector("#detail-content").textContent,
    reference: document.querySelector("#detail-content a")?.getAttribute("href") || null,
    share: document.querySelector("#share").value,
    history: history.length,
    hash: location.hash
  }));
}
async function waitSelected(page, name) {
  // Drain queued popstate/hashchange rendering before inspecting a same-label
  // selection. This is a browser rendering condition, not a fixed-time sleep.
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.waitForFunction(name => document.querySelector(".selected > .row > .select")?.textContent === name && !document.querySelector("#status").textContent.includes("error"), name, { timeout: 10000 });
}
async function empty(page) {
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.waitForFunction(() => !location.hash && !document.querySelector("#rows li") && document.querySelector("#root-title").textContent === "Structure" && document.querySelector("#share").value === "", null, { timeout: 10000 });
  const s = await snapshot(page);
  assert.equal(s.selected, null); assert.equal(s.details, "Select a root or field."); assert.equal(s.reference, null);
  assert.equal(await page.locator("#status").innerText(), "Choose a message or type to begin.");
}
async function historyChecks(page, url, rootName, nestedState, evidence, label) {
  await page.goto(url); await page.locator('body[data-ready="true"]').waitFor(); await empty(page);
  const initialLength = await page.evaluate(() => history.length);
  await page.locator("#root-filter").selectOption("message"); await page.locator("#root-search").fill(rootName);
  const choose = async () => { await page.locator("#root-results button").filter({ hasText: new RegExp("^" + rootName + "$") }).click(); await waitSelected(page, rootName); };
  await choose();
  assert.equal(await page.evaluate(() => history.length), initialLength + 1);
  await page.goBack(); await empty(page);
  await page.locator("#copy").click(); await page.locator("#collapse").click(); await empty(page);
  assert.equal(await page.evaluate(() => history.length), initialLength + 1);
  await page.goForward(); await waitSelected(page, rootName);
  const rootSnapshot = await snapshot(page);
  // Open the path using real disclosures and select its final field. Expansions
  // do not write history; selection does. The final path comes from generated data.
  // A probe obtains the exact labels; the exercised page still uses UI clicks.
  const probe = await page.context().newPage();
  let labels;
  try {
    await probe.goto(url + "#" + fragment(nestedState.state)); await waitSelected(probe, nestedState.selected);
    labels = await probe.locator('.selected').evaluate(li => {
      const labels = [];
      while (li && li.id !== "rows") {
        if (li.tagName === "LI") labels.unshift(li.querySelector(":scope > .row > .select").textContent);
        li = li.parentElement;
      }
      return labels;
    });
  } finally { await probe.close(); }
  for (const label of labels.slice(0, -1)) await page.getByRole("button", { name: "Expand " + label, exact: true }).first().click();
  await page.locator(".select").filter({ hasText: new RegExp("^" + labels.at(-1) + "$") }).first().click();
  await waitSelected(page, nestedState.selected);
  const nested = await snapshot(page); const length = nested.history;
  assert.equal(length, initialLength + 2);
  for (let repeat = 0; repeat < 3; repeat++) {
    await page.goBack(); await waitSelected(page, rootName);
    assert.equal((await snapshot(page)).rows, rootSnapshot.rows);
    await page.goBack(); await empty(page);
    await page.goForward(); await waitSelected(page, rootName);
    await page.goForward(); await waitSelected(page, nestedState.selected);
    const s = await snapshot(page);
    for (const k of ["rows", "selected", "details", "reference", "share", "history"]) assert.equal(s[k], nested[k], `${label}: repeated history ${k}`);
  }
  await page.screenshot({ path: path.join(evidence, label + "-restored.png"), fullPage: true });
  await page.locator('.selected > .row > .select').focus();
  await page.evaluate(() => { location.hash = ""; }); await empty(page);
  assert.equal(await page.locator("#root-search").inputValue(), rootName);
  assert.equal(await page.evaluate(() => document.activeElement.id), "root-search");
  const emptyLength = await page.evaluate(() => history.length);
  await page.locator("#copy").click(); await page.locator("#collapse").click(); await empty(page);
  assert.equal(await page.evaluate(() => history.length), emptyLength);
  await page.screenshot({ path: path.join(evidence, label + "-empty.png"), fullPage: true });
  await choose(); assert.equal((await snapshot(page)).rows, 1);
  await page.getByRole("button", { name: "Expand " + rootName, exact: true }).click();
  assert((await snapshot(page)).rows > 1);
  await page.locator("#collapse").click(); await waitSelected(page, rootName); assert.equal((await snapshot(page)).rows, 1);
  await page.locator("#root-search").focus();
  await page.evaluate(() => { location.hash = ""; }); await empty(page);
  assert.equal(await page.evaluate(() => document.activeElement.id), "root-search", "reset must preserve navigator focus");
  console.log(label, "history A-H PASS");
}
async function equivalentChecks(page, url, states, label) {
  const seen = new Map();
  for (const c of states) {
    // selected is a test expectation, never part of version-1 state.
    const input = fragment(c.state);
    await page.goto(url + "#" + input); await waitSelected(page, c.selected);
    const s = await snapshot(page);
    assert.equal(s.hash, "#" + input, "normalizing input must not rewrite history");
    const canonical = JSON.parse(decodeURIComponent(s.share.split("\n").at(-1).split("#uci=")[1]));
    const expected = { v: 1, kind: c.state.kind, root: c.state.root, path: c.state.path.map(p => p.owner ? { owner: p.owner, compositor: p.compositor } : p) };
    assert.equal(JSON.stringify(canonical), JSON.stringify(expected));
    if (c.equivalent) {
      const identity = { details: s.details, reference: s.reference, share: s.share, rows: s.rows };
      if (seen.has(c.equivalent)) assert.deepEqual(identity, seen.get(c.equivalent)); else seen.set(c.equivalent, identity);
    }
    await page.reload(); await waitSelected(page, c.selected);
    assert.equal((await snapshot(page)).share, s.share);
    await page.locator("#copy").click();
    assert.equal(await page.locator("#share").inputValue(), s.share);
    await page.goto(s.share.split("\n").at(-1)); await waitSelected(page, c.selected);
    assert.equal((await snapshot(page)).details, s.details);
  }
  console.log(label, states.length, "accepted paths + reload/copy/restore PASS");
}
async function pathChecks(page, url, label) {
  await equivalentChecks(page, url, corpus.accepted, label);
  const identities = new Map();
  for (const name of ["deep", "inherited", "sibling", "duplicate-owner", "other-namespace"]) {
    const c = corpus.accepted.find(c => c.name === name);
    await page.goto(url + "#" + fragment(c.state)); await waitSelected(page, c.selected);
    identities.set(name, await snapshot(page));
  }
  assert.notEqual(identities.get("deep").share, identities.get("sibling").share, "independent same-named sibling paths remain distinct");
  assert.notEqual(identities.get("inherited").details, identities.get("duplicate-owner").details, "same-named fields under different owners remain distinct");
  assert.notEqual(identities.get("deep").reference, identities.get("other-namespace").reference, "same local type name in another namespace remains distinct");
  for (const c of corpus.rejected) {
    await page.goto(url); await empty(page);
    await page.evaluate(hash => { location.hash = hash; }, "#" + rejectedFragment(c));
    await page.waitForFunction(() => document.querySelector("#status").textContent.startsWith("Explorer state error:"), null, { timeout: 10000 });
    const s = await snapshot(page); assert.equal(s.rows, 0, c.name); assert.equal(s.selected, null, c.name); assert.equal(s.share, "", c.name);
  }
  // Invalid input also leaves an existing valid exploration untouched, not
  // partially restored. The explicit error distinguishes it from empty state.
  const good = corpus.accepted.find(c => c.name === "deep");
  await page.goto(url + "#" + fragment(good.state)); await waitSelected(page, good.selected);
  const before = await snapshot(page);
  await page.evaluate(hash => { location.hash = hash; }, "#" + rejectedFragment(corpus.rejected.find(c => c.name === "recursive-extension")));
  await page.waitForFunction(() => document.querySelector("#status").textContent.startsWith("Explorer state error:"));
  const after = await snapshot(page);
  for (const k of ["rows", "selected", "details", "share"]) assert.equal(after[k], before[k]);
  console.log(label, corpus.rejected.length, "rejected paths PASS");
}
module.exports = { historyChecks, equivalentChecks, pathChecks, fragment, snapshot };