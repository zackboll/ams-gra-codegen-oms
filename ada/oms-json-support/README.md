# Guarded OMS JSON support (Task 087 work in progress)

`AMS_GRA_OMS_JSON_Guard` provides guarded `Read` and `Write` operations over
`GNATCOLL.JSON.JSON_Value`. This support project is an application dependency;
the Rust generator does not load, link, or require GNATCOLL.

The manifest requires **GNATCOLL and GNATCOLL minimal 26.0.0**. The qualified
upstream `v26.0.0` tag resolves to
`8e270f83719d4284782a965edd80246be15b949b`. Task 086 records the Alire archive
SHA-512 and its byte comparison with that revision. No provider patch is used.
The support sources use the repository's MIT license. GNATCOLL is licensed
`GPL-3.0-or-later WITH GCC-exception-3.1`; consumers must account for that
dependency's license independently.

## Consumer setup

Add to an application's Alire manifest (adjust the local path):

```toml
[[depends-on]]
ams_gra_oms_json = "=0.1.0-dev"

[[pins]]
ams_gra_oms_json = { path = "/absolute/path/to/ada/oms-json-support" }
```

Add `with "ams_gra_oms_json";` to its GPR file and
`with AMS_GRA_OMS_JSON_Guard;` to its Ada sources. Build using `alr -n build`.
Alternatively, make this project and the qualified provider projects available
on `GPR_PROJECT_PATH` and use `gprbuild` directly. This is a reusable source
project, not a precompiled binary library. Object files belong in a task-owned
scratch copy, not in the source checkout.

Ada 2022 is required. Qualification targets GNAT 14.2.0 and 16.1.0; no compiler
is forced by the support manifest. The standalone probe selects GNAT 14.2.0
explicitly, with GNAT 16.1.0 selected in a separate scratch consumer.

## Contract and limitations

* Incoming documents are limited to **1,048,576 bytes**, measured before the
  UTF-8 scanner or provider reader runs. Caller-owned input allocation is not
  bounded by this package.
* Before DOM construction, an iterative scanner rejects more than **128**
  simultaneous object/array openings outside strings. It does not check JSON
  grammar or matching delimiter kinds: GNATCOLL remains authoritative.
* Tree traversal rejects depth greater than 128 or more than **100,000** nodes.
  These limits are not a claim of complete memory-resource containment.
* UTF-8 and surrogate escapes are validated without normalization. Every
  outgoing string and key is checked before serialization; serialized lexical
  encoding is checked afterwards. Concurrent DOM mutation is unsupported.
* Duplicates retain GNATCOLL's **last-value-wins**, including escaped equivalent
  keys. This is not a strict duplicate-key receiver policy.
* Integer tokens retain signed 64-bit precision. Overflow is a provider parsing
  error, never a floating fallback. Integer `-0` normalizes to value zero.
  Fraction/exponent tokens are not evidence of integer or Decimal support.
* Returned parser failures and explicit guard rejections raise
  `Validation_Error`. Unexpected `Constraint_Error` is deliberately **not**
  translated: an indexing or programming defect must remain distinguishable.
  No production caller may bypass this boundary for document read/write.

The generator codec, its readiness decision, consumer vertical, hosted gate,
and real-UCI codec inventory are not yet implemented. This library alone does
not establish Task 087 readiness or complete OMS JSON conformance.