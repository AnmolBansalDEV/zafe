// Pointer and text motion (docs/site.md, research notes on cursors and text).
//
// Pointer: the native cursor always stays (OS size and contrast settings must keep
// working). On fine pointers without reduced motion a small companion follows it: the
// upper half of the Seam mark. Over a "Get Zafe" pill the lower half slides in and the
// mark is whole, and the pill leans toward the pointer. Over the 3D scenes it carries a
// "Scroll" label.
//
// Text: headings rise word by word from a mask when they appear; two "unshield"
// scrambles (the proposal caption, the encrypted chain fields). All of it sets
// transforms and text through the DOM APIs, which the CSP allows (no inline styles in
// markup, no <style> elements).
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
// ?pointer=fine forces the companion on (headless browsers report no fine pointer).
const fine = window.matchMedia('(pointer: fine)').matches || new URLSearchParams(location.search).get('pointer') === 'fine';

// ------------------------------------------------------------------ text: word rise

function splitWords(el) {
  if (el.dataset.split) return el.querySelectorAll('.wi');
  el.dataset.split = '1';
  const nodes = [...el.childNodes];
  el.textContent = '';
  for (const node of nodes) {
    if (node.nodeType !== Node.TEXT_NODE) {
      el.appendChild(node);
      continue;
    }
    const parts = node.textContent.split(/(\s+)/);
    for (const part of parts) {
      if (!part) continue;
      if (/^\s+$/.test(part)) {
        el.appendChild(document.createTextNode(' '));
        continue;
      }
      const w = document.createElement('span');
      w.className = 'w';
      const inner = document.createElement('span');
      inner.className = 'wi';
      inner.textContent = part;
      w.appendChild(inner);
      el.appendChild(w);
    }
  }
  return el.querySelectorAll('.wi');
}

function rise(el) {
  if (!el || reduced) return;
  const words = splitWords(el);
  gsap.fromTo(words, { yPercent: 115 }, { yPercent: 0, duration: 0.9, ease: 'power4.out', stagger: 0.06, overwrite: true });
}

// ------------------------------------------------------------------ text: unshield

const GLYPHS = 'abcdef0123456789';
function scramble(el, duration = 0.8) {
  if (!el || reduced) return;
  const final = el.dataset.text || el.textContent;
  el.dataset.text = final;
  const state = { p: 0 };
  gsap.fromTo(
    state,
    { p: 0 },
    {
      p: 1,
      duration,
      ease: 'none',
      overwrite: true,
      onUpdate() {
        const n = Math.floor(final.length * state.p);
        let out = final.slice(0, n);
        for (let i = n; i < final.length; i++) out += final[i] === ' ' ? ' ' : GLYPHS[(Math.random() * GLYPHS.length) | 0];
        el.textContent = out;
      },
      onComplete() {
        el.textContent = final;
      },
    },
  );
}

// Story captions: the proposal caption decrypts into place.
const story = document.querySelector('.story');
if (story) {
  new MutationObserver(() => {
    if (story.dataset.beat === '2') scramble(story.querySelector('.k2'), 0.7);
  }).observe(story, { attributes: true, attributeFilter: ['data-beat'] });
}

// Feature cards: the heading rises when its island comes up; the chain fields unshield.
const islands = document.querySelector('.islands');
if (islands) {
  let lastIsland = '';
  new MutationObserver(() => {
    const n = islands.dataset.island;
    if (n === lastIsland) return;
    lastIsland = n;
    if (n === '0') rise(islands.querySelector('.islands-head h2'));
    const card = islands.querySelector(`.i${n}`);
    if (card) rise(card.querySelector('h3'));
    if (n === '2') islands.querySelectorAll('.chain-mini dd').forEach((dd, i) => gsap.delayedCall(i * 0.08, () => scramble(dd, 0.9)));
  }).observe(islands, { attributes: true, attributeFilter: ['data-island'] });
}

// Other headings rise once as they enter.
for (const el of document.querySelectorAll('.cta-card h2, .faq h2')) {
  ScrollTrigger.create({ trigger: el, start: 'top 85%', once: true, onEnter: () => rise(el) });
}

// ------------------------------------------------------------------ pointer companion

if (fine && !reduced) {
  const SEAM_UPPER = 'M22 26.66Q22 22 26.66 22L71.6 22Q78 22 78 28.4L78 56.95Q78 58.25 76.7 58.25L49.77 58.25Q48.47 58.25 49.39 57.33L73.55 33.17Q74.47 32.25 73.17 32.25L23.3 32.25Q22 32.25 22 30.95Z';
  const SEAM_LOWER = 'M22 43.05Q22 41.75 23.3 41.75L50.23 41.75Q51.53 41.75 50.61 42.67L26.45 66.83Q25.53 67.75 26.83 67.75L76.7 67.75Q78 67.75 78 69.05L78 73.34Q78 78 73.34 78L28.4 78Q22 78 22 71.6Z';
  const cursor = document.createElement('div');
  cursor.className = 'cursor';
  cursor.setAttribute('aria-hidden', 'true');
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '18 18 64 64');
  for (const [cls, d, fill] of [['upper', SEAM_UPPER, '#00736C'], ['lower', SEAM_LOWER, '#51DDD2']]) {
    const path = document.createElementNS(ns, 'path');
    path.setAttribute('class', cls);
    path.setAttribute('d', d);
    path.setAttribute('fill', fill);
    svg.appendChild(path);
  }
  const label = document.createElement('span');
  label.className = 'cursor-label';
  label.textContent = 'Scroll';
  cursor.append(svg, label);
  document.body.appendChild(cursor);

  const toX = gsap.quickTo(cursor, 'x', { duration: 0.45, ease: 'power3.out' });
  const toY = gsap.quickTo(cursor, 'y', { duration: 0.45, ease: 'power3.out' });
  let snapped = null;
  const scenes = [...document.querySelectorAll('.hero, .story, .islands, .cta-world')];

  window.addEventListener('pointermove', (e) => {
    cursor.classList.add('on');
    if (snapped) {
      const r = snapped.getBoundingClientRect();
      // Sit on the pill's left end; the pill leans toward the pointer.
      toX(r.left + 22);
      toY(r.top + r.height / 2);
      gsap.to(snapped, { x: (e.clientX - (r.left + r.width / 2)) * 0.22, y: (e.clientY - (r.top + r.height / 2)) * 0.3, duration: 0.4, ease: 'power3.out', overwrite: 'auto' });
    } else {
      toX(e.clientX + 16);
      toY(e.clientY + 18);
    }
    const t = e.target;
    const overText = t.closest('a, button, input, textarea, .island-card, .cta-card, .faq, .story-captions, h1, h2, p, nav');
    const inScene = !overText && scenes.some((el) => {
      const r = el.getBoundingClientRect();
      return e.clientY >= r.top && e.clientY <= r.bottom;
    });
    cursor.classList.toggle('label', inScene);
  });
  document.addEventListener('pointerleave', () => cursor.classList.remove('on'));

  for (const pill of document.querySelectorAll('.pill')) {
    pill.addEventListener('pointerenter', () => {
      snapped = pill;
      cursor.classList.add('full');
    });
    pill.addEventListener('pointerleave', () => {
      snapped = null;
      cursor.classList.remove('full');
      gsap.to(pill, { x: 0, y: 0, duration: 0.6, ease: 'elastic.out(1, 0.5)' });
    });
  }
}
