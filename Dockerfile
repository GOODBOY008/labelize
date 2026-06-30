FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features serve --recipe-path recipe.json
COPY . .
RUN cargo build --release  --features serve --bin labelize

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin labelize

COPY --from=builder /app/target/release/labelize /usr/local/bin/labelize

USER labelize
EXPOSE 8080
ENTRYPOINT ["labelize"]
CMD ["serve", "--host", "0.0.0.0", "--port", "8080"]
