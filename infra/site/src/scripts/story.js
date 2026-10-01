// Page motion. One loop: Lenis (smooth scroll) is advanced by GSAP's ticker and tells
// ScrollTrigger about every scroll, so the 3D world (world.js) and the page never drift
// apart. The world starts right away (it's behind the hero), only where it can run
// (WebGL, no reduced-motion preference); otherwise the story section shows its still.
import Lenis from 'lenis';
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';
import './ui.js';

gsap.registerPlugin(ScrollTrigger);
ScrollTrigger.config({ ignoreMobileResize: true });

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

if (!reduced) {
  const lenis = new Lenis({ autoRaf: false, anchors: true, lerp: 0.085 });
  lenis.on('scroll', ScrollTrigger.update);
  gsap.ticker.add((time) => lenis.raf(time * 1000));
  gsap.ticker.lagSmoothing(0);
}

function hasWebGL() {
  try {
    const c = document.createElement('canvas');
    return !!(c.getContext('webgl2') || c.getContext('webgl'));
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
if (canvas && story && features && cta && !reduced && hasWebGL()) {
  import('./world.js')
    .then((m) => m.start({ canvas, story, hero, features, cta, covers }))
    .catch((err) => {
      document.documentElement.classList.remove('world-on');
      console.warn('world:', err);
    });
}
