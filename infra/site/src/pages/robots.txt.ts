// /robots.txt, built from `site` in astro.config.mjs. /join is deliberately NOT disallowed:
// it carries `noindex`, and crawlers only see that if they may fetch the page (a blocked
// URL can still be indexed from links). The invite itself is in the fragment, which
// crawlers never receive.
import type { APIRoute } from 'astro';

export const GET: APIRoute = ({ site }) => {
  if (!site) throw new Error('set `site` in astro.config.mjs');
  const body = `User-agent: *\nAllow: /\n\nSitemap: ${new URL('/sitemap.xml', site).href}\n`;
  return new Response(body, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
};
