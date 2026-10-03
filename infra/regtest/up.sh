#!/usr/bin/env bash
# Starts a disposable Ironwood regtest network: a Zakura node and lightwalletd.
#   infra/regtest/up.sh <miner-unified-address>
# Env: ZAFE_REGTEST_NAME (default zafe-regtest), ZAFE_REGTEST_RPC_PORT (18232),
#      ZAFE_REGTEST_LWD_PORT (9067). Everything binds to 127.0.0.1.
set -euo pipefail
MINER_ADDRESS=${1:?usage: up.sh <miner-unified-address>}
NAME=${ZAFE_REGTEST_NAME:-zafe-regtest}
RPC_PORT=${ZAFE_REGTEST_RPC_PORT:-18232}
LWD_PORT=${ZAFE_REGTEST_LWD_PORT:-9067}
ZAKURA_IMAGE=zakuracore/zakura:1.6.0
LWD_IMAGE=ghcr.io/zcashlabs/thus-spoke-zakura-lightwalletd:0.3.0
DIR=$(cd "$(dirname "$0")" && pwd)
CONFIG_DIR=$(mktemp -d "${TMPDIR:-/tmp}/$NAME.XXXXXX")

sed "s|\${MINER_ADDRESS}|$MINER_ADDRESS|" "$DIR/zakurad.toml.template" > "$CONFIG_DIR/zakurad.toml"
chmod -R a+rX "$CONFIG_DIR"

"$DIR/down.sh" >/dev/null 2>&1 || true
docker network create "$NAME" >/dev/null
docker run -d --name "$NAME-zakura" --network "$NAME" --network-alias zakura \
  --label "zafe.regtest=$NAME" -p "127.0.0.1:$RPC_PORT:18232" \
  -v "$CONFIG_DIR:/config:ro" --entrypoint zakurad "$ZAKURA_IMAGE" -c /config/zakurad.toml start >/dev/null

rpc() { curl -sf -X POST -H 'content-type: application/json' \
  --data "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$1\",\"params\":${2:-[]}}" "http://127.0.0.1:$RPC_PORT"; }
for _ in $(seq 60); do rpc getblockchaininfo >/dev/null 2>&1 && break; sleep 1; done
# lightwalletd needs at least one block to start.
rpc generate '[1]' >/dev/null

docker run -d --name "$NAME-lightwalletd" --network "$NAME" --label "zafe.regtest=$NAME" \
  --user 0:0 -p "127.0.0.1:$LWD_PORT:9067" "$LWD_IMAGE" \
  --no-tls-very-insecure --grpc-bind-addr 0.0.0.0:9067 --rpchost zakura --rpcport 18232 \
  --rpcuser unused --rpcpassword unused --data-dir /tmp/lwd --log-file /dev/stdout >/dev/null

echo "zakura rpc:   http://127.0.0.1:$RPC_PORT"
echo "lightwalletd: http://127.0.0.1:$LWD_PORT"
