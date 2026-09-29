# Zafe tracker

What's left, what we deferred, and ideas worth keeping. Update it whenever something is
decided, deferred, discovered or finished. Agents: add items you leave behind; don't delete
finished ones, tick them and add the commit. Spec references are to `spec.md`.

Legend: `[ ]` open · `[x]` done · **(you)** needs the user · *(idea)* not yet decided

Last updated: 2026-09-29 (after `c508aff`, M1 slice 2)

---

## Next up (proposed order)

0. [ ] **Proposals expire after ~50 min.** PCZTs are built with the library default expiry
   (`DEFAULT_TX_EXPIRY_DELTA` = 40 blocks) and `verify` allows at most 100 blocks, so
   approvals and signing must finish within about 50 minutes. Async multisig needs a long
   expiry (days; configurable per vault, D2). Anchors can go stale safely (V6: witnesses
   can be refreshed after signing, before proving). Blocks any real async use.
1. [ ] **Auto-submit when the threshold is reached.** The member whose approval completes
   the threshold (known from log order, so exactly one per proposal) starts "collect
   signatures & send" automatically. The manual button stays as the fallback. ~1 day.
2. [ ] **Push notifications** so co-signers sign without opening the app: relay `Notifier`
   → APNs/FCM, and round 2 in the iOS Notification Service Extension / Android FCM handler
   (spec V8: 12 ms, 6.8 MB fits). Needs Keychain access-group sharing on iOS.
3. [ ] **Tor** (as in Vizor `rust/src/network_privacy.rs`): `zcash_client_backend` `tor`
   feature (arti), process-wide fail-closed route policy, bootstrap timeout, dormant when
   backgrounded; route both lightwalletd and the relay (relay client moves off reqwest).
   Settings toggle + status. A full slice; deferred on 2026-09-29 as "not small".
4. [ ] **Incoming history**: home shows proposals only; received funds and a real activity
   list (Vizor activity feed, month sections) are missing.
5. [ ] **Scan invite QR** on Join (camera); today it's paste only.

## M1 — app v1 on testnet (spec §16)

Done
- [x] Flutter + FRB app on Vizor's architecture and design system (`c07c687`)
- [x] Create / join (paste) / seal / safety number / keygen, receive, balance (`c07c687`)
- [x] Payment proposals, independent check on device, approve/reject, async signing
      through the relay, send with progress (`c508aff`)
- [x] Settings, app-wide hide amounts, theme (`c508aff`)

Open
- [ ] Auto-submit (see Next up)
- [ ] Push notifications (see Next up)
- [ ] Hosted relay deployment (testnet); relay is SQLite today, Postgres for the hosted tier
- [ ] Invite by link (deep link `zafe://` / universal link) in addition to QR/paste
- [ ] Cancel a proposal (author) in the UI; the log already supports `Cancelled`
- [ ] Proposal expiry: show it, and explain/offer "propose again" when it lapses (D2)
- [ ] "Start over" for a signing round when a chosen signer never answers (today the
      leader can only retry the same round; members must re-approve for fresh nonces)
- [ ] Note reservation across concurrent proposals (`reservedNotes`, spec §9.1): two open
      proposals can pick the same notes; the second fails at broadcast
- [ ] Member names instead of hex keys (local labels, or part of the address book)
- [ ] Endpoint settings editable (today: compile-time dart-defines, read-only)
- [ ] iOS: build and run at all (only Android has been exercised)
- [ ] iOS: exclude the nonce directory from backups (`isExcludedFromBackup`)
- [ ] Biometric/passcode gate before approving and signing (spec §14)
- [ ] SQLCipher for the wallet DB (spec §14; an improvement over Vizor)
- [ ] Pre-warm the proving key when a proposal becomes Approved (2–4 s on a phone)
- [ ] Low-end device benchmark (V7 still open: Cortex-A55-class phone)
- [ ] App icon, launcher name/branding (still the Flutter default icon)
- [ ] Release build + signing config; check size (debug APK ~200 MB with 2 ABIs)

## M2 — v1 feature-complete, mainnet beta (spec §16)

- [ ] Batch payments (1..50 recipients; core supports many payments, UI is single)
- [ ] Address book via `ADDRESS_BOOK` proposals (t approvals, no FROST); warn on unknown
      recipients (§11.2)
- [ ] Rules via `RULES` proposals: per-tx / per-period limits, allowlist-only,
      large-payment extra approvals; enforced by apps before voting and signing (§11.1)
- [ ] CSV export of vault history, generated locally (§11.3)
- [ ] Encrypted backup (Argon2id + XChaCha20-Poly1305; iCloud/Drive/file) and restore;
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

## Upstream and waiting on others

- [ ] **(you)** Send the ZF questions in `upstream-asks.md` (Q1 is the mainnet gate U1)
- [ ] U1: ZF confirms the ZIP 2005 derivation stays recoverable under zips#895; get our
      `test-vectors/zip2005_use_qsk.json` cross-checked upstream
- [ ] U2: non-deprecated external-randomizer API in frost-rerandomized (frost#1094)
- [ ] U3: redpallas ciphersuite home + reddsa 0.5.x security-fix policy (frost#963)
- [ ] U4: COCKTAIL-DKG Pallas + ZIP 312 keygen spec (zips#895, frost#1033)
- [ ] thus-spoke-zakura PR #119 (Ironwood fix, targets `dev`): follow up until merged
- [ ] Zakura's faster prover once it supports Ironwood (V7)

## Known issues and tech debt

- [ ] `AppButton` label is a separate node in accessibility trees (button role is fixed;
      merge still not happening)
- [ ] Proposal `created_at` is the proposer's clock (display only, untrusted)
- [ ] Wallet DB access is serialized with one global lock in the bridge; fine for one
      vault, revisit for multiple vaults
- [ ] Sync error copy: only "can't reach the network" vs "sync failed, retrying"; no
      details screen
- [ ] Copied Vizor `lib/src/core` is from an older snapshot (`ff02152`); upstream
      (`chainapsis/vizor-wallet` @ `4bff2e7`) added tokens and button options. Resync
      deliberately, keeping Zafe's fixes (button semantics)
- [ ] No CI: add GitHub Actions for `cargo fmt/clippy/test` and `flutter analyze`
      (regtest/Docker tests stay manual or nightly)
- [ ] Agent-device flows are manual; capture them as a repeatable script
      (`scripts/app-harness.sh` covers the backend side)
- [ ] Relay: rate limiting / abuse controls for the hosted tier; retention is 30 days

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

- *(idea)* Relay as a Tor onion service (pairs with the Tor slice)
- *(idea)* Fiat values next to amounts (Vizor shows USD; needs a price source, Tor-aware)
- *(idea)* Sensitive-screen protection (Vizor's `SensitivePrivacyOverlay` + Android
  `FLAG_SECURE`) for invite, safety number and backup screens
- *(idea)* "Keep screen awake" while collecting signatures (Vizor has this setting)
- *(idea)* Show which signers are online / last seen (relay metadata; privacy trade-off)
- *(idea)* Explorer link for txids (Vizor has an explorer setting)
- *(idea)* Proposal comments/discussion thread in the encrypted log
- *(idea)* Scheduled / recurring payments (grant programs pay in milestones)
