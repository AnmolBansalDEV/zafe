#!/usr/bin/env bash
# Regenerate the SVGs from scenes.py and render them with flutter_svg (what the app draws).
# Usage: scripts/illustrations/preview.sh [--no-gen] [scene ...]
# Output: app/build/illustration_preview/{<name>.png,<name>_hero.png,contact_sheet.png}
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
if [[ "${1:-}" == "--no-gen" ]]; then shift; else python3 "$here/scenes.py" "$@"; fi
[[ -f ~/android/env.sh ]] && source ~/android/env.sh
cd "$root/app"
flutter test tool/illustrations/preview_test.dart --reporter compact
echo "Previews in $root/app/build/illustration_preview/ (open contact_sheet.png first)"
