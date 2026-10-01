// Page motion. One loop: Lenis (smooth scroll) is advanced by GSAP's ticker and tells
// ScrollTrigger about every scroll, so the 3D world (world.js) and the page never drift
// apart. The world starts right away (it's behind the hero), only where it can run
// (WebGL on a GPU, no reduced-motion preference); otherwise the story section shows its
// stills. world.js also gives up and shows them if frames stay slow at its lowest quality.
import Lenis from 'lenis';
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';
import './ui.js';
import { loadScreens } from './story-screens.js';

gsap.registerPlugin(ScrollTrigger);
ScrollTrigger.config({ ignoreMobileResize: true });

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

if (!reduced) {
  const lenis = new Lenis({ autoRaf: false, anchors: true, lerp: 0.085 });
  lenis.on('scroll', ScrollTrigger.update);
  gsap.ticker.add((time) => lenis.raf(time * 1000));
  gsap.ticker.lagSmoothing(0);
}

// ?world=always keeps the world on where it would be skipped (headless checks render
// WebGL in software).
const forceWorld = new URLSearchParams(location.search).get('world') === 'always';

// WebGL on a GPU. A software renderer (no GPU, or hardware acceleration off) takes
// seconds per frame here and freezes scrolling, so those visitors get the stills.
function hasGpuWebGL() {
  try {
    const c = document.createElement('canvas');
    const gl = c.getContext('webgl2') || c.getContext('webgl');
    if (!gl) return false;
    if (forceWorld) return true;
    const info = gl.getExtension('WEBGL_debug_renderer_info');
    const name = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : '';
    gl.getExtension('WEBGL_lose_context')?.loseContext();
    return !/swiftshader|llvmpipe|softpipe|software|basic render/i.test(name);
  } catch {
    return false;
  }
}

const canvas = document.querySelector('canvas.world');
const story = document.querySelector('.story');
const hero = document.querySelector('.hero');
const features = document.querySelector('.islands');
const cta = document.querySelector('.cta-world');
const covers = [...document.querySelectorAll('.faq, .foot')];
if (canvas && story && features && cta && !reduced && hasGpuWebGL()) {
  // Fetch the app screens in parallel with the world's module, not after it.
  const screens = loadScreens();
  import('./world.js')
    .then((m) => m.start({ canvas, story, hero, features, cta, covers, screens, forceWorld }))
    .catch((err) => {
      document.documentElement.classList.remove('world-on');
      console.warn('world:', err);
    });
}
