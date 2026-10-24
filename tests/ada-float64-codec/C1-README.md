# C1 allocation-boundary investigation (not production qualification)

Disposition and causal account:
`/home/zboll/git/ams-gra-codegen-oms-task097/docs/task-097-ada-float64-codec.md`.
Required contract:
`/home/zboll/git/ams-gra-codegen-oms-task097/docs/task-097-c1-resource-contract.md`.

These diagnostic files deliberately observe an unresolved resource-origin
`Program_Error`. A diagnostic runner completing does **not** certify that
exception as acceptable. No production code reads injector state or matches
exception text.

## Reproduce in fresh directories

Use `scripts/task097/job.py` to supervise the commands; set
`PYTHONDONTWRITEBYTECODE=1`. All paths passed to runners must be absolute and
attempt directories must not exist. `c1_investigate.py` copies the preserved
Task 097 source/fixture and verified provisioned provider, strips provider
objects, builds pristine source, records original allocation failure and a
provider-only reproducer, and makes debugger traces. It requires GDB and the
Task 097-owned provisioning directories. This is workstation reproduction,
not a fresh hosted provisioning claim.

```
python3 /home/zboll/git/ams-gra-codegen-oms-task097/scripts/task097/c1_investigate.py \
  --scratch /home/zboll/task097-scratch/c1/new-pristine14 \
  --compiler 14
```

The original full-loop trace assumes the historical GNAT 16 sequence. GNAT 14
can stop earlier at an encoder model-copy failure. A completed GDB process is
not evidence that its breakpoint was reached; require the first-failure marker
and inspect the inferior's signal/exit. The separately preserved C1 traces
capture both cases and the second finalizer exception.

`c1_candidate.py --scratch NEW --pristine PRISTINE` applies only
`c1_provider-transition.patch`, verifying the exact original source hash in
each isolated provider source tree and rebuilding all consumers. It runs the
old Ada wrapper and the actual C NULL-return interposer. This patch is a
**candidate only**, not linked by production GPRs and not installed in an Alire
cache.

`c1_scope.py --scratch NEW --candidate CANDIDATE` records reached allocation
counts/sizes, no-fault traces, provider/generated/model-only recovery and
exception-separation controls across Normal/Check/Ignore. Its model-copy cases
retain an unacceptable `Program_Error`; use the report, not runner exit alone.
`--provider-only --policies Normal --expect-pristine-unescape-error` permits a
pristine-provider NULL-return comparison with the expected diagnostic red.

## Injection semantics

The original `allocation_faults.*` exports an Ada wrapper for the Ada runtime
allocation entrypoint, not libc `malloc`. The C1 NULL variant arms only the
next underlying real malloc/realloc in the same thread and forwards to the
actual runtime allocator. The C interposer returns NULL; the runtime raises
its own `Storage_Error`. It never throws through C. Counters/expensive reports
are read only after disarming; independent processes own independent budgets.

`c1_allocator_controls.c` with `c1_malloc_fault.c` validates thread isolation,
one-shot behavior, recovery, and realloc retaining ownership on failure.
Compile with `-std=c11 -fno-builtin -Wall -Wextra -Werror -pthread` under each
qualified C compiler at O0/O2. These probes use glibc `__libc_*` deliberately
and make no portability claim beyond the test target.

## Outstanding design requirement

The private repeated sequence exposes only value-returning `Model.Element`.
Returning a Choice/Record containing a Binary vector requires controlled
copies on measured compiler paths. A recoverable `Adjust` allocation failure
becomes `Program_Error` within that function. Solving this without guessing
exception origin requires an explicit ownership/traversal/runtime design and
full model/API preservation review. It cannot be closed by the localized
GNATCOLL small-to-big transition patch.