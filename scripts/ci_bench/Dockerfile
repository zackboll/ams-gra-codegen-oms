# Trusted preparation only. No feature sources or Actions environment in image.
FROM rust:1.95.0-trixie@sha256:f49565f188ee00bc2a18dd418183f2c5f23ef7d6e691890517ed341a598f67c3
RUN apt-get update && apt-get install --no-install-recommends -y \
    python3 gnat-14 gnat curl ca-certificates unzip git time util-linux \
    && rm -rf /var/lib/apt/lists/*
RUN curl --fail --location --retry 3 -o /tmp/alire.zip \
    https://github.com/alire-project/alire/releases/download/v2.1.1/alr-2.1.1-bin-x86_64-linux.zip \
    && echo '09c66bcd8c35dd4b97b72c3d9b76e44caa6964a2db35aba069f396f00f1f64c7  /tmp/alire.zip' | sha256sum --check --strict \
    && unzip -q /tmp/alire.zip -d /opt/alire && rm /tmp/alire.zip
ENV PATH="/opt/alire/bin:${PATH}" PYTHONDONTWRITEBYTECODE=1
COPY . /trusted
WORKDIR /trusted
RUN cargo +1.95.0 fetch --locked && python3 scripts/ci_bench/provision.py
RUN useradd --uid 10001 --create-home bench \
    && chmod -R a+rX /usr/local/cargo /seed /trusted
USER 10001:10001
WORKDIR /work