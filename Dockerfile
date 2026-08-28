FROM node:22-bookworm-slim AS web-builder
WORKDIR /build
COPY package.json package-lock.json ./
COPY frontend ./frontend
RUN npm ci && npm run build

FROM rust:1.89-bookworm AS rust-builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
ARG BUILD_SHA=unknown
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system quiet-hours \
    && useradd --system --gid quiet-hours --home-dir /app quiet-hours \
    && mkdir -p /app/data \
    && chown -R quiet-hours:quiet-hours /app
WORKDIR /app
COPY --from=rust-builder /build/target/release/webhook-quiet-hours /usr/local/bin/webhook-quiet-hours
COPY --from=web-builder /build/dist /app/dist
USER quiet-hours
ENV PORT=8080 \
    APP_ENV=production \
    DATABASE_URL="sqlite:///app/data/quiet-hours.db?mode=rwc" \
    DIST_DIR=/app/dist \
    RUST_LOG=webhook_quiet_hours=info,tower_http=info \
    BUILD_SHA=${BUILD_SHA}
EXPOSE 8080
VOLUME ["/app/data"]
ENTRYPOINT ["/usr/local/bin/webhook-quiet-hours"]
