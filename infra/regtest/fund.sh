#!/usr/bin/env bash
# Funds an address on the ths environment started by up.sh: NOTES faucet payments of
# 5 ZEC (the faucet's maximum), each a separate Ironwood note, then mines enough blocks for
# the notes to be spendable under the wallet's default confirmation policy.
#   infra/regtest/fund.sh <unified-address> [notes (default 4)]
# Env: ZAFE_REGTEST_NAME (default zafe-regtest), THS (default `ths`).
set -euo pipefail
ADDRESS=${1:?usage: fund.sh <unified-address> [notes]}
NOTES=${2:-4}
THS=${THS:-ths}
NAME=${ZAFE_REGTEST_NAME:-zafe-regtest}

for _ in $(seq "$NOTES"); do
  "$THS" --name "$NAME" faucet "$ADDRESS" --amount 5 >/dev/null
done
"$THS" --name "$NAME" mine 12 >/dev/null
echo "funded $ADDRESS with $NOTES x 5 ZEC"
