# Task 097-C1 — required recoverable resource contract

This records the required behavior before introducing any production exception
translation. It is a requirement, **not a claim that the checkpoint meets it**.

## Public lossless operations

The generated `Encode_*` and `Decode_*` functions include their internal
construction, occurrence/provider parsing, model construction, final validation,
and return-result construction/transfer. A failed recoverable allocation in that
scope must not return a nominal success, invalid-payload error, or unexplained
internal exception. It must propagate a distinct resource exception, leaving no
published partial String or typed result. The existing Ada `Storage_Error` is a
resource category distinct from `Codec_Error`, platform exceptions, and internal
`Program_Error`; C locale/formatter resource failures have their documented
support-package `Resource_Error`.

Owned temporary storage must remain safe to finalize. Caller environment and
thread-locale controls must remain unchanged; a subsequent supported operation
after resources are available must succeed in the same process.

A function cannot guarantee rollback of an unrelated assignment/copy by its
caller after a successful return. That separate copy must be identified as
such, not used to exclude allocation during a function's own result construction
or return transfer. The historical generated encoder failure in
`Model.Element` is **inside Encode**, even though Element belongs to the model.
It is not caller-side copying.

## Exception separation

* JSON grammar, unknown members, value/range errors: established validation and
  generated `Codec_Error` translation, without publication.
* Unsupported floating target/environment: original platform exception, before
  numerical work; no caching or normalization of caller controls.
* Recoverable allocation failure: resource exception as above.
* Genuine invariant defect, unrelated `Program_Error`, arbitrary controlled
  exception: original internal exception remains visible.
* Unrecoverable C thread-locale restoration failure: existing explicit fail-stop
  contract, not successful/recoverable restoration.

No message matching, source-line matching, test flag, allocation ordinal,
thread-local last-failure latch, or catch-all `Program_Error` translation may
classify production errors. No new production translation is made by C1's
diagnostic probes or candidate provider patch.

## Supported investigation envelope

One intended allocation fails, with other resources available for unwinding,
cleanup, recovery, and reporting. The injector disarms before throwing an Ada
resource exception or causing the real allocator to return NULL. Logging,
input/expected-value construction and debugger reporting are outside the armed
window. Independent processes own independent budgets. The NULL allocator probe
uses thread-local test state and disarms before returning NULL; it does not
raise an Ada exception across a C frame. No production code reads this state.

Arbitrary stack exhaustion, simultaneous failure of exception machinery, or
process-wide exhaustion beyond this recoverable envelope are not promised.
These exclusions do not excuse the observed ordinary 28-byte unescape failure
or allocating model copies inside a public operation.