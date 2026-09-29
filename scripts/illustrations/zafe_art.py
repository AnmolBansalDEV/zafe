"""Shared toolkit for Zafe's illustrations (original work; see docs/illustrations.md).

Style: ink-engraving line art with hatch and stipple texture, a muted neutral stone
palette, crimson brand accents and Zcash-gold highlights. Every scene is a function
`scene(p) -> str` that takes one of PALETTES and returns a complete SVG document.

Texture fills (`fill="url(#hatch)"`, `hatch2`, `dots`, `fine`, `grit`) are written like
SVG patterns for readability, but `svg()` bakes them into clipped geometry because
flutter_svg renders <pattern> fills wrongly (vertical stripes over the whole shape).
Path elements using a texture need a `bb="x0 y0 x1 y1"` bounding box attribute.
"""
import math
import random
import re

PALETTES = {
    "dark": dict(
        sky0="#1B1F1F", sky1="#0F0F0F", stone="#2D3232", stone2="#393E3E",
        stone3="#232828", light="#626767", hi="#858686", ink="#060808",
        crimson="#A83861", crimson2="#6D243F", gold="#F4B728", gold2="#B8841A",
        gold3="#FFE39A", metal="#4D5252", metal2="#626767", dot="#000000",
        glow="#F4B728", paper="#C2C3C3", texop="0.35", dark=True,
    ),
    "light": dict(
        sky0="#FFFFFF", sky1="#F7F7F7", stone="#E1E1E1", stone2="#EBEBEB",
        stone3="#D4D4D4", light="#F7F7F7", hi="#FFFFFF", ink="#2E3232",
        crimson="#A83861", crimson2="#862D4E", gold="#F4B728", gold2="#C98A10",
        gold3="#FFF1C4", metal="#D4D4D4", metal2="#EBEBEB", dot="#4D5252",
        glow="#F4B728", paper="#FFFFFF", texop="0.22", dark=False,
    ),
}


def f(v):
    return str(int(round(v)))


def pts(points):
    return " ".join(f"{f(x)},{f(y)}" for x, y in points)


def line_d(points):
    """Path data for an open polyline (flutter_svg ignores <polyline>/<line>; use <path>)."""
    return "M" + "L".join(f"{f(x)} {f(y)}" for x, y in points)


def defs(p, extra=""):
    return f"""<defs>
<radialGradient id="vig" cx="50%" cy="42%" r="70%"><stop offset="55%" stop-color="{p['sky1']}" stop-opacity="0"/><stop offset="100%" stop-color="{p['dot']}" stop-opacity="{'0.55' if p['dark'] else '0.10'}"/></radialGradient>
<radialGradient id="glow"><stop offset="0%" stop-color="{p['glow']}" stop-opacity="0.75"/><stop offset="45%" stop-color="{p['glow']}" stop-opacity="0.22"/><stop offset="100%" stop-color="{p['glow']}" stop-opacity="0"/></radialGradient>
<radialGradient id="glowc"><stop offset="0%" stop-color="{p['crimson']}" stop-opacity="0.6"/><stop offset="100%" stop-color="{p['crimson']}" stop-opacity="0"/></radialGradient>
<linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{p['sky0']}"/><stop offset="1" stop-color="{p['sky1']}"/></linearGradient>
<linearGradient id="goldg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{p['gold3']}"/><stop offset=".45" stop-color="{p['gold']}"/><stop offset="1" stop-color="{p['gold2']}"/></linearGradient>
{extra}</defs>"""


_TEX_RE = re.compile(r'<(polygon|rect|circle|path)\b([^>]*?)fill="url\(#(hatch|hatch2|dots|fine|grit)\)"([^>]*)/>')
_tex_n = [0]


def _num(attrs, name, default=0.0):
    m = re.search(r'\b' + name + r'="([-\d.]+)"', attrs)
    return float(m.group(1)) if m else default


def _bbox(tag, attrs):
    if tag == "rect":
        x, y = _num(attrs, "x"), _num(attrs, "y")
        return x, y, x + _num(attrs, "width"), y + _num(attrs, "height")
    if tag == "circle":
        cx, cy, r = _num(attrs, "cx"), _num(attrs, "cy"), _num(attrs, "r")
        return cx - r, cy - r, cx + r, cy + r
    if tag == "polygon":
        v = [float(t) for t in re.search(r'points="([^"]*)"', attrs).group(1).replace(",", " ").split()]
        return min(v[0::2]), min(v[1::2]), max(v[0::2]), max(v[1::2])
    v = [float(t) for t in re.search(r'bb="([^"]*)"', attrs).group(1).split()]
    return tuple(v)


def _texture(kind, box, ink, rnd):
    x0, y0, x1, y1 = box
    if kind in ("hatch", "hatch2"):
        step, sw, sgn = (10, 2.2, 1) if kind == "hatch" else (8, 1.6, -1)
        h = y1 - y0
        d = []
        c = x0 - h - step
        while c < x1 + h + step:
            if sgn > 0:
                d.append(f"M{f(c)} {f(y1)}L{f(c+h)} {f(y0)}")
            else:
                d.append(f"M{f(c)} {f(y0)}L{f(c+h)} {f(y1)}")
            c += step
        return f'<path d="{"".join(d)}" stroke="{ink}" stroke-width="{sw}"/>'
    area = (x1 - x0) * (y1 - y0)
    per = {"dots": 1000, "fine": 1500, "grit": 1500}[kind]
    n = int(area / per)
    sizes = [2.4, 3.2, 4.2] if kind != "grit" else [2, 3, 4.6]
    buckets = {w: [] for w in sizes}
    for _ in range(n):
        x, y = rnd.uniform(x0, x1), rnd.uniform(y0, y1)
        buckets[rnd.choice(sizes)].append(f"M{x:.0f} {y:.0f}h.1")
    out = "".join(f'<path d="{"".join(v)}" stroke="{ink}" stroke-width="{w:g}" stroke-linecap="round"/>' for w, v in buckets.items() if v)
    if kind == "grit":
        d = [f"M{rnd.uniform(x0, x1):.0f} {rnd.uniform(y0, y1):.0f}l{rnd.randint(4,10)} {rnd.randint(-3,3)}" for _ in range(n // 10)]
        out += f'<path d="{"".join(d)}" stroke="{ink}" stroke-width="1.6"/>'
    return out


def detexture(body, ink):
    """flutter_svg renders <pattern> fills unreliably, so bake every texture into clipped geometry."""
    rnd = random.Random(1234)
    _tex_n[0] = 0

    def sub(m):
        tag, a, kind, b = m.groups()
        attrs = a + b
        box = _bbox(tag, attrs)
        op = re.search(r'opacity="([^"]*)"', attrs)
        shape_attrs = re.sub(r'\s(opacity|bb|stroke[-\w]*)="[^"]*"', "", attrs)
        _tex_n[0] += 1
        cid = f"t{_tex_n[0]}"
        full = tag == "rect" and "x=" not in attrs and "y=" not in attrs
        clip = "" if full else f'<clipPath id="{cid}"><{tag}{shape_attrs}/></clipPath>'
        g_attrs = (f' clip-path="url(#{cid})"' if not full else "") + (f' opacity="{op.group(1)}"' if op else "")
        return f'{clip}<g{g_attrs}>{_texture(kind, box, ink, rnd)}</g>'

    return _TEX_RE.sub(sub, body)


def svg(w, h, body, ink='#000'):
    body = detexture(body, ink)
    i = body.index("</defs>")
    body = body[:i] + f'<clipPath id="cv"><rect width="{w}" height="{h}"/></clipPath>' + body[i:i + 7] + '<g clip-path="url(#cv)">' + body[i + 7:] + "</g>"
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">'
            + body + "</svg>")


def stone_wall(p, rnd, x0, y0, x1, y1, row_h=62, blk=(120, 210), stroke=3, hole=None):
    """Irregular ashlar courses: ink outlines, shadow bands, highlights and cracks, merged per layer.

    `hole` = (cx, cy, r) skips stones hidden behind a round object.
    """
    fills = {k: [] for k in ("stone", "stone2", "stone3")}
    shade, hil, cracks = [], [], []
    y = y0
    while y < y1:
        h = row_h + rnd.randint(-8, 8)
        x = x0 - rnd.randint(0, 120)
        while x < x1:
            w = rnd.randint(*blk)
            j = lambda: rnd.randint(-4, 4)
            poly = [(x + 4 + j(), y + 4 + j()), (x + w - 4 + j(), y + 4 + j()),
                    (x + w - 4 + j(), y + h - 4 + j()), (x + 4 + j(), y + h - 4 + j())]
            fill = rnd.choice(["stone", "stone", "stone2", "stone3"])
            crack = rnd.random() < 0.3
            ccx = x + rnd.randint(20, max(21, w - 30))
            cj = (rnd.randint(-8, 8), rnd.randint(-10, 10))
            if hole and all(math.hypot(px - hole[0], py - hole[1]) < hole[2] for px, py in poly):
                x += w
                continue
            fills[fill].append("M" + "L".join(f"{f(px)} {f(py)}" for px, py in poly) + "Z")
            shade.append(f"M{f(x+4)} {f(y+h-16)}L{f(x+w-4)} {f(y+h-12)}V{f(y+h-4)}H{f(x+4)}Z")
            hil.append(f"M{f(x+10)} {f(y+10)}H{f(x+w*0.55)}")
            if crack:
                cracks.append(f"M{f(ccx)} {f(y+6)}l{cj[0]} 14l{cj[1]} 12")
            x += w
        y += h
    out = [f'<g stroke="{p["ink"]}" stroke-width="{stroke}" stroke-linejoin="round">']
    out += [f'<path d="{"".join(v)}" fill="{p[k]}"/>' for k, v in fills.items() if v]
    out.append("</g>")
    out.append(f'<path d="{"".join(shade)}" fill="{p["ink"]}" opacity=".28"/>')
    out.append(f'<path d="{"".join(hil)}" stroke="{p["hi"]}" stroke-width="2" opacity=".5" stroke-linecap="round"/>')
    out.append(f'<path d="{"".join(cracks)}" fill="none" stroke="{p["ink"]}" stroke-width="2"/>')
    return "".join(out)


def coin(p, cx, cy, rx, ry, rot=0):
    return (f'<g transform="rotate({rot} {f(cx)} {f(cy)})">'
            f'<ellipse cx="{f(cx)}" cy="{f(cy+4)}" rx="{f(rx)}" ry="{f(ry)}" fill="{p["gold2"]}" stroke="{p["ink"]}" stroke-width="3"/>'
            f'<ellipse cx="{f(cx)}" cy="{f(cy)}" rx="{f(rx)}" ry="{f(ry)}" fill="url(#goldg)" stroke="{p["ink"]}" stroke-width="3"/>'
            f'<ellipse cx="{f(cx)}" cy="{f(cy)}" rx="{f(rx*0.68)}" ry="{f(ry*0.68)}" fill="none" stroke="{p["gold2"]}" stroke-width="2.5"/>'
            "</g>")


