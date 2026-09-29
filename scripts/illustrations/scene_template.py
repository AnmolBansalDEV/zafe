"""Starting point for a new Zafe illustration. Copy the function into scenes.py,
rename it, register it in SCENES and run scripts/illustrations/preview.sh <name>.

Conventions (docs/illustrations.md has the full guide):
- Canvas 1080 wide. Hero (full-screen top art): 1080 x 1240, subject centred around
  y 450-560, nothing important below y ~620 (it fades out). Banner: 1080 x 560.
- Draw back to front: background, texture, mid-ground, subject, glow, vignette last.
- Ink outlines 3-5 px, stroke-linejoin round. Gold = the thing that matters
  (keys, coins, light), crimson = members/brand marks, neutrals for everything else.
- Texture fills: fill="url(#hatch|hatch2|dots|fine|grit)" (+ bb="x0 y0 x1 y1" on paths).
- No <pattern>, <polyline>, <line>, <text>, filters or blur (flutter_svg): use <path>
  with line_d(), and radial gradients (#glow, #glowc) for light.
"""
import random

from zafe_art import defs, f, line_d, svg  # noqa: F401


def my_scene(p):
    W, H = 1080, 1240
    rnd = random.Random(1)  # fixed seed: output must be deterministic
    cx, cy = 540, 500
    b = [f'<rect width="{W}" height="{H}" fill="url(#sky)"/>']
    b.append(f'<rect width="{W}" height="{H}" fill="url(#fine)" opacity="{p["texop"]}"/>')
    # subject
    b.append(f'<circle cx="{cx}" cy="{cy}" r="260" fill="url(#glow)"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="140" fill="url(#goldg)" stroke="{p["ink"]}" stroke-width="5"/>')
    b.append(f'<circle cx="{cx}" cy="{cy}" r="140" fill="url(#hatch)" opacity=".3"/>')
    b.append(f'<circle cx="{cx}" cy="{cy - 150}" r="18" fill="{p["crimson"]}" stroke="{p["ink"]}" stroke-width="4"/>')
    b.append(f'<path d="{line_d([(cx - 200, cy + 190), (cx, cy + 170), (cx + 200, cy + 190)])}" '
             f'fill="none" stroke="{p["ink"]}" stroke-width="4" stroke-linecap="round"/>')
    for _ in range(12):  # sparks
        x, y = rnd.randint(80, W - 80), rnd.randint(120, 800)
        b.append(f'<path d="M{x} {y - 8}L{x + 3} {y}L{x} {y + 8}L{x - 3} {y}Z" fill="{p["gold"]}"/>')
    b.append(f'<rect width="{W}" height="{H}" fill="url(#vig)"/>')
    return svg(W, H, defs(p) + "".join(b), p["dot"])
