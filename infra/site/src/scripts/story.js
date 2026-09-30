// Page motion: smooth scrolling (Lenis) and the 3D scroll story, loaded only when its
// section comes near and only where it can run (WebGL, no reduced-motion preference).
// Without it the story section is a still of the ending (index.astro, .story-static).
import Lenis from 'lenis';

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

if (!reduced) {
  new Lenis({ autoRaf: true, anchors: true, lerp: 0.09 });
}

function hasWebGL() {
  try {
    const c = document.createElement('canvas');
    return !!(c.getContext('webgl2') || c.getContext('webgl'));
  } catch {
    return false;
  }
}

const section = document.querySelector('.story');
if (section && !reduced && hasWebGL()) {
  const near = new IntersectionObserver(
    (entries) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      near.disconnect();
      import('./story-scene.js')
        .then((m) => m.start(section))
        .catch((err) => {
          section.classList.remove('story-live');
          console.warn('story:', err);
        });
    },
    { rootMargin: '150% 0px' },
  );
  near.observe(section);
}
