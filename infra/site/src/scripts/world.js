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
  ghost: 0, lone: 0, leak: 0.8,
  shardsUp: 0, // shards rise out of the phones
  pulse: 0, // the floor pulse from Bob to the others
  shardA: 0, shardB: 0, fuse: 0, insert: 0, open: 0,
  coins: 0, stream: 0, closeDoor: 0,
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

export async function start({ canvas, story, hero }) {
  const images = await loadScreens();

  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, powerPreference: 'high-performance', stencil: false });
  const mobile = window.matchMedia('(max-width: 760px)').matches;
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, mobile ? 1.5 : 2));
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
  ];
  // Portrait screens: the wide shots pull back so the side phones stay in frame.
  let camPos = null;
  let camTarget = null;
  function buildCamera(portrait) {
    const pos = CAM.map(([p, t]) => {
      const d = p.distanceTo(t);
      return d > 14 && portrait ? t.clone().add(p.clone().sub(t).multiplyScalar(1.55)) : p;
    });
    camPos = new THREE.CatmullRomCurve3(pos, false, 'centripetal');
    camTarget = new THREE.CatmullRomCurve3(CAM.map((c) => c[1]), false, 'centripetal');
  }
  buildCamera(false);
  const cur = { pos: CAM[0][0].clone(), target: CAM[0][1].clone(), frameY: S.frameY, frameX: S.frameX };

  // ---------------------------------------------------------------- post
  const composer = new EffectComposer(renderer, { multisampling: mobile ? 0 : 4, frameBufferType: THREE.HalfFloatType });
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

  function frame() {
    if (!running) return;
    const now = performance.now();
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    const time = (now - t0) / 1000;

    // Camera along the take, damped, with pointer tilt.
    const u = clamp01(S.cam / (CAM.length - 1));
    camPos.getPoint(u, vA);
    camTarget.getPoint(u, vB);
    cur.pos.x = damp(cur.pos.x, vA.x, 5, dt);
    cur.pos.y = damp(cur.pos.y, vA.y, 5, dt);
    cur.pos.z = damp(cur.pos.z, vA.z, 5, dt);
    cur.target.x = damp(cur.target.x, vB.x, 5, dt);
    cur.target.y = damp(cur.target.y, vB.y, 5, dt);
    cur.target.z = damp(cur.target.z, vB.z, 5, dt);
    tilt.x = damp(tilt.x, px, 3, dt);
    tilt.y = damp(tilt.y, py, 3, dt);
    camera.position.copy(cur.pos);
    camera.position.x += tilt.x * 0.9;
    camera.position.y -= tilt.y * 0.6;
    camera.lookAt(cur.target);
    // The hero leaves the top of the screen to the headline: shift the frame down.
    cur.frameY = damp(cur.frameY, portrait ? S.frameY + 0.26 * S.frameX / 0.2 : S.frameY, 4, dt);
    cur.frameX = damp(cur.frameX, portrait ? 0 : S.frameX, 4, dt);
    const fw = renderer.domElement.clientWidth;
    const fh = renderer.domElement.clientHeight;
    camera.setViewOffset(fw, fh, -fw * cur.frameX, -fh * cur.frameY, fw, fh);

    // Phones face the camera (yaw only), float a little.
    for (const [i, p] of [bob, me, cara].entries()) {
      p.root.getWorldPosition(vC);
      const yaw = Math.atan2(camera.position.x - vC.x, camera.position.z - vC.z);
      p.phone.rotation.y = damp(p.phone.rotation.y, yaw, 4, dt);
      p.phone.position.y = PEDESTAL_H + BODY_H / 2 + 0.06 + Math.sin(time * 0.9 + i * 2) * 0.03;
      p.phone.rotation.z = Math.sin(time * 0.6 + i) * 0.015;
    }
    bob.root.updateMatrixWorld(true);
    me.root.updateMatrixWorld(true);
    cara.root.updateMatrixWorld(true);

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

  // Render only while the world is on screen.
  ScrollTrigger.create({
    trigger: story,
    start: 'top bottom',
    end: 'bottom top',
    onToggle: (self) => {
      running = self.isActive || window.scrollY < window.innerHeight;
      document.documentElement.classList.toggle('world-off', !running);
    },
  });
  const loop = () => frame();
  gsap.ticker.add(loop);
  document.documentElement.classList.add('world-on');
  ScrollTrigger.refresh();
  return { S, hero };
}
