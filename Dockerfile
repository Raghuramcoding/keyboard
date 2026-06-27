# T.C.K (TalentCloud Keyboard) — production image.
# Stage 1 builds the WASM UI (Trunk) and the native server (cargo); stage 2 is a
# slim runtime that serves the bundle and runs the terminal/AI/GitHub backend.

# ---------- build ----------
FROM rust:1-bookworm AS build
WORKDIR /app

# WASM target + Trunk (prebuilt binary; Trunk fetches wasm-bindgen/wasm-opt itself).
ARG TRUNK_VERSION=v0.21.14
RUN rustup target add wasm32-unknown-unknown \
 && curl -sSL "https://github.com/trunk-rs/trunk/releases/download/${TRUNK_VERSION}/trunk-x86_64-unknown-linux-gnu.tar.gz" \
    | tar -xz -C /usr/local/bin

COPY . .

# Build the WebAssembly UI, then the server binary.
RUN cd crates/tck-ui && trunk build --release
RUN cargo build -p tck-server --release

# ---------- runtime ----------
FROM debian:bookworm-slim
# ca-certificates: outbound HTTPS to GitHub/Anthropic/providers. bash: the PTY shell.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates bash git curl \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -m -u 10001 tck

WORKDIR /app
COPY --from=build /app/target/release/tck-server /usr/local/bin/tck-server
COPY --from=build /app/crates/tck-ui/dist /app/dist

ENV TCK_DIST=/app/dist \
    TCK_ADDR=0.0.0.0:3000

# Run as an unprivileged user — the terminal grants a shell, so limit its reach.
USER tck
EXPOSE 3000
CMD ["tck-server"]
