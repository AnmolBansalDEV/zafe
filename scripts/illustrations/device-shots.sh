#!/usr/bin/env bash
# Screenshots the onboarding illustrations in the real app on an Android emulator
# (light and dark). WIPES the app's data on the device (pm clear) so onboarding shows.
#
# Usage: scripts/illustrations/device-shots.sh [out_dir] [--no-build]
# Needs: a running x86_64 emulator (adb devices), agent-device on PATH, ~/android/env.sh.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="${1:-$root/app/build/illustration_device}"
pkg=xyz.zafe.zafe
mkdir -p "$out"
[[ -f ~/android/env.sh ]] && source ~/android/env.sh

if [[ "${2:-}" != "--no-build" ]]; then
  (cd "$root/app" && flutter build apk --debug --target-platform android-x64)
  adb install -r "$root/app/build/app/outputs/flutter-apk/app-debug.apk"
fi

shoot() { # theme
  local theme=$1
  adb shell cmd uimode night "$([[ $theme == dark ]] && echo yes || echo no)" >/dev/null
  adb shell pm clear "$pkg" >/dev/null
  agent-device close >/dev/null 2>&1 || true
  agent-device open "$pkg" --foreground >/dev/null
  agent-device wait text "Create a vault" >/dev/null
  sleep 1
  agent-device screenshot "$out/welcome_$theme.png"
  agent-device find "Create a vault" click >/dev/null
  agent-device wait text "Vault name" >/dev/null
  sleep 1
  agent-device screenshot "$out/create_$theme.png"
  # Relaunch instead of pressing back: back can dismiss only the keyboard, or leave the
  # app entirely (it once landed in another app), and a plain reopen resumes the screen.
  adb shell am force-stop "$pkg"
  agent-device close >/dev/null 2>&1 || true
  agent-device open "$pkg" --foreground >/dev/null
  agent-device wait text "Join with an invite" >/dev/null
  agent-device find "Join with an invite" click >/dev/null
  agent-device wait text "Paste" >/dev/null
  sleep 1
  agent-device screenshot "$out/join_$theme.png"
  agent-device close >/dev/null
}

shoot light
shoot dark
adb shell cmd uimode night no >/dev/null
# The key_shards hero shows on the "Creating your vault keys" screen, which needs a live
# DKG with other members; check it with scripts/illustrations/preview.sh instead.
echo "Screenshots in $out"
