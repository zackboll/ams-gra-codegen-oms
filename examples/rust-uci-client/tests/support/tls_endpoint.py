#!/usr/bin/env python3
"""Test-only OpenSSL TLS terminator. Forward bytes, never interpret OWP/JSON.

No host trust changes. The Rust harness sets SSL_CERT_FILE and SSL_CERT_DIR on
each child only. Fixed certificate windows make time-based controls repeatable.
"""
import argparse
import asyncio
from pathlib import Path
import ssl
import subprocess


def generate(directory):
    directory.mkdir(parents=True, exist_ok=True)

    def openssl(*args):
        result = subprocess.run(["openssl", *map(str, args)], capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError(f"openssl {args}: {result.stderr}")

    for ca in ("ca", "rogue-ca"):
        openssl("req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "3650",
                "-keyout", directory / f"{ca}.key", "-out", directory / f"{ca}.pem",
                "-subj", f"/CN=RUST-OMS-009 {ca}",
                "-addext", "basicConstraints=critical,CA:TRUE",
                "-addext", "keyUsage=critical,keyCertSign,cRLSign")
    (directory / "empty-roots").mkdir(exist_ok=True)
    (directory / "index").write_text("")
    (directory / "serial").write_text("1000\n")
    for name, ca, host, start, end in (
        ("valid", "ca", "localhost", "20200101000000Z", "20400101000000Z"),
        ("hostname", "ca", "wrong.invalid", "20200101000000Z", "20400101000000Z"),
        ("expired", "ca", "localhost", "20000101000000Z", "20010101000000Z"),
        ("future", "ca", "localhost", "20900101000000Z", "20910101000000Z"),
        ("untrusted", "rogue-ca", "localhost", "20200101000000Z", "20400101000000Z"),
    ):
        openssl("req", "-new", "-newkey", "rsa:2048", "-nodes", "-subj", f"/CN={host}",
                "-keyout", directory / f"{name}.key", "-out", directory / f"{name}.csr")
        config = directory / "ca.cnf"
        config.write_text(f"""[ca]
default_ca=local
[local]
database={directory}/index
serial={directory}/serial
new_certs_dir={directory}
certificate={directory}/{ca}.pem
private_key={directory}/{ca}.key
default_md=sha256
policy=policy
unique_subject=no
x509_extensions=server
[policy]
commonName=supplied
[server]
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:{host}
""")
        openssl("ca", "-batch", "-notext", "-config", config, "-in", directory / f"{name}.csr",
                "-out", directory / f"{name}.pem", "-startdate", start, "-enddate", end)
    # A real intermediate signs the leaf, but is deliberately omitted from the
    # served chain. Trusting the root cannot repair the missing issuer.
    openssl("req", "-new", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=Intermediate",
            "-keyout", directory / "intermediate.key", "-out", directory / "intermediate.csr")
    extensions = directory / "intermediate.ext"
    extensions.write_text("basicConstraints=critical,CA:TRUE,pathlen:0\nkeyUsage=critical,keyCertSign,cRLSign\n")
    openssl("x509", "-req", "-in", directory / "intermediate.csr", "-CA", directory / "ca.pem",
            "-CAkey", directory / "ca.key", "-CAcreateserial", "-days", "3650",
            "-extfile", extensions, "-out", directory / "intermediate.pem")
    extensions.write_text("basicConstraints=critical,CA:FALSE\nsubjectAltName=DNS:localhost\n"
                          "keyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n")
    openssl("x509", "-req", "-in", directory / "valid.csr", "-CA", directory / "intermediate.pem",
            "-CAkey", directory / "intermediate.key", "-CAcreateserial", "-days", "3650",
            "-extfile", extensions, "-out", directory / "chain.pem")
    (directory / "chain.key").write_bytes((directory / "valid.key").read_bytes())


async def serve(args):
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(args.directory / f"{args.identity}.pem",
                            args.directory / f"{args.identity}.key")

    def handshake_attempt(_socket, hostname, _context):
        with (args.directory / f"{args.epoch}-handshakes.txt").open("a") as evidence:
            evidence.write(f"{hostname}\n")

    context.set_servername_callback(handshake_attempt)
    counter = 0

    async def forward(reader, writer):
        nonlocal counter
        counter += 1
        connection = counter
        backend_writer = None
        try:
            backend_reader, backend_writer = await asyncio.open_connection("127.0.0.1", args.backend)

            async def copy(source, destination, direction):
                with (args.directory / f"{args.epoch}-{connection}-{direction}.bin").open("wb") as wire:
                    while data := await source.read(65536):
                        wire.write(data)
                        wire.flush()
                        destination.write(data)
                        await destination.drain()

            tasks = [asyncio.create_task(copy(reader, backend_writer, "client")),
                     asyncio.create_task(copy(backend_reader, writer, "server"))]
            try:
                await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
            finally:
                for task in tasks:
                    task.cancel()
                await asyncio.gather(*tasks, return_exceptions=True)
        except (OSError, ssl.SSLError):
            pass
        finally:
            writer.close()
            if backend_writer is not None:
                backend_writer.close()

    server = await asyncio.start_server(forward, "127.0.0.1", args.port, ssl=context,
                                        ssl_handshake_timeout=5)
    print(f"READY {server.sockets[0].getsockname()[1]}", flush=True)
    async with server:
        await server.serve_forever()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--backend", type=int)
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--identity", default="valid")
    parser.add_argument("--epoch", default="1")
    args = parser.parse_args()
    if args.generate:
        generate(args.directory.resolve())
    else:
        asyncio.run(serve(args))


if __name__ == "__main__":
    main()