# syntax=docker/dockerfile:1

FROM harbor-eq-dc.sw-ci.kraken.as/dockerhub-proxy/ubuntu:22.04 as final

# Install essential system dependencies for Rust development + docs builds
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        cmake \
        curl \
        git \
        git-lfs \
        openssh-client \
        pkg-config \
        coreutils \
    && rm -rf /var/lib/apt/lists/*

# Install the stable Rust toolchain for local development.
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
ENV PATH="/opt/cargo/bin:${PATH}"
ARG RUST_TOOLCHAIN=stable
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain ${RUST_TOOLCHAIN} \
    && rustup component add clippy rustfmt llvm-tools-preview

# Install Rust nightly toolchain for sanitizers and Miri
RUN rustup toolchain install nightly --component rust-src,miri

# Ferrocene setup reserved for later use.
#
# ARG FERROCENE_RELEASE="stable-26.02.0"
# ENV RUSTUP_HOME=/opt/rustup
# ENV CARGO_HOME=/opt/cargo
# ENV PATH="/root/.cargo/bin:${PATH}"
# RUN cat > /criticalup.toml <<EOF
# manifest-version = 1
#
# [products.ferrocene]
# release = "${FERROCENE_RELEASE}"
# packages = [
#   "cargo-\${rustc-host}",
#   "rustc-\${rustc-host}",
#   "rust-std-\${rustc-host}",
#   "clippy-\${rustc-host}",
#   "rustfmt-\${rustc-host}",
# ]
# EOF
# RUN --mount=type=secret,id=criticalup_token,target=/run/secrets/criticalup_token \
#     CRITICALUP_TOKEN=$(cat /run/secrets/criticalup_token) && \
#     /root/.cargo/bin/criticalup auth set "${CRITICALUP_TOKEN}" && \
#     /root/.cargo/bin/criticalup install && \
#     /root/.cargo/bin/criticalup auth remove || true

# Install cargo tools via binstall
RUN cargo install cargo-binstall --locked \
    && cargo-binstall cargo-nextest --locked --version 0.9.136 \
    && cargo-binstall cargo-audit --locked --version 0.22.1 \
    && cargo-binstall cargo-deny --locked --version 0.19.7 \
    && cargo-binstall cargo-hack --locked --version 0.6.44 \
    && cargo-binstall ci-cargo --locked --version 0.2.1 \
    && cargo-binstall cargo-criterion --locked --version 1.1.0 \
    && cargo-binstall cargo-fuzz --locked --version 0.11.0 \
    && cargo-binstall cargo-llvm-cov --locked --version 0.8.7 \
    && cargo-binstall cargo-semver-checks --locked --version 0.47.0 \
    && cargo-binstall cargo-vet --locked --version 0.10.2 \
    && cargo-binstall cargo-udeps --locked --version 0.1.61 \
    && cargo-binstall cargo-cyclonedx --locked --version 0.5.9 \
    && cargo-binstall cargo-release --locked --version 1.1.2 \
    && cargo-binstall sccache --locked --version 0.15.0


# Install Python 3.14
COPY --from=harbor-eq-dc.sw-ci.kraken.as/dockerhub-proxy/astral/uv:latest /uv /uvx /bin/
ARG PYTHON_VERSION=3.14
RUN uv python install ${PYTHON_VERSION}
ENV PATH="/root/.local/bin:${PATH}"
ENV PATH="/opt/venv/bin:${PATH}"

# To enable github actions/checkout@v4
RUN mkdir -m 1777 /__w && mkdir -m 1777 -p /runner/_work
