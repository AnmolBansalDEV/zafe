// The scroll story in 3D (Three.js), scrubbed by GSAP ScrollTrigger. One payment between
// two members: Bob's phone (graphite, app in dark mode) proposes, a notification swirls
// across to yours (silver, light mode), Bob approves (the vault fills 1 of 2), yours
// comes forward and approves (2 of 2), the vault opens, coins pour into your phone's
// sending screen, and it's sent. Scroll back and it rewinds.
//
// Everything the timeline changes lives in `S` (plain numbers, mostly 0..1). The frame
// loop reads S to pose the phones, redraw their screens (story-screens.js), draw the
// seam between the dark and light halves, and move the swirl, vault, sparks and coins.

import * as THREE from 'three';
import { RoundedBoxGeometry } from 'three/addons/geometries/RoundedBoxGeometry.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';
import { W, H, SPOT, loadScreens, makeCanvas, drawBob, drawMe, drawMark, roundRect } from './story-screens.js';

gsap.registerPlugin(ScrollTrigger);

const S = {
  aActive: 0, bActive: 0, seam: 0.5, finale: 0,
  aPush1: 0, aPush2: 0, aPush3: 0, aScroll: 0,
  tapA1: 0, tapA2: 0, tapA3: 0, tapB1: 0, tapB2: 0,
  flight: 0, swirlFade: 0, banner: 0,
  bSheet: 0, bScroll: 0, bPush: 0, bDone: 0,
  vaultIn: 0, sparkA: 0, sparkB: 0, dot1: 0, dot2: 0, unlock: 0, coins: 0,
};

// Phone model (world units): screen 0.92 wide at the renders' aspect, 0.04 bezel.
const SCREEN_W = 0.92;
const SCREEN_H = (SCREEN_W * H) / W;
const BODY_W = SCREEN_W + 0.08;
const BODY_H = SCREEN_H + 0.08;
const DEPTH = 0.09;

const lerp = (a, b, t) => a + (b - a) * t;
const clamp01 = (v) => Math.min(1, Math.max(0, v));
const ease = (t) => t * t * (3 - 2 * t);

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

function makePhone(frameColor, renderer) {
  const group = new THREE.Group();
  const body = new THREE.Mesh(
    new RoundedBoxGeometry(BODY_W, BODY_H, DEPTH, 8, 0.13),
    new THREE.MeshPhysicalMaterial({ color: frameColor, metalness: 0.75, roughness: 0.28, clearcoat: 0.8, clearcoatRoughness: 0.2 }),
  );
  group.add(body);
  const canvas = makeCanvas();
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = renderer.capabilities.getMaxAnisotropy();
  const screen = new THREE.Mesh(
    roundedPlane(SCREEN_W, SCREEN_H, 0.1),
    new THREE.MeshBasicMaterial({ map: texture, toneMapped: false }),
  );
  screen.position.z = DEPTH / 2 + 0.002;
  group.add(screen);
  return { group, canvas, g: canvas.getContext('2d'), texture };
}

// A soft round shadow under a floating object.
function makeShadow() {
  const c = document.createElement('canvas');
  c.width = c.height = 128;
  const g = c.getContext('2d');
  const grad = g.createRadialGradient(64, 64, 4, 64, 64, 62);
  grad.addColorStop(0, 'rgba(0,0,0,0.55)');
  grad.addColorStop(1, 'rgba(0,0,0,0)');
  g.fillStyle = grad;
  g.fillRect(0, 0, 128, 128);
  const mesh = new THREE.Mesh(
    new THREE.PlaneGeometry(1.4, 0.5),
    new THREE.MeshBasicMaterial({ map: new THREE.CanvasTexture(c), transparent: true, depthWrite: false }),
  );
  return mesh;
}

// The vault card: a lock, three signer dots with a divider after two, "2 of 3".
function drawVault(g, s) {
  const w = g.canvas.width;
  const h = g.canvas.height;
  g.clearRect(0, 0, w, h);
  roundRect(g, 0, 0, w, h, 60);
  g.fillStyle = '#FFFFFF';
  g.fill();
  // Lock: the shackle lifts and swings open with `unlock`.
  const cx = w / 2;
  const u = s.unlock;
  g.save();
  g.lineWidth = 16;
  g.lineCap = 'round';
  g.strokeStyle = u > 0.5 ? '#835A00' : '#004A46';
  g.translate(cx + 34, 118 - 26 * u);
  g.rotate(-0.5 * u);
  g.beginPath();
  g.moveTo(0, 0);
  g.lineTo(0, -34);
  g.arc(-34, -34, 34, 0, Math.PI, true);
  g.lineTo(-68, 0);
  g.stroke();
  g.restore();
  roundRect(g, cx - 58, 110, 116, 88, 18);
  g.fillStyle = u > 0.5 ? '#C98F14' : '#004A46';
  g.fill();
  g.fillStyle = '#FFFFFF';
  g.beginPath();
  g.arc(cx, 148, 11, 0, Math.PI * 2);
  g.fill();
  // Dots.
  const dots = [s.dot1, s.dot2, 0];
  const xs = [cx - 70, cx - 22, cx + 58];
  dots.forEach((d, i) => {
    const pop = d > 0 && d < 1 ? 1 + 0.5 * Math.sin(d * Math.PI) : 1;
    g.beginPath();
    g.arc(xs[i], 262, 17 * pop, 0, Math.PI * 2);
    if (d > 0.5) {
      g.fillStyle = '#00736C';
      g.fill();
    } else {
      g.lineWidth = 6;
      g.strokeStyle = '#090E0E';
      g.stroke();
    }
  });
  g.fillStyle = '#C3CCCB';
  g.fillRect(cx + 16, 238, 5, 48);
  g.fillStyle = '#55615F';
  g.font = '500 40px "DM Sans", sans-serif';
  g.textAlign = 'center';
  g.fillText('2 of 3', cx, 348);
}

export async function start(section) {
  const pin = section.querySelector('.story-pin');
  const bg = section.querySelector('.story-bg');
  const gl = section.querySelector('.story-gl');
  const images = await loadScreens();
  // Tall (scroll room) and pinned before anything measures the section.
  section.classList.add('story-live');

  const renderer = new THREE.WebGLRenderer({ canvas: gl, antialias: true, alpha: true, powerPreference: 'high-performance' });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;

  const scene = new THREE.Scene();
  const pmrem = new THREE.PMREMGenerator(renderer);
  scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  const key = new THREE.DirectionalLight(0xffffff, 1.2);
  key.position.set(3, 5, 6);
  scene.add(key, new THREE.AmbientLight(0xffffff, 0.35));

  const camera = new THREE.PerspectiveCamera(26, 1, 0.1, 100);
  camera.position.set(0, 0, 7.4);

  const bob = makePhone(0x2b3131, renderer);
  const me = makePhone(0xd5dcdb, renderer);
  const bobShadow = makeShadow();
  const meShadow = makeShadow();
  scene.add(bob.group, me.group, bobShadow, meShadow);

  // The vault on the seam.
  const vaultCanvas = document.createElement('canvas');
  vaultCanvas.width = 360;
  vaultCanvas.height = 400;
  const vaultG = vaultCanvas.getContext('2d');
  const vaultTex = new THREE.CanvasTexture(vaultCanvas);
  vaultTex.colorSpace = THREE.SRGBColorSpace;
  const vault = new THREE.Group();
  const vaultBody = new THREE.Mesh(
    new RoundedBoxGeometry(0.62, 0.69, 0.1, 6, 0.08),
    new THREE.MeshPhysicalMaterial({ color: 0xf6f8f8, metalness: 0.1, roughness: 0.4, clearcoat: 1 }),
  );
  const vaultFace = new THREE.Mesh(
    new THREE.PlaneGeometry(0.6, 0.667),
    new THREE.MeshBasicMaterial({ map: vaultTex, transparent: true, toneMapped: false }),
  );
  vaultFace.position.z = 0.051;
  const glow = new THREE.Mesh(
    new THREE.TorusGeometry(0.5, 0.02, 12, 64),
    new THREE.MeshBasicMaterial({ color: 0xf3ba3c, transparent: true, opacity: 0 }),
  );
  vault.add(vaultBody, vaultFace, glow);
  scene.add(vault);

  // The notification token (the app icon) and its swirl.
  const iconCanvas = document.createElement('canvas');
  iconCanvas.width = iconCanvas.height = 256;
  drawMark(iconCanvas.getContext('2d'), 0, 0, 256);
  const iconTex = new THREE.CanvasTexture(iconCanvas);
  iconTex.colorSpace = THREE.SRGBColorSpace;
  const token = new THREE.Group();
  token.add(
    new THREE.Mesh(
      new RoundedBoxGeometry(0.34, 0.34, 0.06, 4, 0.07),
      new THREE.MeshPhysicalMaterial({ color: 0x004a46, metalness: 0.3, roughness: 0.35, clearcoat: 1 }),
    ),
  );
  const iconFace = new THREE.Mesh(new THREE.PlaneGeometry(0.34, 0.34), new THREE.MeshBasicMaterial({ map: iconTex, transparent: true, toneMapped: false }));
  iconFace.position.z = 0.031;
  token.add(iconFace);
  scene.add(token);
  const swirlMat = new THREE.MeshBasicMaterial({ color: 0x51ddd2, transparent: true, toneMapped: false });
  const swirlGlowMat = new THREE.MeshBasicMaterial({ color: 0x51ddd2, transparent: true, opacity: 0.18, toneMapped: false, depthWrite: false });
  let swirl = null;
  let swirlGlow = null;
  let curve = null;

  // Sparks from each approval to its dot, and the coins.
  const sparkGeo = new THREE.SphereGeometry(0.05, 16, 16);
  const sparkA = new THREE.Mesh(sparkGeo, new THREE.MeshBasicMaterial({ color: 0x51ddd2, toneMapped: false }));
  const sparkB = new THREE.Mesh(sparkGeo, new THREE.MeshBasicMaterial({ color: 0x00a79c, toneMapped: false }));
  scene.add(sparkA, sparkB);
  const COINS = 16;
  const coins = new THREE.InstancedMesh(
    new THREE.CylinderGeometry(0.085, 0.085, 0.02, 40),
    new THREE.MeshPhysicalMaterial({ color: 0xf3ba3c, metalness: 0.95, roughness: 0.22, clearcoat: 1 }),
    COINS,
  );
  scene.add(coins);
  const tmp = new THREE.Object3D();

  // Layout: phones side by side (landscape) or stacked (portrait); world size of the view.
  let L = null;
  function layout() {
    const w = pin.clientWidth;
    const h = pin.clientHeight;
    renderer.setSize(w, h, false);
    bg.width = Math.round(w * Math.min(window.devicePixelRatio, 2));
    bg.height = Math.round(h * Math.min(window.devicePixelRatio, 2));
    camera.aspect = w / h;
    const portrait = w / h < 0.8;
    camera.position.z = portrait ? 9.6 : 7.4;
    camera.updateProjectionMatrix();
    const viewH = 2 * camera.position.z * Math.tan(THREE.MathUtils.degToRad(camera.fov / 2));
    const viewW = viewH * camera.aspect;
    L = { w, h, portrait, viewW, viewH, scale: portrait ? 0.74 : Math.min(1.12, viewW / 5.6) };
    buildSwirl();
  }

  // Poses. `side` is -1 for Bob, +1 for you.
  function pose(side, active, t, phase) {
    const idle = {
      x: L.portrait ? 0.35 * side : (side * L.viewW) / 4.2,
      y: L.portrait ? -side * L.viewH * 0.24 : 0.08,
      z: -0.7,
      rx: L.portrait ? -0.45 : -0.28,
      ry: L.portrait ? 0.42 * -side : 0.62 * -side,
      rz: 0.1 * side,
      s: 0.86,
    };
    const act = {
      x: L.portrait ? 0 : (side * L.viewW) / 5.4,
      y: L.portrait ? -side * L.viewH * 0.23 : 0,
      z: 0.35,
      rx: -0.04,
      ry: 0.1 * -side,
      rz: 0,
      s: L.portrait ? 1.0 : 1.08,
    };
    const k = ease(active);
    const p = {};
    for (const key of Object.keys(idle)) p[key] = lerp(idle[key], act[key], k);
    const float = 1 - 0.65 * k;
    p.y += Math.sin(t * 0.9 + phase) * 0.05 * float;
    p.rz += Math.sin(t * 0.6 + phase) * 0.025 * float;
    p.ry += Math.sin(t * 0.45 + phase) * 0.05 * float;
    p.s *= L.scale;
    return p;
  }

  function apply(phone, shadow, p) {
    phone.group.position.set(p.x, p.y, p.z);
    phone.group.rotation.set(p.rx, p.ry, p.rz);
    phone.group.scale.setScalar(p.s);
    shadow.position.set(p.x, p.y - (BODY_H / 2) * p.s - 0.28, p.z - 0.4);
    shadow.scale.setScalar(p.s);
    shadow.material.opacity = L.portrait ? 0.25 : 0.4;
  }

  // A point on a phone's screen (fractions of the viewport) in world space.
  const screenPoint = (phone, fx, fy, out = new THREE.Vector3()) =>
    phone.group.localToWorld(out.set((fx - 0.5) * SCREEN_W, (0.5 - fy) * SCREEN_H, DEPTH / 2 + 0.02));

  // The swirl, built from the poses at flight time: up from Bob's phone, one loop above
  // the seam, down onto your banner.
  function buildSwirl() {
    if (swirl) {
      scene.remove(swirl, swirlGlow);
      swirl.geometry.dispose();
      swirlGlow.geometry.dispose();
    }
    const a = pose(-1, 1, 0, 0);
    const b = pose(1, 0.35, 0, 0);
    apply(bob, bobShadow, a);
    apply(me, meShadow, b);
    bob.group.updateMatrixWorld(true);
    me.group.updateMatrixWorld(true);
    const p0 = screenPoint(bob, 0.85, 0.06);
    const p3 = screenPoint(me, 0.2, 0.06);
    const mid = p0.clone().lerp(p3, 0.5);
    // One loop just above the phones: in from Bob's side to the loop's bottom, round it
    // (right, top, left), back through the bottom, and down onto your banner.
    const r = L.portrait ? 0.26 : 0.3;
    const c = L.portrait
      ? mid.clone().add(new THREE.Vector3(0.75, 0, 0.35))
      : mid.clone().add(new THREE.Vector3(0, 0.12 + r, 0.35));
    const V = (x, y, z = 0) => new THREE.Vector3(x, y, z);
    const pts = L.portrait
      ? [p0, p0.clone().add(V(0.35, -0.2, 0.15)), c.clone().add(V(-r, 0, 0)), c.clone().add(V(0, r, 0.05)), c.clone().add(V(r, 0, 0.1)), c.clone().add(V(0, -r, 0.05)), c.clone().add(V(-r * 0.8, -r * 0.2)), p3.clone().add(V(0.35, 0.2, 0.15)), p3]
      : [p0, p0.clone().add(V(0.3, 0.12, 0.15)), c.clone().add(V(-0.05, -r, 0)), c.clone().add(V(r, 0, 0.05)), c.clone().add(V(0, r, 0.1)), c.clone().add(V(-r, 0, 0.05)), c.clone().add(V(0.05, -r)), p3.clone().add(V(-0.3, 0.12, 0.15)), p3];
    curve = new THREE.CatmullRomCurve3(pts, false, 'centripetal');
    swirl = new THREE.Mesh(new THREE.TubeGeometry(curve, 400, 0.011, 10, false), swirlMat);
    swirlGlow = new THREE.Mesh(new THREE.TubeGeometry(curve, 400, 0.035, 10, false), swirlGlowMat);
    scene.add(swirl, swirlGlow);
  }

  // The dark and light halves, split by a slowly moving curve.
  function drawBackground(t) {
    const g = bg.getContext('2d');
    const w = bg.width;
    const h = bg.height;
    g.fillStyle = '#F1F5F5';
    g.fillRect(0, 0, w, h);
    g.fillStyle = '#080B0B';
    g.beginPath();
    const steps = 48;
    if (!L.portrait) {
      g.moveTo(0, 0);
      for (let i = 0; i <= steps; i++) {
        const y = (i / steps) * h;
        const x = w * S.seam + Math.sin((i / steps) * Math.PI * 1.6 + t * 0.35) * w * 0.028 + (i / steps - 0.5) * w * 0.07;
        g.lineTo(x, y);
      }
      g.lineTo(0, h);
    } else {
      g.moveTo(0, 0);
      g.lineTo(w, 0);
      for (let i = 0; i <= steps; i++) {
        const x = w - (i / steps) * w;
        const y = h * S.seam + Math.sin((i / steps) * Math.PI * 1.6 + t * 0.35) * h * 0.02 + (0.5 - i / steps) * h * 0.04;
        g.lineTo(x, y);
      }
    }
    g.closePath();
    g.fill();
  }

  const vA = new THREE.Vector3();
  const vB = new THREE.Vector3();
  const vDot = new THREE.Vector3();
  let pointerX = 0;
  let pointerY = 0;
  window.addEventListener('pointermove', (e) => {
    pointerX = e.clientX / window.innerWidth - 0.5;
    pointerY = e.clientY / window.innerHeight - 0.5;
  });

  let visible = true;
  const start = performance.now();
  function frame() {
    if (!visible) return;
    const t = (performance.now() - start) / 1000;

    // Camera: a slow drift, a little parallax, and a push-in for the finale.
    camera.position.x += (pointerX * 0.35 - camera.position.x) * 0.04;
    camera.position.y += (-pointerY * 0.25 - camera.position.y) * 0.04;
    camera.lookAt(0, 0, 0);

    const pa = pose(-1, S.aActive, t, 0);
    const pb = pose(1, S.bActive, t, 1.7);
    if (S.finale > 0) {
      pb.x = lerp(pb.x, L.portrait ? 0 : L.viewW * 0.1, ease(S.finale));
      pb.s *= 1 + 0.1 * ease(S.finale);
      pa.x = lerp(pa.x, L.portrait ? pa.x : -L.viewW * 0.3, ease(S.finale));
      pa.z -= 0.6 * S.finale;
    }
    apply(bob, bobShadow, pa);
    apply(me, meShadow, pb);
    bob.group.updateMatrixWorld(true);
    me.group.updateMatrixWorld(true);

    drawBob(bob.g, images, S);
    bob.texture.needsUpdate = true;
    drawMe(me.g, images, S, t);
    me.texture.needsUpdate = true;
    drawBackground(t);

    // Vault on the seam.
    const vx = L.portrait ? -L.viewW * 0.3 : (pa.x + pb.x) / 2;
    const vy = L.portrait ? (pa.y + pb.y) / 2 : -0.35;
    vault.position.set(vx, vy + Math.sin(t * 1.1) * 0.03, 1.1);
    vault.rotation.set(-0.12 + Math.sin(t * 0.7) * 0.05, Math.sin(t * 0.5) * 0.18, 0);
    const vs = ease(S.vaultIn) * (1 + 0.08 * Math.sin(S.unlock * Math.PI)) * (L.portrait ? 0.8 : 1);
    vault.scale.setScalar(Math.max(0.0001, vs));
    drawVault(vaultG, S);
    vaultTex.needsUpdate = true;
    glow.material.opacity = S.unlock > 0 && S.unlock < 1 ? 0.7 * (1 - S.unlock) : 0;
    glow.scale.setScalar(1 + 1.4 * S.unlock);

    // Swirl and token.
    if (swirl) {
      const n = swirl.geometry.index.count;
      const shown = Math.floor((n * ease(S.flight)) / 6) * 6;
      swirl.geometry.setDrawRange(0, shown);
      swirlGlow.geometry.setDrawRange(0, shown);
      swirlMat.opacity = 1 - S.swirlFade;
      swirlGlowMat.opacity = 0.18 * (1 - S.swirlFade);
    }
    token.visible = S.flight > 0.001 && S.flight < 0.999;
    if (token.visible && curve) {
      const u = ease(S.flight);
      token.position.copy(curve.getPointAt(u));
      const tan = curve.getTangentAt(u);
      token.rotation.set(tan.y * 0.5, -tan.x * 0.5, Math.sin(t * 3) * 0.12);
      token.scale.setScalar(L.scale * (0.8 + 0.4 * Math.sin(u * Math.PI)));
    }

    // Sparks: from each approve button to its dot.
    const dotPos = (i) => vault.localToWorld(vDot.set(i === 0 ? -0.12 : -0.04, -0.12, 0.08));
    vault.updateMatrixWorld(true);
    sparkA.visible = S.sparkA > 0 && S.sparkA < 1;
    if (sparkA.visible) {
      screenPoint(bob, 0.5, 0.834, vA);
      dotPos(0);
      sparkA.position.lerpVectors(vA, vDot, ease(S.sparkA));
      sparkA.position.y += Math.sin(S.sparkA * Math.PI) * 0.5;
    }
    sparkB.visible = S.sparkB > 0 && S.sparkB < 1;
    if (sparkB.visible) {
      screenPoint(me, 0.5, 0.834, vB);
      dotPos(1);
      sparkB.position.lerpVectors(vB, vDot, ease(S.sparkB));
      sparkB.position.y += Math.sin(S.sparkB * Math.PI) * 0.5;
    }

    // Coins: from the vault into your sending screen's slot.
    coins.visible = S.coins > 0 && S.coins < 1;
    if (coins.visible) {
      const from = vault.position;
      const to = screenPoint(me, SPOT.slot[0], SPOT.slot[1], vB);
      for (let i = 0; i < COINS; i++) {
        const k = clamp01(S.coins * 1.8 - (i / COINS) * 0.8);
        const e = ease(k);
        tmp.position.lerpVectors(from, to, e);
        tmp.position.y += Math.sin(e * Math.PI) * (0.45 + (i % 4) * 0.08);
        tmp.position.x += Math.sin(i * 12.9) * 0.18 * Math.sin(e * Math.PI);
        tmp.rotation.set(Math.PI / 2 + t * 3 + i, t * 2 + i, 0);
        tmp.scale.setScalar(k > 0 && k < 1 ? L.scale * (1 - 0.6 * e) : 0.0001);
        tmp.updateMatrix();
        coins.setMatrixAt(i, tmp.matrix);
      }
      coins.instanceMatrix.needsUpdate = true;
    }

    renderer.render(scene, camera);
  }

  layout();
  window.addEventListener('resize', () => {
    layout();
    ScrollTrigger.refresh();
  });

  // The timeline. Units are arbitrary; ScrollTrigger maps the pinned scroll onto it.
  const tl = gsap.timeline({
    defaults: { ease: 'power2.inOut', duration: 3 },
    scrollTrigger: { trigger: section, start: 'top top', end: 'bottom bottom', scrub: 1.1 },
  });
  const to = (vars, at, duration, ease) => tl.to(S, { ...vars, duration: duration ?? 3, ease: ease ?? 'power2.inOut' }, at);
  to({ aActive: 1, seam: 0.56 }, 0, 6);
  to({ tapA1: 1 }, 7, 2.5, 'none');
  to({ aPush1: 1 }, 9, 3);
  to({ tapA2: 1 }, 14, 2.5, 'none');
  to({ aPush2: 1 }, 16, 3);
  to({ vaultIn: 1 }, 19, 3, 'back.out(1.6)');
  to({ flight: 1 }, 21, 15, 'power1.inOut');
  to({ bActive: 0.35 }, 30, 6);
  to({ banner: 1 }, 35, 3, 'back.out(1.4)');
  to({ swirlFade: 1 }, 38, 5);
  to({ aScroll: 1 }, 40, 5);
  to({ tapA3: 1 }, 45, 2.5, 'none');
  to({ sparkA: 1 }, 46.5, 2.5);
  to({ dot1: 1 }, 49, 1.2, 'none');
  to({ aPush3: 1 }, 48, 3);
  to({ aActive: 0, bActive: 1, seam: 0.42 }, 52, 6);
  to({ tapB1: 1 }, 58, 2.5, 'none');
  to({ banner: 0 }, 60, 1.5);
  to({ bSheet: 1 }, 60, 4, 'power3.out');
  to({ bScroll: 1 }, 65, 4);
  to({ tapB2: 1 }, 70, 2.5, 'none');
  to({ sparkB: 1 }, 71.5, 2.5);
  to({ dot2: 1 }, 74, 1.2, 'none');
  to({ unlock: 1 }, 75, 3, 'back.out(1.4)');
  to({ coins: 1 }, 76, 8, 'none');
  to({ bPush: 1 }, 79, 3);
  to({ bDone: 1 }, 90, 3);
  to({ finale: 1, seam: 0.34 }, 92, 8);

  const beats = [20, 38, 51, 75, 90];
  let beat = 0;
  section.dataset.beat = '0';
  tl.eventCallback('onUpdate', () => {
    const at = tl.time();
    const next = beats.findIndex((b) => at < b);
    const n = next === -1 ? beats.length : next;
    if (n !== beat) {
      beat = n;
      section.dataset.beat = String(n);
    }
  });

  new IntersectionObserver((entries) => {
    visible = entries.some((e) => e.isIntersecting);
  }).observe(section);
  gsap.ticker.add(frame);
  ScrollTrigger.refresh();
}
