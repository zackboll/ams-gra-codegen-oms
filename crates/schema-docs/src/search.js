"use strict";
(() => {
  const input = document.getElementById("search");
  const count = document.getElementById("match-count");
  const list = document.getElementById("results");
  input.addEventListener("input", () => {
    list.replaceChildren();
    const query = input.value.trim().toLocaleLowerCase();
    if (!query) { count.textContent = ""; return; }
    const matches = window.schemaSearchIndex.filter(record => record.terms.toLocaleLowerCase().includes(query));
    count.textContent = `${matches.length} match(es); showing up to 50`;
    for (const record of matches.slice(0, 50)) {
      const item = document.createElement("li");
      const link = document.createElement("a");
      link.href = record.url;
      link.textContent = record.label;
      item.append(link);
      list.append(item);
    }
  });
})();
