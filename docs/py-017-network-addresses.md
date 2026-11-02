# PY-017 Python address profiles

This capability is limited to the existing Task 060 IPv4 and Task 062 IPv6
constrained String domains. It is not a networking API or general regex support.

## Admission and representation

Python preflight calls `string_profile`. For IPv4, the admitted shared
`AlternatingAscii` body must factor through `factor_delimited_ascii` into the
complete four-component dotted product with exactly the five authored octet
bodies and length range 7–15. No other alternating profile is admitted. IPv6
requires the shared `Ipv6Address` classification (the pinned expression,
independent 2–45 facets and no extra authored facets). Names do not authorize
either domain. Every named restriction edge must have the same proven profile;
cycles, abstract forms, unresolved bases and incompatible edges fail closed.
Direct constrained fields and address aliases are not newly admitted.

Both profiles are frozen, slotted named dataclasses with a required exact
built-in `str` value. Spelling is preserved, including IPv6 case, zeros and unusual
schema-valid separators. There is no normalization, regex engine, platform IP
parser or dependency. Standalone IPv4 rejects padded octets; the separate IPv6
embedded-decimal branch deliberately accepts the Task 062 padded forms.

```python
ipv4 = IPv4_AddressType("198.51.100.42")
ipv6 = IPv6_AddressType("fE80::aBcD")
```

Existing model/Choice/repetition machinery recursively revalidates carriers in
address-bearing schemas, including schemas without Choice. OMS codecs use the
existing checked-string path, encode strings, enforce exact carrier types,
revalidate storage on egress and construct checked carriers on ingress.
Schema-derived qualified and indexed diagnostic paths remain intact.

## Qualification ownership

The CLI integration fixture exercises both worlds, model-only and codec-enabled
output, normal/optimized Python and standalone/package imports. It serializes
the unchanged independent `tests/task062_cases.rs` expectations to an executable
Python fixture and separately cross-checks the shared Rust scanner. Standalone
IPv4 expectations enumerate all decimal spellings of widths one through three
at every octet position, independently of generated code.

`measure-py017-uci.py` records actual results for the unchanged five-service
campaign; it has no hardcoded READY verdict. `check-py017-uci.py` is its successor
acceptance gate and executable full-schema RDMA qualification. Historical PY-016
fixtures, authentication and tests retain their original NOT READY oracle.
Fast/UCI/Sleet tests join existing source discovery; no runner lane is added.

The transport probe uses unchanged pinned Sleet, real generated service endpoints
and an independent raw OWP observer. It attributes malformed traffic using
decoder-call instrumentation, tests recovery on the same subscription,
unsubscribes, checks callback silence and verifies teardown. Broker rejection is
not reported as generated-decoder rejection.

Qualification results and immutable identities are recorded in the versioned
PY-017 evidence bundle. Readiness alone is not a claim of executable RDMA or
transport interoperability, nor does this selection campaign claim all UCI.