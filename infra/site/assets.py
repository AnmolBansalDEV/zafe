"""Regenerates the site's images and web fonts from the app (run from the repo root).

    cd app && flutter test tool/screens/ && flutter test tool/illustrations/preview_test.dart
    python3 infra/site/assets.py          # needs: pip install pillow fonttools brotli

Images: the app's own renders (fake data, app/build/screen_preview/) and illustrations
(app/build/illustration_preview/), cropped to the part the page talks about and saved as
WebP, light and dark. Fonts: the app's DM Sans, Space Grotesk and JetBrains Mono, subset
to Latin and saved as WOFF2, with their OFL licences. The outputs are committed, so
building or deploying the site doesn't need Flutter or Python packages.
"""

import os
import shutil

from fontTools import subset
from PIL import Image

ROOT = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))
BUILD = os.path.join(ROOT, "app", "build")
APP_FONTS = os.path.join(ROOT, "app", "assets", "fonts")
OUT = os.path.join(ROOT, "infra", "site", "public", "assets")

# (output name, source under app/build without the _light/_dark suffix, crop box). Screen
# renders are 780 px wide (390 logical at 2x); boxes are in those pixels.
CROPS = [
    # The payment waiting for approvals: header, payment card, approvals and signers.
    ("approval", "screen_preview/proposal_review", (0, 0, 780, 1136)),
    # The last detail rows down to "Checked on this device: Matches", then Approve and
    # sign (the page fades the top edge).
    ("check", "screen_preview/proposal_review", (0, 1560, 780, 2400)),
    # The key split into shares (illustration, 1080 wide), trimmed to the art.
    ("key_shards", "illustration_preview/key_shards", (0, 260, 1080, 1000)),
]

FONTS = [
    "DMSans-Regular",
    "DMSans-Medium",
    "SpaceGrotesk-Medium",
    "SpaceGrotesk-SemiBold",
    "JetBrainsMono-Regular",
]
# Basic Latin, Latin-1, and the punctuation the copy uses (dashes, quotes, bullet,
# ellipsis, arrows, minus).
UNICODES = "U+0000-00FF,U+2013-2014,U+2018-201D,U+2022,U+2026,U+2190-2193,U+2212"


def screens():
    out = os.path.join(OUT, "screens")
    if os.path.isdir(out):
        shutil.rmtree(out)  # drop crops that are no longer listed
    os.makedirs(out)
    for name, source, box in CROPS:
        for theme in ("light", "dark"):
            src = os.path.join(BUILD, f"{source}_{theme}.png")
            if not os.path.exists(src):
                raise SystemExit(f"missing {src}: run the render tests first (see the docstring)")
            image = Image.open(src).convert("RGB").crop(box)
            dest = os.path.join(out, f"{name}_{theme}.webp")
            image.save(dest, "WEBP", quality=86, method=6)
            print(f"{dest}  {image.size[0]}x{image.size[1]}  {os.path.getsize(dest) // 1024} KB")


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
    for licence in ("DMSans-OFL.txt", "SpaceGrotesk-OFL.txt", "JetBrainsMono-OFL.txt"):
        shutil.copy(os.path.join(APP_FONTS, "licenses", licence), out)


if __name__ == "__main__":
    screens()
    fonts()
