# Task 043 — UCI HTML Type Browser

## Motivation and architecture

`docs --schema PATH [--overlay PATH]... --output DIR` loads through the same
frontend overlay composition as validation and generation, validates the
normalized `SchemaIr`, builds a documentation/index model, then renders all
files in memory and writes them with the CLI's existing safe file writer.
There is no second XSD interpretation. Documentation includes declarations
unsupported by source-code backends. It is not a `Backend`: no generated
representation is requested, so neither `--language` nor `--world` applies.

## Layout and identity

`index.html` contains summary counts, namespaces, an all-type index and message
inventory with `message-000000`-style anchors. `types/000000.html` and later
pages are assigned in canonical qualified-name order by one QName → path map;
raw namespace strings never become paths. `assets/style.css`, `search.js` and
`search-index.js` are bundled. All links are relative and work on `file://`.
Prefixes are display metadata, not identity. Source paths and optional lines
are provenance, not hyperlinks or identity.

Effective constraints are read from `ConstraintSet`, including numeric/length
facets and whitespace policy. Pattern groups are shown in base-to-derived
order: AND between groups and OR within each group's alternatives. The
shared `direct_named_dependencies` supplies direct Uses links; a reverse index
is built once and includes message payload references. Structural pages use
a shared **lossless** ancestry/segment traversal over valid normalized `SchemaIr`,
retaining base-to-derived owners, empty ancestry levels, and Record/Choice
compositor distinctions instead of flattening mixed inheritance. Inherited
duplicate member names remain visible under both owners in documentation;
source backends impose stricter representability rules and can still reject
those duplicates as `InheritedMemberNameCollision`. Ordinal
paths, sorted indexes and schema-ordered message anchors ensure deterministic
files for the same complete IR, including source provenance.

Search is a case-insensitive substring match over type identity, prefix,
namespace, kind, documentation, declared member names, enum wire values and
message names. JSON-serialized records are installed by local
`search-index.js`, not loaded with `fetch` or XHR; static `search.js` renders at
most 50 clickable results and reports the total. No network/runtime packages
or HTTP server are needed. All schema-derived HTML text and attributes are
escaped (`&`, `<`, `>`, quotes and apostrophes); JavaScript results use
`textContent` and JSON serialization, not HTML insertion or manual quoting.

## Evidence and boundaries

Frontend-backed fixture and CLI tests cover repeated overlays, navigation,
escaping/injection, pattern group semantics, search data and deterministic
file sets. A valid IR fixture with `Base.Same` and `Derived.Same` verifies
that both members are documented even though codegen projection rejects the
collision. The public API accepts only normalized IR. No language snippets,
backend badges, world selection, service-specific views, web server, source
copying or schema diff are part of this task.

## Pinned UCI smoke tests

Both pinned release inputs were available locally and were read without
committing either source trees or generated HTML. The local 2.5 schema comes
from `093610b7753944059360d3236770ab446d039556` and the 2.6 extraction
from `78eb61b6112c8bffa40820c33124b57787fc5bd9`.

| Release | Normalized types | Messages | Generated files | Run 1 | Run 2 | Rerun |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| UCI 2.5 | 5,557 | 722 | 5,561 | 227 s | 240 s | byte-identical files and paths |
| UCI 2.6 | 5,570 | 725 | 5,574 | 297 s | 332 s | byte-identical files and paths |

Both runs completed without panic. A full scan of generated `href` and `src`
attributes found zero broken or external links in either version. Text was
inspected for `NATO_SpecialWordsType`, `DateTimeType`,
`YearOfEquinoxEnum`, the inherited `ZoneViolationTriggerDataType`, and the
message-referenced `WeatherRadarSettingsCommandStatusMT` in each release.
Timing includes the existing frontend's full schema load and IR validation,
in-memory HTML rendering, and writing thousands of files. Browser execution
was not tested; file-relative links and offline JavaScript were inspected.
