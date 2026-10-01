---
name: landing-page
description: Design, build and ship the product's landing page (or redesign it) the way a product designer plus a 3D/motion designer would - research, a show-don't-tell scroll story in one WebGL world with the app's real screens, intro logo motion, custom cursor and text motion, under a strict CSP - then verify it frame by frame and deploy. Use when asked for a landing page, website, marketing site, site redesign, hero/scroll animation, "make the site look great", or a page section (features, CTA, FAQ) in the 3D style.
---

# Landing page

How Zafe's site (`infra/site/`, Astro) went from a plain page to "the Seam Vault": one 3D
world behind the whole page (docs/site.md has every decision round). Read `docs/site.md`,
`docs/brand.md` and the website notes in `AGENTS.md` first; the user's standing taste is
in memory (Vizor-like layout, light only, few words, logo motion).

## 1. Brief: product, audience, the one idea

- Who lands here (Zafe: DAOs and teams who hold money together) and the single thing they
  must get without reading: *nobody can move the money alone, and nobody outside sees it*.
- Copy: less is more. One short line per idea, never repeat the ecosystem name ("no need
  to say zcash zcash everywhere"), only claim shipped features, real numbers only (the
  chain card is a real testnet tx).
- Layout reference the user picked: vizor.cash (centred nav with the mark in the middle,
  big headline, generous paper). Light mode only.

## 2. Research (delegate, keep building)

The user asked for research, and to "think like a 3D designer". Spawn general-purpose
agents in parallel, each writing a report to the job tmp dir, while you keep working:
1. **Choreography**: 10-20 reference sites (scroll stories, product dioramas), how 3D web
   designers plan beats, camera, pacing, cursor and text motion. Ask for ideas, not code.
2. **Tech**: Three.js + postprocessing + GSAP ScrollTrigger + Lenis under our CSP (probe
   it: no inline script/style, `connect-src 'self'`), perf on phones, fallbacks.
3. **Assets**: procedural geometry vs models, SVG extrusion of the logo, screen textures.
Fold the findings into a plan with the beats before building.

## 3. Story: show the mechanism, don't explain it

- Find a physical metaphor built from the brand: Zafe's vault door **is** the Seam mark
  (two extruded leaves), the key **is** the Z, split into shards held by the phones; two
  shards fuse (the key never exists on one phone), the door opens, coins pour into the
  sending screen. Someone who never heard of multisig should vaguely get it.
- Write beats (one caption each, ~8-10 words) and a camera framing per beat **in story
  order** (a curve through non-adjacent framings flies through the scene).
- Use the **app's real screens**: Flutter render tests (`app/tool/screens/*_render_test.dart`)
  → `infra/site/assets.py` crops → WebP → canvas textures redrawn only when their state
  changes. Never rebuild app UI in HTML (rejected as "half-assed"). Show off the best
  screen (the sending screen).
- Things the user rejected, don't bring back: generic "AI slop" sections, pillar/step
  columns, phones side by side and static, small phones, a lock instead of a vault, a
  dark/light vertical split, a cursor companion beside the system arrow, a text label
  following the cursor.

## 4. Build

Stack (all npm, bundled same-origin): Three.js (RoundedBoxGeometry, RoomEnvironment,
SVGLoader `path.toShapes(true)`, VSMShadowMap), pmndrs `postprocessing` (bloom on
emissive/gold only, grain, vignette), GSAP ScrollTrigger, Lenis.
- **CSP**: `script-src 'self'; style-src 'self'`; `build.sh` fails on any inline
  script/style/handler. JS may set `el.style` (CSSOM is allowed); markup may not.
- **One loop**: `new Lenis({ autoRaf: false })`, `lenis.on('scroll', ScrollTrigger.update)`,
  `gsap.ticker.add((t) => lenis.raf(t * 1000))`, `lagSmoothing(0)`.
- **State object** `S` (camera index, frame offsets, per-beat 0..1 values) driven by
  scrubbed timelines; the render loop reads `S` and damps the camera. **One scrubbed
  timeline per scroll range that writes a property**: two triggers writing `S.cam`
  raced after a fast scroll and left the camera short (use `endTrigger` to span
  sections). DOM captions follow `data-beat` set from the timeline.
- **One fixed canvas** behind transparent sections; opaque sections (FAQ, footer) pause
  rendering only when they cover the whole screen.
- **Composition**: asymmetric hero via `camera.setViewOffset` (headline left, world
  right). Portrait: pull wide framings back, convert the side offset into a vertical
  one, and lift the world above bottom cards (`portraitY`). Fog relative to the camera's
  distance, or far framings (the view from above) fog out.
- **Intro**: CSS-only keyframes (full-screen tile → ripple → the nav mark). Centre on
  `50%` of a fixed layer, never `50vw` (includes a classic scrollbar: ripples were off
  the logo).
- **Cursor**: replace the system pointer (the user wants it gone, not accompanied): dot
  exactly on the pointer + trailing ring; links swell the ring; the CTA pill gets a
  wrapping ring and a magnetic lean. Fine pointers and no reduced motion only; add
  `cursor: none` only after the first draw. No labels on the cursor.
- **Text**: masked word rise (own splitter, no plugin), a scramble "unshield" at most
  twice, a rolling counter for the threshold ("2 of 2 approvals").
- **Fallbacks**: no WebGL or reduced motion → static images + stacked cards; the world
  must never be required to read the page.

## 5. Verify every change by looking

```bash
(cd infra/site && ./build.sh)                         # includes the inline-code guard
python3 -m http.server 8768 --bind 127.0.0.1 --directory infra/site/dist   # background
S=.claude/skills/landing-page
bash $S/frames.sh "http://127.0.0.1:8768/?pointer=fine" .story 1440 900 12 "$OUT" story
python3 $S/sheet.py "$OUT" story 3 0.36               # one contact sheet, then Read it
bash $S/frames.sh http://127.0.0.1:8768/ .story 390 844 12 "$OUT" story_m && python3 $S/sheet.py "$OUT" story_m 6 0.5
```
- Desktop 1440x900 **and** phone 390x844, every section you touched; open the sheet and
  the key single frames; fix, rebuild, recapture before reporting.
- Headless Chrome reports no fine pointer: use `?pointer=fine`; move the mouse with
  `agent-browser mouse move` twice before a cursor shot.
- The sandbox refuses complex inline shell and `eval` strings: put scripts in files and
  run them with `bash`.

## 6. Record and ship

- Add a decision round at the top of `docs/site.md` (the feedback quoted, what changed,
  what's next) and update `AGENTS.md` website notes when you learn a gotcha.
- Work in a worktree; commit and push the branch (a PR's `site.yml` run deploys a Vercel
  preview). **When done, push to main and clean up, always** (the user's standing rule):
  merge/push to main once the site build is OK (no need to wait for Rust/Flutter CI),
  confirm the branch is in `origin/main`, delete the remote branch, remove the worktree
  (ExitWorktree remove), `git pull --ff-only` the main checkout. Main deploys
  production: watch the Site run, then check the live URL (status 200, the CSP header,
  new sections present, a screenshot, no page errors).
