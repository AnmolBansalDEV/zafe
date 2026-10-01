// Static site: no client framework, no JS except the hand-written /assets/join.js. Nothing
// may be inlined (the CSP allows only same-origin files), see README.md.
import { defineConfig } from 'astro/config';

export default defineConfig({
  // The site's public origin: canonical URLs, og:url/og:image, sitemap.xml and robots.txt
  // all derive from it (Astro.site). Change it when the site moves to its own domain.
  site: 'https://zafe-pink.vercel.app',
  output: 'static',
  build: {
    format: 'file', // /join -> join.html, as the hosts' configs in README.md expect
    inlineStylesheets: 'never',
    assets: 'assets/_astro',
  },
  compressHTML: true,
  devToolbar: { enabled: false },
  prefetch: false,
  vite: {
    build: { assetsInlineLimit: 0 },
  },
});
