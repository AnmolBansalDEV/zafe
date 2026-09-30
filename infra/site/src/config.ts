// Build-time links (build.sh validates them and passes them in the environment).
const repo = 'https://github.com/AnmolBansalDEV/zafe';

export const downloadUrl = process.env.ZAFE_DOWNLOAD_URL || `${repo}/releases`;
export const sourceUrl = process.env.ZAFE_SOURCE_URL || repo;

// Same policy as public/_headers, for hosts that can't set headers. frame-ancestors only
// works as a header.
export const csp =
  "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; " +
  "font-src 'self'; base-uri 'none'; form-action 'none'";
