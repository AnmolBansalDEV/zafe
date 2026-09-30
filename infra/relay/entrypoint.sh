#!/bin/sh
# Runs zafe-relay as the unprivileged `zafe` user. When started as root (the default),
# first makes the data directory writable for it: volumes are often mounted root-owned.
set -eu

data_dir=$(dirname "${ZAFE_RELAY_DB:-/data/relay.sqlite}")

if [ "$(id -u)" = "0" ]; then
    mkdir -p "$data_dir"
    chown zafe:zafe "$data_dir"
    # Existing database files (and WAL/SHM) from an earlier root-run container.
    find "$data_dir" -maxdepth 1 -name '*.sqlite*' -exec chown zafe:zafe {} +
    exec setpriv --reuid=zafe --regid=zafe --init-groups --inh-caps=-all \
        /usr/local/bin/zafe-relay "$@"
fi

exec /usr/local/bin/zafe-relay "$@"
