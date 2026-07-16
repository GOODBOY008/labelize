# syntax=docker/dockerfile:1

# ---- Build stage ----------------------------------------------------------
# Alpine's Rust builds natively against musl, so `cargo build` produces a
# fully static binary with no dynamic-linking or libc-loading at startup.
FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Cache dependency compilation: copy manifests first, build a stub, then the
# real sources. A source-only change reuses the dependency layer.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release --features serve --bin labelize \
    && rm -rf src

COPY src ./src
# Touch so cargo notices the real sources over the stub timestamps.
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --features serve --bin labelize \
    && strip target/release/labelize

# ---- Runtime stage --------------------------------------------------------
# `scratch` = nothing but the static binary. Fonts and the playground HTML are
# embedded in the binary, so no extra files are needed at runtime.
FROM scratch

COPY --from=builder /app/target/release/labelize /labelize

EXPOSE 8080
ENTRYPOINT ["/labelize"]
CMD ["serve", "--host", "0.0.0.0", "--port", "8080"]