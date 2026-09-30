# Zafe tracker

What's left, what we deferred, and ideas worth keeping. Update it whenever something is
decided, deferred, discovered or finished. Agents: add items you leave behind; don't delete
finished ones, tick them and add the commit. Spec references are to `spec.md`.

Legend: `[ ]` open · `[x]` done · **(you)** needs the user · *(idea)* not yet decided

Last updated: 2026-09-30 (note reservation; versioned formats; relay TLS + packaging; wallet DB encryption; incoming payments)

---

## Next up (proposed order)

0. [x] **Proposals expire after ~50 min.** Fixed: vault descriptor `proposal_expiry_blocks` (default 7 days), proposer sets it, members accept it plus ~2 h tip slack; `bridge_e2e` lets 300 blocks pass before approving. PCZTs are built with the library default expiry
   (`DEFAULT_TX_EXPIRY_DELTA` = 40 blocks) and `verify` allows at most 100 blocks, so
   approvals and signing must finish within about 50 minutes. Async multisig needs a long
   expiry (days; configurable per vault, D2). Anchors can go stale safely (V6: witnesses
   can be refreshed after signing, before proving). Blocks any real async use.
1. [x] **One-tap approvals + auto-submit** (spec §9.5.2): approving signs; the member who
   completes the signatures sends automatically (per-proposal setting), or anyone taps
   "Send now". Tested in `one_tap.rs`, `vault.rs`, `bridge_e2e` and on the emulator.
   Follow-ups below. ~~**Auto-submit when the threshold is reached.** The member whose approval completes
   the threshold (known from log order, so exactly one per proposal) starts "collect
   signatures & send" automatically. The manual button stays as the fallback. ~1 day.~~
2. [~] **Notifications** (in progress 2026-09-29): relay pushes on every log append
   (FCM sender done, tested against a mock); app shows local notifications from a
   background check (WorkManager 15 min, FCM wake-up, one-off after backgrounding) and
   opens the payment on tap. FCM is configured (project `zafe-18c4d`) and verified end to
   end on the emulator (relay → Google → device → background check → notification).
   Next: confirm latency on a real phone.
   Open: APNs sender + iOS (NSE). Done: dead FCM tokens are pruned (2026-09-30).
   Done: relay-client TLS (2026-09-30, see M1 "Hosted relay").
3. [ ] **Tor** (as in Vizor `rust/src/network_privacy.rs`): `zcash_client_backend` `tor`
   feature (arti), process-wide fail-closed route policy, bootstrap timeout, dormant when
   backgrounded; route both lightwalletd and the relay (relay client moves off reqwest).
   Settings toggle + status. A full slice; deferred on 2026-09-29 as "not small".
4. [x] **Incoming history** (2026-09-30): received payments (any transaction paying the
   vault that spends none of its notes, so never change; coinbase included) are read
   from the wallet DB (`VaultWallet::received_payments`, bridge `api/received.rs`), merged
   with proposals by time on Home and `/activity`, open `/received/:txid`, and are
   announced by the background check ("payment received"). Tested: wallet unit test on a
   hand-built DB, `bridge_e2e` (coinbase receipts listed, own spend excluded), Dart tests.
   Checked on the emulator: rows on Home, `/received/:txid` page. Notes: a received payment's sender is
   unknowable (shielded), so there is no "from"; pending (unmined) receipts only appear if the wallet learns of them, which
   `sync::run` doesn't do (no mempool scan).
5. [x] **Scan invite QR** on Join (camera, `mobile_scanner`; accepts the raw invite or the
   invite link; permission-denied state with "Open settings"). Invite links
   `zafe://join?invite=...` (app_links; cold and warm start; from inside a vault it's
   "Add vault"), "Share link" on the setup screen, and the setup QR now carries the link.
   Verified on the emulator (2026-09-30): warm-start deep link prefills Join with the
   "Opened from a link" warning, Back → add-vault welcome → ✕ returns to the vault,
   scanner opens with a live preview. Still unverified: cold start, permission-denied state. iOS untested (URL scheme + camera string added).

## M1 — app v1 on testnet (spec §16)

Done
- [x] Flutter + FRB app on Vizor's architecture and design system (`c07c687`)
- [x] Create / join (paste) / seal / safety number / keygen, receive, balance (`c07c687`)
- [x] Payment proposals, independent check on device, approve/reject, async signing
      through the relay, send with progress (`c508aff`)
- [x] Settings, app-wide hide amounts, theme (`c508aff`)

Open
- [x] Auto-submit (see Next up 1)
- [~] Push notifications (see Next up 2)
- [x] Clients speak TLS (2026-09-30): relay client (reqwest `rustls-tls`) and lightwalletd
      (tonic `tls-ring` + `tls-webpki-roots`, explicit `ClientTlsConfig` for `https`), both
      with the bundled Mozilla roots; `crates/zafe-core/tests/tls.rs` (local rustls server:
      trusted cert works, untrusted and wrong-hostname refused; ignored live test against
      `testnet.zec.rocks:443` passed). App `ZAFE_NETWORK=test` preset (TLS lightwalletd,
      placeholder relay URL). `cargo ndk` check for arm64 passes with ring
- [x] Relay packaging (2026-09-30): `infra/relay/Dockerfile` (non-root, `/data` volume,
      `$PORT`), `GET /health`, SIGTERM shutdown, FCM key from a file or secret env var;
      Fly.io template, VPS recipe (systemd + Caddy), nightly backup timer, README
- [ ] **(you)** Hosted relay deployment (testnet): waiting on the host choice (Fly.io or a
      VPS) and account/DNS; steps in `infra/relay/README.md`. Then build the testnet app
      with `ZAFE_RELAY_URL`. Relay is SQLite today, Postgres for the hosted tier
- [x] Relay rate and size limits (2026-09-30): token buckets per signing key (after the
      signature verifies) and per client IP (`zafe_relay::limits`, 429 + `Retry-After`,
      env-tunable, proxy header for Fly/Caddy), 1 MiB body cap; client
      `RelayClientError::RateLimited` → `NotReady` ("the relay is busy"). Keygen
      (`node_keygen`) and the whole payment flow (`bridge_e2e`) run under the hosted limits
- [ ] Relay follow-ups: storage quotas (undelivered envelopes per recipient, log size per
      mailbox), off-site backups (Litestream) *(idea)*. Done: dead FCM tokens (404
      `UNREGISTERED`) are deleted (`FcmNotifier::on_unregistered` →
      `Relay::forget_push_token`)
- [x] Invite by link: `zafe://join?invite=...` custom scheme (see Next up 5)
- [ ] Universal/App Links (`https://…/join#invite`) so a link works for people without
      Zafe installed (landing page + `assetlinks.json` / AASA). Put the invite in the URL
      fragment so the web server never sees the join token
- [ ] iOS: `permission_handler` needs `PERMISSION_CAMERA=1` in the Podfile
      `GCC_PREPROCESSOR_DEFINITIONS` if we ever request through it (today only
      `openAppSettings` is used; `mobile_scanner` asks for the camera itself)
- [x] Cancel a proposal (author) in the UI (2026-09-30): "Cancel payment" on the proposal
      page with a confirmation that warns when every signature is already in (it stays
      sendable until expiry); bridge `cancel_proposal` → `node::cancel`. Tested in `bridge_e2e`
- [x] Proposal expiry in the UI (2026-09-30): `ProposalInfo.expiry_height`; "Expires in
      about N days" on open/approved proposals; once the synced tip reaches it, rows, chip
      and page say "Expired", it stops counting as needing action and isn't auto-sent;
      "Propose again" opens Send on the review step prefilled (`SendPrefill`, single
      payment). Dart tests in `test/proposal_expiry_test.dart`. Emulator (2026-09-30): the
      "Expires in about 7 days" line; after mining past expiry the row, chip, title and
      page read "Expired" (amount struck through), and "Propose again" opened the review
      step prefilled and logged a new proposal. The CLI's `proposals` still prints an
      expired one as `Open` (no tip there)
- [ ] Expiry window configurable at vault creation (D2): every member builds and signs the
      descriptor, so the choice has to travel in the invite (invite format bump) or the
      seal; today it's `DEFAULT_PROPOSAL_EXPIRY_BLOCKS` (7 days)
- [ ] Privacy: a 7-day expiry delta differs from the 40-block wallet default, so vault spends are distinguishable on chain by expiry. Consider rounding or a shared convention
- [x] "Start over" for a signing round (2026-09-30): after a timeout the leader can "Start
      over with other signers" (bridge `restart_signing` drops `<id>.req`/`.own`; used
      commitment sets stay used); members whose approval's nonces are gone
      (`ProposalInfo.needs_reapproval`: used in the unfinished round, or restored from a
      backup) see "Approve again". Tested in `bridge_e2e` (round 1 times out, start over,
      A re-approves, C joins, sent). Emulator walkthrough (2026-09-30, app + CLI B/C):
      round with C timed out, start over reported "0 of 2 signers are ready", B approved,
      the app showed "Approve again", the new round with B was sent. It found and fixed:
      the proposal page kept the failed-send card with only "Try again" and never offered
      "Approve again"; the sending screen had no "Start over". Cancel checked too
- [ ] Start-over follow-ups: the unresponsive signer keeps nonces for commitments no
      leader will use (harmless, never reused; deleted only when the proposal closes if at
      all). Done: members get a one-time "approve a payment again" notification
      (`reapprovalKey` in the seen snapshot) and it counts as needing action
- [x] Note reservation across concurrent proposals (`reservedNotes`, spec §9.1; done
      2026-09-30): before building, `node::propose` replays the log and locks, in this
      member's wallet, every note an open/approved/broadcast proposal (or a cancelled one
      with a complete group) spends, until that transaction's expiry
      (`node::note_holds` → `VaultWallet::reserve`, upstream `OutputLockStore`, owner =
      PCZT hash). Works across members, not just on one device. Short of unheld funds →
      `WalletError::FundsReserved` / `ZafeErrorKind::FundsReserved`. Tested in `regtest_e2e`
- [x] Reservation follow-ups (done 2026-09-30): (a) **replay rule**: a `Proposal` that
      spends a note of an earlier open/approved proposal not yet expired (as of the new
      proposal's `tip_height`) is ignored by every member (`VaultError::NotesInUse`);
      `node::propose` sees it at append time and rebuilds from the remaining notes (3 tries);
      (b) `sync_vault` refreshes holds after every sync (`node::reserve_notes`), so every
      member's spendable balance leaves out held notes; (c) a broadcast tx that isn't mined
      and isn't in lightwalletd's mempool any more releases its notes (the whole mempool is
      read, `wallet::mempool_txids`, so lightwalletd doesn't learn the txid). Cancelled
      proposals hold nothing now, so a new proposal can respend (invalidate) their notes
- [ ] Rebroadcast a dropped transaction instead of only releasing its notes (the
      broadcaster doesn't keep the raw tx today; one-tap proposals could be rebuilt by
      any member from the log)
- [x] Member names (local labels, 2026-09-30): tap a signer on Home to name them; the
      name (`<vaultDir>/names.json`, `memberNamesProvider`) shows in signer rows (short
      key below) and as "Proposed by". Local to this device: not synced, not in backups,
      not in notifications yet. Checked on the emulator
- [x] Member names follow-ups (2026-09-30): names are in encrypted backups (`BACKUP` v2,
      v1 still restores with no names) and restored with the vault; notifications name
      a named proposer / rejecters / canceller (unnamed signers aren't mentioned); the
      CSV's proposer and approvers columns read "Name (hexkey)" (approvers now `; `-
      separated). Not on a device yet
- [ ] Share signer names via the log (today each device names signers on its own)
- [ ] Endpoint settings editable (today: compile-time dart-defines, read-only)
- [ ] iOS: build and run at all (only Android has been exercised)
- [ ] iOS: exclude the nonce directory from backups (`isExcludedFromBackup`)
- [x] Biometric/passcode gate before approving and signing (spec §14; done 2026-09-30,
      `local_auth`; approve, send, propose, backup export, vault removal; setting in
      Settings, default on). Needs an on-device check with a real screen lock
- [ ] Unlock gate follow-ups: it is a UI gate only (key material in secure storage is not
      bound to user authentication; a Keystore key with `setUserAuthenticationRequired` /
      Keychain `.userPresence` would make it cryptographic, but background round-2 signing
      needs the key while locked, spec V8). No "open security settings" shortcut from the
      no-screen-lock warning. Done 2026-09-30: Launch/NormalTheme use AppCompat parents
      (+ explicit `androidx.appcompat`), so the biometric prompt can't crash on Android 8
      and below; checked the app still launches (emulator, API 35)
- [x] SQLCipher for the wallet DB (spec §14; done 2026-09-30): per-vault random key in
      secure storage, `PRAGMA key` on every connection, unreadable/plain DBs are deleted
      and resynced. Vendored OpenSSL (static libcrypto) on every target
- [ ] Wallet DB follow-ups: iOS could use CommonCrypto instead of vendored OpenSSL
      (smaller binary; target-specific rusqlite features); the key lives in the same secure
      storage as the vault material, so it protects against file-level copies (backups,
      forensic dumps of app data), not against an attacker who can read the Keystore
- [x] Pre-warm the proving key when a proposal becomes Approved (2026-09-30): bridge
      `prewarm_prover` (proving + verifying key `OnceLock`s), started once per process by
      `ProposalsNotifier` when any unexpired proposal is approved. Costs ~113 MB for the
      process lifetime on a phone; only on devices that see an approved payment
- [ ] Low-end device benchmark (V7 still open: Cortex-A55-class phone)
- [x] App icon, launcher name/branding: original vault-dial icon (adaptive + themed
      monochrome, legacy mipmaps, iOS AppIcon), label "Zafe", native splash in the window
      colour (Android 12+ and older, iOS LaunchScreen). Regenerate: `scripts/brand/icons.sh`.
      Needs a look on a real launcher (Android and iOS) once a device build is made
- [x] Release build + signing + GitHub Releases (2026-09-30): `.github/workflows/release.yml`
      (tag `v*` or manual) builds a signed arm64 testnet APK (`--split-per-abi`: plugins
      ship extra ABIs our Rust lib lacks, and Flutter's Gradle plugin overwrites buildType
      abiFilters) and publishes a pre-release with SHA-256 + certificate fingerprint;
      refuses without `ZAFE_RELAY_URL` or signing secrets. Gradle signs from
      `android/key.properties`. Setup: `docs/releasing.md`. Local release builds checked:
      arm64 61.8 MB (3 ABIs before the split), x86_64 split 57.8 MB launched on the
      emulator, created a vault and shared the invite QR (R8 kept every plugin working).
      **(you)** keystore, secrets, `ZAFE_RELAY_URL`; the workflow has not run on GitHub yet
- [ ] Reproducible / F-Droid builds (Vizor `scripts/build-android-reproducible.sh`) *(idea)*

## Multiple vaults, import and export (requested 2026-09-30)

Multiple vaults on one device (done 2026-09-30, verified on the emulator: migration from the
single-vault layout, add via the switcher, cross-vault notification tap, remove)
- [x] **Storage per vault**: secure store keyed by vault id (today one identity, invite and
      material slot); per-vault `state_dir` (`used_commitments.bin` and the notification
      snapshot are shared today); wallet DBs are already per vault (`vault-<id>.sqlite`)
- [x] **A fresh member identity per vault** by default, so the relay can't link one
      person's memberships across vaults (it sees each member's public key)
- [x] **Vault switcher**: Vizor's account sheet pattern (tap the top-nav vault name/avatar),
      showing each vault's name, rule (2/3), balance (respecting hide amounts) and a badge
      for payments needing approval; "Add vault" (create or join) from the sheet
- [x] Per-vault settings; **remove vault from this device** with a clear warning (it
      doesn't leave the vault; say what the other members lose if this was needed for t)
- [x] Background checks, notifications and push registration for **every** vault
      (notification title already carries the vault name)
- [ ] Revisit the bridge's single global wallet lock (Known issues) for parallel syncs

Export and import (done 2026-09-30, verified on the emulator: export with a suggested
passphrase and share sheet, restore of another member's seat from pasted text, wrong
passphrase and already-on-this-phone refusals, backup prompt after creating a vault)
- [x] **Export this vault** as an encrypted file (spec §12.2: identity, FROST key package,
      `sk`, descriptor, log key, `use_qsk`; **never** nonces or pool nonces; Argon2id ≥
      64 MiB + XChaCha20-Poly1305; strong passphrase), via the share sheet or saved file.
      Versioned format with a magic header
- [x] **Import a vault** on a new or second device from that file + passphrase, then
      resync the log and the wallet from the birthday; publishes a fresh commitment pool
- [x] Decide **move vs. copy** semantics (decided: backups are passive copies; "move" = restore on the new phone, then Remove on the old one, as the export screen says): two live devices holding the same member share
      can both vote and sign (fresh nonces each, so no key leak, but confusing votes).
      Proposal: "move to a new phone" exports, then retires this device's copy
      (deletes material and pool nonces after the import is confirmed); plain backups
      stay passive files
- [x] **Viewing-key export** for auditors (2026-09-30): Settings → "Viewing key" (unlock
      first) → `/viewing-key` (SecureScreen): the UFVK (`vault_viewing_key`, derived from
      the material and checked against the signed descriptor) as text, QR and copy. Not
      on a device yet
- [ ] **Import a vault as view-only** (auditor mode: balance and history from a UFVK, no
      signing)
- [~] Backup health: per-device prompt after creation + home reminder done; members attesting backups in the log (so others see vault-wide backup health) still open
- [ ] Multi-part QR for device-to-device transfer (material is a few KB) as an
      alternative to files, *(idea)*

## M2 — v1 feature-complete, mainnet beta (spec §16)

- [x] Batch payments UI (2026-09-30): "Add another recipient" on the review step (up to
      50), a recipients list with remove, available balance net of added recipients;
      proposal pages list every recipient (`RecipientsCard`); "Propose again" keeps the
      batch. Emulator: a 2-recipient proposal verified on B (fee 15000 zat, 3 actions),
      approved, signed interactively with B and sent
- [x] Import batch recipients from CSV (2026-09-30): "Import recipients from CSV" on the
      recipient step, "Import CSV" on review; `address,amount[,memo]`, optional header,
      ZEC decimals, RFC 4180 quoting (`features/send/recipients_csv.dart`, unit-tested);
      all-or-nothing with row-numbered errors; respects the 50 cap. Not on a device yet
- [ ] Batch follow-ups: per-recipient edit (today: remove and re-add)
- [ ] Address book via `ADDRESS_BOOK` proposals (t approvals, no FROST); warn on unknown
      recipients (§11.2)
- [ ] Rules via `RULES` proposals: per-tx / per-period limits, allowlist-only,
      large-payment extra approvals; enforced by apps before voting and signing (§11.1)
- [x] CSV export of vault history, generated locally (§11.3; 2026-09-30):
      `zafe_core::history` (sent rows from the log, one per payment, fee on the first;
      received rows from the wallet; UTC ISO dates, exact 8-decimal ZEC, RFC 4180 quoting,
      formula injection defused for memos), bridge `export_history_csv`, "Export" on
      Activity (unlock, then share sheet). Tested in `bridge_e2e` and on the emulator.
      Contact names stay empty until the address book exists
- [x] Encrypted backup (Argon2id + XChaCha20-Poly1305; file via the share sheet, or text) and restore;
      never includes nonces (§12.2–12.3)
- [ ] Backup health: members attest a verified backup in the log (§12.2)
- [ ] Repair a lost device's share (frost-core repairable, with verifying-share checks and
      a test signature before deleting old shares) (§10.1, §10.4)
- [ ] Transparent / TEX recipients with a warning (spec allows; verify.rs is
      Ironwood-only today)
- [ ] External security audit of zafe-core and the protocol
- [ ] **Mainnet gate U1** (see Upstream) before any real funds

## M3 / M4 — later (spec §16)

- [ ] Rotation via DKG refresh (remove members; t fixed) (§10.2)
- [ ] Migration to a new vault (change t or add members) (§10.3)
- [ ] Desktop builds (Vizor has desktop layouts to copy)
- [ ] Self-hostable relay packaging; paid hosted tiers (D3 limits)
- [ ] dApp SDK: proposal requests from third-party apps
- [ ] Hardware members (Keystone/Ledger) if they gain FROST support
- [ ] Payment disclosures (ZIP 311-style proofs for auditors)

## One-tap follow-ups

- [ ] **Cancel after completion**: a cancelled proposal whose signer group is complete can
      still be sent until expiry. Offer "invalidate now" (spend its notes to self) and
      explain it in the cancel UI. The log and wallet holds already allow it (cancelled
      proposals hold no notes); what's missing is the UI and a self-send proposal
- [x] Delete pool nonces of **expired** proposals (2026-09-30): `forget_closed` takes the
      synced tip (bridge `list_proposals(tip_height)`, passed by the app and background
      check); tested in `tests/vault.rs`. Interactive nonces of closed or expired
      proposals are deleted too (`node::forget_closed_nonces`, same refresh)
- [x] Surface "one-tap unavailable" (2026-09-30): the Approve step explains that approvers
      sign again when it's sent. Pools: the app tops up on every proposals refresh (first
      Home refresh after keygen); the CLI only with `zafe pool`, so harness vaults are
      interactive unless each CLI member runs it
- [ ] A member whose device lost its pool nonces approves interactively (fallback works);
      show that their approval still needs a signing round
- [ ] Push notifications now matter less (approvals are final at tap), but still needed
      so members learn about new proposals
- [ ] CLI: `approve` doesn't auto-send (by design for scripts); document `zafe send`

## Upstream and waiting on others

- [ ] **(you)** Send the ZF questions in `upstream-asks.md` (Q1 is the mainnet gate U1)
- [ ] U1: ZF confirms the ZIP 2005 derivation stays recoverable under zips#895; get our
      `test-vectors/zip2005_use_qsk.json` cross-checked upstream
- [ ] U2: non-deprecated external-randomizer API in frost-rerandomized (frost#1094)
- [ ] U3: redpallas ciphersuite home + reddsa 0.5.x security-fix policy (frost#963)
- [ ] U4: COCKTAIL-DKG Pallas + ZIP 312 keygen spec (zips#895, frost#1033)
- [ ] U5: ZF confirms one-tap signing (pre-published commitments, PCZT-fixed alpha) is within the Re-Randomized FROST model — `upstream-asks.md` Q7 **(you: send with Q1)**
- [ ] thus-spoke-zakura PR #119 (Ironwood fix, targets `dev`): follow up until merged
- [ ] Zakura's faster prover once it supports Ironwood (V7)

## Known issues and tech debt

- [ ] **First sync after keygen stuck on "Syncing..."** (emulator, 2026-09-30, once):
      Rust and platform threads were idle (native + Java stack dumps), no `sync failed`
      log, `ZafeSecureStore._creatingWalletKey` empty; a restart synced at once. Not
      reproducible with lightwalletd down (that fails cleanly). Guarded: `sync()` now times
      out after 6 minutes so `syncing` can't stick. Root cause unknown; add breadcrumbs if
      it recurs
- [x] A payment to the vault's **own address** failed every member's check (verification
      counts the output as change): `node::propose` now refuses it up front
      (`WalletError::Payment` → `InvalidInput`, "that is this vault's own address"),
      tested in `bridge_e2e`. The Send screen still only finds out at "Propose payment"

- [x] `AppButton` label was a separate node in accessibility trees (2026-09-30): `Focus`
      sat outside the `MergeSemantics` and added its own unlabeled focusable node. Now
      `MergeSemantics(Semantics(button, enabled, onTap, Focus(...)))`, the detector
      excluded from semantics; `test/app_button_semantics_test.dart` checks one node.
      Upstream Vizor `4bff2e7` has no `Semantics` at all: keep ours when resyncing.
      Still worth a TalkBack pass on a device
- [ ] Proposal `created_at` is the proposer's clock (display only, untrusted)
- [ ] Wallet DB access is serialized with one global lock in the bridge; fine for one
      vault, revisit for multiple vaults
- [ ] Sync error copy: only "can't reach the network" vs "sync failed, retrying"; no
      details screen. Upstream typed kinds to copy: `lib/src/providers/sync_failure.dart`
      (`SyncFailureKind` incl. `torUnavailable`) + `core/formatting/sync_status_label.dart`
      ("Sync paused" states); Vizor has no details screen either
- [x] Copied Vizor `lib/src/core` resynced to `chainapsis/vizor-wallet` @ `4bff2e7`
      (was `ff02152`), keeping Zafe's button semantics; `docs/vizor-reference.md` §11.
      Also adopted: transaction progress screen (`/proposal/:id/send`), activity screen
      with Vizor's sections (`/activity`), entry-card backup reminder, destructive error
      toasts, `DecimalAmountInputFormatter`, full-address verify sheet
- [x] Zafe visual identity (2026-09-30): slate + jade palette, Space Grotesk / DM Sans /
      JetBrains Mono, rounded-rect buttons, vault-dial home card, left-aligned page titles,
      Vizor references removed from code comments; illustrations redrawn in the palette
- [x] Identity follow-ups: proposal page and send review use the payment card (dark vault
      card, amount + compact recipient), approvals block with signer dots, and signer rows
      with key-derived tiles (also on the home Signers card); icon and splash done.
      Previews without a device: `flutter test tool/screens/proposal_render_test.dart`
- [ ] Member tiles are derived from keys only; switch to names/colours once member names land
- [ ] iOS: `xyz.zafe/modal_corners` has no Swift handler yet (Dart falls back to fixed
      corners); port Vizor's `NativeModalCorners` when iOS work starts
- [x] CI: `.github/workflows/ci.yml` runs `cargo fmt --check`, clippy `-D warnings`,
      `cargo test --workspace`, the ZIP 2005 Python check, `flutter analyze` and
      `flutter test test` (regtest/Docker tests stay manual). No GitHub remote yet, so it
      runs once the repo is pushed
- [ ] Agent-device flows are manual; capture them as a repeatable script
      (`scripts/app-harness.sh` covers the backend side). Model: Vizor's app-level regtest
      E2E, `integration_test/payment_uri_prefill_test.dart` driven by
      `scripts/e2e/flutter-ios-regtest-mobile-*.sh`
- [ ] Background check ANR (2026-09-30, emulator under heavy host load): Android reported
      "No response to onStartJob" for the WorkManager vault check. Likely load, but check
      that the job's startup (RustLib.init, Firebase, secure storage reads) doesn't block
      the main thread before WorkManager gets its answer
- [ ] Relay: storage quotas for the hosted tier (rate limits done); retention is 30 days
- [x] **Versioned formats** (2026-09-30): every wire and stored format carries a version
      tag (`zafe_proto::version`; inventory and bump rules in AGENTS.md), signed where a
      downgrade matters (envelope and log-entry headers, relay requests, descriptor).
      Unknown versions fail with a typed `UnsupportedVersion`; the relay answers 426 and
      the app shows "update the app" (`ZafeErrorKind::UpdateRequired` / `RelayOutdated`);
      replay ignores newer events after `Created`. Unversioned pre-release data is not
      readable: reset devices, harness and relay DBs
- [x] "Update Zafe" notice on Home when the log has entries from a newer version
      (`list_proposals` now returns `ProposalList { items, newer_version_entries }`;
      `ProposalsState.newerVersionEntries`; Home `_NoticeCard`, shared with the backup
      reminder). No store link yet
- [ ] Versioning follow-ups: decide a migration
      policy (which old versions each decoder keeps) before external testers; a
      `/v1/version` endpoint or response header so the app can warn before the first
      failing call *(idea)*

## Ideas (not decided)

- *(idea, proposed 2026-09-29)* **One-tap approvals: sign at approval time.** Members
  pre-publish pools of FROST nonce commitments (FROST's preprocessing round, done ahead
  of time). When a proposal is built, its signing packages are fixed, so an approver's tap
  produces its signature shares immediately; shares go into the encrypted log. Either the
  proposer fixes the signer set, or each approver signs one package per t-subset that
  includes them (C(n-1, t-1) shares: 2-of-3 → 2, 3-of-5 → 6, 4-of-7 → 20), so any t
  approvers complete it with no second round. Each nonce is used once, with consumption
  ordered by the log. Fall back to today's interactive round for large vaults. Pairs with
  auto-submit: the approval that completes the threshold aggregates, proves and
  broadcasts; a vault setting keeps "send manually" for timing control.

- *(idea)* Relay as a Tor onion service (pairs with the Tor slice; Vizor routes broadcasts on
  an isolated circuit, `open_isolated_lwd_channel` in `rust/src/wallet/sync_engine/lwd.rs`)
- *(idea)* Fiat values next to amounts (Vizor shows USD; needs a price source, Tor-aware;
  Vizor fetches through `lib/src/core/network/network_http_client.dart`)
- [x] Sensitive-screen protection, Android (2026-09-30): `SecureScreen` (counted) sets
  `FLAG_SECURE` through `xyz.zafe/secure_screen` (`MainActivity`) on /setup, /join,
  /export and /restore; checked on the emulator (flag on while open, off after).
  Still open: iOS capture shield and screenshot warning, an app-switcher privacy overlay.
  Earlier idea text: Sensitive-screen protection (Vizor's `SensitivePrivacyOverlay` + Android
  `FLAG_SECURE`) for invite, safety number and backup screens. Upstream now also blocks
  capture on iOS (`SecureScreenshotShield` in `ios/Runner/AppDelegate.swift`) and warns
  after screenshots (`lib/src/core/platform/screenshot_observer.dart`)
- *(idea)* "Keep screen awake" while collecting signatures (Vizor has this setting):
  `lib/src/providers/sync_keep_awake_provider.dart`, `lib/src/services/native_screen_awake.dart`
  (ETA-based prompt, privacy lock after 1 min idle)
- *(idea)* Show which signers are online / last seen (relay metadata; privacy trade-off)
- *(idea)* Explorer link for txids with a custom explorer setting
  (`lib/src/features/settings/screens/mobile/mobile_explorer_screen.dart`,
  `lib/src/core/config/zcash_explorer.dart`; default CipherScan, `{txid}` templates) and
  txids in explorer byte order
- *(idea)* Proposal comments/discussion thread in the encrypted log
- *(idea)* Scheduled / recurring payments (grant programs pay in milestones). Model: Vizor's
  background outbox of fully signed txs with `scheduledHeight` / `expiryHeight` and a
  `needsResign` state (`ios/Runner/BackgroundMigrationOutbox*.swift`,
  `BackgroundMigrationManager.swift`)
- *(idea)* Grantee payment requests: "Request ZEC" on the vault receive screen with a ZIP-321
  builder that never emits `label`/`message` (`lib/src/core/zcash/zip321_payment_request_builder.dart`,
  `lib/src/features/receive/widgets/mobile/receive_request_sheet.dart`)
- [x] Recipient QR and payment links (2026-09-30): "Scan QR code" on Send's recipient step
  and a "Choose image" option on every scanner (a shared QR picture or screenshot,
  `MobileScannerController.analyzeImage`); pasting a `zcash:` link works too. Bridge
  `parse_payment_request` (ZIP 321 via `zip321`: amount, memo, several recipients → a
  batch; every address checked for the vault network), tested in
  `app/rust/tests/payment_request.rs`. The invite card has "Share QR image" (PNG, white
  background, quiet zone) now that the screen blocks screenshots; Join's scanner can read
  it with "Choose image". Emulator: picked a QR image → amount, recipient and memo filled.
  Not done: opening `zcash:` links from other apps (intent filter), live-camera test on a
  phone
- *(idea)* Propose from a request: a scanned/pasted/opened `zcash:` URI shows a card that
  prechecks and builds the proposal up front (`lib/src/features/send/widgets/payment_request_host.dart`,
  `lib/src/features/send/services/payment_request_precheck.dart`)
- *(idea)* One link intake for `zafe://` invites and `zcash:` URIs: classify host before
  scheme, exact route allowlist, pure drain policy for locked/onboarding states
  (`lib/src/core/navigation/incoming_link_dispatch.dart`, `payment_uri_drain_policy.dart`);
  pairs with M1 "Invite by link"
- *(idea)* Put invite secrets in the URL fragment as a compact positional payload so a link
  host never sees them (`docs/compact-gift-links.md`)
- *(idea)* Auditor export of the vault UFVK behind the passcode, reusing Vizor's copy
  (`lib/src/features/settings/viewing_key_copy.dart`, `mobile_viewing_key_screen.dart`);
  complements M3 payment disclosures
- *(idea)* Batch payments UI from Gift Card groups: "Per card × N + Network fee = Total",
  fee estimated for the whole output set, CSV export
  (`lib/src/features/payment_links/widgets/payment_link_bulk_desktop_flow.dart`,
  `services/payment_link_batch_export.dart`); feeds M2 batch payments
- *(idea)* Unknown broadcast results: "Payment status pending" / "Check status" (scan only),
  never offer a second send (`docs/gift-card-groups.md`,
  `lib/src/features/pay/screens/mobile/mobile_pay_submitted_screen.dart`)
- *(idea)* Durable checkpoint of the aggregated tx before broadcast, with a startup recovery
  loop (`signed_pending_broadcast` / `result_pending_ack`) and a claim registry
  (`rust/src/wallet/ledger/operations.rs`, `lib/src/features/ledger/services/ledger_operation_recovery.dart`)
- *(idea)* Honest signing stages from protocol boundaries, no timers/percentages
  (`docs/ledger/signing-phase-guidance.md`, `lib/src/features/ledger/services/ledger_signing_progress.dart`);
  voting's named submission stages (`lib/src/features/voting/screens/mobile/mobile_voting_submission_progress_screen.dart`)
- *(idea)* Proposal list UX from ballots: jump to question, unanswered-first, auto-advance
  countdown, "Resume to complete the submission" (`lib/src/features/voting/widgets/voting_proposal_navigation.dart`,
  `voting_auto_advance_indicator.dart`, `voting_resume_plan.dart`)
- *(idea)* Local notifications as a bridge before push, deduplicated per scope and kind
  (`ios/Runner/MigrationPreparationNotificationCoordinator.swift`)
- *(idea)* Evidence-based payee states ("Confirming", "Unverified" with a reason, used after
  6 confirmations; never inferred from empty history) (`docs/gift-card-claim-outcomes.md`,
  `docs/gift-card-usage-tracking.md`)
- *(idea)* Contact names on every address surface and "New address detected. Add to contacts"
  without guessing between duplicates; basis for member names and the M2 address book
  (`lib/src/features/address_book/models/address_book_label_lookup.dart`,
  `widgets/contact_name_inline.dart`, `lib/src/features/pay/models/pay_recent_recipients.dart`)
- *(idea)* Errors across the bridge as stable codes with a "retry is pointless" flag
  (`lib/src/features/ledger/ledger_error_codes.dart`, `ledger_failure_guidance.dart`), or
  typed like voting (`lib/src/services/voting/voting_rust_exception.dart`)
- *(idea)* Send-status screen with outcome haptics: `MobileTransactionProgressScreen`
  (`lib/src/core/widgets/mobile/mobile_transaction_progress_screen.dart`) and
  `AppHaptics.sendSuccess/sendFailure` (`lib/src/core/feedback/app_haptics.dart`)
- *(idea)* Customise-account step (name + avatar, generated default name) after vault
  creation/join (`lib/src/features/onboarding/mobile/mobile_customise_account_screen.dart`,
  `onboarding/create/account_persona_generator.dart`)
- *(idea)* Small mobile fixes: Android back-exit guard (`lib/src/core/navigation/mobile_exit_back_guard.dart`)
  and iOS number-pad Done bar (`lib/src/core/widgets/mobile/mobile_numeric_keyboard_toolbar.dart`)
- *(idea)* Private transaction-detail lookups via PIR once available for Ironwood
  (`lib/src/providers/enhance_pir_provider.dart`, `rust/src/wallet/sync_engine/enhancement/`)
- *(idea)* Design review tooling: Widgetbook galleries + headless PNG renders
  (`lib/widgetbook/`, `lib/figma_compare.dart`, `scripts/figma-compare.sh`)
- *(idea)* iOS Keychain `first_unlock_this_device` for shares from day one (Vizor migrated to
  it: `lib/src/core/storage/app_secure_store.dart`, `ios/Runner/KeychainAccessibilityMigrator.swift`)
