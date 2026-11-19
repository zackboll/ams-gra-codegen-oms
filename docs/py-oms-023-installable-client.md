# PY-OMS-023 Installable standalone Python UCI client

PY-OMS-023 makes the PY-OMS-020/021/022 receiver
(`examples/python-uci-client/`, [PY-OMS-022](py-oms-022-survivability-client.md))
an installable Python application. The new
`examples/python-uci-client/pyproject.toml` builds the wheel
**`ams-gra-oms-python-uci-client` 0.1.0**. The wheel provides the console
command **`python-uci-client = python_uci_client:main`** and depends on the
repository's existing **`ams-gra-oms-lacal-runtime==0.1.0`**, which itself
pins `websockets==17.2`. Installing it lets the receiver be deployed and run
outside the source checkout. The deployment machine uses previously
generated bindings through `--bindings`.

`python_uci_client.py` is packaged **unmodified**: no moves, renames or
source changes. The setuptools configuration lists exactly one module
(`py-modules = ["python_uci_client"]`) and disables automatic discovery.
`generate_bindings.py`, `contracts/` and `tests/` therefore cannot enter the
wheel.

| Wheel member | Purpose |
| --- | --- |
| `python_uci_client.py` | The application module (byte-identical to the source) |
| `ams_gra_oms_python_uci_client-0.1.0.dist-info/{METADATA,WHEEL,entry_points.txt,top_level.txt,RECORD}` | Packaging metadata and the console script |

The wheel is `py3-none-any` with `Requires-Python: >=3.11` and the classifier
`Private :: Do Not Upload`, so PyPI refuses it. It contains no generated
`model.py`/`service_api.py`/`service_codec.py`, contracts, schemas, Sleet
files, certificates, credentials or test fixtures. It is not published.

## What did not change

- the generator and the generated-bindings format (manifest formats 1/2/3);
- the runtime sources and dependencies;
- the three modes, flags, pinned UCI 2.5/2.6 identities, type and codec
  validation, message/topic bindings, output, exit statuses;
- buffering, callback order, SID ownership, teardown and TLS verification.

The source usage `python examples/python-uci-client/python_uci_client.py …`
is unchanged. No CI workflow, Sleet, Rust, Ada or C++ file changed. This
change adds no reconnect, retry, replay, dynamic schema discovery, generic
subscription configuration, new transport, message type or AHT-TLS-005 work.
OMS Issue #212 stays deferred.

The [README](../examples/python-uci-client/README.md) section 7 documents:

- developer usage;
- wheel building;
- offline installation;
- binding generation and deployment;
- the installed command;
- verified WSS;
- limitations;
- a complete checkout-to-deployment example.

That example was executed literally (with a local generator and a refused
endpoint), building from copies and leaving the checkout free of tracked,
untracked or ignored output.

## Test harness

`tests/installed.py` is test support only and is never packaged. Per test
module it:

1. copies the runtime and application sources to an **external** staging
   directory and builds both wheels with `pip wheel` (setuptools backend).
   It downloads the runtime's own `Requires-Dist` (`websockets==17.2`) as a
   wheel, giving a three-file local wheelhouse;
2. creates a **fresh** virtual environment of the running interpreter and
   installs `ams-gra-oms-python-uci-client==0.1.0` with `--no-index
   --only-binary=:all: --find-links WHEELHOUSE`. It uses no pip configuration
   (`PIP_CONFIG_FILE=/dev/null`) and points HTTP(S) proxies at a closed port;
3. copies all six generated binding sets (formats 1/2/3 × UCI 2.5/2.6) to a
   separate **deployment** directory.

The installed `bin/python-uci-client` runs in a minimal environment. That
environment has no `PYTHONPATH`/`PYTHONHOME`, sets `PYTHONNOUSERSITE=1`,
uses a private `HOME` and puts only the environment's `bin` on `PATH`. The
working directory is outside the repository and holds **decoy**
`python_uci_client.py` and `oms_lacal.py` modules. Importing either decoy
exits `97`. Under `-O` the harness sets `PYTHONOPTIMIZE=1` for the installed
command.

`tests/support.py` gains one opt-in hook, `support.LAUNCHER`. When it is set,
`run_cli` launches the installed console script with that environment and
working directory instead of `sys.executable python_uci_client.py`. The
default (`None`) launcher is unchanged, so every existing suite still runs
the source file.

## New suites

`test_installed_cli.py` (8) needs no broker:

- **wheel**: exactly the six members listed above; the packaged module is
  byte-identical to the source; `console_scripts` contains only
  `python-uci-client = python_uci_client:main`; Name, Version,
  Requires-Python, `Requires-Dist: ams-gra-oms-lacal-runtime==0.1.0`, License
  and the private classifier are checked; the runtime still pins only
  `websockets==17.2`; the suite leaves the in-tree ignored state (`build/`,
  `*.egg-info`, caches) unchanged.
- **offline install**: no download; installed set is exactly the client,
  runtime and websockets (plus venv seeds); `pip check` is clean;
  `include-system-site-packages = false`. Control: the same offline
  configuration without the wheelhouse cannot resolve the client.
- **import origin**:
  - `python_uci_client`, `oms_lacal` and `websockets` load from the
    environment's `site-packages`;
  - `sys.prefix` is the environment;
  - the user site is disabled;
  - no `sys.path` entry is inside the repository;
  - the console script is the generated launcher for `main`;
  - the decoys are armed: a plain `python -c 'import python_uci_client'` in
    that directory exits `97`.
- **import audit**: one full installed session (`PYTHONVERBOSE=1`) loads
  bindings and attempts a connection. No module is loaded from the
  repository, and the client and runtime come from `site-packages`.
- **help/usage**: `--help` is identical to the module's `HELP` (exit `0`).
  Twelve malformed or contradictory invocations exit `2` with no stdout.
- **deployed bindings**: each of the six deployed sets is verified, loaded
  and executed in its own mode. Each exits `4` (connection refused) with the
  mode's exact `CLOSED …` line.
- **refusals**: 20 cases exit `3` with the documented diagnostics:
  - every set under the other release;
  - every set in both other modes;
  - a modified `service_codec.py`;
  - a missing directory.
- **optimized**: under `PYTHONOPTIMIZE=1` the installed module is cached as
  `.opt-1.pyc` in `site-packages`, and `--help`, a mode-A session (exit 4)
  and a release refusal (exit 3) behave identically.

`test_installed_real_sleet.py` (6) runs the **installed console script**
against the unchanged pinned Sleet `e38f61d8…`. It reuses the
PY-OMS-020/021/022 helpers unchanged:

- broker configuration;
- independent raw-websocket publisher and observer;
- Task092 RDMA and PY-016 Log/SurvivabilityRiskLevel goldens;
- session and broker-ownership assertions.

Only the launcher and the binding paths (the deployed copies) differ. Every
test asserts that the installed launcher was used. The six tests are:

- `--with-log --with-survivability` for **UCI 2.5** and **UCI 2.6**:
  - three SUB acks and distinct SIDs on one broker connection after one INIT;
  - nine typed callbacks in interleaved publication order, witnessed
    independently on the wire;
  - broker vs decoder rejection;
  - three UNSUBs in reverse order, `CLOSED … 3/0 3/0 3/1`;
  - a clean broker exit.
- mode A (UCI 2.5) and mode B (UCI 2.6) sessions with the PY-OMS-020/021
  assertions;
- an unauthorized `uci.survivability` exits `5`, with RDMA and Log UNSUBed;
- SIGTERM through the console-script launcher exits `143` with all three
  SIDs released;
- verified **WSS** (UCI 2.6, three-message) through the existing TLS
  terminator with isolated trust and `--ca-file`, where SNI carries
  `localhost`. Its negative control is a hostname mismatch, refused with
  exit `4` before INIT.

This is a representative subset. It does not duplicate the PY-OMS-020/021/022
four-cell campaigns, which still run against the source file.

## Qualification

`python/qualification/sleet/test_python_uci_client.py` is the existing
source-discovered pinned-Sleet lane. It now requires two more suites with
exact counts (8 and 6) and PY-OMS-023 markers, still with no skips. All
twelve suites run on Python 3.11 and 3.13, each in normal and `-O` mode;
under `-O` the installed command runs with `PYTHONOPTIMIZE=1`. All
wheelhouses, environments, deployments and generated bindings live in
temporary storage outside the repository. The lane still asserts that the
application tree is clean afterwards.
