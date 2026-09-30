# Releasing the Android app

Releases are testnet APKs on GitHub Releases, built and signed by
`.github/workflows/release.yml`. Mainnet builds wait for the U1 gate (`docs/tracker.md`).

## One-time setup (you)

1. **Deploy the relay** (`infra/relay/README.md`) and note its `https://` URL.
2. **Create the upload key** and keep it safe: every future release must be signed with the
   same key, or phones refuse the update. Store the `.jks` and its passwords in a password
   manager, not in the repo.

   ```bash
   keytool -genkeypair -keystore zafe-upload.jks -alias zafe \
     -keyalg RSA -keysize 4096 -validity 10000 -dname "CN=Zafe"
   base64 -w0 zafe-upload.jks > zafe-upload.jks.b64
   ```
3. **GitHub settings** of the repo (Settings > Secrets and variables > Actions):
   - Variables: `ZAFE_RELAY_URL` = the relay URL. Optional `ZAFE_LINK_HOST` = the host
     of the invite landing site (`infra/site/README.md`; deploy the site first, with
     this key's certificate fingerprint in its `assetlinks.json`). Without it, invites
     are `zafe://` links, which only work where Zafe is installed.
   - Secrets: `ANDROID_KEYSTORE_BASE64` (contents of `zafe-upload.jks.b64`),
     `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` (`zafe`), `ANDROID_KEY_PASSWORD`.
   - Optional secret `GOOGLE_SERVICES_JSON`: the contents of
     `app/android/app/google-services.json` (Firebase project `zafe-18c4d`), for push.
     The relay then needs the FCM service account too.

## Each release

```bash
# bump `version:` in app/pubspec.yaml if you like; the tag decides the version name
git tag v0.1.0
git push origin main v0.1.0
```

The workflow builds `zafe-<version>-testnet-arm64.apk`, publishes it as a **pre-release**
with a `.sha256` file and the signing certificate fingerprint in the notes. The build
number is the workflow run number, so each release installs over the previous one. You
can also start it from the Actions tab with a version (it creates the tag).

## Locally

`app/android/key.properties` (gitignored) with `storeFile`, `storePassword`, `keyAlias`,
`keyPassword` makes `flutter build apk --release` sign with that key; without it release
builds use the debug key (fine for testing, but such an APK can't update a real one).

```bash
cd app && flutter build apk --release --split-per-abi --target-platform android-arm64 \
  --dart-define=ZAFE_NETWORK=test --dart-define=ZAFE_RELAY_URL=https://<relay> \
  --dart-define=ZAFE_LINK_HOST=<site host>   # optional
```
