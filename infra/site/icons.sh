#!/usr/bin/env bash
# Regenerates the site's raster icons (ImageMagick 6+) from the brand sources:
#   public/favicon.ico            16/32/48, the rounded Seam tile (public/assets/zafe.svg)
#   public/apple-touch-icon.png   180, full-bleed (iOS rounds it), from the app's iOS icon
#   public/assets/icons/icon-{192,512}.png   rounded tile, manifest "any"
#   public/assets/icons/maskable-512.png     full-bleed, manifest "maskable" (the mark
#                                            sits inside the 80% safe circle)
# The SVG favicon stays the main one; these are for /favicon.ico requests, Google's
# favicon crawler, iOS home screens and the web manifest.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
svg="$here/public/assets/zafe.svg"
ios="$here/../../app/ios/Runner/Assets.xcassets/AppIcon.appiconset/Icon-App-1024x1024@1x.png"
icons="$here/public/assets/icons"
mkdir -p "$icons"

tile() { convert -background none -density 96 "$svg" -resize "$1x$1" -strip "PNG32:$2"; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
for s in 16 32 48; do tile "$s" "$tmp/$s.png"; done
convert "$tmp/16.png" "$tmp/32.png" "$tmp/48.png" "$here/public/favicon.ico"

tile 192 "$icons/icon-192.png"
tile 512 "$icons/icon-512.png"
convert "$ios" -resize 512x512 -strip "PNG24:$icons/maskable-512.png"
convert "$ios" -resize 180x180 -strip "PNG24:$here/public/apple-touch-icon.png"
echo "Icons written"
