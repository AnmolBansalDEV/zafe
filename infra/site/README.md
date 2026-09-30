# Invite landing site

A static site for invite links, `https://<host>/join#<invite>`:

- **Zafe installed:** the link opens the app on its Join screen (Android App Links, iOS
  Universal Links). The browser never loads the page.
- **Not installed:** `/join` explains what Zafe is, links to the Android download, and
  offers "Open in Zafe" (`zafe://join?invite=…`) and "Copy invite" for after the install.

The invite sits in the URL **fragment**, so the web server and any CDN never receive it.
The page reads it in the browser, moves it to tab-scoped `sessionStorage` and removes it
from the address bar (and so from browser history). The page loads nothing from other
origins, and the CSP (`_headers`, plus a `<meta>` copy) forbids it. Whoever controls this
host's pages can read invites, so only deploy it on a domain you control, and only then
build the app with `ZAFE_LINK_HOST`.

```
public/
  index.html               short home page
  join.html                the invite page (served at /join)
  assets/join.js, site.css, zafe.svg
  _headers                 CSP and content types (Cloudflare Pages, Netlify)
build.sh                   → dist/ with .well-known/{assetlinks.json, apple-app-site-association}
```

## Build

```sh
ZAFE_ANDROID_CERT_SHA256=<release cert SHA-256>[,<debug cert SHA-256>] \
ZAFE_IOS_APP_IDS=<TeamID>.xyz.zafe.zafe \   # optional, once there is an iOS build
ZAFE_DOWNLOAD_URL=https://…                  # optional, default: GitHub releases
infra/site/build.sh
```

### Signing fingerprints

`assetlinks.json` must list the SHA-256 of **every certificate that signs an APK you want
links verified for**. Android checks it when the app is installed, and a mismatch means
the link opens the browser instead of the app.

- Release: the "signing certificate SHA-256" line in each GitHub release's notes, or
  `keytool -list -v -keystore upload.jks -alias <alias>`.
- Local debug builds (optional, for testing): `keytool -list -v -keystore
  ~/.android/debug.keystore -alias androiddebugkey -storepass android`.
- Play App Signing (if the app is ever on Play): add the fingerprint from Play Console →
  App integrity as well.

## Deploy

Any static host with HTTPS on the exact host in `ZAFE_LINK_HOST` works. Requirements:

- `/.well-known/assetlinks.json` and `/.well-known/apple-app-site-association` served
  directly (no redirects) with `Content-Type: application/json`.
- `/join` serves `join.html` (with or without a trailing slash).
- The `_headers` security headers, or their equivalent.

**Cloudflare Pages** (recommended; reads `_headers`, serves `/join` from `join.html`):

```sh
npx wrangler pages deploy infra/site/dist --project-name zafe-site
```

then add the custom domain in the dashboard.

**Caddy** (e.g. on the relay's VPS, `infra/relay/vps`):

```
zafe.example {
	root * /srv/zafe-site
	try_files {path} {path}.html
	file_server
	header /.well-known/* Content-Type application/json
	header {
		Content-Security-Policy "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
		Referrer-Policy no-referrer
		X-Content-Type-Options nosniff
	}
}
```

GitHub Pages works for the page (`build.sh` writes `.nojekyll` so `.well-known` is
served), but it can't set headers, so the AASA file is served with the wrong content type.

## Then

1. Build the app with `--dart-define=ZAFE_LINK_HOST=<host>` (release workflow: repository
   variable `ZAFE_LINK_HOST`). The setup screen's QR and "Share link" switch to the https
   form; `zafe://` links keep working.
2. Check Android verification after installing that build:

   ```sh
   adb shell pm verify-app-links --re-verify xyz.zafe.zafe
   adb shell pm get-app-links xyz.zafe.zafe          # <host>: verified
   adb shell am start -a android.intent.action.VIEW \
     -d "'https://<host>/join#zafe-invite-v1:…'"      # opens Join, not the browser
   ```

   Google's checker: `https://digitalassetlinks.googleapis.com/v1/statements:list?source.web.site=https://<host>&relation=delegate_permission/common.handle_all_urls`.
3. iOS (once it builds): add the Associated Domains capability with
   `applinks:<host>` to the Runner target, and put the team's app ID in
   `ZAFE_IOS_APP_IDS`. `app_links` already delivers Universal Links to Dart.
