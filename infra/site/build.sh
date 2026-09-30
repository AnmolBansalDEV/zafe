#!/usr/bin/env bash
# Builds the site (home page, and the invite page https://<ZAFE_LINK_HOST>/join#<invite>) into
# infra/site/dist, ready to upload to any static host. See README.md.
#
#   ZAFE_ANDROID_CERT_SHA256   required: SHA-256 fingerprints of the APK signing
#                              certificates, comma-separated (AA:BB:... or plain hex)
#   ZAFE_IOS_APP_IDS           optional: <TeamID>.xyz.zafe.zafe, comma-separated
#   ZAFE_DOWNLOAD_URL          optional: where "Download for Android" points
#                              (default: the GitHub releases page)
#   ZAFE_SOURCE_URL            optional: where "Read the source" points
#                              (default: the GitHub repository)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
out="$here/dist"
package="xyz.zafe.zafe"
repo="https://github.com/AnmolBansalDEV/zafe"
download="${ZAFE_DOWNLOAD_URL:-$repo/releases}"
source_url="${ZAFE_SOURCE_URL:-$repo}"

die() { echo "error: $*" >&2; exit 1; }

plain_https() {
  [[ "$1" =~ ^https://[A-Za-z0-9._~:/?#@!\$\&\(\)*+,\;=%-]+$ && "$1" != *"'"* ]]
}

[[ -n "${ZAFE_ANDROID_CERT_SHA256:-}" ]] ||
  die "set ZAFE_ANDROID_CERT_SHA256 (see README.md: 'Signing fingerprints')"
plain_https "$download" || die "ZAFE_DOWNLOAD_URL must be a plain https:// URL"
plain_https "$source_url" || die "ZAFE_SOURCE_URL must be a plain https:// URL"

# Fingerprints → "AA:BB:...", upper case, exactly 32 bytes.
fingerprints=()
IFS=',' read -ra raw <<< "$ZAFE_ANDROID_CERT_SHA256"
for f in "${raw[@]}"; do
  hex="$(tr -d ': \t' <<< "$f" | tr 'a-f' 'A-F')"
  [[ "$hex" =~ ^[0-9A-F]{64}$ ]] || die "not a SHA-256 fingerprint: '$f'"
  fingerprints+=("$(sed 's/../&:/g; s/:$//' <<< "$hex")")
done

app_ids=()
if [[ -n "${ZAFE_IOS_APP_IDS:-}" ]]; then
  IFS=',' read -ra raw <<< "$ZAFE_IOS_APP_IDS"
  for id in "${raw[@]}"; do
    id="$(tr -d ' \t' <<< "$id")"
    [[ "$id" =~ ^[A-Z0-9]{10}\.[A-Za-z0-9.-]+$ ]] || die "not an iOS app ID (<TeamID>.<bundle>): '$id'"
    app_ids+=("$id")
  done
fi

join_quoted() { local IFS=,; local q=(); for v in "$@"; do q+=("\"$v\""); done; echo "${q[*]}"; }

cd "$here"
[[ -d node_modules ]] || npm ci --no-audit --no-fund
ZAFE_DOWNLOAD_URL="$download" ZAFE_SOURCE_URL="$source_url" ASTRO_TELEMETRY_DISABLED=1 \
  npx --no-install astro build --silent
touch "$out/.nojekyll" # GitHub Pages: serve .well-known

# The CSP allows only same-origin files: refuse a build that inlined any script or style.
if grep -l -E '<script(>| (type|is)=)|<style|[[:space:]](style|on[a-z]+)=' "$out"/*.html; then
  die "inline script or style in the pages above (the CSP would block it)"
fi

mkdir -p "$out/.well-known"
cat > "$out/.well-known/assetlinks.json" <<EOF
[
  {
    "relation": ["delegate_permission/common.handle_all_urls"],
    "target": {
      "namespace": "android_app",
      "package_name": "$package",
      "sha256_cert_fingerprints": [$(join_quoted "${fingerprints[@]}")]
    }
  }
]
EOF

# Universal Links, only for the join path (the invite is in the fragment).
cat > "$out/.well-known/apple-app-site-association" <<EOF
{
  "applinks": {
    "details": [
      {
        "appIDs": [$(join_quoted "${app_ids[@]+"${app_ids[@]}"}")],
        "components": [
          { "/": "/join", "comment": "Invite links" },
          { "/": "/join/", "comment": "Invite links" }
        ]
      }
    ]
  }
}
EOF

if command -v python3 > /dev/null; then
  for f in "$out/.well-known/assetlinks.json" "$out/.well-known/apple-app-site-association"; do
    python3 -m json.tool "$f" > /dev/null || die "invalid JSON: $f"
  done
fi

echo "Built $out"
echo "  Android certificates: ${#fingerprints[@]}"
echo "  iOS app IDs: ${#app_ids[@]}${app_ids[*]:+ (${app_ids[*]})}"
echo "  Download: $download"
