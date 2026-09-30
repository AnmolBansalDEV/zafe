"""Regenerates the site's screenshots and web fonts from the app (run from the repo root).

    cd app && flutter test tool/screens/home_render_test.dart tool/screens/proposal_render_test.dart
    python3 infra/site/assets.py          # needs: pip install pillow fonttools brotli

Screens: the app's own renders (fake data, app/build/screen_preview/), cropped to a phone
viewport and saved as WebP. Fonts: the app's DM Sans and Space Grotesk, subset to Latin
and saved as WOFF2, with their OFL licences. The outputs are committed, so building or
deploying the site doesn't need Flutter or Python packages.
"""

import os
import shutil

from fontTools import subset
from PIL import Image

ROOT = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))
RENDERS = os.path.join(ROOT, "app", "build", "screen_preview")
APP_FONTS = os.path.join(ROOT, "app", "assets", "fonts")
OUT = os.path.join(ROOT, "infra", "site", "public", "assets")

# Phone viewport of the renders: 390 x 844 logical pixels at 2x.
PHONE = (780, 1688)
SCREENS = ["home_light", "home_dark", "proposal_review_light", "proposal_review_dark"]

FONTS = [
    "DMSans-Regular",
    "DMSans-Medium",
    "SpaceGrotesk-Medium",
    "SpaceGrotesk-SemiBold",
]
# Basic Latin, Latin-1, and the punctuation the copy uses (dashes, quotes, bullet,
# ellipsis, arrows, minus).
UNICODES = "U+0000-00FF,U+2013-2014,U+2018-201D,U+2022,U+2026,U+2190-2193,U+2212"


def screens():
    out = os.path.join(OUT, "screens")
    os.makedirs(out, exist_ok=True)
    for name in SCREENS:
        src = os.path.join(RENDERS, f"{name}.png")
        if not os.path.exists(src):
            raise SystemExit(f"missing {src}: run the render tests first (see the docstring)")
        image = Image.open(src).convert("RGB")
        image = image.crop((0, 0, PHONE[0], min(PHONE[1], image.height)))
        dest = os.path.join(out, f"{name}.webp")
        image.save(dest, "WEBP", quality=84, method=6)
        print(f"{dest}  {os.path.getsize(dest) // 1024} KB")


def fonts():
    out = os.path.join(OUT, "fonts")
    os.makedirs(out, exist_ok=True)
    for name in FONTS:
        dest = os.path.join(out, f"{name}.woff2")
        subset.main([
            os.path.join(APP_FONTS, f"{name}.ttf"),
            f"--unicodes={UNICODES}",
            "--layout-features=kern,liga,calt,tnum",
            "--flavor=woff2",
            f"--output-file={dest}",
        ])
        print(f"{dest}  {os.path.getsize(dest) // 1024} KB")
    for licence in ("DMSans-OFL.txt", "SpaceGrotesk-OFL.txt"):
        shutil.copy(os.path.join(APP_FONTS, "licenses", licence), out)


if __name__ == "__main__":
    screens()
    fonts()
