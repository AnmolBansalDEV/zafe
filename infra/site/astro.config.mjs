// Static site: no client framework, no JS except the hand-written /assets/join.js. Nothing
// may be inlined (the CSP allows only same-origin files), see README.md.
import { defineConfig } from 'astro/config';

export default defineConfig({
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
