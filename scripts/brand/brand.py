"""Zafe's app icon and splash mark (original work), as SVG.

The mark is a vault dial: a ring of tick marks with a Zcash-gold index at 12 o'clock,
a heavy jade bezel, and a geometric Z on the dial face. Everything is drawn in a
1000-unit box centred on (0, 0) (radius 500) and placed into each output with a
transform.

Writes app/tool/brand/svg/*.svg; app/tool/brand/render_test.dart renders them with
flutter_svg into the Android and iOS resources. Run scripts/brand/icons.sh, not this
file directly.

flutter_svg notes: no <line>/<polyline>/<pattern>/filters (use <path>), gradients are
fine.
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "..", "app", "tool", "brand", "svg")

# app/lib/src/core/theme: ink ladder, jade, Zcash gold.
INK_WINDOW = "#080A0F"
INK_RAISED = "#11151D"
INK_CARD = "#1A2030"
JADE = "#2EC4A6"
JADE_HI = "#7BE8D2"
JADE_DEEP = "#178270"
GOLD = "#F4B728"
GOLD_HI = "#FFE39A"

DARK = dict(
    ring=JADE, tick=JADE, face0=INK_CARD, face1=INK_RAISED, z0=JADE_HI, z1=JADE,
    z2=JADE_DEEP, gold=GOLD, gold_hi=GOLD_HI, inner=JADE,
)
LIGHT = dict(
    ring="#0F8C76", tick="#0F8C76", face0="#FFFFFF", face1="#E8ECF2", z0="#17A58B",
    z1="#0F8C76", z2="#0B6E5E", gold="#E0A00F", gold_hi="#F4B728", inner="#0F8C76",
)
MONO = dict(
    ring="#FFFFFF", tick="#FFFFFF", face0=None, face1=None, z0="#FFFFFF", z1="#FFFFFF",
    z2="#FFFFFF", gold="#FFFFFF", gold_hi="#FFFFFF", inner=None,
)


def n(v):
    return f"{v:.1f}".rstrip("0").rstrip(".")


def polar(r, deg):
    a = math.radians(deg - 90)
    return r * math.cos(a), r * math.sin(a)


def z_points(s=1.0):
    """A geometric Z, 300 x 320 units, stroke ~62."""
    p = [(-140, -160), (140, -160), (140, -100), (-48, 100), (140, 100), (140, 160),
         (-140, 160), (-140, 100), (48, -100), (-140, -100)]
    return " ".join(f"{n(x * s)},{n(y * s)}" for x, y in p)


def mark(c, uid, ticks=True):
    """The dial mark, radius 500, centred on (0, 0)."""
    out = []
    grads = []
    mono = c["face0"] is None
    if not mono:
        grads.append(
            f'<radialGradient id="face{uid}" cx="40%" cy="30%" r="80%">'
            f'<stop offset="0" stop-color="{c["face0"]}"/>'
            f'<stop offset="1" stop-color="{c["face1"]}"/></radialGradient>')
        grads.append(
            f'<linearGradient id="z{uid}" x1="0" y1="0" x2="1" y2="1">'
            f'<stop offset="0" stop-color="{c["z0"]}"/><stop offset=".5" stop-color="{c["z1"]}"/>'
            f'<stop offset="1" stop-color="{c["z2"]}"/></linearGradient>')
        grads.append(
            f'<linearGradient id="g{uid}" x1="0" y1="0" x2="0" y2="1">'
            f'<stop offset="0" stop-color="{c["gold_hi"]}"/>'
            f'<stop offset="1" stop-color="{c["gold"]}"/></linearGradient>')
    if ticks:
        # 48 ticks; every 4th is a major. The top one is replaced by the gold index.
        d_minor, d_major = [], []
        for i in range(48):
            if i == 0:
                continue
            major = i % 4 == 0
            r0, r1 = (412, 482) if major else (428, 470)
            x0, y0 = polar(r0, i * 7.5)
            x1, y1 = polar(r1, i * 7.5)
            (d_major if major else d_minor).append(f"M{n(x0)} {n(y0)}L{n(x1)} {n(y1)}")
        out.append(
            f'<path d="{"".join(d_minor)}" stroke="{c["tick"]}" stroke-opacity="{".55" if not mono else "1"}" '
            f'stroke-width="14" stroke-linecap="round" fill="none"/>')
        out.append(
            f'<path d="{"".join(d_major)}" stroke="{c["tick"]}" stroke-width="22" '
            f'stroke-linecap="round" fill="none"/>')
        gold = f'url(#g{uid})' if not mono else c["gold"]
        # Gold index: a rounded wedge pointing at the dial.
        out.append(
            f'<path d="M-40 -498Q0 -506 40 -498L10 -418Q0 -404 -10 -418Z" fill="{gold}"/>')
    # Bezel.
    out.append(f'<circle cx="0" cy="0" r="364" fill="none" stroke="{c["ring"]}" stroke-width="40"/>')
    if not mono:
        out.append(f'<circle cx="0" cy="0" r="342" fill="url(#face{uid})"/>')
        out.append(
            f'<circle cx="0" cy="0" r="292" fill="none" stroke="{c["inner"]}" '
            f'stroke-opacity=".28" stroke-width="6"/>')
    z = f'url(#z{uid})' if not mono else c["z1"]
    out.append(f'<polygon points="{z_points(0.92)}" fill="{z}"/>')
    return "".join(grads), "".join(out)


def svg(size, body, defs=""):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" '
            f'viewBox="0 0 {size} {size}"><defs>{defs}</defs>{body}</svg>\n')


def placed(c, uid, cx, cy, radius, ticks=True):
    g, body = mark(c, uid, ticks)
    s = radius / 500
    return g, f'<g transform="translate({n(cx)} {n(cy)}) scale({s:.5f})">{body}</g>'


def backdrop(size, uid="bg", rounded=0.0):
    """Ink background with a soft jade glow and faint dial rings behind the mark."""
    h = size / 2
    defs = (
        f'<radialGradient id="{uid}ink" cx="50%" cy="34%" r="75%">'
        f'<stop offset="0" stop-color="{INK_CARD}"/><stop offset="1" stop-color="{INK_WINDOW}"/></radialGradient>'
        f'<radialGradient id="{uid}glow" cx="50%" cy="50%" r="50%">'
        f'<stop offset="0" stop-color="{JADE}" stop-opacity=".30"/>'
        f'<stop offset="1" stop-color="{JADE}" stop-opacity="0"/></radialGradient>')
    rx = f' rx="{n(size * rounded)}"' if rounded else ""
    body = f'<rect x="0" y="0" width="{size}" height="{size}"{rx} fill="url(#{uid}ink)"/>'
    body += f'<circle cx="{n(h)}" cy="{n(h)}" r="{n(size * 0.48)}" fill="url(#{uid}glow)"/>'
    for i, r in enumerate((0.40, 0.47, 0.56, 0.66)):
        body += (f'<circle cx="{n(h)}" cy="{n(h)}" r="{n(size * r)}" fill="none" stroke="{JADE}" '
                 f'stroke-opacity="{0.10 - i * 0.02:.2f}" stroke-width="{n(size * 0.004)}"/>')
    return defs, body


def write(name, text):
    with open(os.path.join(OUT, name), "w") as fh:
        fh.write(text)
    print(os.path.relpath(os.path.join(OUT, name)))


def main():
    os.makedirs(OUT, exist_ok=True)
    S = 1024
    # Full icon (iOS, store listing): square ink, mark fills ~74%.
    d, b = backdrop(S)
    g, m = placed(DARK, "i", S / 2, S / 2, S * 0.37)
    write("icon.svg", svg(S, b + m, d + g))
    # Legacy Android launcher: the same on a rounded square with a small margin.
    d, b = backdrop(S, rounded=0.22)
    g, m = placed(DARK, "l", S / 2, S / 2, S * 0.36)
    write("icon_legacy.svg", svg(S, f'<g transform="translate(40 40) scale(.921875)">{b}</g>' + m, d + g))
    # Adaptive (108 dp canvas, 66 dp safe circle => radius 0.305 of the canvas).
    d, b = backdrop(S)
    write("adaptive_background.svg", svg(S, b, d))
    g, m = placed(DARK, "f", S / 2, S / 2, S * 0.30)
    write("adaptive_foreground.svg", svg(S, m, g))
    g, m = placed(MONO, "m", S / 2, S / 2, S * 0.30)
    write("adaptive_monochrome.svg", svg(S, m, g))
    # Splash marks (transparent). Android 12+: 288 dp canvas, content inside the 192 dp
    # circle; the mark is 160 dp across. Pre-12 and iOS use a 160 dp/pt bitmap.
    for theme, c in (("dark", DARK), ("light", LIGHT)):
        g, m = placed(c, "s" + theme, S / 2, S / 2, S * 80 / 288)
        write(f"splash_icon_{theme}.svg", svg(S, m, g))
        g, m = placed(c, "k" + theme, S / 2, S / 2, S / 2)
        write(f"splash_mark_{theme}.svg", svg(S, m, g))


if __name__ == "__main__":
    main()
