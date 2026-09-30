# syntax=docker/dockerfile:1.7
#
# zafe-relay image. Build from the repository root:
#   docker build -f infra/relay/Dockerfile -t zafe-relay .
#
# Runtime contract:
#   - listens on 0.0.0.0:$PORT (default 8080); ZAFE_RELAY_LISTEN overrides
#   - SQLite at $ZAFE_RELAY_DB (default /data/relay.sqlite): mount a volume at /data
#   - GET /health -> 200 "ok"
#   - optional ZAFE_FCM_SERVICE_ACCOUNT=/path/to/key.json (mount it as a secret file;
#     never bake it into the image)
#   - the relay process runs as the unprivileged `zafe` user (uid 10001)

ARG RUST_VERSION=1.97

FROM rust:${RUST_VERSION}-slim-bookworm AS build
WORKDIR /src
# The whole workspace manifest is needed to resolve the lockfile, so every member's
# sources are copied (.dockerignore keeps the Flutter app, docs and targets out).
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p zafe-relay \
    && install -Dm755 target/release/zafe-relay /out/zafe-relay

FROM debian:bookworm-slim
# sqlite3: online backups from inside the container (`sqlite3 /data/relay.sqlite
# ".backup /data/backup.sqlite"`). ca-certificates is not needed: the relay's HTTPS
# client (FCM) uses the bundled webpki roots.
RUN apt-get update \
    && apt-get install -y --no-install-recommends sqlite3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 zafe \
    && useradd --system --uid 10001 --gid zafe --home-dir /data --shell /usr/sbin/nologin zafe \
    && mkdir -p /data && chown zafe:zafe /data
COPY --from=build /out/zafe-relay /usr/local/bin/zafe-relay
COPY infra/relay/entrypoint.sh /usr/local/bin/entrypoint.sh

ENV PORT=8080 \
    ZAFE_RELAY_DB=/data/relay.sqlite \
    RUST_LOG=zafe_relay=info
VOLUME ["/data"]
EXPOSE 8080
STOPSIGNAL SIGTERM
# Starts as root only to fix the ownership of a freshly mounted volume (Fly volumes and
# bind mounts arrive root-owned), then drops to `zafe`. With `--user 10001` it runs the
# relay directly.
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
