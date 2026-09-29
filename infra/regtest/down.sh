#!/usr/bin/env bash
# Removes the regtest containers and network started by up.sh (and nothing else).
set -uo pipefail
NAME=${ZAFE_REGTEST_NAME:-zafe-regtest}
docker rm -f "$NAME-lightwalletd" "$NAME-zakura" >/dev/null 2>&1
docker network rm "$NAME" >/dev/null 2>&1
exit 0
