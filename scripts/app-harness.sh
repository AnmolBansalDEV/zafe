#!/usr/bin/env bash
# Device-testing harness for the app: a relay and two headless CLI members (B, C) on this
# machine; the app on a phone/emulator is the third member. `adb reverse` lets the device
# reach the relay (8787) and lightwalletd (9067) at 127.0.0.1, matching the app defaults.
#
#   scripts/app-harness.sh start          relay + B creates a 2-of-3 vault; prints the invite
#   scripts/app-harness.sh join-c         C joins
#   scripts/app-harness.sh seal           B seals (after the app joined); prints safety number
#   scripts/app-harness.sh keygen         B and C run keygen in the background (app joins in)
#   scripts/app-harness.sh fund           regtest up, mining to the vault; 120 blocks
#   scripts/app-harness.sh mine N         mine N blocks
#   scripts/app-harness.sh cli WHO ARGS   run the zafe CLI as B or C (sync, approve, respond, ...)
#   scripts/app-harness.sh stop           stop everything and delete the state
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=${ZAFE_HARNESS_DIR:-${TMPDIR:-/tmp}/zafe-harness}
export ZAFE_RELAY=http://127.0.0.1:8787 ZAFE_LIGHTWALLETD=http://127.0.0.1:9067
ZAFE="$ROOT/target/debug/zafe"
member() { local who=$1; shift; "$ZAFE" --home "$WORK/$who" "$@"; }
mine() {
  curl -sf -X POST -H 'content-type: application/json' \
    --data "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"generate\",\"params\":[$1]}" \
    http://127.0.0.1:18232 >/dev/null
}

case "${1:-}" in
  start)
    mkdir -p "$WORK"
    cargo build -q -p zafe-cli -p zafe-relay
    ZAFE_RELAY_LISTEN=127.0.0.1:8787 ZAFE_RELAY_DB="$WORK/relay.sqlite" \
      nohup "$ROOT/target/debug/zafe-relay" > "$WORK/relay.log" 2>&1 &
    echo $! > "$WORK/relay.pid"
    for _ in $(seq 50); do (exec 3<>/dev/tcp/127.0.0.1/8787) 2>/dev/null && break; sleep 0.2; done
    adb reverse tcp:8787 tcp:8787
    adb reverse tcp:9067 tcp:9067
    member B init >/dev/null
    member C init >/dev/null
    member B vault create --name "${ZAFE_VAULT_NAME:-Grants}" --threshold 2 --members 3 | tee "$WORK/invite"
    ;;
  join-c) member C vault join "$(cat "$WORK/invite")" ;;
  seal) member B vault seal; member B vault members ;;
  keygen)
    SN=$(member B vault members | sed -n 's/^safety number: //p')
    echo "safety number: $SN"
    nohup "$ZAFE" --home "$WORK/B" vault keygen --safety-number "$SN" --birthday 2 > "$WORK/kB" 2>&1 &
    nohup "$ZAFE" --home "$WORK/C" vault keygen --safety-number "$SN" > "$WORK/kC" 2>&1 &
    ;;
  fund)
    ADDR=$(member B vault show | sed -n 's/^address //p')
    "$ROOT/infra/regtest/up.sh" "$ADDR"
    mine 120
    echo "funded $ADDR"
    ;;
  mine) mine "${2:-1}" ;;
  cli) who=$2; shift 2; member "$who" "$@" ;;
  stop)
    [[ -f "$WORK/relay.pid" ]] && kill "$(cat "$WORK/relay.pid")" 2>/dev/null || true
    "$ROOT/infra/regtest/down.sh" || true
    rm -rf "$WORK"
    ;;
  *) sed -n '2,15p' "$0"; exit 1 ;;
esac
