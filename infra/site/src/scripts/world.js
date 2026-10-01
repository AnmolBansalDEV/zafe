// The Seam Vault: the page's 3D world (docs/site.md, "seventh round").
//
// An isometric diorama on grained paper. The vault door is the Seam mark: its two pieces
// are the door leaves and the Z channel between them is the keyhole. Three members'
// phones stand on pedestals around it; each holds a key shard (half of the Z). Scrolling
// plays one payment: a lone shard can't open the lock; Bob proposes (his phone, dark
// mode, under a warm lamp); a pulse runs through the floor to the others; you approve
// with one tap (your phone, light mode, daylight); two shards seat in the Z and fuse into
// the gold key (it never exists on any phone); the door splits along the Z and gold
// light floods out; coins pour into your phone's sending screen; it's sent and joins a
// stream of identical payments; the door closes back into the logo.
//
// State: everything the scroll timeline moves lives in `S`. The frame loop damps toward
// it, poses the objects, redraws the phone screens only when their state changes
// (story-screens.js), and renders through bloom and grain (postprocessing).

import * as THREE from 'three';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { SVGLoader } from 'three/addons/loaders/SVGLoader.js';
import { EffectComposer, RenderPass, EffectPass, BloomEffect, NoiseEffect, VignetteEffect, BlendFunction } from 'postprocessing';
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';
import { W, H, SPOT, loadScreens, makeCanvas, drawBob, drawMe, drawCara, drawMark } from './story-screens.js';

gsap.registerPlugin(ScrollTrigger);

// ---------------------------------------------------------------------------- state

const S = {
  cam: 0, // position along the camera path (0 .. CAM.length - 1)
  frameY: 0.02, // how far down the frame the world sits
  frameX: 0.25, // how far right (the hero's headline is on the left)
  portraitY: 0, // portrait only: extra shift (negative lifts the world above bottom cards)
  ghost: 0, lone: 0, leak: 0.8,
  shardsUp: 0, // shards rise out of the phones
  pulse: 0, // the floor pulse from Bob to the others
  shardA: 0, shardB: 0, fuse: 0, insert: 0, open: 0,
  coins: 0, stream: 0, closeDoor: 0,
  i1: 0, i2: 0, i3: 0, fpulse: 0, ctaZ: 0, // feature islands and the call to action
  aPush1: 0, aPush2: 0, aPush3: 0, aScroll: 0,
  tapA1: 0, tapA2: 0, tapA3: 0, tapB1: 0, tapB2: 0,
  banner: 0, sentBanner: 0, caraDim: 0,
  bSheet: 0, bScroll: 0, bPush: 0, bDone: 0,
};

// ---------------------------------------------------------------------------- helpers

const lerp = (a, b, t) => a + (b - a) * t;
const clamp01 = (v) => Math.min(1, Math.max(0, v));
const smooth = (t) => t * t * (3 - 2 * t);
const damp = (a, b, lambda, dt) => lerp(a, b, 1 - Math.exp(-lambda * dt));

// The Seam mark in its own 0..100 space (public/assets/zafe.svg).
const MARK_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
<path d="M22 26.66Q22 22 26.66 22L71.6 22Q78 22 78 28.4L78 56.95Q78 58.25 76.7 58.25L49.77 58.25Q48.47 58.25 49.39 57.33L73.55 33.17Q74.47 32.25 73.17 32.25L23.3 32.25Q22 32.25 22 30.95Z"/>
<path d="M22 43.05Q22 41.75 23.3 41.75L50.23 41.75Q51.53 41.75 50.61 42.67L26.45 66.83Q25.53 67.75 26.83 67.75L76.7 67.75Q78 67.75 78 69.05L78 73.34Q78 78 73.34 78L28.4 78Q22 78 22 71.6Z"/>
</svg>`;
// The Z channel's centreline, in mark space: top bar, diagonal, bottom bar.
const Z = [
  [22, 37],
  [62, 37],
  [38, 63],
  [78, 63],
];

const DOOR = 2.7; // the door frame's side, world units
const K = DOOR / 100; // mark units to world
const FRAME_DEPTH = 0.5;
const LEAF_DEPTH = 0.22;

// Mark space to door-local (x right, y up, z out of the door).
const markToDoor = (mx, my, z = 0) => new THREE.Vector3((mx - 50) * K, (50 - my) * K, z);

// A paper texture: warm off-white with grain and a few fibres.
function paperTexture() {
  const c = document.createElement('canvas');
  c.width = c.height = 512;
  const g = c.getContext('2d');
  g.fillStyle = '#E4EAE8';
  g.fillRect(0, 0, 512, 512);
  const img = g.getImageData(0, 0, 512, 512);
  for (let i = 0; i < img.data.length; i += 4) {
    const n = (Math.random() - 0.5) * 18;
    img.data[i] += n;
    img.data[i + 1] += n;
    img.data[i + 2] += n;
  }
  g.putImageData(img, 0, 0);
  g.strokeStyle = 'rgba(120,140,136,0.06)';
  g.lineWidth = 1;
  for (let i = 0; i < 90; i++) {
    const x = Math.random() * 512;
    const y = Math.random() * 512;
    const a = Math.random() * Math.PI;
    g.beginPath();
    g.moveTo(x, y);
    g.lineTo(x + Math.cos(a) * 24, y + Math.sin(a) * 24);
    g.stroke();
  }
  const t = new THREE.CanvasTexture(c);
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  t.repeat.set(10, 10);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}

// A plane with rounded corners and 0..1 UVs (the phone screens).
function roundedPlane(w, h, r) {
  const s = new THREE.Shape();
  const x = -w / 2;
  const y = -h / 2;
  s.moveTo(x + r, y);
  s.lineTo(x + w - r, y);
  s.quadraticCurveTo(x + w, y, x + w, y + r);
  s.lineTo(x + w, y + h - r);
  s.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  s.lineTo(x + r, y + h);
  s.quadraticCurveTo(x, y + h, x, y + h - r);
  s.lineTo(x, y + r);
  s.quadraticCurveTo(x, y, x + r, y);
  const geo = new THREE.ShapeGeometry(s, 24);
  const pos = geo.attributes.position;
  const uv = new Float32Array(pos.count * 2);
  for (let i = 0; i < pos.count; i++) {
    uv[i * 2] = (pos.getX(i) - x) / w;
    uv[i * 2 + 1] = (pos.getY(i) - y) / h;
  }
  geo.setAttribute('uv', new THREE.BufferAttribute(uv, 2));
  return geo;
}

// A key shard: bars along part of the Z centreline, in door-local space.
function zBars(segments, material) {
  const group = new THREE.Group();
  for (const [[x1, y1], [x2, y2]] of segments) {
    const a = markToDoor(x1, y1);
    const b = markToDoor(x2, y2);
    const len = a.distanceTo(b) + 8 * K;
    const bar = new THREE.Mesh(new RoundedBoxGeometry(len, 8 * K, 0.12, 3, 0.03), material);
    bar.position.copy(a).lerp(b, 0.5);
    bar.rotation.z = Math.atan2(b.y - a.y, b.x - a.x);
    bar.castShadow = true;
    group.add(bar);
  }
  return group;
}

// ---------------------------------------------------------------------------- scene

export async function start({ canvas, story, hero, features, cta, covers, screens, forceWorld = false, still = false, showStills = () => {} }) {
  // story.js starts fetching the screens while this module downloads.
  const images = await (screens ?? loadScreens());

  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, powerPreference: 'high-performance', stencil: false });
  const mobile = window.matchMedia('(max-width: 760px)').matches;
  // Quality levels, best first. The page starts at the device's level and steps down
  // (never back up, so it can't flicker) while frames are slow; see adapt().
  const dpr = window.devicePixelRatio || 1;
  const LEVELS = [
    { dpr: Math.min(dpr, 2), msaa: 4 },
    { dpr: Math.min(dpr, 1.5), msaa: 2 },
    { dpr: Math.min(dpr, 1.5), msaa: 0 }, // phones start here (as before adaptive quality)
    { dpr: 1, msaa: 0 },
    { dpr: 0.75, msaa: 0 },
  ];
  let level = still || !mobile ? 0 : 2;
  canvas.dataset.quality = String(level);
  let settleUntil = performance.now() + 2000; // adapt() ignores frames until then
  renderer.setPixelRatio(LEVELS[level].dpr);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.NoToneMapping;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.VSMShadowMap;

  const scene = new THREE.Scene();
  const PAPER = new THREE.Color('#F1F5F5');
  scene.background = PAPER;
  scene.fog = new THREE.Fog(PAPER, 30, 60);
  const pmrem = new THREE.PMREMGenerator(renderer);
  scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  scene.environmentIntensity = 0.35;

  // Light: a soft key from the upper left with shadows, sky and ground fill.
  const key = new THREE.DirectionalLight('#FFF3E2', 2.6);
  key.position.set(-6, 12, 8);
  key.castShadow = true;
  key.shadow.mapSize.set(mobile ? 1024 : 2048, mobile ? 1024 : 2048);
  key.shadow.camera.left = -9;
  key.shadow.camera.right = 9;
  key.shadow.camera.top = 9;
  key.shadow.camera.bottom = -9;
  key.shadow.radius = 10;
  key.shadow.blurSamples = 16;
  key.shadow.bias = -0.0005;
  scene.add(key, new THREE.HemisphereLight('#FFFFFF', '#B9C7C4', 0.45));

  // Floor.
  const floor = new THREE.Mesh(
    new THREE.PlaneGeometry(90, 90),
    new THREE.MeshStandardMaterial({ map: paperTexture(), roughness: 1, metalness: 0 }),
  );
  floor.rotation.x = -Math.PI / 2;
  floor.receiveShadow = true;
  scene.add(floor);

  const clay = (color, extra = {}) => new THREE.MeshStandardMaterial({ color, roughness: 0.78, metalness: 0.02, ...extra });

  // ---------------------------------------------------------------- the vault
  const VAULT_POS = new THREE.Vector3(0, 0, -0.6);
  const vault = new THREE.Group();
  vault.position.copy(VAULT_POS);
  scene.add(vault);
  const plinth = new THREE.Mesh(new RoundedBoxGeometry(3.6, 0.4, 1.5, 4, 0.08), clay('#CFD9D7'));
  plinth.position.y = 0.2;
  plinth.castShadow = plinth.receiveShadow = true;
  vault.add(plinth);
  const door = new THREE.Group(); // door-local: origin at the door's centre, +z out
  door.position.set(0, 0.4 + DOOR / 2, 0);
  vault.add(door);
  const doorFrame = new THREE.Mesh(new RoundedBoxGeometry(DOOR, DOOR, FRAME_DEPTH, 6, 0.36), clay('#004A46', { roughness: 0.55 }));
  doorFrame.castShadow = doorFrame.receiveShadow = true;
  door.add(doorFrame);
  const FRONT = FRAME_DEPTH / 2;
  // Gold light behind the leaves: only the Z channel lets it out.
  const glowCanvas = document.createElement('canvas');
  glowCanvas.width = glowCanvas.height = 256;
  {
    const g = glowCanvas.getContext('2d');
    const grad = g.createRadialGradient(128, 128, 10, 128, 128, 180);
    grad.addColorStop(0, '#FFF1C4');
    grad.addColorStop(0.5, '#F3BA3C');
    grad.addColorStop(1, '#8A5E08');
    g.fillStyle = grad;
    g.fillRect(0, 0, 256, 256);
  }
  const glowTex = new THREE.CanvasTexture(glowCanvas);
  glowTex.colorSpace = THREE.SRGBColorSpace;
  const glowMat = new THREE.MeshBasicMaterial({ map: glowTex, color: new THREE.Color('#FFFFFF'), transparent: true, toneMapped: false });
  const glow = new THREE.Mesh(new THREE.PlaneGeometry(58 * K, 58 * K), glowMat);
  glow.position.z = FRONT + 0.004;
  door.add(glow);
  const vaultLight = new THREE.PointLight('#F6C35A', 0, 9, 1.4);
  vaultLight.position.set(0, 0, FRONT - 0.2);
  door.add(vaultLight);
  // Inside: a gold-lit recess with stacks of coins and rays, seen when the door opens.
  const inside = new THREE.Group();
  inside.position.z = FRONT - 0.02;
  door.add(inside);
  const recess = new THREE.Mesh(
    new THREE.BoxGeometry(58 * K, 58 * K, 0.6),
    new THREE.MeshStandardMaterial({ color: '#3A2A08', roughness: 0.9, side: THREE.BackSide }),
  );
  recess.position.z = -0.3;
  inside.add(recess);
  const stackMat = new THREE.MeshPhysicalMaterial({ color: '#F3BA3C', metalness: 1, roughness: 0.25, emissive: '#A5750F', emissiveIntensity: 0.5 });
  for (let i = 0; i < 7; i++) {
    const n = 3 + ((i * 5) % 6);
    const stack = new THREE.Mesh(new THREE.CylinderGeometry(0.11, 0.11, 0.028 * n, 32), stackMat);
    stack.position.set(-0.55 + (i % 4) * 0.36 + (i > 3 ? 0.18 : 0), -0.62 + 0.014 * n, -0.45 + (i > 3 ? 0.2 : 0));
    inside.add(stack);
  }
  const haloCanvas = document.createElement('canvas');
  haloCanvas.width = haloCanvas.height = 256;
  {
    const g = haloCanvas.getContext('2d');
    const grad = g.createRadialGradient(128, 128, 0, 128, 128, 128);
    grad.addColorStop(0, 'rgba(255,226,150,0.9)');
    grad.addColorStop(0.35, 'rgba(243,186,60,0.35)');
    grad.addColorStop(1, 'rgba(243,186,60,0)');
    g.fillStyle = grad;
    g.fillRect(0, 0, 256, 256);
  }
  const rayMat = new THREE.SpriteMaterial({ map: new THREE.CanvasTexture(haloCanvas), transparent: true, opacity: 0, depthWrite: false, blending: THREE.AdditiveBlending, toneMapped: false });
  const halo = new THREE.Sprite(rayMat);
  halo.scale.setScalar(4.2);
  halo.position.z = 0.9;
  inside.add(halo);

  // The leaves: the mark's two pieces, extruded.
  const svg = new SVGLoader().parse(MARK_SVG);
  const leafMats = [clay('#D9F5F2', { roughness: 0.6 }), clay('#51DDD2', { roughness: 0.6 })];
  const leaves = svg.paths.map((path, i) => {
    const geo = new THREE.ExtrudeGeometry(path.toShapes(true), {
      depth: LEAF_DEPTH / K,
      bevelEnabled: true,
      bevelThickness: 0.8,
      bevelSize: 0.6,
      bevelSegments: 3,
      curveSegments: 12,
    });
    geo.translate(-50, -50, 0);
    geo.rotateX(Math.PI); // mark y is down: turn over so the front faces +z
    geo.scale(K, K, K);
    geo.translate(0, 0, FRONT + LEAF_DEPTH + 0.01);
    const leaf = new THREE.Mesh(geo, leafMats[i]);
    leaf.castShadow = true;
    const pivot = new THREE.Group();
    pivot.add(leaf);
    door.add(pivot);
    return pivot;
  });
  const LEAF_FRONT = FRONT + LEAF_DEPTH + 0.02;
  // The ghost keyhole: a teal outline of the Z.
  const ghostCurve = new THREE.CurvePath();
  for (let i = 0; i < Z.length - 1; i++) {
    ghostCurve.add(new THREE.LineCurve3(markToDoor(...Z[i], LEAF_FRONT + 0.02), markToDoor(...Z[i + 1], LEAF_FRONT + 0.02)));
  }
  const ghostMat = new THREE.MeshBasicMaterial({ color: new THREE.Color('#51DDD2').multiplyScalar(2.2), transparent: true, opacity: 0, toneMapped: false });
  const ghost = new THREE.Mesh(new THREE.TubeGeometry(ghostCurve, 120, 0.035, 8, false), ghostMat);
  door.add(ghost);

  // ---------------------------------------------------------------- key shards
  const shardMat = () =>
    new THREE.MeshPhysicalMaterial({ color: '#51DDD2', emissive: new THREE.Color('#51DDD2'), emissiveIntensity: 1.8, roughness: 0.25, clearcoat: 1, metalness: 0.1 });
  const MID = [50, 50];
  const halfA = [[Z[0], Z[1]], [Z[1], MID]]; // top bar + upper diagonal
  const halfB = [[MID, Z[2]], [Z[2], Z[3]]]; // lower diagonal + bottom bar
  const shards = {
    bob: zBars(halfA, shardMat()),
    me: zBars(halfB, shardMat()),
    cara: zBars(halfB, shardMat()),
  };
  for (const sh of Object.values(shards)) scene.add(sh);
  const goldKey = new THREE.MeshPhysicalMaterial({ color: '#F3BA3C', metalness: 1, roughness: 0.22, emissive: new THREE.Color('#F3BA3C'), emissiveIntensity: 0, clearcoat: 1 });

  // ---------------------------------------------------------------- phones
  const SCREEN_W = 0.96;
  const SCREEN_H = (SCREEN_W * H) / W;
  const BODY_W = SCREEN_W + 0.1;
  const BODY_H = SCREEN_H + 0.1;
  const PHONE_DEPTH = 0.1;
  const PEDESTAL_H = 0.3;
  function makePhone(frameColor, lampColor, lampIntensity) {
    const root = new THREE.Group();
    const pedestal = new THREE.Mesh(new THREE.CylinderGeometry(0.7, 0.78, PEDESTAL_H, 48), clay('#CFD9D7'));
    pedestal.position.y = PEDESTAL_H / 2;
    pedestal.castShadow = pedestal.receiveShadow = true;
    root.add(pedestal);
    const phone = new THREE.Group();
    phone.position.y = PEDESTAL_H + BODY_H / 2 + 0.06;
    root.add(phone);
    const body = new THREE.Mesh(
      new RoundedBoxGeometry(BODY_W, BODY_H, PHONE_DEPTH, 8, 0.14),
      new THREE.MeshPhysicalMaterial({ color: frameColor, metalness: 0.7, roughness: 0.32, clearcoat: 0.8 }),
    );
    body.castShadow = true;
    phone.add(body);
    const c = makeCanvas();
    const tex = new THREE.CanvasTexture(c);
    tex.colorSpace = THREE.SRGBColorSpace;
    tex.anisotropy = renderer.capabilities.getMaxAnisotropy();
    const screenMat = new THREE.MeshBasicMaterial({ map: tex, toneMapped: false });
    const screen = new THREE.Mesh(roundedPlane(SCREEN_W, SCREEN_H, 0.11), screenMat);
    screen.position.z = PHONE_DEPTH / 2 + 0.002;
    phone.add(screen);
    const lamp = new THREE.PointLight(lampColor, lampIntensity, 5, 2);
    lamp.position.set(0.6, 2.6, 1.2);
    root.add(lamp);
    scene.add(root);
    return { root, phone, canvas: c, g: c.getContext('2d'), tex, screenMat, key: '' };
  }
  const bob = makePhone('#2B3131', '#FFB36B', 5);
  const me = makePhone('#D5DCDB', '#D8ECFF', 3);
  const cara = makePhone('#C8B79A', '#FFFFFF', 0);
  const PHONES = {
    bob: new THREE.Vector3(-3.4, 0, 1.2),
    me: new THREE.Vector3(3.4, 0, 1.2),
    cara: new THREE.Vector3(2.4, 0, -3.3),
  };
  bob.root.position.copy(PHONES.bob);
  me.root.position.copy(PHONES.me);
  cara.root.position.copy(PHONES.cara);

  // A point on a phone's screen (fractions of the viewport) in world space.
  const screenPoint = (p, fx, fy, out = new THREE.Vector3()) =>
    p.phone.localToWorld(out.set((fx - 0.5) * SCREEN_W, (0.5 - fy) * SCREEN_H, PHONE_DEPTH / 2 + 0.03));
  const phoneTop = (p, out = new THREE.Vector3()) => p.phone.localToWorld(out.set(0, BODY_H / 2 + 0.95, 0));

  // ---------------------------------------------------------------- floor channels and the pulse
  // Z-shaped grooves from Bob's pedestal to the other two.
  const channelMat = clay('#CBD6D4', { roughness: 0.9 });
  const pulseMat = new THREE.MeshBasicMaterial({ color: new THREE.Color('#51DDD2').multiplyScalar(3), toneMapped: false });
  function zPath(a, b) {
    const mid1 = new THREE.Vector3(lerp(a.x, b.x, 0.25), 0.012, a.z + 1.2);
    const mid2 = new THREE.Vector3(lerp(a.x, b.x, 0.75), 0.012, b.z + 1.2);
    const p = new THREE.CurvePath();
    const pts = [new THREE.Vector3(a.x, 0.012, a.z), mid1, mid2, new THREE.Vector3(b.x, 0.012, b.z)];
    for (let i = 0; i < 3; i++) p.add(new THREE.LineCurve3(pts[i], pts[i + 1]));
    return p;
  }
  const paths = [zPath(PHONES.bob, PHONES.me), zPath(PHONES.bob, PHONES.cara)];
  for (const p of paths) {
    const groove = new THREE.Mesh(new THREE.TubeGeometry(p, 80, 0.07, 6, false), channelMat);
    groove.scale.y = 0.25;
    groove.receiveShadow = true;
    scene.add(groove);
  }
  const pulses = paths.map(() => {
    const m = new THREE.Mesh(new THREE.SphereGeometry(0.11, 16, 16), pulseMat);
    scene.add(m);
    return m;
  });

  // ---------------------------------------------------------------- coins and the stream
  const coinGeo = new THREE.CylinderGeometry(0.1, 0.1, 0.025, 40);
  const COINS = 18;
  const coins = new THREE.InstancedMesh(coinGeo, new THREE.MeshPhysicalMaterial({ color: '#F3BA3C', metalness: 1, roughness: 0.2, emissive: '#6B4A00', emissiveIntensity: 0.4 }), COINS);
  coins.castShadow = true;
  scene.add(coins);
  const STREAM = 44;
  const stream = new THREE.InstancedMesh(coinGeo, clay('#C3CCCB', { roughness: 0.5, metalness: 0.4 }), STREAM);
  scene.add(stream);
  const tmp = new THREE.Object3D();

  // ---------------------------------------------------------------- feature islands
  const ISLANDS = [new THREE.Vector3(14, 0, 0.6), new THREE.Vector3(23.5, 0, -1.4), new THREE.Vector3(33, 0, 0.6)];
  const islandBase = (pos, w, d) => {
    const base = new THREE.Mesh(new RoundedBoxGeometry(w, 0.32, d, 4, 0.1), clay('#CFD9D7'));
    base.position.copy(pos).setY(0.16);
    base.castShadow = base.receiveShadow = true;
    scene.add(base);
    return base;
  };
  // A label in a white pill with a teal check (the island's badges).
  function badge(text) {
    const c = document.createElement('canvas');
    c.width = 512;
    c.height = 128;
    const g = c.getContext('2d');
    g.fillStyle = '#FFFFFF';
    g.beginPath();
    g.roundRect(8, 8, 496, 112, 56);
    g.fill();
    g.fillStyle = '#E0F3F1';
    g.beginPath();
    g.arc(64, 64, 34, 0, Math.PI * 2);
    g.fill();
    g.strokeStyle = '#00736C';
    g.lineWidth = 9;
    g.lineCap = 'round';
    g.lineJoin = 'round';
    g.beginPath();
    g.moveTo(48, 66);
    g.lineTo(60, 78);
    g.lineTo(82, 52);
    g.stroke();
    g.fillStyle = '#090E0E';
    g.font = '500 52px "DM Sans", sans-serif';
    g.fillText(text, 116, 82);
    const t = new THREE.CanvasTexture(c);
    t.colorSpace = THREE.SRGBColorSpace;
    const sp = new THREE.Sprite(new THREE.SpriteMaterial({ map: t, transparent: true, toneMapped: false }));
    sp.scale.set(1.6, 0.4, 1);
    scene.add(sp);
    return sp;
  }

  // 1. Checked on every phone: the review screen, a scan beam, three badges.
  islandBase(ISLANDS[0], 3.4, 2.2);
  const checker = makePhone('#D5DCDB', '#D8ECFF', 2);
  checker.root.position.copy(ISLANDS[0]).setY(0.32);
  checker.g.drawImage(images.meReview, 0, -(images.meReview.height - H) * (W / images.meReview.width) * 0.86, W, (images.meReview.height * W) / images.meReview.width);
  checker.tex.needsUpdate = true;
  const beam = new THREE.Mesh(
    new THREE.PlaneGeometry(SCREEN_W, 0.12),
    new THREE.MeshBasicMaterial({ color: new THREE.Color('#51DDD2').multiplyScalar(2), transparent: true, opacity: 0, toneMapped: false, depthWrite: false, blending: THREE.AdditiveBlending }),
  );
  beam.position.z = PHONE_DEPTH / 2 + 0.01;
  checker.phone.add(beam);
  const badges = ['Recipient', 'Amount', 'Fee'].map(badge);

  // 2. Invisible on-chain: a chain of glassy blocks of identical tokens; a gold coin joins.
  islandBase(ISLANDS[1], 7.4, 1.8);
  const blockMat = new THREE.MeshPhysicalMaterial({ color: '#7FCFC6', roughness: 0.1, clearcoat: 1, transparent: true, opacity: 0.32, depthWrite: false });
  const tokenMat = clay('#9FB0AD', { roughness: 0.45, metalness: 0.4 });
  const BLOCKS = 6;
  const blockX = (i) => ISLANDS[1].x - 2.75 + i * 1.1;
  for (let i = 0; i < BLOCKS; i++) {
    const block = new THREE.Mesh(new RoundedBoxGeometry(0.86, 0.86, 0.86, 4, 0.12), blockMat);
    block.position.set(blockX(i), 0.32 + 0.45, ISLANDS[1].z);
    scene.add(block);
    for (let j = 0; j < 3; j++) {
      const tok = new THREE.Mesh(coinGeo, tokenMat);
      tok.position.set(blockX(i) - 0.2 + j * 0.2, 0.32 + 0.3 + j * 0.12, ISLANDS[1].z + (j - 1) * 0.12);
      tok.rotation.set(Math.PI / 2 + j, 0.3 * j, 0);
      scene.add(tok);
    }
    if (i < BLOCKS - 1) {
      const link = new THREE.Mesh(new THREE.CylinderGeometry(0.05, 0.05, 0.3, 12), clay('#B9C7C4'));
      link.rotation.z = Math.PI / 2;
      link.position.set(blockX(i) + 0.55, 0.77, ISLANDS[1].z);
      scene.add(link);
    }
  }
  const traveller = new THREE.Mesh(coinGeo, coins.material);
  traveller.scale.setScalar(2.2);
  scene.add(traveller);

  // 3. Private all the way: a tunnel of arches a packet passes through; a sealed backup.
  islandBase(ISLANDS[2], 4.6, 2.6);
  const archMats = [];
  const arches = [0, 1, 2, 3].map((i) => {
    const m = clay(['#004A46', '#00736C', '#2FA79E', '#51DDD2'][i], { roughness: 0.5, emissive: new THREE.Color('#51DDD2'), emissiveIntensity: 0 });
    archMats.push(m);
    const arch = new THREE.Mesh(new THREE.TorusGeometry(1.0 - i * 0.06, 0.11, 16, 48, Math.PI), m);
    arch.rotation.y = Math.PI / 2;
    arch.position.set(ISLANDS[2].x - 1.3 + i * 0.75, 0.32, ISLANDS[2].z - 0.2);
    arch.castShadow = true;
    scene.add(arch);
    return arch;
  });
  const packet = new THREE.Mesh(new RoundedBoxGeometry(0.4, 0.4, 0.4, 3, 0.08), new THREE.MeshBasicMaterial({ color: new THREE.Color('#51DDD2').multiplyScalar(2.5), toneMapped: false }));
  scene.add(packet);
  const envelope = new THREE.Group();
  const envBody = new THREE.Mesh(new RoundedBoxGeometry(1.1, 0.72, 0.06, 2, 0.03), clay('#FFFFFF', { roughness: 0.7 }));
  const flapShape = new THREE.Shape();
  flapShape.moveTo(-0.55, 0.36);
  flapShape.lineTo(0.55, 0.36);
  flapShape.lineTo(0, -0.05);
  flapShape.closePath();
  const flap = new THREE.Mesh(new THREE.ShapeGeometry(flapShape), clay('#EEF2F1', { side: THREE.DoubleSide }));
  flap.position.z = 0.035;
  const seal = new THREE.Mesh(new THREE.CylinderGeometry(0.11, 0.11, 0.04, 32), coins.material);
  seal.rotation.x = Math.PI / 2;
  seal.position.set(0, -0.02, 0.06);
  envelope.add(envBody, flap, seal);
  envelope.position.set(ISLANDS[2].x + 1.6, 0.32 + 0.5, ISLANDS[2].z + 0.6);
  envelope.rotation.set(-0.2, -0.5, 0.08);
  envelope.traverse((m) => m.isMesh && (m.castShadow = true));
  scene.add(envelope);

  // Floor channels from the vault on to the islands, and a pulse that runs ahead.
  const islandPath = new THREE.CurvePath();
  {
    const pts = [
      new THREE.Vector3(2.4, 0.012, -0.2),
      new THREE.Vector3(8, 0.012, -0.2),
      new THREE.Vector3(ISLANDS[0].x - 1.8, 0.012, ISLANDS[0].z + 1.4),
      new THREE.Vector3(ISLANDS[1].x - 4, 0.012, ISLANDS[1].z + 1.2),
      new THREE.Vector3(ISLANDS[1].x + 4, 0.012, ISLANDS[1].z + 1.2),
      new THREE.Vector3(ISLANDS[2].x - 2.4, 0.012, ISLANDS[2].z + 1.6),
    ];
    for (let i = 0; i < pts.length - 1; i++) islandPath.add(new THREE.LineCurve3(pts[i], pts[i + 1]));
  }
  {
    const groove = new THREE.Mesh(new THREE.TubeGeometry(islandPath, 160, 0.07, 6, false), channelMat);
    groove.scale.y = 0.25;
    groove.receiveShadow = true;
    scene.add(groove);
  }
  const islandPulse = new THREE.Mesh(new THREE.SphereGeometry(0.12, 16, 16), pulseMat);
  scene.add(islandPulse);

  // The big Z in the floor around the vault, lit for the call to action (read from above).
  const bigZ = new THREE.CurvePath();
  {
    const z = [new THREE.Vector3(-5.6, 0.015, -4.6), new THREE.Vector3(5.6, 0.015, -4.6), new THREE.Vector3(-5.6, 0.015, 4.8), new THREE.Vector3(5.6, 0.015, 4.8)];
    for (let i = 0; i < 3; i++) bigZ.add(new THREE.LineCurve3(z[i], z[i + 1]));
  }
  const bigZMat = new THREE.MeshBasicMaterial({ color: new THREE.Color('#51DDD2').multiplyScalar(1.8), transparent: true, opacity: 0, toneMapped: false });
  const bigZMesh = new THREE.Mesh(new THREE.TubeGeometry(bigZ, 200, 0.16, 8, false), bigZMat);
  bigZMesh.scale.y = 0.2;
  scene.add(bigZMesh);

  // ---------------------------------------------------------------- camera
  const camera = new THREE.PerspectiveCamera(20, 1, 0.5, 200);
  // One continuous take: positions and targets per beat, joined by curves.
  const iso = (target, dist, yaw = 0.55, pitch = 0.5) =>
    [target.clone().add(new THREE.Vector3(Math.sin(yaw) * Math.cos(pitch), Math.sin(pitch), Math.cos(yaw) * Math.cos(pitch)).multiplyScalar(dist)), target];
  const doorCentre = VAULT_POS.clone().add(new THREE.Vector3(0, 0.4 + DOOR / 2, 0));
  const bobFace = PHONES.bob.clone().add(new THREE.Vector3(0, 1.45, 0));
  const meFace = PHONES.me.clone().add(new THREE.Vector3(0, 1.45, 0));
  // In the order the story visits them, so every move goes to the next point on the path.
  const CAM = [
    iso(new THREE.Vector3(0, 1.2, 0), 25, 0.62, 0.4), // 0 hero: the whole diorama
    iso(new THREE.Vector3(0, 1.3, -0.2), 24, 0.4), // 1 shares
    iso(doorCentre.clone(), 17, 0.25, 0.35), // 2 alone: the lock
    iso(bobFace.clone(), 9.5, -0.62, 0.2), // 3 Bob proposes
    iso(new THREE.Vector3(0, 0.8, 0.4), 26, 0.3, 0.75), // 4 the pulse, from above
    iso(bobFace.clone(), 9.8, -0.5, 0.24), // 5 Bob approves
    iso(doorCentre.clone(), 13, -0.1, 0.25), // 6 his shard seats
    iso(meFace.clone(), 9.5, 0.72, 0.2), // 7 you approve
    iso(doorCentre.clone(), 12, 0.18, 0.22), // 8 yours seats: the key fuses
    iso(doorCentre.clone(), 7.5, 0.12, 0.1), // 9 the door opens: in close
    iso(meFace.clone(), 10.5, 0.78, 0.28), // 10 sending
    iso(new THREE.Vector3(0, 1.1, 0), 30), // 11 sent: back to the diorama
    iso(ISLANDS[0].clone().setY(1.3).add(new THREE.Vector3(0.6, 0, 0)), 13.5, 0.38, 0.26), // 12 island: checked
    iso(ISLANDS[1].clone().setY(0.8), 14.5, 0.3, 0.34), // 13 island: on-chain
    iso(ISLANDS[2].clone().setY(0.9), 13, 0.45, 0.3), // 14 island: private
    [new THREE.Vector3(0.8, 44, 12), new THREE.Vector3(0, 0, 0.6)], // 15 from above: the Seam
  ];
  // Portrait screens: the wide shots pull back so the side phones stay in frame.
  let camPos = null;
  let camTarget = null;
  function buildCamera(portrait) {
    // Portrait pulls the wide shots back just enough for the diorama to fit the width
    // (1.55 left it at half the screen on phones, 1.3 cut the edge phones), and the islands (12-14) further, as
    // their cards sit below them.
    const pos = CAM.map(([p, t], i) => {
      const d = p.distanceTo(t);
      return portrait && (d > 14 || (i >= 12 && i <= 14)) ? t.clone().add(p.clone().sub(t).multiplyScalar(i >= 12 && i <= 14 ? 1.7 : 1.42)) : p;
    });
    camPos = new THREE.CatmullRomCurve3(pos, false, 'centripetal');
    camTarget = new THREE.CatmullRomCurve3(CAM.map((c) => c[1]), false, 'centripetal');
  }
  buildCamera(false);
  const cur = { pos: CAM[0][0].clone(), target: CAM[0][1].clone(), frameY: S.frameY, frameX: S.frameX };

  // ---------------------------------------------------------------- post
  const composer = new EffectComposer(renderer, { multisampling: LEVELS[level].msaa, frameBufferType: THREE.HalfFloatType });
  composer.addPass(new RenderPass(scene, camera));
  const bloom = new BloomEffect({ luminanceThreshold: 0.92, luminanceSmoothing: 0.2, intensity: 1.1, mipmapBlur: true, radius: 0.7 });
  const noise = new NoiseEffect({ blendFunction: BlendFunction.OVERLAY });
  noise.blendMode.opacity.value = 0.12;
  const vignette = new VignetteEffect({ offset: 0.35, darkness: 0.28 });
  composer.addPass(new EffectPass(camera, bloom, noise, vignette));

  // ---------------------------------------------------------------- layout
  let portrait = false;
  function resize() {
    const w = window.innerWidth;
    const h = window.innerHeight;
    renderer.setSize(w, h, false);
    composer.setSize(w, h);
    camera.aspect = w / h;
    portrait = w / h < 0.8;
    camera.fov = portrait ? 30 : 20;
    if (camPos) buildCamera(portrait);
    camera.updateProjectionMatrix();
    settleUntil = performance.now() + 2000;
  }
  resize();
  window.addEventListener('resize', resize);

  // ---------------------------------------------------------------- pointer
  let px = 0;
  let py = 0;
  window.addEventListener('pointermove', (e) => {
    px = e.clientX / window.innerWidth - 0.5;
    py = e.clientY / window.innerHeight - 0.5;
  });
  const tilt = { x: 0, y: 0 };

  // ---------------------------------------------------------------- frame
  const vA = new THREE.Vector3();
  const vB = new THREE.Vector3();
  const vC = new THREE.Vector3();
  const qTmp = new THREE.Quaternion();
  let last = performance.now();
  const t0 = last;
  let running = true;

  function shardPose(shard, owner, progress, lone, time, seatOffset) {
    // Rest: floating above the phone. Flight: an arc to the door. Seated: in the Z.
    const rise = smooth(clamp01(S.shardsUp));
    phoneTop(owner, vA);
    vA.y += Math.sin(time * 1.4 + seatOffset) * 0.06 - (1 - rise) * 0.9;
    // The seat, in world space (door-local origin, pushed in by `insert`).
    door.localToWorld(vB.set(0, 0, 0.62 - 0.24 * S.insert));
    const k = smooth(clamp01(progress));
    shard.position.lerpVectors(vA, vB, k);
    shard.position.y += Math.sin(k * Math.PI) * 1.4;
    if (lone > 0) {
      // A lone shard: flies at the lock, bounces off, drops back.
      const up = Math.sin(clamp01(lone) * Math.PI);
      door.localToWorld(vC.set(0, 0, 0.9));
      shard.position.lerp(vC, up * 0.92);
      shard.position.x += Math.sin(clamp01(lone) * Math.PI * 3) * 0.12 * up;
    }
    // Orientation: spinning gently at rest, squared up with the door when seated.
    door.getWorldQuaternion(qTmp);
    const spin = new THREE.Quaternion().setFromEuler(new THREE.Euler(0.25 * Math.sin(time + seatOffset), time * 0.6 + seatOffset, 0.15));
    shard.quaternion.copy(spin).slerp(qTmp, Math.max(k, Math.sin(clamp01(lone) * Math.PI)));
    const s = rise * (0.82 + 0.18 * k);
    shard.scale.setScalar(Math.max(0.0001, s));
  }

  function screenKey(obj) {
    return Object.values(obj).map((v) => (typeof v === 'number' ? v.toFixed(3) : v)).join('|');
  }

  // Adaptive quality: average frame time over ~1 s windows; a slow window (under ~45
  // fps) drops one level. Ignores frames for 2 s after start or a resize (shader
  // compilation, texture uploads) and 1 s after the world or the tab was hidden. Still
  // under ~20 fps at the lowest level: give up and show the stills (giveUp).
  let winMs = 0;
  let winFrames = 0;
  document.addEventListener('visibilitychange', () => {
    settleUntil = performance.now() + 1000;
  });
  function adapt(now, ms) {
    if (still) return; // capture mode keeps the best quality however slow
    if (now < settleUntil) {
      winMs = 0;
      winFrames = 0;
      return;
    }
    winMs += Math.min(ms, 1000);
    winFrames++;
    if (winMs < 1000) return;
    const avg = winMs / winFrames;
    winMs = 0;
    winFrames = 0;
    if (avg <= 22) return;
    if (level === LEVELS.length - 1) {
      if (avg > 50 && !forceWorld) giveUp();
      return;
    }
    level++;
    renderer.setPixelRatio(LEVELS[level].dpr);
    composer.multisampling = LEVELS[level].msaa;
    composer.setSize(window.innerWidth, window.innerHeight);
    if (level >= 2 && key.shadow.mapSize.x > 1024) {
      // Soft (VSM) shadows hide the lower resolution; three rebuilds the map.
      key.shadow.mapSize.set(1024, 1024);
      key.shadow.map?.dispose();
      key.shadow.map = null;
    }
    canvas.dataset.quality = String(level); // for perf checks
    settleUntil = now + 1000;
  }

  function frame() {
    if (!running) return;
    const now = performance.now();
    adapt(now, now - last);
    const dt = Math.min(0.05, (now - last) / 1000);
    const camDt = still ? 1 : dt; // capture mode: the camera lands at once
    last = now;
    const time = (now - t0) / 1000;

    // Camera along the take, damped, with pointer tilt.
    const u = clamp01(S.cam / (CAM.length - 1));
    camPos.getPoint(u, vA);
    camTarget.getPoint(u, vB);
    cur.pos.x = damp(cur.pos.x, vA.x, 5, camDt);
    cur.pos.y = damp(cur.pos.y, vA.y, 5, camDt);
    cur.pos.z = damp(cur.pos.z, vA.z, 5, camDt);
    cur.target.x = damp(cur.target.x, vB.x, 5, camDt);
    cur.target.y = damp(cur.target.y, vB.y, 5, camDt);
    cur.target.z = damp(cur.target.z, vB.z, 5, camDt);
    tilt.x = damp(tilt.x, px, 3, dt);
    tilt.y = damp(tilt.y, py, 3, dt);
    camera.position.copy(cur.pos);
    camera.position.x += tilt.x * 0.9;
    camera.position.y -= tilt.y * 0.6;
    camera.lookAt(cur.target);
    // The hero leaves the top of the screen to the headline: shift the frame down.
    cur.frameY = damp(cur.frameY, portrait ? S.frameY + 0.26 * S.frameX / 0.2 + S.portraitY : S.frameY, 4, camDt);
    cur.frameX = damp(cur.frameX, portrait ? 0 : S.frameX, 4, camDt);
    const fw = renderer.domElement.clientWidth;
    const fh = renderer.domElement.clientHeight;
    camera.setViewOffset(fw, fh, -fw * cur.frameX, -fh * cur.frameY, fw, fh);
    // Haze relative to what the camera looks at, so far framings (from above) aren't fogged out.
    const look = cur.pos.distanceTo(cur.target);
    scene.fog.near = Math.max(30, look + 5);
    scene.fog.far = scene.fog.near + 30;

    // Phones face the camera (yaw only), float a little.
    for (const [i, p] of [bob, me, cara, checker].entries()) {
      p.root.getWorldPosition(vC);
      const yaw = Math.atan2(camera.position.x - vC.x, camera.position.z - vC.z);
      p.phone.rotation.y = damp(p.phone.rotation.y, yaw, 4, dt);
      p.phone.position.y = PEDESTAL_H + BODY_H / 2 + 0.06 + Math.sin(time * 0.9 + i * 2) * 0.03;
      p.phone.rotation.z = Math.sin(time * 0.6 + i) * 0.015;
    }
    bob.root.updateMatrixWorld(true);
    me.root.updateMatrixWorld(true);
    cara.root.updateMatrixWorld(true);
    checker.root.updateMatrixWorld(true);

    // Screens: redraw only when their inputs change (yours while sending: every frame).
    const bobState = { a: S.aPush1, b: S.aPush2, c: S.aPush3, d: S.aScroll, e: S.tapA1, f: S.tapA2, g: S.tapA3, h: S.sentBanner };
    const kb = screenKey(bobState);
    if (kb !== bob.key) {
      drawBob(bob.g, images, S);
      bob.tex.needsUpdate = true;
      bob.key = kb;
    }
    const sending = S.bPush > 0.5;
    const meState = { a: S.banner, b: S.tapB1, c: S.bSheet, d: S.bScroll, e: S.tapB2, f: S.bPush, g: S.bDone, h: S.sentBanner, t: sending ? time.toFixed(2) : 0 };
    const km = screenKey(meState);
    if (km !== me.key) {
      drawMe(me.g, images, S, time);
      me.tex.needsUpdate = true;
      me.key = km;
    }
    const kc = screenKey({ a: S.banner, b: S.sentBanner, c: S.caraDim });
    if (kc !== cara.key) {
      drawCara(cara.g, images, S);
      cara.tex.needsUpdate = true;
      cara.key = kc;
    }

    // Vault: the leak, the ghost Z, the door opening along the Z and closing again.
    const open = smooth(clamp01(S.open)) * (1 - smooth(clamp01(S.closeDoor)));
    leaves[0].position.set(0.95 * open, 0.75 * open, 0.25 * open);
    leaves[0].rotation.set(-0.2 * open, 0.35 * open, -0.12 * open);
    leaves[1].position.set(-0.95 * open, -0.5 * open, 0.35 * open);
    leaves[1].rotation.set(0.15 * open, -0.35 * open, 0.1 * open);
    const leak = S.leak * (0.85 + 0.15 * Math.sin(time * 2)) + 5 * open + 1.5 * S.fuse;
    glowMat.color.setScalar(0.55 + 0.35 * leak);
    glowMat.opacity = 1 - 0.85 * open;
    rayMat.opacity = 0.75 * open * (0.85 + 0.15 * Math.sin(time * 2));
    vaultLight.intensity = 14 * open + 1.5 * S.fuse;
    ghostMat.opacity = clamp01(S.ghost) * (0.6 + 0.4 * Math.sin(time * 3)) * (1 - clamp01(S.fuse));

    // Shards.
    shardPose(shards.bob, bob, S.shardA, 0, time, 0);
    shardPose(shards.me, me, S.shardB, 0, time, 2.1);
    shardPose(shards.cara, cara, 0, S.lone, time, 4.2);
    // Fused: gold, seated, pushed in; hidden when the door opens.
    const gold = clamp01(S.fuse);
    for (const sh of [shards.bob, shards.me]) {
      sh.traverse((m) => {
        if (!m.isMesh) return;
        if (gold > 0.5 && m.material !== goldKey) m.material = goldKey;
        if (gold <= 0.5 && m.material === goldKey) m.material = sh === shards.bob ? shards.bobMat : shards.meMat;
      });
      sh.visible = open < 0.35;
    }
    goldKey.emissiveIntensity = 0.3 + 2.2 * Math.sin(clamp01(S.fuse) * Math.PI);

    // Floor pulse from Bob to the others.
    const pu = clamp01(S.pulse);
    pulses.forEach((m, i) => {
      m.visible = pu > 0 && pu < 1;
      if (m.visible) paths[i].getPointAt(pu, m.position);
    });

    // Coins: from the open vault into your phone's slot.
    coins.visible = S.coins > 0 && S.coins < 1;
    if (coins.visible) {
      door.localToWorld(vA.set(0, 0, 0.6));
      screenPoint(me, SPOT.slot[0], SPOT.slot[1], vB);
      for (let i = 0; i < COINS; i++) {
        const k = clamp01(S.coins * 1.7 - (i / COINS) * 0.7);
        const e = smooth(k);
        tmp.position.lerpVectors(vA, vB, e);
        tmp.position.y += Math.sin(e * Math.PI) * (1.3 + (i % 4) * 0.2);
        tmp.position.x += Math.sin(i * 12.9) * 0.25 * Math.sin(e * Math.PI);
        tmp.rotation.set(Math.PI / 2 + time * 4 + i, time * 3 + i, 0);
        tmp.scale.setScalar(k > 0 && k < 1 ? 1 - 0.5 * e : 0.0001);
        tmp.updateMatrix();
        coins.setMatrixAt(i, tmp.matrix);
      }
      coins.instanceMatrix.needsUpdate = true;
    }

    // The stream: identical payments crossing behind the diorama; yours joins it.
    stream.visible = S.stream > 0;
    if (stream.visible) {
      for (let i = 0; i < STREAM; i++) {
        const k = (time * 0.035 + i / STREAM) % 1;
        tmp.position.set(-16 + 32 * k, 2.6 + Math.sin(i * 7.1) * 0.5, -7.5 + Math.sin(i * 3.3) * 0.8);
        tmp.rotation.set(Math.PI / 2, 0, time + i);
        tmp.scale.setScalar(clamp01(S.stream) * (0.9 + 0.2 * Math.sin(i)));
        tmp.updateMatrix();
        stream.setMatrixAt(i, tmp.matrix);
      }
      stream.instanceMatrix.needsUpdate = true;
    }

    // Islands (time-based loops, faded in by their state).
    const i1 = smooth(clamp01(S.i1));
    beam.material.opacity = 0.7 * i1;
    beam.position.y = (0.5 - ((time * 0.35) % 1)) * SCREEN_H;
    badges.forEach((b, i) => {
      const k = smooth(clamp01(S.i1 * 3 - i * 0.6));
      checker.phone.localToWorld(vA.set(1.45, 0.55 - i * 0.55, 0.25));
      b.position.copy(vA);
      b.position.y += Math.sin(time * 1.3 + i) * 0.04;
      b.scale.set(1.6 * k, 0.4 * k, 1);
      b.visible = k > 0.01;
    });
    const i2 = clamp01(S.i2);
    traveller.visible = i2 > 0.02;
    if (traveller.visible) {
      const k = (time * 0.18) % 1;
      const x = blockX(-0.6) + k * (blockX(BLOCKS - 0.4) - blockX(-0.6));
      traveller.position.set(x, 0.32 + 1.0 + Math.sin(k * Math.PI * 6) * 0.05, ISLANDS[1].z);
      traveller.rotation.set(Math.PI / 2, time * 2, 0);
      const inside = x > blockX(1.5);
      traveller.material = inside ? tokenMat : coins.material;
      traveller.scale.setScalar((inside ? 1 : 2.2) * (0.4 + 0.6 * i2));
    }
    const i3 = clamp01(S.i3);
    packet.visible = i3 > 0.02;
    const pk = (time * 0.22) % 1;
    packet.position.set(arches[0].position.x - 0.8 + pk * 3.9, 0.32 + 0.5, arches[0].position.z);
    packet.rotation.set(time, time * 1.3, 0);
    arches.forEach((a, i) => {
      archMats[i].emissiveIntensity = i3 * 1.4 * Math.max(0, 1 - Math.abs(packet.position.x - a.position.x) * 1.6);
    });
    envelope.position.y = 0.32 + 0.5 + Math.sin(time * 1.1) * 0.05;
    const fp = clamp01(S.fpulse);
    islandPulse.visible = fp > 0.001 && fp < 0.999;
    if (islandPulse.visible) islandPath.getPointAt(fp, islandPulse.position);
    bigZMat.opacity = clamp01(S.ctaZ) * (0.75 + 0.25 * Math.sin(time * 2));

    composer.render(dt);
  }

  // ---------------------------------------------------------------- the timeline
  shards.bobMat = shards.bob.children[0].material;
  shards.meMat = shards.me.children[0].material;

  const tl = gsap.timeline({
    defaults: { ease: 'power2.inOut' },
    scrollTrigger: { trigger: story, start: 'top bottom', end: 'bottom bottom', scrub: 1 },
  });
  const at = (vars, pos, duration = 3, ease) => tl.to(S, { ...vars, duration, ...(ease ? { ease } : {}) }, pos);
  // Beats (timeline units; captions switch at BEATS below).
  at({ cam: 1, frameX: 0 }, 0, 8); // hero -> shares
  at({ shardsUp: 1, ghost: 1 }, 3, 5, 'back.out(1.4)');
  at({ cam: 2 }, 9, 6); // the lock
  at({ lone: 1 }, 11, 6, 'none'); // a lone shard bounces off
  at({ cam: 3 }, 17, 6); // Bob
  at({ tapA1: 1 }, 23, 2, 'none');
  at({ aPush1: 1 }, 24.5, 2.5);
  at({ tapA2: 1 }, 28, 2, 'none');
  at({ aPush2: 1 }, 29.5, 2.5);
  at({ cam: 4 }, 32, 5); // from above: the pulse
  at({ pulse: 1 }, 33, 5, 'power1.inOut');
  at({ banner: 1 }, 37, 2, 'back.out(1.4)');
  at({ cam: 5 }, 38, 4); // back to Bob: he approves his own proposal
  at({ aScroll: 1 }, 41, 3);
  at({ tapA3: 1 }, 44, 2, 'none');
  at({ aPush3: 1 }, 45.5, 2.5);
  at({ shardA: 1, cam: 6 }, 46, 6); // his shard seats; watch it at the door
  at({ cam: 7 }, 53, 6); // you
  at({ tapB1: 1 }, 59, 2, 'none');
  at({ banner: 0 }, 60.5, 1.5);
  at({ bSheet: 1 }, 60.5, 3, 'power3.out');
  at({ bScroll: 1 }, 64, 3);
  at({ tapB2: 1 }, 67.5, 2, 'none');
  at({ caraDim: 1 }, 69, 3);
  at({ cam: 8 }, 69, 5); // your shard seats: 2 of 3
  at({ shardB: 1 }, 69.5, 5);
  at({ fuse: 1 }, 75, 3, 'none'); // the key, gold
  at({ insert: 1 }, 77, 2);
  at({ cam: 9, open: 1 }, 79, 6, 'power3.inOut'); // the door splits; dive
  at({ cam: 10 }, 86, 6); // your phone
  at({ bPush: 1 }, 86, 3);
  at({ coins: 1 }, 86, 8, 'none');
  at({ bDone: 1, sentBanner: 1 }, 95, 3);
  at({ cam: 11, stream: 1, closeDoor: 1 }, 98, 8);

  const BEATS = [0, 9, 17, 32, 46, 53, 75, 86, 95, 104];
  let beat = -1;
  tl.eventCallback('onUpdate', () => {
    const t = tl.time();
    let n = 0;
    for (let i = 0; i < BEATS.length; i++) if (t >= BEATS[i]) n = i;
    if (n !== beat) {
      beat = n;
      story.dataset.beat = String(n);
    }
  });
  story.dataset.beat = '0';

  // Features: on to the islands, the cards follow data-island.
  // One timeline from the islands through the call to action, so a single scrub writes
  // the camera (two scrubbed triggers on the same props race after a fast scroll).
  const ft = gsap.timeline({
    defaults: { ease: 'power2.inOut', immediateRender: false },
    scrollTrigger: { trigger: features, start: 'top bottom', endTrigger: cta, end: 'bottom bottom', scrub: 1 },
  });
  ft.fromTo(S, { cam: 11, frameX: 0, portraitY: 0 }, { cam: 12, frameX: 0.2, portraitY: -0.5, duration: 6 }, 0);
  ft.fromTo(S, { fpulse: 0 }, { fpulse: 0.36, duration: 6 }, 0);
  ft.fromTo(S, { i1: 0 }, { i1: 1, duration: 4 }, 4);
  ft.fromTo(S, { cam: 12 }, { cam: 13, duration: 6 }, 12);
  ft.fromTo(S, { fpulse: 0.36 }, { fpulse: 0.7, duration: 6 }, 12);
  ft.fromTo(S, { i2: 0 }, { i2: 1, duration: 4 }, 15);
  ft.fromTo(S, { cam: 13 }, { cam: 14, duration: 6 }, 23);
  ft.fromTo(S, { fpulse: 0.7 }, { fpulse: 1, duration: 6 }, 23);
  ft.fromTo(S, { i3: 0 }, { i3: 1, duration: 4 }, 26);
  ft.to({}, { duration: 4 }, 30);
  const ISL = [0, 6, 17, 28];
  let island = -1;
  ft.eventCallback('onUpdate', () => {
    const t = ft.time();
    let n = 0;
    for (let i = 0; i < ISL.length; i++) if (t >= ISL[i]) n = i;
    if (n !== island) {
      island = n;
      features.dataset.island = String(n);
    }
  });

  // The call to action: up above the vault; the floor Z lights. Islands take 34 units
  // over 520vh, so the CTA's 220vh is 14.4: the move, then a hold.
  ft.fromTo(S, { cam: 14, frameX: 0.2, frameY: 0.02, portraitY: -0.5, ctaZ: 0 }, { cam: 15, frameX: 0, frameY: -0.1, portraitY: -0.06, ctaZ: 1, duration: 10.8 }, 34);
  ft.to({}, { duration: 3.6 }, 44.8);

  // Render unless an opaque section (FAQ, footer) covers the whole screen.
  const covered = () =>
    covers.some((el) => {
      const r = el.getBoundingClientRect();
      return r.top <= 0 && r.bottom >= window.innerHeight;
    });
  const loop = () => {
    const was = running;
    running = !covered();
    if (running && !was) settleUntil = performance.now() + 1000;
    frame();
  };
  gsap.ticker.add(loop);
  document.documentElement.classList.add('world-on');
  ScrollTrigger.refresh();

  // Too slow even at the lowest quality: stop, free the GPU, and fall back to the
  // stills layout (the same one as without WebGL).
  function giveUp() {
    gsap.ticker.remove(loop);
    for (const t of [tl, ft]) {
      t.scrollTrigger?.kill();
      t.kill();
    }
    composer.dispose();
    renderer.dispose();
    showStills();
    canvas.dataset.quality = 'off';
    ScrollTrigger.refresh();
  }
  return { S, hero };
}
