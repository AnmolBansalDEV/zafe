// Pointer and text motion (docs/site.md, research notes on cursors and text).
//
// Cursor: on fine pointers without reduced motion the system pointer is replaced by a
// dot exactly on the pointer and a trailing ring (the system one stays until ours draws,
// so a failed script never leaves the page without a pointer). Links swell the ring; on
// a "Get Zafe" pill the ring wraps it, the pill leans toward the pointer and the dot
// steps aside.
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

// ------------------------------------------------------------------ custom cursor

if (fine && !reduced) {
  const el = (cls, parent) => {
    const e = document.createElement('div');
    e.className = cls;
    parent?.appendChild(e);
    return e;
  };
  const cursor = el('cursor');
  cursor.setAttribute('aria-hidden', 'true');
  const ring = el('cursor-ring', cursor);
  const shape = el('ring-shape', ring);
  const dot = el('cursor-dot', cursor);
  document.body.appendChild(cursor);

  // The dot sits exactly on the pointer; the ring trails it.
  const dotX = gsap.quickSetter(dot, 'x', 'px');
  const dotY = gsap.quickSetter(dot, 'y', 'px');
  const ringX = gsap.quickTo(ring, 'x', { duration: 0.35, ease: 'power3.out' });
  const ringY = gsap.quickTo(ring, 'y', { duration: 0.35, ease: 'power3.out' });
  const CLICKABLE = 'a, button, summary, label, [role="button"]';
  let pill = null;

  function onMove(e) {
    // Swap the system pointer out only once ours is drawing.
    document.documentElement.classList.add('cursor-on');
    cursor.classList.add('on');
    dotX(e.clientX);
    dotY(e.clientY);
    if (pill) {
      // The ring wraps the pill, which leans toward the pointer.
      const r = pill.getBoundingClientRect();
      const cx = r.left + r.width / 2;
      const cy = r.top + r.height / 2;
      const lx = (e.clientX - cx) * 0.22;
      const ly = (e.clientY - cy) * 0.3;
      gsap.to(pill, { x: lx, y: ly, duration: 0.4, ease: 'power3.out', overwrite: 'auto' });
      ringX(cx + lx);
      ringY(cy + ly);
    } else {
      ringX(e.clientX);
      ringY(e.clientY);
    }
    const t = e.target instanceof Element ? e.target : null;
    const link = !pill && t?.closest(CLICKABLE);
    cursor.classList.toggle('link', !!link);
  }
  window.addEventListener('pointermove', onMove, { passive: true });
  document.documentElement.addEventListener('pointerleave', () => cursor.classList.remove('on'));
  window.addEventListener('pointerdown', () => cursor.classList.add('down'));
  window.addEventListener('pointerup', () => cursor.classList.remove('down'));

  for (const p of document.querySelectorAll('.pill')) {
    p.addEventListener('pointerenter', () => {
      pill = p;
      const r = p.getBoundingClientRect();
      shape.style.width = `${r.width + 14}px`;
      shape.style.height = `${r.height + 14}px`;
      cursor.classList.add('full');
    });
    p.addEventListener('pointerleave', () => {
      pill = null;
      shape.style.width = '';
      shape.style.height = '';
      cursor.classList.remove('full');
      gsap.to(p, { x: 0, y: 0, duration: 0.6, ease: 'elastic.out(1, 0.5)' });
    });
  }
}
