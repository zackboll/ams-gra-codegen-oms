Standalone C++17 UCI client deployment bundle (CPP-OMS-013)

Use manifest/deployment-manifest.json to identify the executable. Verify the
bundle came from your trusted release channel before running any of its files.
SHA256SUMS detects corruption; it is not a signature or build attestation.

Offline preflight (Python 3.11+ and binutils on Linux x86-64):
  python3 /absolute/bundle/check.py --release 2.6 --profile C --variant tls /absolute/bundle
Substitute the expected release (2.5 or 2.6), profile (A/B/C) and variant
(plain/tls) from your deployment plan. Preflight does not dial the broker.

Run the selected executable as a non-root user from any working directory.
Use --url URL --service-id ID --duration-seconds N --max-messages N. Profile A
needs no extra switch, B needs --with-rdma, C needs --with-rdma and
--with-survivability. UCI release is fixed at build time, not a client flag.
Plain supports ws:// only. TLS supports ws:// and verified wss://. For wss://
only, use --ca-file /absolute/operator-ca.pem for additional PEM CA certs
if needed. System trust remains active; SSL_CERT_FILE and SSL_CERT_DIR can
affect its selection. Use a matching DNS SAN (with SNI) or IP SAN (no SNI).
No CN fallback, insecure option or automatic downgrade. Do not put keys,
credentials or development/test trust anchors in this bundle.

CONNECTED confirms startup through INIT/+OK/INFO. READY means all enabled
SUBs were acknowledged and the connection checked; it is not delivery or
future availability. Look for RECEIVED, DECODE-ERROR, STOP, UNSUBSCRIBED
and CLOSED. SIGINT/SIGTERM request clean shutdown; capture the actual exit
status and check CLOSED resources-released as well as close=. The pinned
Sleet broker does not reciprocate WebSocket Close; verified WSS can deliver
messages and release resources yet exit 6. Do not treat 6 as success.

Runbook: docs/task-cpp-oms-013-deployment-readiness.md in the release source.