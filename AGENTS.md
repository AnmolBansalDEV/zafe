# AGENTS.md

Source of truth for agents working on Zafe. Read this before changing code; update it when
you learn something a later run would otherwise have to rediscover (a gotcha, an invariant,
an upstream status change). `CLAUDE.md` only contains `@AGENTS.md`.

Zafe is a Safe-style **shielded multisig for Zcash** (Ironwood pool, NU6.3) using
re-randomized FROST. Product spec: `spec.md`. Original review: `spec-review.md`. Open
questions to the Zcash Foundation / others: `upstream-asks.md`.
**Tracker: `docs/tracker.md`** lists what's left, deferred items and ideas. Read it at the
start of a task; when you defer something, discover a gap or finish an item, update it in
the same change.

## Commands

```bash
cargo fmt --all && cargo clippy --workspace --all-targets     # must be clean
cargo test --workspace                                        # ~58 tests, 2 ignored

# ZIP 2005 vectors: regenerate, then check independently in Python
ZAFE_REGEN_VECTORS=1 cargo test -p zafe-core --test zip2005_vectors
python3 scripts/check_zip2005_vectors.py

# Live Ironwood regtest (Docker): in-process end-to-end spend
cargo test -p zafe-core --test regtest_e2e -- --ignored --nocapture
# M0 acceptance: three separate `zafe` CLI processes through the relay on regtest
scripts/m0-e2e.sh

# Cross-check against frost-tools zcash-sign (build it from ZcashFoundation/frost-tools)
ZCASH_SIGN_BIN=/path/to/zcash-sign cargo test -p zafe-core --test zcash_sign_crosscheck -- --ignored

# Phone benchmark (proving + round-2 signing) over adb
scripts/android-bench.sh

# The app's payment flow through the Flutter bridge API (3 members, regtest, Docker)
cargo test -p rust_lib_zafe --test bridge_e2e -- --ignored --nocapture
```

Commits end with the attribution lines the harness gives (Co-Authored-By, and
Claude-Session when present). Never commit unless the user asked for the work.

## Layout

```
crates/zafe-core    keys (ZIP 2005), keygen (DKG + sk), signing, tx (PCZT), verify (§9.3),
                    session (approve/sign/leader), vault (descriptor, events, replay),
                    wallet (zcash_client_backend/sqlite), relay_client, node (orchestration)
crates/zafe-proto   identities, signed/HPKE envelopes, vault log, relay API types
                    (no Zcash deps, so the relay can use it)
crates/zafe-relay   blind axum relay on SQLite (ZAFE_RELAY_DB), push hook, pruning
crates/zafe-cli     `zafe` binary: headless member for tests (dev-only plain-file state)
infra/regtest       Zakura + lightwalletd regtest with NU6.3 active (up.sh / down.sh)
scripts/            m0-e2e.sh, android-bench.sh, check_zip2005_vectors.py
```

## Protocol invariants (breaking any of these is a security or funds bug)

- **Vault keys (ZIP 2005, `use_qsk = true`)**: `ak` from the FROST DKG; everything else from
  the shared vault secret `sk`: `nk = ToBase(PRF^expand_sk[0x07])`,
  `qsk = trunc32(PRF^expand_sk[0x0C])`, `qk = BLAKE3.derive_key("Zcash ZIP 2005 qk-derivation v1", qsk)`,
  `rivk_ext = ToScalar(PRF^expand_qk[0x0D] || ak || nk)`. FVK via `FullViewingKey::from_bytes`.
  Never use frost-tools' `from_sk_ak_incompatible_with_quantum_recoverability...` (not recoverable).
- **DKG**: safety number confirmed out of band before starting; round-1 echo hashes must all
  match; round-2 packages and `sk` contributions are HPKE-sealed; every member signs the
  descriptor. `reddsa` 0.5.2's `post_dkg` normalizes `ak` to even Y (orchard rejects odd).
- **Randomizer**: the FROST randomizer for each spend is the PCZT's own `alpha` (fixed before
  round 1 because `rk` feeds the sighash). Secure per the Re-Randomized FROST paper; matches
  frost-tools. Uses the deprecated `frost_rerandomized::sign` in one wrapper (frost#1094).
- **What to sign**: every Ironwood action whose `spend_auth_sig` is `None` — never filter by
  value (zero-value vault spends exist). True dummy spends are already signed by the IO
  Finalizer. Reject any unsigned Orchard-pool spend (vaults never hold Orchard funds, ZIP 326).
- **Member verification** (`verify::verify_pczt`) before approving and again before signing:
  v6 + branch id + expiry; Ironwood only; **all spends checked before any output** (stable
  errors); each spend is a vault note (nullifier/rk vs vault FVK) or a zero-value pre-signed
  dummy; each payment output recovered with the vault **external OVK** and matched exactly
  (recipient, amount, memo); change must belong to the vault **and trial-decrypt** with its IVK;
  fee must **equal** ZIP 317 (5000 × max(2, actions)); sighash computed locally.
- **Proposals are built with `OvkPolicy::Sender`** and Ironwood change, or verification fails.
- **Nonce storage**: `nonce_store::FileNonceStore` (atomic write+rename; `put` returns an
  error so a failed write never publishes an approval). The directory must be excluded from
  backups/device transfer: the Android app disables both (`allowBackup=false`,
  `res/xml/data_extraction_rules.xml`); iOS needs `isExcludedFromBackup` (not done yet).
- **Nonces**: one pair per spend, keyed by (proposal, pczt hash); check the request's
  packages against stored commitments **before** consuming; delete before sending a share;
  never reuse. Re-approval produces fresh commitments; leaders track used commitment sets.
- **Proving runs in parallel with share collection** (`node::finalize`, Vizor's Keystone
  pattern): the Halo 2 proof doesn't depend on spend-auth signatures, so the unsigned PCZT
  is proved on a blocking thread, then signatures are applied to the proved PCZT (verified
  on regtest). Proving/verifying keys are process-wide `OnceLock`s (`node::proving_key()`).
- **The leader never messages itself**: the relay rejects self-addressed envelopes (403).
  When the leader is one of the chosen signers, `request_signatures` skips it and the
  leader signs locally with `node::sign_own_shares`, keeping the serialized shares
  (`<id>.own`) until broadcast, because signing consumes its nonces; `finalize` takes them
  as `own_shares`. M0/regtest tests originally missed this (their leader never approved);
  `bridge_e2e` now makes the leader an approver.
- **Signing requests are deterministic** (same approvals → same bytes → same request
  hash), so re-running `request_signatures` after a partial failure is safe.
- **Leader resumability**: the app persists each signing request (`<state>/leader/<id>.req`)
  and the used-commitments set; `send_proposal` after a timeout resumes the same round.
  Known gap: if an approver never answers, there is no "start over" yet.
- **Shares are bound to the exact request** (request hash); aggregation always goes through
  `session::aggregate_request` (signer-set check + per-share verification).
- **Vault log replay is lenient after creation**: invalid entries go to `VaultState.ignored`
  (deterministic across members); only "Created first + all descriptor signatures" is fatal.
  Check `VaultState::check` before appending.
- **Wallet**: import the vault UFVK as `AccountPurpose::Spending { derivation: None }`
  (Keystone pattern). `ViewOnly` accounts don't track witnesses and can never spend.
  Birthday height must be ≥ 2 (sync fetches tree state at start−1; lightwalletd treats 0 as unset).
- **Relay is blind**: it only sees public keys, ciphertext, metadata. Clients drop envelopes
  for another mailbox, from non-members, badly signed, or with non-increasing seq.

## Dependency gotchas

- Exact pins live in `Cargo.toml` (spec §4.4): `reddsa =0.5.2` (`frost` feature; 0.6 removed
  FROST), `frost-core`/`frost-rerandomized` 3.0.0, `orchard =0.15.5` (no `unstable-frost`
  needed), `pczt =0.9.3`, `zcash_client_backend =0.24.0`, `zcash_client_sqlite =0.22.0`.
- `zcash_client_backend/pczt` turns on `transparent-inputs`; `zcash_client_sqlite` must enable
  `transparent-inputs` **and** `serde` or it fails to compile.
- `zcash_keys` feature `unstable-frost` gives `UnifiedFullViewingKey::from_orchard_fvk`.
- Messaging crypto is the **previous generation** (`ed25519-dalek` 2, `hpke` 0.12,
  `chacha20poly1305` 0.10): the latest (dalek 3 / hpke 0.14) needs stable `sha2` 0.11, which
  conflicts with `bip32`'s `sha2 =0.11.0-pre.4` pin via zcash_client_backend.
- `rusqlite` must use `bundled` at the workspace level (the relay links it on its own).
- `[profile.dev.package."*"] opt-level = 3`: Halo 2 is unusable unoptimized.
- `zcash_primitives` `non-standard-fees` is a **dev-dependency only** (tests model an
  overpaying proposer).
- `cargo build -p a -p b --examples` builds only examples — build bins separately.
- `propose_transfer` / `create_pczt_from_proposal` need explicit error type params
  (commitment_tree::Error, GreedyInputSelectorError, zip317::FeeError).

## Regtest (infra/regtest)

- Node: `zakuracore/zakura:1.4.0`; NU5..NU6.3 all at height 1, `disable_pow = true`.
- NU6.1's activation block needs a ZIP 271 lockbox disbursement: use the **zero-value marker**
  `t26YoyZ1iPgiMEWL4zGUm74eVWfhyDMXzY2` amount 0, or blocks are rejected.
- Coinbase to a **unified address lands in Ironwood**: fund a vault by mining to its address;
  coinbase matures after 100 blocks (mine ~120). Mining fees return via coinbase.
- lightwalletd: `ghcr.io/zcashlabs/thus-spoke-zakura-lightwalletd:0.2.1` (v0.5.4+7, Ironwood-aware).
- `ths` (thus-spoke-zakura) itself ran pre-Ironwood; fixed upstream in PR #119 (targets `dev`).

## Mobile findings (spec V7/V8)

- Phone (Snapdragon SM8735, Android 16): proving key 3.4 s / proof 4.6 s single-thread,
  2.1 s / 1.7 s on 8 threads, ~113 MB. Round 2 (verify + sign) 12 ms, 6.8 MB → fits an iOS
  Notification Service Extension (24 MB, ~30 s). Only the leader proves.
- WSL2 has no USB: use adb **wireless debugging**; `adb pair IP:PORT CODE` must have the code
  on the same line (non-interactive shell). NDK r29 at `~/android/android-ndk-r29`,
  `cargo ndk -t arm64-v8a`.

## M1 app: follow Vizor (chainapsis/vizor-wallet, Apache-2.0)

Take Vizor's architecture and UI as the reference (the user asked for close alignment):
Flutter (pinned **3.41.6** via fvm) + `flutter_rust_bridge` **2.11.1** + Rust core; Riverpod
+ go_router; `flutter_secure_storage`; design tokens with Desktop/Mobile sets selected at
build time by a `--dart-define` form-factor flag; sentence-case copy; Rust API surface
limited to primitives/flat structs (complex types stay behind it); bootstrap snapshot before
the first frame; broadcast-before-store for PCZT sends. Zafe's vault account is exactly
Vizor's Keystone account shape (UFVK-only, external signer), with FROST instead of a device.
Keep attribution/NOTICE for anything copied from Vizor.

Upstream is **github.com/chainapsis/vizor-wallet** (not the stale `valargroup` mirror the
first study used). Detailed reference (tokens, components, screens, bridge setup):
`docs/vizor-reference.md`, written from an older snapshot; check upstream for newer work.
Upstream features to borrow later: Tor via `zcash_client_backend`'s `tor` feature
(`rust/src/network_privacy.rs`: process-wide fail-closed route policy, bootstrap timeout,
dormant mode when backgrounded), settings screens, address book.
Learned while studying it:
- **Copy** architecture, tokens, component specs, screen structures, and the Keystone
  signing UX (it starts proving in the background while the external signer works — do the
  same while FROST round 2 runs). Keep Apache-2.0 attribution + a modification notice.
- **Do not copy** Vizor's knight/castle illustrations and profile pictures (brand identity, no
  documented origin), the Vizor name/wordmark, `com.keplr.vizor` bundle IDs or
  `com.zcash.wallet/*` channel names.
- **Fonts** (Geist, Geist Mono, Inter, Young Serif) are OFL 1.1 and Vizor ships no license
  texts: fetch from upstream and bundle the OFL texts.
- **Improve on Vizor**: encrypt the wallet DB (SQLCipher, spec §14), typed errors across the
  bridge instead of substring matching.
- FRB: mark cheap calls `#[flutter_rust_bridge::frb(sync)]` (otherwise Dart gets a Future).
  `flutter_rust_bridge_codegen generate` works without `cargo-expand` (it only warns).
  The bridge crate `app/rust` (`rust_lib_zafe`) is a workspace member so it shares pins.
- App layout: `lib/src/core/` is copied Vizor code (keep in sync with NOTICE); Zafe code is
  `lib/src/{providers,features}`, `lib/src/app.dart` (GoRouter + redirect on vault state),
  `lib/main.dart` (RustLib.init → `VaultBootstrap.load()` → ProviderScope override).
  Secrets (identity, invite, key material) live in `flutter_secure_storage` via
  `core/storage/zafe_secure_store.dart`. Network/relay/lightwalletd come from dart-defines
  (`ZAFE_NETWORK`, default regtest) in `core/config/network_config.dart`.
- **Bridge errors are typed**: API functions return `Result<T, ZafeError>` (`api/error.rs`,
  `kind` + `message`); Dart maps `ZafeErrorKind` to copy in `core/errors/zafe_error_copy.dart`.
  FRB treats a `type Result<T> = ...` alias as **anyhow** — always write
  `Result<T, ZafeError>` in public signatures or the typed error silently disappears.
- FRB `StreamSink<T>` gives Dart a `Stream` (used for send progress). A streaming
  function's returned `Err` never reaches the Dart listener (unhandled exception), and
  `sink.add_error(ZafeError)` arrives as an undecodable `AnyhowException`: report failures
  as a normal event (`SendStage::Failed` + `error: Option<ZafeError>`). The generated Dart
  `ZafeError` has no useful `toString`; log with `describeError`. Keep a plain-callback
  twin marked `#[frb(ignore)]` (`send_with_progress`) so Rust tests can drive it.
- Flows: `/send` (recipient → amount → review → "Propose payment"), `/proposal/:id`
  (independent check on this device, votes, approve/reject, "Collect signatures & send").
  Home polls every 15 s: proposals refresh + answering signing requests, then wallet sync.
  Wallet DB access is serialized by `wallet_lock()` in the bridge.
- `VaultWallet::create` does all network calls **before** creating the DB file; the bridge
  also deletes a DB with no account (`VaultWallet::exists`). Previously the first sync with
  lightwalletd down left an empty DB that failed forever ("expected one account, found 0").
- Vizor's `AppButton` used an onTapUp-only detector (no semantics tap action); Zafe wraps
  it in `Semantics(button, onTap)`. Its label still shows as a separate node in
  accessibility trees (open item). Use `expand: true` inside `Expanded` rows.
- The pushed-page serif title truncates past ~14 characters: keep page titles short.
- **Privacy mode is app-wide** (`privacyModeProvider`, persisted in prefs, read in the
  bootstrap): every amount goes through `amountWithTicker(text, hide:)`. Vizor's
  `hideAmountIfPrivacyMode` only appends the unit to the *mask*, so passing a bare amount
  drops the ticker when visible. Payment rows use a 3-star mask (Vizor's activity rows).
- Settings (`/settings`, opened from the vault name on home): vault info, signer key,
  hide amounts, theme (`themeModeProvider`, persisted), endpoints (read-only; compile-time
  dart-defines), open-source licenses (fonts + NOTICE registered in `main.dart`).
- Font family names must match Vizor's tokens exactly (`Geist Mono`, `Young Serif`).
- `pubspec.yaml` must have a single `flutter:` key (a duplicate silently breaks FRB codegen).
- Cargokit is patched (`rust_builder/cargokit/gradle/plugin.gradle`): debug builds no longer
  add x86/x64 unless `-Pzafe.debugEmulatorAbis=true`. Build for a phone with
  `flutter build apk --debug --target-platform android-arm64` (~5 min cold).
  Stale emulator `.so` files can linger in `build/rust_lib_zafe/jniLibs`; delete them.

Device testing (agent-device): emulator AVD `zafe` (API 35 x86_64; needs `/dev/kvm`
access and `libxkbfile.so.1`, which was unpacked into `~/android/sdk/emulator/lib64` without
root). Start it windowed with `emulator -avd zafe -gpu swiftshader_indirect -no-snapshot
-no-audio`; build with `--target-platform android-x64`. `scripts/app-harness.sh` runs the
relay plus CLI members B and C and sets `adb reverse` for 8787/9067, so the app's localhost
defaults work on a device. Its `cli` subcommand does not rebuild: `cargo build -p zafe-cli`
after core changes. agent-device tips: prefer `find "<text>" click`; refs go stale after
every snapshot; `scroll down --until 'label="..."'` before pressing bottom buttons.

App commands (from `app/`, after `source ~/android/env.sh`):
`flutter_rust_bridge_codegen generate` (after changing `app/rust/src/api`), `flutter analyze`
(must be clean), `flutter build apk --debug --target-platform android-arm64`,
`adb install -r build/app/outputs/flutter-apk/app-debug.apk`.

Toolchain (installed by `~/android/install-toolchain.sh`; `source ~/android/env.sh`): Flutter at `~/flutter`, JDK 17 at
`~/android/jdk-17`, Android SDK at `~/android/sdk` (platform 36, build-tools 36.0.0).

## Upstream status (check before relying on it)

- **Mainnet gate (U1)**: ZF has not yet confirmed that Zafe's ZIP 2005 derivation stays
  recoverable whatever `sk`-agreement zips#895 standardizes (frost#1094). Testnet/regtest only
  until then; small capped amounts at most.
- COCKTAIL-DKG (frost#1033) not production-ready; Zafe uses frost-core DKG + own echo/transcript.
- RedPallas FROST ciphersuite moving out of `reddsa` (frost#963); stay on 0.5.2 until then.

## User preferences

- Research primary sources (repos, ZIPs, issues) before asking the user or relying on memory;
  Zcash moves fast (Ironwood/NU6.3 postdates older knowledge).
- Record durable learnings here as you go.
