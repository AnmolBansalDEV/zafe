#!/usr/bin/env python3
"""Builds Zafe's illustrations into app/assets/illustrations as <name>_light.svg / <name>_dark.svg.

    python3 scripts/illustrations/scenes.py               # all scenes
    python3 scripts/illustrations/scenes.py key_shards    # one scene

Add a scene: write `def my_scene(p): ...` (copy scene_template.py) and register it in SCENES.
"""
import argparse
import math
import random
from pathlib import Path

from zafe_art import PALETTES, coin, defs, f, line_d, pts, stone_wall, svg

ASSETS = Path(__file__).resolve().parents[2] / "app" / "assets" / "illustrations"


# ---------------------------------------------------------------- (a) welcome vault
def welcome(p):
    W, H = 1080, 1240
    rnd = random.Random(7)
    cx, cy = 540, 560
    b = [f'<rect width="{W}" height="{H}" fill="{p["sky1"]}"/>']
    b.append(stone_wall(p, rnd, 0, 0, W, 930, hole=(cx, cy, 395)))
    b.append(f'<rect width="{W}" height="{H}" fill="url(#grit)" opacity=".55"/>')
    # floor
    b.append(f'<rect y="930" width="{W}" height="{H-930}" fill="{p["stone3"]}"/>')
    for i, y in enumerate([930, 975, 1035, 1110, 1200]):
        b.append(f'<path d="M0 {y}H{W}" stroke="{p["ink"]}" stroke-width="3"/>')
    for k in range(-8, 9):
        b.append(f'<path d="M{f(cx + k*40)} 930L{f(cx + k*190)} {H}" stroke="{p["ink"]}" stroke-width="2.5"/>')
    b.append(f'<rect y="930" width="{W}" height="{H-930}" fill="url(#dots)" opacity="{p["texop"]}"/>')
    # recess shadow around the door
    b.append(f'<circle cx="{cx}" cy="{cy}" r="410" fill="{p["ink"]}" opacity=".35"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="392" fill="{p["stone3"]}" stroke="{p["ink"]}" stroke-width="4"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="392" fill="url(#hatch2)" opacity=".45"/>')
    # frame ring
    b.append(f'<circle cx="{cx}" cy="{cy}" r="368" fill="{p["metal"]}" stroke="{p["ink"]}" stroke-width="5"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="336" fill="none" stroke="{p["ink"]}" stroke-width="4"/>')
    for a in range(0, 360, 12):
        r = math.radians(a)
        b.append(f'<circle cx="{f(cx+352*math.cos(r))}" cy="{f(cy+352*math.sin(r))}" r="6" fill="{p["metal2"]}" stroke="{p["ink"]}" stroke-width="2.5"/>')
    # locking bolts at left/right into the frame
    for side in (-1, 1):
        for dy in (-120, 0, 120):
            x = cx + side * 318
            b.append(f'<rect x="{f(x - 40 if side > 0 else x - 20)}" y="{cy+dy-14}" width="60" height="28" rx="6" fill="{p["metal2"]}" stroke="{p["ink"]}" stroke-width="3.5"/>')
    # door face
    b.append(f'<circle cx="{cx}" cy="{cy}" r="318" fill="{p["stone2"]}" stroke="{p["ink"]}" stroke-width="5"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="318" fill="url(#grit)" opacity=".4"/>')
    # lower-right shading on the door (crescent of hatch)
    b.append(f'<path d="M{cx+318} {cy}A318 318 0 0 1 {cx} {cy+318}A318 318 0 0 1 {cx-225} {cy+225}A300 300 0 0 0 {cx+318} {cy}Z" bb="{cx-230} {cy} {cx+318} {cy+318}" fill="url(#hatch)" opacity=".55"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="290" fill="none" stroke="{p["ink"]}" stroke-width="3"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="178" fill="none" stroke="{p["ink"]}" stroke-width="3"/>')
    b.append(f'<path d="M{cx-270} {cy-40}A275 275 0 0 1 {cx-40} {cy-272}" fill="none" stroke="{p["hi"]}" stroke-width="4" stroke-linecap="round" opacity=".7"/>')
    # threshold arc joining the three turned keys (members 0,1,4 -> top, upper-right, upper-left)
    ring = 225
    ang = [-90 + 72 * k for k in range(5)]
    turned = [0, 1, 4]
    a0, a1 = math.radians(ang[4]), math.radians(ang[1])
    b.append(f'<path d="M{f(cx+ring*math.cos(a0))} {f(cy+ring*math.sin(a0))}A{ring} {ring} 0 0 1 {f(cx+ring*math.cos(a1))} {f(cy+ring*math.sin(a1))}" fill="none" stroke="{p["gold"]}" stroke-width="10" stroke-linecap="round" opacity=".9"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="{ring}" fill="none" stroke="{p["ink"]}" stroke-width="2.5" stroke-dasharray="4 10" opacity=".8"/>')
    for k, a in enumerate(ang):
        r = math.radians(a)
        kx, ky = cx + ring * math.cos(r), cy + ring * math.sin(r)
        on = k in turned
        if on:
            b.append(f'<circle cx="{f(kx)}" cy="{f(ky)}" r="110" fill="url(#glow)"/>')
        # escutcheon
        b.append(f'<circle cx="{f(kx)}" cy="{f(ky)}" r="38" fill="{p["metal2"] if not on else p["gold2"]}" stroke="{p["ink"]}" stroke-width="4"/>')
        b.append(f'<circle cx="{f(kx)}" cy="{f(ky)}" r="28" fill="none" stroke="{p["ink"]}" stroke-width="2"/>')
        if on:
            # key inserted: bow pointing outward, rotated a quarter turn (turned)
            t = a + 180  # turned a quarter: the bow lies along the ring
            bx, by = kx, ky - 96
            lobes = "".join(
                f'<circle cx="{f(bx + 24*math.cos(math.radians(q)))}" cy="{f(by + 24*math.sin(math.radians(q)))}" r="22" fill="url(#goldg)" stroke="{p["ink"]}" stroke-width="4"/>'
                for q in (-90, 30, 150))
            b.append(f'<g transform="rotate({f(t)} {f(kx)} {f(ky)})">'
                     f'<rect x="{f(kx-8)}" y="{f(ky-76)}" width="16" height="70" rx="3" fill="url(#goldg)" stroke="{p["ink"]}" stroke-width="4"/>'
                     f'<rect x="{f(kx-17)}" y="{f(ky-50)}" width="34" height="12" rx="4" fill="{p["gold2"]}" stroke="{p["ink"]}" stroke-width="3.5"/>'
                     + lobes +
                     f'<circle cx="{f(bx)}" cy="{f(by)}" r="22" fill="url(#goldg)"/>'
                     f'<circle cx="{f(bx)}" cy="{f(by)}" r="9" fill="{p["stone2"]}" stroke="{p["ink"]}" stroke-width="3.5"/>'
                     f'<path d="M{f(bx-34)} {f(by-6)}a36 36 0 0 1 18 -22" fill="none" stroke="{p["gold3"]}" stroke-width="4" stroke-linecap="round"/>'
                     f'<circle cx="{f(kx)}" cy="{f(ky)}" r="13" fill="{p["gold"]}" stroke="{p["ink"]}" stroke-width="3"/>'
                     "</g>")
        else:
            # empty keyhole
            b.append(f'<path d="M{f(kx)} {f(ky-14)}a10 10 0 1 1 -.1 0zM{f(kx-6)} {f(ky-2)}h12l5 24h-22z" fill="{p["ink"]}"/>')
    # central hand wheel: rim, five spokes (one per member), crimson grips
    b.append(f'<circle cx="{cx}" cy="{cy}" r="122" fill="none" stroke="{p["ink"]}" stroke-width="30"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="122" fill="none" stroke="{p["metal2"]}" stroke-width="20"/>')
    b.append(f'<path d="M{cx-110} {cy-30}A114 114 0 0 1 {cx-30} {cy-110}" fill="none" stroke="{p["hi"]}" stroke-width="5" stroke-linecap="round"/>')
    b.append(f'<path d="M{cx+116} {cy+20}A118 118 0 0 1 {cx+20} {cy+116}" fill="none" stroke="{p["ink"]}" stroke-width="6" stroke-linecap="round" opacity=".6"/>')
    for k, a in enumerate(ang):
        r = math.radians(a + 36)
        x2, y2 = cx + 110 * math.cos(r), cy + 110 * math.sin(r)
        b.append(f'<path d="M{cx} {cy}L{f(x2)} {f(y2)}" stroke="{p["ink"]}" stroke-width="24" stroke-linecap="round"/>')
        b.append(f'<path d="M{cx} {cy}L{f(x2)} {f(y2)}" stroke="{p["metal2"]}" stroke-width="14" stroke-linecap="round"/>')
        gx, gy = cx + 150 * math.cos(r), cy + 150 * math.sin(r)
        b.append(f'<path d="M{f(cx+124*math.cos(r))} {f(cy+124*math.sin(r))}L{f(gx)} {f(gy)}" stroke="{p["ink"]}" stroke-width="26" stroke-linecap="round"/>')
        b.append(f'<path d="M{f(cx+124*math.cos(r))} {f(cy+124*math.sin(r))}L{f(gx)} {f(gy)}" stroke="{p["crimson"]}" stroke-width="17" stroke-linecap="round"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="50" fill="{p["metal"]}" stroke="{p["ink"]}" stroke-width="5"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="50" fill="url(#hatch2)" opacity=".35"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="22" fill="{p["crimson2"]}" stroke="{p["ink"]}" stroke-width="4"/>')
    # coins on the floor
    for (x, y, rx, rot) in [(150, 1010, 34, -8), (205, 1040, 30, 6), (930, 1000, 36, 10),
                            (880, 1060, 28, -4), (990, 1075, 32, 0), (120, 1090, 26, 12)]:
        b.append(coin(p, x, y, rx, rx * 0.42, rot))
    b.append(f'<rect width="{W}" height="{H}" fill="url(#vig)"/>')
    return svg(W, H, defs(p) + "".join(b), p["dot"])


# ---------------------------------------------------------------- (b) create: stone circle
def create(p):
    W, H = 1080, 560
    rnd = random.Random(3)
    b = [f'<rect width="{W}" height="{H}" fill="url(#sky)"/>']
    b.append(f'<rect width="{W}" height="{H}" fill="url(#fine)" opacity="{float(p["texop"])*0.7:.2f}"/>')
    # rising coin-moon
    b.append(f'<circle cx="540" cy="250" r="230" fill="url(#glow)" opacity=".7"/>')
    b.append(f'<circle cx="540" cy="250" r="118" fill="url(#goldg)" stroke="{p["ink"]}" stroke-width="4"/>')
    b.append(f'<circle cx="540" cy="250" r="88" fill="none" stroke="{p["gold2"]}" stroke-width="3"/>')
    b.append(f'<path d="M462 238a80 80 0 0 1 50 -62" fill="none" stroke="{p["gold3"]}" stroke-width="5" stroke-linecap="round"/>')
    # distant hills
    hill = [(0, 360)] + [(x, 350 - 30 * math.sin(x / 150) - rnd.randint(0, 10)) for x in range(0, W + 60, 60)] + [(W, H), (0, H)]
    b.append(f'<polygon points="{pts(hill)}" fill="{p["stone3"]}" stroke="{p["ink"]}" stroke-width="3"/>')
    b.append(f'<polygon points="{pts(hill)}" fill="url(#dots)" opacity="{p["texop"]}"/>')
    # ground
    b.append(f'<path d="M0 430Q540 380 1080 430V560H0Z" fill="{p["stone"]}" stroke="{p["ink"]}" stroke-width="4"/>')
    b.append(f'<path d="M0 470Q540 430 1080 470V560H0Z" bb="0 440 1080 560" fill="url(#hatch2)" opacity=".35"/>')
    for _ in range(40):  # grass ticks
        x, y = rnd.randint(0, W), rnd.randint(440, 550)
        b.append(f'<path d="M{x} {y}l-4 -12M{x+5} {y}l2 -14" stroke="{p["ink"]}" stroke-width="2" opacity=".6"/>')

    def stone(x, base, w, h, lit, lean=0):
        t = base - h
        poly = [(x - w / 2 + lean, t + 10), (x - w / 2 + 8 + lean, t), (x + w / 2 - 6 + lean, t + 4),
                (x + w / 2 + 4, base), (x - w / 2 - 4, base)]
        s = [f'<polygon points="{pts(poly)}" fill="{p["stone2"]}" stroke="{p["ink"]}" stroke-width="4" stroke-linejoin="round"/>']
        sh = [(x + w * 0.1 + lean, t + 6), (x + w / 2 - 6 + lean, t + 4), (x + w / 2 + 4, base), (x + w * 0.15, base)]
        s.append(f'<polygon points="{pts(sh)}" fill="url(#hatch)" opacity=".6"/>')
        s.append(f'<path d="M{f(x-w/2+10+lean)} {f(t+18)}V{f(base-10)}" stroke="{p["hi"]}" stroke-width="3" opacity=".6"/>')
        if lit:
            ry = t + h * 0.4
            s.append(f'<circle cx="{f(x)}" cy="{f(ry)}" r="{f(w*0.9)}" fill="url(#glow)"/>')
            s.append(f'<path d="M{f(x)} {f(ry-18)}l12 18l-12 18l-12 -18z" fill="{p["gold"]}" stroke="{p["ink"]}" stroke-width="3" stroke-linejoin="round"/>')
        else:
            s.append(f'<path d="M{f(x)} {f(t+h*0.4-16)}l10 16l-10 16l-10 -16z" fill="none" stroke="{p["ink"]}" stroke-width="3" opacity=".7"/>')
        return "".join(s)

    # outer ring of unlit stones
    b.append(stone(250, 420, 56, 150, False, -4))
    b.append(stone(830, 420, 56, 150, False, 4))
    b.append(stone(120, 540, 76, 200, False, -6))
    b.append(stone(960, 540, 76, 190, False, 6))
    # the stones that complete the arch
    b.append(stone(410, 480, 92, 260, True))
    b.append(stone(670, 480, 92, 260, True))
    lint = [(344, 218), (736, 212), (742, 262), (338, 268)]
    b.append(f'<circle cx="540" cy="240" r="120" fill="url(#glow)"/>')
    b.append(f'<polygon points="{pts(lint)}" fill="{p["stone2"]}" stroke="{p["ink"]}" stroke-width="4" stroke-linejoin="round"/>')
    b.append(f'<polygon points="340,248 740,242 742,262 338,268" fill="url(#hatch)" opacity=".6"/>')
    b.append(f'<path d="M362 228H716" stroke="{p["hi"]}" stroke-width="3" opacity=".6"/>')
    b.append(f'<path d="M540 222l13 18l-13 18l-13 -18z" fill="{p["gold"]}" stroke="{p["ink"]}" stroke-width="3" stroke-linejoin="round"/>')
    # crimson banner of light connecting the lit stones on the ground
    b.append(f'<path d="M410 486Q540 520 670 486" fill="none" stroke="{p["crimson"]}" stroke-width="6" stroke-dasharray="2 12" stroke-linecap="round"/>')
    b.append(f'<rect width="{W}" height="{H}" fill="url(#vig)"/>')
    return svg(W, H, defs(p) + "".join(b), p["dot"])


# ---------------------------------------------------------------- (c) join: doorway + sealed invite
def join(p):
    W, H = 1080, 560
    rnd = random.Random(11)
    b = [f'<rect width="{W}" height="{H}" fill="{p["sky1"]}"/>']
    b.append('<g transform="translate(0 30)">')
    b.append(stone_wall(p, rnd, 0, -60, W, 440, row_h=54, blk=(100, 180)))
    b.append(f'<rect y="-60" width="{W}" height="500" fill="url(#grit)" opacity=".55"/>')
    cx = 540
    # arch opening
    arch = f"M{cx-150} 440V190A150 150 0 0 1 {cx+150} 190V440Z"
    # voussoirs
    for k in range(11):
        a0 = math.radians(180 + k * 180 / 11)
        a1 = math.radians(180 + (k + 1) * 180 / 11)
        r0, r1 = 150, 205
        q = [(cx + r0 * math.cos(a0), 190 + r0 * math.sin(a0)), (cx + r1 * math.cos(a0), 190 + r1 * math.sin(a0)),
             (cx + r1 * math.cos(a1), 190 + r1 * math.sin(a1)), (cx + r0 * math.cos(a1), 190 + r0 * math.sin(a1))]
        b.append(f'<polygon points="{pts(q)}" fill="{p["stone2"] if k != 5 else p["crimson2"]}" stroke="{p["ink"]}" stroke-width="4" stroke-linejoin="round"/>')
    for side in (-1, 1):
        b.append(f'<rect x="{cx + side*177 - 27}" y="190" width="54" height="250" fill="{p["stone2"]}" stroke="{p["ink"]}" stroke-width="4"/>')
        for y in (250, 320, 390):
            b.append(f'<path d="M{cx + side*177 - 27} {y}h54" stroke="{p["ink"]}" stroke-width="3"/>')
    # warm light inside
    b.append(f'<path d="{arch}" fill="{p["gold3"]}" stroke="{p["ink"]}" stroke-width="5"/>')
    b.append(f'<circle cx="{cx}" cy="300" r="220" fill="url(#glow)"/>')
    # inner room hint: far wall line + a pedestal with a coin
    b.append(f'<path d="M{cx-150} 380H{cx+150}" stroke="{p["gold2"]}" stroke-width="3" opacity=".7"/>')
    # door leaf swung open inward (left), in perspective
    leaf = [(cx - 150, 440), (cx - 150, 190), (cx - 70, 150 + 25), (cx - 70, 420)]
    b.append(f'<path d="M{cx-150} 440V190Q{cx-120} 150 {cx-70} 158V420Z" fill="{p["crimson2"]}" stroke="{p["ink"]}" stroke-width="4" stroke-linejoin="round"/>')
    b.append(f'<path d="M{cx-150} 440V190Q{cx-120} 150 {cx-70} 158V420Z" bb="{cx-150} 150 {cx-70} 440" fill="url(#hatch2)" opacity=".45"/>')
    for y in (230, 360):
        b.append(f'<path d="M{cx-150} {y}L{cx-70} {y-12}" stroke="{p["ink"]}" stroke-width="9"/>')
        b.append(f'<path d="M{cx-150} {y}L{cx-70} {y-12}" stroke="{p["metal2"]}" stroke-width="4"/>')
    # light spilling on the floor
    b.append(f'<rect y="440" width="{W}" height="{H-400}" fill="{p["stone3"]}"/>')
    b.append(f'<path d="M0 440H{W}" stroke="{p["ink"]}" stroke-width="5"/>')
    b.append(f'<path d="M{cx-150} 440L{cx-330} {H}H{cx+390}L{cx+150} 440Z" fill="{p["gold"]}" opacity=".35"/>')
    for y in (470, 510):
        b.append(f'<path d="M0 {y}H{W}" stroke="{p["ink"]}" stroke-width="2.5" opacity=".7"/>')
    b.append(f'<rect y="440" width="{W}" height="{H-440}" fill="url(#dots)" opacity="{p["texop"]}"/>')
    # step
    b.append(f'<path d="M{cx-200} 440h400l20 26h-440z" fill="{p["stone2"]}" stroke="{p["ink"]}" stroke-width="4" stroke-linejoin="round"/>')
    # sealed invite on the step
    ex, ey = cx + 60, 462
    b.append(f'<g transform="rotate(-8 {ex} {ey})">'
             f'<rect x="{ex-70}" y="{ey-40}" width="140" height="86" rx="4" fill="{p["paper"]}" stroke="{p["ink"]}" stroke-width="4"/>'
             f'<path d="M{ex-70} {ey-40}L{ex} {ey+8}L{ex+70} {ey-40}" fill="none" stroke="{p["ink"]}" stroke-width="3.5" stroke-linejoin="round"/>'
             f'<circle cx="{ex}" cy="{ey+8}" r="20" fill="{p["crimson"]}" stroke="{p["ink"]}" stroke-width="3.5"/>'
             f'<path d="M{ex-7} {ey+8}l5 6l10 -12" fill="none" stroke="{p["gold3"]}" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"/>'
             "</g>")
    # wall lanterns
    for side in (-1, 1):
        lx = cx + side * 300
        b.append(f'<circle cx="{lx}" cy="250" r="90" fill="url(#glow)"/>')
        b.append(f'<path d="M{lx} 205v18" stroke="{p["ink"]}" stroke-width="4"/>')
        b.append(f'<path d="M{lx-16} 225h32l-6 50h-20z" fill="{p["gold"]}" stroke="{p["ink"]}" stroke-width="4" stroke-linejoin="round"/>')
        b.append(f'<path d="M{lx-20} 225h40M{lx-14} 277h28" stroke="{p["ink"]}" stroke-width="5" stroke-linecap="round"/>')
    b.append("</g>")
    b.append(f'<rect width="{W}" height="{H}" fill="url(#vig)"/>')
    return svg(W, H, defs(p) + "".join(b), p["dot"])


# ---------------------------------------------------------------- (d) keys: one key, never in one place
def key_shapes(p, fill, stroke, sw, extra=""):
    """A large ornate key in local coordinates, centred on the origin, pointing right."""
    ink = stroke
    out = []
    for q in (180, 60, -60):
        x, y = -300 + 62 * math.cos(math.radians(q)), 62 * math.sin(math.radians(q))
        out.append(f'<circle cx="{f(x)}" cy="{f(y)}" r="66" fill="{fill}" stroke="{ink}" stroke-width="{sw}" {extra}/>')
    out.append(f'<circle cx="-300" cy="0" r="74" fill="{fill}" stroke="{ink}" stroke-width="{sw}" {extra}/>')
    out.append(f'<rect x="-232" y="-34" width="570" height="68" rx="10" fill="{fill}" stroke="{ink}" stroke-width="{sw}" {extra}/>')
    out.append(f'<rect x="-196" y="-58" width="38" height="116" rx="8" fill="{fill}" stroke="{ink}" stroke-width="{sw}" {extra}/>')
    out.append(f'<path d="M212 30V150H256V108H288V150H338V30Z" fill="{fill}" stroke="{ink}" stroke-width="{sw}" stroke-linejoin="round" {extra}/>')
    return "".join(out)


def shards(p):
    W, H = 1080, 1240
    rnd = random.Random(5)
    cx, cy = 540, 470
    b = [f'<rect width="{W}" height="{H}" fill="url(#sky)"/>']
    for a in range(0, 360, 6):
        r = math.radians(a)
        b.append(f'<path d="M{f(cx+140*math.cos(r))} {f(cy+140*math.sin(r))}L{f(cx+900*math.cos(r))} {f(cy+900*math.sin(r))}" stroke="{p["stone3"] if not p["dark"] else p["stone2"]}" stroke-width="{2 if a % 12 else 3.5}" opacity=".8"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="480" fill="url(#fine)" opacity="{p["texop"]}"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="420" fill="url(#glow)" opacity=".35"/>')
    for r in (330, 430):
        b.append(f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{p["hi"] if p["dark"] else p["ink"]}" stroke-width="2.5" stroke-dasharray="3 12" opacity=".5"/>')
    rot = -20
    # ghost of the whole key: dashed, empty. It is never assembled.
    ghost = p["hi"] if p["dark"] else p["ink"]
    b.append(f'<g transform="translate({cx} {cy}) rotate({rot})" opacity=".55">'
             + key_shapes(p, "none", ghost, 3, 'stroke-dasharray="8 10"') + "</g>")
    b[-1] = b[-1].replace(f'rotate({rot})"', f'rotate({rot}) scale(1.04)"')
    # the shards: the same key cut into five jagged pieces, each held apart
    cuts = [-420, -148, -28, 90, 200, 420]

    def zig(x, down=True):
        ys = list(range(-160, 161, 32))
        pts_ = [(x + (rnd.randint(-16, 16) if -48 < y < 48 or x < -150 else 0), y) for y in ys]
        return pts_ if down else pts_[::-1]

    edges = [zig(c) for c in cuts]
    offsets = [(-80, -18, -7), (-30, 48, 6), (0, -46, -4), (34, 42, 6), (84, -14, 10)]
    for i in range(5):
        left, right = edges[i], edges[i + 1]
        poly = left[::-1] + right  # up the left edge, down the right
        poly = [(x, y) for x, y in left] + [(x, y) for x, y in right[::-1]]
        dx, dy, dr = offsets[i]
        body = key_shapes(p, "url(#goldg)", p["ink"], 5)
        # engraving + shade inside the piece
        body += f'<path d="M-150 -16H320" stroke="{p["gold3"]}" stroke-width="6" stroke-linecap="round"/>'
        body += f'<rect x="-232" y="8" width="570" height="26" fill="url(#hatch)" opacity=".35"/>'
        body += f'<circle cx="-300" cy="0" r="30" fill="{p["sky1"]}" stroke="{p["ink"]}" stroke-width="5"/>'
        mid = (cuts[i] + cuts[i + 1]) / 2
        gx = -300 if i == 0 else mid
        gy = -62 if i == 0 else 0
        body += f'<circle cx="{f(gx)}" cy="{f(gy)}" r="18" fill="{p["crimson"]}" stroke="{p["ink"]}" stroke-width="4"/>'
        body += f'<path d="M{f(gx-8)} {f(gy-5)}a9 9 0 0 1 9 -6" stroke="{p["gold3"]}" stroke-width="3" fill="none" stroke-linecap="round"/>' 
        g = (f'<g transform="translate({f(cx+dx)} {f(cy+dy)}) rotate({rot+dr}) scale(1.04)">'
             f'<clipPath id="c{i}"><polygon points="{pts(poly)}"/></clipPath>'
             f'<g clip-path="url(#c{i})">{body}</g>'
             "</g>")
        b.append(g)
        # a member mark on each piece
        mx = max(-300, min(300, mid)) if i else -300
        gx = cx + dx + mx * math.cos(math.radians(rot + dr))
        gy = cy + dy + mx * math.sin(math.radians(rot + dr))

    # jagged ink edges drawn on top so the breaks read clearly
    for i in range(5):
        dx, dy, dr = offsets[i]
        for e in (edges[i], edges[i + 1]):
            if abs(e[0][0]) >= 420:
                continue
            b.append(f'<g transform="translate({f(cx+dx)} {f(cy+dy)}) rotate({rot+dr}) scale(1.04)"><clipPath id="k{i}{f(e[0][0]+500)}">{key_shapes(p, "#000", "none", 0)}</clipPath>'
                     f'<path clip-path="url(#k{i}{f(e[0][0]+500)})" d="{line_d(e)}" fill="none" stroke="{p["ink"]}" stroke-width="7" stroke-linejoin="round"/></g>')
    for _ in range(30):
        a = math.radians(rnd.randint(0, 359))
        r = rnd.randint(300, 520)
        x, y = cx + r * math.cos(a), cy + r * math.sin(a)
        s_ = rnd.randint(4, 10)
        b.append(f'<path d="M{f(x)} {f(y-s_)}L{f(x+s_/3)} {f(y)}L{f(x)} {f(y+s_)}L{f(x-s_/3)} {f(y)}Z" fill="{p["gold"]}"/>')
    b.append(f'<rect width="{W}" height="{H}" fill="url(#vig)"/>')
    return svg(W, H, defs(p) + "".join(b), p["dot"])


SCENES = {"welcome_vault": welcome, "create_stones": create, "join_doorway": join, "key_shards": shards}

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, default=ASSETS, help="output directory (default: app/assets/illustrations)")
    ap.add_argument("names", nargs="*", help="scene names to build (default: all)")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    for name in args.names or SCENES:
        for theme, pal in PALETTES.items():
            path = args.out / f"{name}_{theme}.svg"
            path.write_text(SCENES[name](pal))
            kb = path.stat().st_size / 1024
            print(f"{path}  {kb:.1f} KB" + ("  <-- over the 60 KB budget" if kb > 60 else ""))


if __name__ == "__main__":
    main()
