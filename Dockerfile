# ---- build ----
FROM rust:1-slim AS builder
WORKDIR /app

# build-essential é necessário para compilar `ring` (TLS via rustls).
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release --bin server

# ---- runtime ----
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/server /usr/local/bin/server
# Templates e migrations são embutidos no binário; só os estáticos vão para o runtime.
COPY --from=builder /app/static ./static
ENV PORT=3000
EXPOSE 3000
CMD ["server"]
