"""Zafe's app icon and splash mark (original work), as SVG.

The mark is the "lime tile": a flat lime square with a bold charcoal geometric Z.
Flat colours only, no gradients: iOS Tinted/Clear icons and Android themed icons
repaint the icon, and a flat shape survives that.

The Z is drawn in a 100-unit box (the full icon tile) and placed into each output with a
transform. It is point-symmetric, so its geometric centre is its optical centre.
It is a little narrower than tall (46 x 48) because a square Z looks wide, and the
diagonal is ~11 units thick, just under the 12.5-unit bars, so all strokes read as the
same weight. At 24 px the bars are 3 px.

Writes app/tool/brand/svg/*.svg; app/tool/brand/render_test.dart renders them with
flutter_svg into the Android and iOS resources. Run scripts/brand/icons.sh, not this
file directly.

flutter_svg notes: no <line>/<polyline>/<pattern>/filters (use <path>).
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "..", "app", "tool", "brand", "svg")

# "Signal" palette (app/lib/src/core/theme).
LIME = "#C9EE6E"      # the tile, in both themes (it is the app icon)
CHARCOAL = "#141414"  # the Z
WHITE = "#FFFFFF"     # monochrome silhouettes (the launcher tints them)

# Z outline (100-unit tile). Bars 12.5 thick; the diagonal is 17.5 wide horizontally.
_L, _R, _T, _B = 27.0, 73.0, 26.0, 74.0
_BAR = 12.5
_DIAG = 17.5
Z_POINTS = [
    (_L, _T), (_R, _T), (_R, _T + _BAR),                   # top bar, right end
    (_L + _DIAG, _B - _BAR), (_R, _B - _BAR), (_R, _B),    # diagonal foot, bottom bar
    (_L, _B), (_L, _B - _BAR),                             # bottom bar, left end
    (_R - _DIAG, _T + _BAR), (_L, _T + _BAR),              # diagonal head, top bar
]
# Corner radius per vertex: outer corners soft, the two inner (concave) corners
# almost sharp so the counters stay crisp at small sizes.
Z_RADII = [2.2, 2.2, 1.6, 0.8, 2.2, 2.2, 2.2, 1.6, 0.8, 2.2]


def n(v):
    return f"{v:.2f}".rstrip("0").rstrip(".")


def rounded_path(points, radii):
    """Closed polygon path with each vertex rounded (quadratic, control at the vertex)."""
    k = len(points)
    parts = []
    for i in range(k):
        px, py = points[i - 1]
        x, y = points[i]
        nx, ny = points[(i + 1) % k]
        r = radii[i]
        d0 = math.hypot(px - x, py - y)
        d1 = math.hypot(nx - x, ny - y)
        a = (x + (px - x) * r / d0, y + (py - y) * r / d0)
        b = (x + (nx - x) * r / d1, y + (ny - y) * r / d1)
        parts.append(("M" if i == 0 else "L") + f"{n(a[0])} {n(a[1])}")
        parts.append(f"Q{n(x)} {n(y)} {n(b[0])} {n(b[1])}")
    return "".join(parts) + "Z"


Z_PATH = rounded_path(Z_POINTS, Z_RADII)


def z(fill, x, y, size):
    """The Z for a tile of `size` at (x, y)."""
    s = size / 100
    return (f'<g transform="translate({n(x)} {n(y)}) scale({s:.5f})">'
            f'<path d="{Z_PATH}" fill="{fill}"/></g>')


def tile(x, y, size, radius, fill=LIME):
    return (f'<rect x="{n(x)}" y="{n(y)}" width="{n(size)}" height="{n(size)}" '
            f'rx="{n(radius)}" fill="{fill}"/>')


def svg(size, body):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" '
            f'viewBox="0 0 {size} {size}">{body}</svg>\n')


def write(name, text):
    with open(os.path.join(OUT, name), "w") as fh:
        fh.write(text)
    print(os.path.relpath(os.path.join(OUT, name)))


def main():
    os.makedirs(OUT, exist_ok=True)
    for old in ("adaptive_background.svg",):
        path = os.path.join(OUT, old)
        if os.path.exists(path):
            os.remove(path)
    S = 1024
    # iOS (and store listing): full-bleed lime square; iOS applies its own mask.
    write("icon.svg", svg(S, f'<rect width="{S}" height="{S}" fill="{LIME}"/>'
                             + z(CHARCOAL, 0, 0, S)))
    # Legacy Android launcher (pre-8): a rounded tile with a small margin.
    m = 40
    write("icon_legacy.svg", svg(S, tile(m, m, S - 2 * m, (S - 2 * m) * 0.22)
                                    + z(CHARCOAL, m, m, S - 2 * m)))
    # Adaptive icon: 108 dp canvas, the launcher shows the middle 72 dp, which maps to
    # the 100-unit tile. The Z (46 x 48 units = 33 x 35 dp, 24 dp from centre to corner)
    # sits well inside the 66 dp safe circle (radius 33 dp). The background is the
    # colour resource @color/zafe_icon_tile.
    inner = S * 72 / 108
    off = (S - inner) / 2
    write("adaptive_foreground.svg", svg(S, z(CHARCOAL, off, off, inner)))
    write("adaptive_monochrome.svg", svg(S, z(WHITE, off, off, inner)))
    # Splash. Android 12+: 288 dp canvas, masked to the 192 dp circle; a 144 dp tile
    # (radius 22.5 %) reaches 88 dp from the centre, inside the 96 dp mask radius.
    # Pre-12 and iOS: a 160 dp/pt bitmap with the same 144 dp tile. The tile is the app
    # icon, so it is the same lime in both themes.
    for theme in ("dark", "light"):
        t = S * 144 / 288
        write(f"splash_icon_{theme}.svg",
              svg(S, tile((S - t) / 2, (S - t) / 2, t, t * 0.225)
                  + z(CHARCOAL, (S - t) / 2, (S - t) / 2, t)))
        t = S * 144 / 160
        write(f"splash_mark_{theme}.svg",
              svg(S, tile((S - t) / 2, (S - t) / 2, t, t * 0.225)
                  + z(CHARCOAL, (S - t) / 2, (S - t) / 2, t)))


def write_notification_icon():
    """Android notification small icon: the Z alone, white on transparent (Android uses
    only the alpha and tints it). A vector drawable, so no rendering step. The 56-unit
    viewport around the Z leaves the 1 dp margin of Android's 24 dp status bar icons."""
    res = os.path.join(HERE, "..", "..", "app", "android", "app", "src", "main", "res",
                       "drawable", "ic_notification.xml")
    with open(res, "w") as fh:
        fh.write(
            '<?xml version="1.0" encoding="utf-8"?>\n'
            "<!-- Generated by scripts/brand/brand.py: the Z from the app icon. -->\n"
            '<vector xmlns:android="http://schemas.android.com/apk/res/android"\n'
            '    android:width="24dp" android:height="24dp"\n'
            '    android:viewportWidth="56" android:viewportHeight="56">\n'
            '    <group android:translateX="-22" android:translateY="-22">\n'
            f'        <path android:fillColor="#FFFFFFFF" android:pathData="{Z_PATH}"/>\n'
            "    </group>\n"
            "</vector>\n")
    print(os.path.relpath(res))


if __name__ == "__main__":
    main()
    write_notification_icon()
