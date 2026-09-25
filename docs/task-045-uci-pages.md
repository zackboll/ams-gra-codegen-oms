# Task 045 — UCI Pages publication

The static project Pages site is assembled with
`scripts/build-uci-pages.sh <empty-output-directory>`. The script builds the
production Rust CLI once and invokes `docs --schema ... --output ...` twice.
The existing Task 043 XSD frontend → validated normalized `SchemaIr` →
`schema-docs` pipeline is the sole source of generated HTML. No UCI source or
HTML is committed. The top-level page is copied from `site/index.html`.

| Release | Tag | Exact source revision | Input | Types | Messages |
| --- | --- | --- | --- | ---: | ---: |
| 2.5 | v2.5 | `093610b7753944059360d3236770ab446d039556` | `03_OAC-STD-002_RevE_UCI_Schema_v2_5/UCI_MessageDefinitions_v2_5_0.xsd` | 5,557 | 722 |
| 2.6 | v2.6 | `78eb61b6112c8bffa40820c33124b57787fc5bd9` | `UCI Release Documentation - UCI Schema/03_UCI-STD-002_Rev6_UCI_Schema_v2_6-CDRL.zip` → exactly one `UCI_MessageDefinitions_v2_6_0.xsd` | 5,570 | 725 |

The public authority is https://gitlab.com/open-arsenal/uci/standard.git.
Both tags are fetched into temporary storage and checked against the full
pinned SHA; each detached checkout is verified before use. The ZIP is extracted
only into temporary storage, with traversal, duplicate, symlink and ambiguous
root guards. Nothing from this checkout is executed or copied to the site.
The CLI receives paths relative to each temporary source root; its existing
source provenance therefore remains useful without leaking the random `/tmp`
checkout path into the HTML or making the output nondeterministic. Task 043's
generator and normalized IR are unchanged.

The assembled root has `index.html`, `2.5/index.html` and `2.6/index.html`;
each release retains Task 043's relative `types/` and `assets/` links, so it
works under a project Pages prefix and locally. Validation checks identity,
search inventory counts (the generated structured JSON records), type page
counts, assets, every local HTML `href`/`src`, and artifact purity and size.
It fails closed on unexpected content or an artifact at/above 1 GB.

`.github/workflows/uci-pages.yml` builds and validates on PRs without deploying.
On main pushes (and manual runs on main) it uploads the validated Pages artifact
and deploys that artifact to the `github-pages` environment. Workflow-level
concurrency cancels stale runs for the same ref, including stale deployments;
PRs have separate concurrency groups and never deploy. No `gh-pages` branch or
repository write token is used. The official action majors were checked against
their upstream releases: `configure-pages@v6`, `upload-pages-artifact@v5`, and
`deploy-pages@v5`.
Repository Settings → Pages → Build and deployment → Source must be set to
**GitHub Actions** once by a maintainer. The read-only Pages API returned 404
and the repository reported `has_pages: false` before this PR: Pages was not
enabled at implementation time.

## Reproducibility evidence

The complete site is built twice from separately fetched pinned inputs into
empty destinations; recursive file contents and paths are compared without
exclusions (`diff -qr`). The builder refuses nonempty output directories.
The validator also checks that source paths do not leak from temporary storage,
verifies the structured Task 043 search index against both release inventories,
checks HTML links, anchors, and search URLs, and rejects unrecognized files.
`python3 -m unittest discover -s scripts -p 'test_*.py'` covers malformed ZIP
inputs and representative missing-link, missing-anchor and provenance failures.
Local `cargo fmt`, `check`, `clippy`, `test` and `git diff --check` are also
required.

| Release | Normalized types | Messages | Generated files |
| --- | ---: | ---: | ---: |
| UCI 2.5 | 5,557 | 722 | 5,561 |
| UCI 2.6 | 5,570 | 725 | 5,574 |

The validated complete artifact contains **11,136 files** totaling
**63,672,092 bytes**. The complete HTML navigation, search URLs, and assets
scan passed with zero broken internal links and zero leaked temporary source
paths. The artifact contains no UCI checkout, XSD, ZIP, `.git` or `target`.
Local `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`AMS_GRA_REQUIRE_GNAT=1 cargo test --workspace` passed (including GNAT-backed
tests); `git diff --check` passed. Two complete builds into separate empty
directories, each fetching the pinned inputs independently, passed validation
at 11,136 files and 63,672,092 bytes. `diff -qr` over their entire trees
returned zero differences: every file path and byte is identical.
