# Task 096 research probes

These files do not enable or implement a production-generated Float64 codec.
The unmodified production generator emits only the model/API checkpoint.
`task096_adapter.*`, `task096_boundary.*`, and `boundary.c` are handwritten,
platform-limited research; the occurrence child is a research-only view of
original node-owned numeric text.

The full command, target identities, source observations, measured limitations,
design and publication boundary are in the Task 096 report. The standalone
numerical runner is deliberately not named a full-boundary certification.

Fast oracle/accounting tests (no Ada dependencies):

```sh
python3 -m unittest discover -s /absolute/repo/tests/ada-float64-probe -p 'test_*.py' -v
```

Regenerate the frozen corpus into a **nonexisting** scratch directory:

```sh
python3 /absolute/repo/tests/ada-float64-probe/oracle.py /absolute/scratch/corpus
```

Then compare its `manifest.json` with the committed `corpus-manifest.json`.
Expect 27,167 unique decimal spellings and 20,474 unique finite bit patterns.
Expected bits come only from bounded exact integer/rational arithmetic;
`crosscheck.rs` is a cross-check, never the source of expectations.

Complete fresh-home execution:

```sh
nohup python3 /absolute/repo/scripts/task096/job.py /absolute/scratch/job \
  -- python3 /absolute/repo/scripts/task096/qualify.py \
  --scratch /absolute/scratch/run > /absolute/scratch/launch.log 2>&1 < /dev/null &
```

Prerequisites are exact installed external GNAT 14.2.0, Alire 2.1.1, Rust,
GCC, Linux/x86-64 glibc 2.41, German UTF-8 locale, and network access to Alire
archives/index. Native GNAT 16.1.0, GPRbuild 26.0.1 and GNATCOLL 26.0.0 are
resolved in new task-owned Alire homes. This runner does not install system
packages or alter global locale. Each provider/consumer build is isolated.

Validate completed evidence without trusting a marker alone:

```sh
python3 /absolute/repo/scripts/task096/account.py --evidence /absolute/scratch/run
```

Any missing/duplicate/changed output or interrupted stage fails accounting.
Existing DOM/Ada conversion and writer discrepancies are intentionally measured,
not rewritten as numerical passes. The final disposition concerns the complete
SAX/strtod_l/typed structural emission path on the named target only.