# Deploying the relay (testnet)

The relay is one small binary with one SQLite file. It is blind (public keys,
ciphertext and metadata only), so the host never holds anything that spends or
decrypts funds, but it does see who talks to which mailbox and when: pick a host you
trust with that metadata, and keep access logs off.

Nothing in this directory has been deployed. Two paths, pick one:

| | Fly.io | VPS (Caddy + systemd) |
|---|---|---|
| TLS | Fly's edge (`*.fly.dev` or your domain) | Caddy + Let's Encrypt, automatic |
| Files | `fly.toml`, `Dockerfile` | `vps/` |
| Cost (roughly) | shared-cpu-1x 256 MB + 1 GB volume | any 1 vCPU / 512 MB box |
| You do | `fly auth login`, create app/volume, secrets | a server, a DNS record |

Runtime contract (both paths):

- `GET /health` returns `200 ok` when the database answers.
- `ZAFE_RELAY_LISTEN` (address) or `PORT` (then `0.0.0.0:$PORT`), default `127.0.0.1:8787`.
- `ZAFE_RELAY_DB`: SQLite path. It runs in WAL mode, so the `-wal`/`-shm` files sit next
  to it: the whole directory must be on persistent storage.
- FCM pushes (optional): `ZAFE_FCM_SERVICE_ACCOUNT=/path/key.json` or
  `ZAFE_FCM_SERVICE_ACCOUNT_JSON='<the JSON>'`. Never commit the key or bake it into an
  image (`.dockerignore` excludes the usual names). Without it pushes are only logged
  and phones rely on background checks.
- Rate limits (on by default): 300 requests/min per signing key (charged only after the
  signature verifies, burst 150) and 1200/min per client IP (burst 600); over them the
  relay answers `429` with `Retry-After`, and the app says the relay is busy. Tune with
  `ZAFE_RELAY_KEY_RATE` / `ZAFE_RELAY_IP_RATE` (per minute, `0` = off), or
  `ZAFE_RELAY_LIMITS=off`. Behind a proxy set `ZAFE_RELAY_CLIENT_IP_HEADER` (Fly:
  `fly-client-ip`, already in `fly.toml`; Caddy: `x-forwarded-for`, in the systemd unit),
  or every client shares the proxy's address. Never point it at a header clients can set
  directly.
- Request bodies are capped at 1 MiB (`413` above it).
- Long polls (`POST /v1/wait`): an open app holds one request for up to **25 s** so other
  members' activity reaches it at once. Any proxy in front must let a response take
  longer than that: Caddy's `reverse_proxy` has no response timeout by default (keep it
  that way, or set it above 40 s), and Fly's proxy idles out after 60 s. Each open app
  counts as one concurrent request on Fly, so `fly.toml` sets the concurrency limits to
  1000/1500. The relay caps waits at 2 per signing key and 4096 in total (over them:
  `429`); an older relay without the endpoint just makes apps fall back to polling.
- Storage quotas per vault (on by default): 10,000 undelivered envelopes per member,
  256 MiB of undelivered envelopes and 512 MiB of vault log. A write over a quota is
  refused whole with `507 Insufficient Storage`, and the app says the relay's storage for
  the vault is full. Undelivered envelopes expire after 30 days (hourly pruning), which
  frees their share; the log is kept forever. Tune with `ZAFE_RELAY_MAX_INBOX` (envelopes),
  `ZAFE_RELAY_MAX_DELIVERY_MB` / `ZAFE_RELAY_MAX_LOG_MB` (MiB), `0` = that quota off, or
  `ZAFE_RELAY_QUOTAS=off`. A database from schema 1 gets the quota counters added at
  startup (one scan); keep a backup before upgrading as usual.
- SIGTERM shuts down gracefully.
- Exactly **one** instance per database. Don't scale out.

## The image

```bash
# from the repository root
docker build -f infra/relay/Dockerfile -t zafe-relay .
docker run --rm -p 8080:8080 -v zafe-relay-data:/data zafe-relay
curl http://127.0.0.1:8080/health     # ok
```

Debian slim, the relay binary and `sqlite3` (for backups). The entrypoint starts as root
only to `chown` the data volume (Fly volumes and bind mounts arrive root-owned), then
runs the relay as `zafe` (uid 10001) through `setpriv`. `docker run --user 10001 ...`
skips that step if the volume is already writable by uid 10001.

## Path A: Fly.io

One-time setup (you, logged in to your Fly account):

```bash
fly auth login
# 1. pick a unique app name and region, and write them into infra/relay/fly.toml
fly apps create zafe-relay-testnet
# 2. the volume (same region as primary_region)
fly volumes create zafe_relay_data --app zafe-relay-testnet --region fra --size 1
# 3. optional: FCM pushes
fly secrets set --app zafe-relay-testnet \
  ZAFE_FCM_SERVICE_ACCOUNT_JSON="$(cat ~/.config/zafe/fcm-service-account.json)"
```

Deploy (from the repository root; the build context is the Cargo workspace):

```bash
fly deploy --config infra/relay/fly.toml --dockerfile infra/relay/Dockerfile --ha=false .
curl https://zafe-relay-testnet.fly.dev/health
```

`--ha=false` keeps it to one machine (the default would create two, and a second machine
would have its own empty volume). The template turns auto-stop off: the relay must stay
up to receive envelopes and send pushes.

Own domain (optional): `fly certs add relay.example.com --app zafe-relay-testnet`, then
create the DNS records `fly certs show` lists (CNAME to `<app>.fly.dev`, or A/AAAA).

Backups on Fly: volumes get daily snapshots (kept 5 days by default; `fly volumes
snapshots list <vol id>`). For an extra copy:

```bash
fly ssh console --app zafe-relay-testnet -C \
  "sqlite3 /data/relay.sqlite '.backup /data/backup.sqlite'"
fly ssh sftp get /data/backup.sqlite ./relay-backup.sqlite --app zafe-relay-testnet
```

## Path B: a VPS (Debian/Ubuntu) with Caddy

You provide: a server with a public IP, ports 80 and 443 open, and a DNS **A/AAAA
record** for the relay's domain pointing at it.

```bash
# 1. the binary (build on the server, or extract it from the image)
cargo build --release --locked -p zafe-relay
sudo install -m755 target/release/zafe-relay /usr/local/bin/zafe-relay
#   or: id=$(docker create zafe-relay) && docker cp $id:/usr/local/bin/zafe-relay . && docker rm $id

# 2. the service (DynamicUser, state in /var/lib/zafe-relay, localhost:8787)
sudo cp infra/relay/vps/zafe-relay.service /etc/systemd/system/
#   optional FCM: key at /etc/zafe-relay/fcm-service-account.json (root, 0600), then
#   uncomment LoadCredential/Environment in the unit
sudo systemctl daemon-reload && sudo systemctl enable --now zafe-relay
curl http://127.0.0.1:8787/health

# 3. Caddy (https://caddyserver.com/docs/install), then set your domain in the Caddyfile
sudo cp infra/relay/vps/Caddyfile /etc/caddy/Caddyfile
sudo systemctl reload caddy
curl https://relay.example.com/health

# 4. nightly backups (see below)
sudo apt install sqlite3
sudo cp infra/relay/vps/zafe-relay-backup.{service,timer} /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now zafe-relay-backup.timer
```

Caddy gets and renews the certificate on its own once DNS resolves to the server.

## Backups

What's lost with the database: mailboxes, member lists, undelivered envelopes and the
encrypted vault logs. Members keep their keys (funds are safe), but today there is no way
to re-seed a relay from members' devices, so a lost database means vaults stop
coordinating. Back it up.

- **Nightly `sqlite3 .backup`** (VPS: `vps/zafe-relay-backup.{service,timer}`): an online,
  consistent copy through SQLite's backup API, checked with `integrity_check`, gzipped,
  kept 14 days in `/var/backups/zafe-relay`. Ship that directory off the machine (restic,
  rclone, the provider's snapshots).
- **Litestream** (suggestion, either path): streams the WAL to S3-compatible storage
  continuously, with point-in-time restore. Run it as a sidecar/second process with
  `litestream replicate /data/relay.sqlite s3://bucket/relay`. Not wired up here.

Restore: stop the relay, replace `relay.sqlite` (delete stale `-wal`/`-shm`), start it.

## Pointing the app at it

Build the app with the testnet preset and the relay URL:

```bash
flutter build apk --dart-define=ZAFE_NETWORK=test \
  --dart-define=ZAFE_RELAY_URL=https://zafe-relay-testnet.fly.dev
```

The testnet preset already uses `https://testnet.zec.rocks:443` (Ironwood-aware
lightwalletd, over TLS); override with `ZAFE_LIGHTWALLETD_URL`. Without
`ZAFE_RELAY_URL` a testnet build points at a placeholder (`relay.zafe.invalid`) and
Settings shows the relay as "Not configured". The CLI takes `--relay https://...` (or `ZAFE_RELAY`).
