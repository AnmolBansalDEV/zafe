# Zafe — Shielded Multisig for Zcash

**Spec version:** 1.0-draft
**Date:** 2026-09-29
**Status:** Draft for implementation. Items marked **[VERIFY]** depend on upstream behaviour that must be confirmed before the code that depends on them is written.
**Supersedes:** `spec.v0.md`. See `spec-review.md` for why it was rewritten.

---

## 1. What Zafe is

Zafe is a **Safe-style multisig for shielded ZEC**. A group of members jointly controls a shielded Zcash address. Any member can see the balance and history, propose payments, and approve them. A payment goes out only when at least **t of the N** members sign.

It is a mobile app (iOS and Android, with desktop from the same codebase), plus a small hosted relay that forwards encrypted messages between members' devices.

### 1.1 How it differs from Safe on Ethereum

Zcash's shielded pools have no scripts or contracts, so there is no on-chain multisig. Zafe uses **FROST threshold signatures** (ZIP 312). The members' devices jointly produce **one ordinary spend signature**, and the chain can't tell a Zafe vault from a single-user wallet.

| | Safe (Ethereum) | Zafe (Zcash, Ironwood) |
|---|---|---|
| Threshold enforced by | Contract, on-chain | Cryptography, off-chain (FROST) |
| Visible on-chain | Owners, threshold, all transactions | Nothing, not even that it is a multisig |
| Creating a vault | One person deploys | All N members run a key ceremony |
| Changing members | On-chain transaction | Off-chain key ceremony (§10) |
| Fully revoking a member | Remove the owner | New vault + move funds (§10.3) |
| Rules (guards, modules, limits) | Enforced on-chain | Enforced by each member's app (§11) |
| Lost keys below threshold | Funds stuck | Funds lost permanently |

### 1.2 Goals

1. Non-custodial: neither Zafe the company nor the relay can spend, and nothing short of t members' keys can.
2. Private: only members see balances, counterparties, amounts and memos. The relay and Zafe servers never get viewing keys or plaintext transaction data.
3. Mobile-first UX comparable to Zodl or Vizor for everyday actions: receive, view history, propose, approve.
4. No blind signing: every member's app independently checks exactly what a transaction does before contributing a signature.
5. Built only on audited, upstream Zcash and FROST libraries. No custom cryptography.

### 1.3 Non-goals (v1)

- Hiding members from each other or roles with less than full view access (every member sees everything; §2.3).
- Transparent-pool (t-address) multisig.
- Rules enforced on-chain (impossible in the shielded pool).
- Hardware-wallet members (Phase 3; §17).
- Custodial recovery by Zafe.

---

## 2. Concepts

### 2.1 Glossary

| Term | Meaning |
|---|---|
| **Vault** | A shielded account controlled by a FROST group: one Ironwood/Orchard receiver and one unified full viewing key (UFVK) |
| **Member** | A person with a Zafe device that holds one FROST share of a vault, plus the vault's shared secret. Member = signer; there are no other roles in v1 |
| **t / N** | Signing threshold / number of members. `2 ≤ t ≤ N ≤ 15` in v1 |
| **Identity key** | Per-device, per-member long-term keypair used to authenticate and encrypt messages. Separate from any Zcash key |
| **Vault secret `sk`** | The ZIP 2005 shared secret that every member holds. It derives `nk`, `qsk`, `qk` and `rivk` |
| **Proposal** | A request to perform a vault action (payment, address-book change, rule change, and so on) |
| **Session leader** | The member device that runs a FROST signing round for a proposal. Always a member, never the relay |
| **Relay** | Zafe-hosted, open-source store-and-forward server for encrypted envelopes and push notifications |
| **Vault log** | Hash-chained, member-signed, encrypted log of every vault event (§6.3) |

### 2.2 Keys held by each member device

| Key | Source | Shared? | Purpose | Revocable? |
|---|---|---|---|---|
| Identity signing key (Ed25519) | Random, on device | Public key shared | Signs every message and vote | Yes (rotate) |
| Identity encryption key (X25519) | Random, on device | Public key shared | Receives HPKE-sealed messages | Yes (rotate) |
| FROST key package (share of `ask`) | FROST DKG | **No**, unique per member | Spend authorization (t of N) | Yes, via share refresh (§10.2) |
| Vault secret `sk` | Joint agreement at creation (§7.4) | **Yes**, all members | Derives `nk`, `qsk`, `qk`, `rivk` per ZIP 2005 | **No** (fixed for the address) |
| UFVK (`ak`, `nk`, `rivk`) | `ak` from DKG + `sk` | Yes, all members | Scan chain, build PCZTs, verify outputs | **No** |
| Vault log key | Random, rotated on membership change | Yes, current members | Encrypts vault log entries | Yes |

### 2.3 Consequences of ZIP 2005 you can't design around

- Every member holds `sk`, so every member has **permanent full view access** to the vault: all past and future incoming and outgoing transactions. This can't be revoked without moving funds to a new vault.
- `sk` alone **cannot spend**. Spending always needs a RedPallas signature valid under `ak`, which requires t FROST shares.
- Per ZIP 2005, each member MUST protect `qsk` (derived from `sk`) as carefully as their FROST share, because it is required for quantum recovery of Ironwood funds.

---

## 3. Protocol foundation

| Reference | What Zafe uses it for |
|---|---|
| ZIP 258: NU6.3 deployment | Ironwood pool, active since block 3,428,143 (2026-07-28); Orchard is withdrawal-only |
| ZIP 229: v6 transaction format | Transaction format and sighash (includes the Ironwood digest); anchors are in authorizing data, so a transaction can be re-anchored after signing |
| ZIP 2005: Ironwood quantum recoverability | Key derivation for FROST vaults (`use_qsk = true`, shared `sk`); note plaintext lead byte `0x03` |
| ZIP 312: FROST for spend authorization | Re-randomized FROST over RedPallas; signers must verify the sighash |
| ZIP 317 | Fee calculation and signer fee checks |
| ZIP 316 | Unified addresses and UFVK encoding |
| ZIP 311 (optional) | Payment disclosures (future) |

Zafe vaults hold funds in the **Ironwood pool**. The receiver is an Orchard-type receiver in a unified address, derived per ZIP 2005.

---

## 4. Architecture

```text
 ┌──────────────── Member device (×N) ────────────────┐
 │  Flutter UI                                         │
 │    vaults · balance · history · proposals · approve │
 │                     │ flutter_rust_bridge           │
 │  zafe-core (Rust)                                   │
 │    wallet   zcash_client_backend + _sqlite          │
 │             (view-only account from vault UFVK)     │
 │    pczt     build · prove · verify · sign · extract │
 │    frost    reddsa frost::redpallas (rerandomized)  │
 │    vault    DKG · sk agreement · repair · refresh   │
 │    comms    identity keys · HPKE · vault log        │
 │    rules    signer-side rule engine                 │
 │  Secure storage (iOS Keychain / Android Keystore)   │
 └───────┬─────────────────────────────┬───────────────┘
         │ encrypted envelopes         │ compact blocks,
         │ (HTTPS / WebSocket)         │ tx broadcast (gRPC/TLS)
         ▼                             ▼
 ┌─────────────────┐            ┌───────────────┐
 │  Zafe Relay     │            │  lightwalletd │──► Zcash network
 │  (blind)        │            │  (public or   │    (Ironwood)
 │  mailboxes      │            │   self-host)  │
 │  vault log store│            └───────────────┘
 │  push (APNs/FCM)│
 └─────────────────┘
```

### 4.1 Design principle: FROST vault = external-signer account

Zodl and Vizor already support **Keystone** accounts: the phone holds only a viewing key, syncs, builds a PCZT, and an external device supplies the spend signature. A Zafe vault works the same way. The only difference is that the "external device" is a FROST round among members.

Each device therefore:

1. Imports the vault UFVK as a **view-only account** with the vault's birthday height.
2. Syncs with lightwalletd (Spend-before-Sync) to show balance and history locally.
3. Builds PCZTs for proposals from its own wallet state.
4. Contributes FROST signature shares and, as leader, injects the aggregate signature into the PCZT.

The relay never needs the UFVK. No server builds transactions.

### 4.2 Components

| Component | Tech | Holds | Trust |
|---|---|---|---|
| Zafe app | Flutter + `flutter_rust_bridge` | UI state | Trusted, on the member's device |
| zafe-core | Rust | All keys and secrets for this member; wallet DB | Security-critical |
| Relay | Rust (axum) + Postgres | Encrypted envelopes, encrypted vault log, push tokens, public identity keys | Untrusted for custody and content; sees metadata |
| lightwalletd | Existing | Nothing vault-specific | Untrusted; availability and IP-level privacy |

### 4.3 Stack decision and rationale

- **Flutter + Rust core via `flutter_rust_bridge`.** This is Vizor's architecture: one UI codebase for iOS, Android, macOS, Windows and Linux. Vizor is Apache-2.0, so its patterns and code can be reused.
- **Upstream crates directly:** `zcash_client_backend`, `zcash_client_sqlite`, `zcash_keys`, `zcash_primitives`, `pczt`, `orchard`, `reddsa` (`frost-rerandomized` / `frost::redpallas`), `frost-core`. These are MIT/Apache.
- **Pinned crate set:** see §4.4.
- **Zodl's SDKs are a reference only.** They are AGPL-3.0-only. Don't copy code from them unless the licensing is resolved.
- **ZF `frost-tools` (`zcash-sign`) and `zcash-devtool`** are reference implementations and test oracles for PCZT signing and byte-compatibility.

### 4.4 Pinned dependency set (open item V9, researched 2026-09-29)

All crates come from crates.io. **No git forks.**

| Crate | Version | Features | Why |
|---|---|---|---|
| `reddsa` | **0.5.2** (2026-05-22) | `frost` | The redpallas FROST ciphersuite, built on `frost-rerandomized` / `frost-core` **3.0.0**. Includes `dkg` and `repairable` wrappers and the `EvenY` `post_dkg` hook (even-Y normalization after DKG is automatic) |
| `frost-core` | 3.0.0 | — | Refresh (`keys::refresh`, generic over the ciphersuite), `min_signers` in packages, cheater detection |
| `frost-rerandomized` | 3.0.0 | — | `sign`, `aggregate`, `RandomizedParams::from_randomizer` (§9.5.1) |
| `orchard` | 0.15.5 | (no `unstable-frost`) | `FullViewingKey::from_bytes` (§7.4), PCZT signer `apply_signature` |
| `pczt` | 0.9.3 | `signer`, `orchard`, `prover`, `io-finalizer`, `spend-finalizer`, `tx-extractor` | Ironwood-aware PCZT roles |
| `zcash_client_backend` | 0.24.0 | `orchard`, `pczt`, `lightwalletd-tonic` | Sync (Ironwood scanning), proposals, PCZT creation |
| `zcash_client_sqlite` | 0.22.0 | `orchard` | Wallet database |
| `zcash_primitives` | 0.30.1 | — | v6 transaction and sighash |
| `zcash_keys` | 0.16.1 | `orchard`, `unstable-frost` | UFVK and UA encoding; `unstable-frost` exposes `UnifiedFullViewingKey::from_orchard_fvk` (it only enables the `orchard` dependency, not orchard's own `unstable-frost`) |

**Why `reddsa` 0.5.2 and not the alternatives:**
- `reddsa` **0.6.x** (2026-09-25) removed the `frost` feature ("the FROST ciphersuites will be moved to the `frost` repository"). As of `frost` main (2026-07-21) there's no redpallas crate there yet, and nothing like `frost-redpallas` is on crates.io.
- **frost-tools** pins `reddsa` at git `ed49e9ca` (2024-08), with `frost-core` 2.2 and `frost-rerandomized` 2.0.0-rc.0. Those predate DKG refresh and `min_signers` in `PublicKeyPackage` (added in 3.0.0-rc.0 and 3.0.0), which §10.4 depends on.
- **Compatibility:** `orchard` 0.15.5 itself depends on `reddsa` 0.5 and `pasta_curves` 0.5, the same as `reddsa` 0.5.2. Turning on `frost` doesn't add a second copy of the curve crate, so RedPallas keys, signatures and `α` pass between orchard/pczt and FROST without byte conversions (bytes are still used across the FFI and messaging boundaries).
- **Known upgrade later:** when the redpallas ciphersuite ships from the `frost` repository (probably with the `pasta_curves` 0.6 / `group` 0.14 move that `orchard` `main` has started), switch in one step together with `orchard`. Key packages are serialized with the `frost-core` 3.x encoding, so check that the move preserves serialization, or plan a migration of stored packages.

**Messaging crates (zafe-proto), found during implementation:** `ed25519-dalek` 2.2, `hpke` 0.12.0 (X25519-HKDF-SHA256, ChaCha20-Poly1305), `chacha20poly1305` 0.10 (XChaCha20-Poly1305 for the vault log). The newest generation (`ed25519-dalek` 3, `hpke` 0.14) needs stable `sha2` 0.11, which conflicts with `bip32`'s pin `sha2 =0.11.0-pre.4` pulled in by `zcash_client_backend` 0.24. The chosen generation also shares `rand_core` 0.6 with the Zcash stack. Revisit when librustzcash moves to stable `sha2` 0.11.

**Feature gotchas:** `zcash_client_backend`'s `pczt` feature enables `transparent-inputs`, so `zcash_client_sqlite` must enable `transparent-inputs` too, and `serde` for PCZT creation. Otherwise sqlite fails to compile. `zcash-devtool` documents the same constraint.

**Supply chain:** use `cargo vet` or `cargo deny` with this pinned set. Keep `Cargo.lock` checked in for the app and CLI. Security-critical crates (`reddsa`, `frost-*`, `orchard`, `pczt`) are reviewed when they change versions.

---

## 5. Identity and secure messaging

### 5.1 Identity keys

- Generated on first launch. One identity per member per vault. This avoids linking a member's vaults at the relay.
- `IdentityPublic = { sigPk: Ed25519, encPk: X25519 }`, published to the relay.
- The FROST `Identifier` for a member is `Identifier::derive(sigPk || vaultId)`, which binds the share to the identity.

### 5.2 Envelope format

Every message between members goes through the relay as an `Envelope`:

```typescript
interface Envelope {
  v: 1
  mailbox: string              // opaque relay mailbox id for this vault
  from: string                 // sender sigPk (hex)
  to: string[] | "all"         // recipient sigPks, or all current members
  seq: number                  // per-sender monotonic
  ciphertext: Uint8Array       // HPKE-sealed body (one per recipient), or
                               // AEAD under the vault log key for log entries
  sig: Uint8Array              // Ed25519 over all fields above
}
```

- Direct messages (DKG round 2, `sk` contributions, FROST signing packages and shares): **HPKE** (RFC 9180, X25519 + HKDF-SHA256 + ChaCha20-Poly1305), sealed per recipient.
- Vault log entries: XChaCha20-Poly1305 under the vault log key.
- Recipients drop any envelope whose signature fails, whose sender isn't a current member, or whose `seq` was already seen.

### 5.3 Safety number

A **safety number** is a short fingerprint over the sorted list of all members' `IdentityPublic` keys plus `vaultId`, shown as 12 digits and as a QR code. All members must confirm the same safety number **before the DKG starts** (§7.2). This is the only defence against a malicious relay impersonating members during setup.

---

## 6. Relay

### 6.1 Responsibilities

- Create vault mailboxes. Register member identity public keys (as a mailbox access list).
- Store and forward envelopes. Delete them after every recipient acknowledges, or after 30 days.
- Store the **encrypted vault log** (append-only) so new or restored devices can catch up.
- Send push notifications through APNs and FCM with **no content**: just "vault activity".
- Authenticate every request by identity-key signature. Rate-limit.

### 6.2 What the relay can see and do

| Can see | Cannot see |
|---|---|
| Mailbox ids, member public keys, IP addresses, message timing, sizes, push tokens | Balances, addresses, amounts, recipients, memos, proposals, UFVK, `sk`, shares |

| Can do (and the mitigation) | Cannot do |
|---|---|
| Censor or delay messages (members see stalled sessions and can switch relay; relay is self-hostable) | Spend |
| Withhold or reorder log entries (hash chain and member signatures detect gaps and forks) | Forge member messages or votes |
| Impersonate members during setup **if the safety number is skipped** (the UI blocks the DKG until it is confirmed) | Read content |

### 6.3 Vault log

The vault log is the shared, ordered state of the vault. Each entry:

```typescript
interface LogEntry {
  vaultId: string
  index: number
  prevHash: Uint8Array         // hash of previous entry (index-1)
  author: string               // sigPk
  body: VaultEvent             // encrypted under vault log key
  sig: Uint8Array              // author's Ed25519 over (vaultId, index, prevHash, H(body))
}

type VaultEvent =
  | { type: "VAULT_CREATED"; descriptor: VaultDescriptor; memberSigs: Record<string, Uint8Array> }
  | { type: "PROPOSAL"; proposal: Proposal }
  | { type: "VOTE"; proposalId: string; pcztHash: Uint8Array; vote: "approve" | "reject"; commitments?: Uint8Array[] }
  | { type: "PROPOSAL_CANCELLED"; proposalId: string }
  | { type: "TX_BROADCAST"; proposalId: string; txid: string }
  | { type: "MEMBERSHIP_CHANGED"; descriptor: VaultDescriptor; memberSigs: Record<string, Uint8Array> }
  | { type: "LOG_KEY_ROTATED"; epoch: number }
```

- The relay assigns `index`. Clients reject an entry if `prevHash` doesn't match their local head. On a fork, the app stops and warns: "relay inconsistency".
- Anything that changes the vault (address book, rules, membership) is valid only after it passes as a proposal with t approvals. Clients check that when replaying the log.
- FROST signing packages and shares are **not** logged. They go as direct HPKE messages.

---

## 7. Vault creation

### 7.1 Inputs

Name, network (`test` / `main`), `N`, `t`, and optional initial rules and address book. The UI warns that losing more than `N − t` members' keys permanently locks the funds, and blocks `t = N` unless the creator explicitly confirms.

### 7.2 Flow

```text
1  Creator: create vault draft on relay → mailbox + invite
2  Invite:  link (remote) or QR (in person); carries mailbox id,
            creator sigPk fingerprint, one-time join token
3  Members join, publish IdentityPublic
4  ALL members confirm the safety number
      remote: read the 12 digits over a call / trusted channel
      in person: scan each other's QR
   → DKG is disabled in the UI until every member has confirmed
5  FROST DKG (frost-core dkg part1/2/3, redpallas):
      part1 packages → broadcast via relay
      echo check: each member broadcasts H(all part1 packages it received);
                  any mismatch → abort
      part2 packages → HPKE-sealed to each recipient
      part3 → key package (own share) + group public key ak
      normalize to even Y: apply redpallas `EvenY::into_even_y` to the
      key package and public key package (ZIP 2005 requires the last bit
      of repr(ak) to be 0; orchard rejects ak otherwise). frost-client
      does the same in dkg/cli.rs
6  sk agreement (§7.4)
7  Derive UFVK from (ak, sk) per ZIP 2005; derive unified address
8  Build VaultDescriptor; every member signs H(descriptor)
   → VAULT_CREATED logged with all N signatures
9  Each device: import UFVK as view-only account, birthday = current
   tip − small margin; start sync
10 Prompt each member to make an encrypted backup (§12.2)
```

Every member's app displays the resulting address, and the address is part of the descriptor all members sign. If any device derives a different address, it refuses to sign.

### 7.3 Vault descriptor

```typescript
interface VaultDescriptor {
  vaultId: string                  // random 16 bytes
  version: 1
  name: string
  network: "test" | "main"
  threshold: number                // t
  members: {
    identity: IdentityPublic
    frostId: Uint8Array            // Identifier
    displayName: string
  }[]
  groupPublicKey: Uint8Array       // ak
  ufvk: string                     // ZIP 316 encoding
  address: string                  // unified address, Ironwood/Orchard receiver
  useQsk: true
  birthdayHeight: number
  epoch: number                    // increments on every membership change
}
```

The descriptor contains the UFVK, so it is only ever stored or sent encrypted (log key or HPKE).

### 7.4 Agreement on the vault secret `sk`

ZIP 2005 requires the participants to *privately agree* on `sk`. Zafe uses a contribution scheme so no single device's random number generator determines it:

1. Each member i generates 32 random bytes `r_i` and HPKE-seals `r_i` to every other member.
2. `sk = BLAKE2b-256("Zafe_vault_sk" || vaultId || H(DKG transcript) || r_1 || … || r_N)`, with contributions ordered by `frostId`.
3. Derive the key components (ZIP 2005 § "Changes to the Protocol Specification", § 4.2.3, `use_qsk = true`):

   ```text
   nk       = ToBase^Orchard( PRF^expand_sk([0x07]) )
   qsk      = truncate_32( PRF^expand_sk([0x0C]) )
   qk       = BLAKE3.derive_key("Zcash ZIP 2005 qk-derivation v1", qsk, 32)
   rivk_ext = ToScalar^Orchard( PRF^expand_qk([0x0D] || I2LEOSP_256(ak) || I2LEOSP_256(nk)) )
   ```

   `PRF^expand_k(t) = BLAKE2b-512("Zcash_ExpandSeed", k || t)`. `sk` is 32 bytes, and `H^ask(sk)` is **not** used (`ask` is the FROST-shared key). Internal (change) keys come from `rivk_ext` through the standard derivation (`H^rivk_int`, `0x83`), which `orchard` already implements.
4. Build the FVK with `orchard::keys::FullViewingKey::from_bytes(ak || nk || rivk_ext)`. This is public in `orchard` 0.15.5 and validates all three components. Wrap it as a UFVK with `zcash_keys::keys::UnifiedFullViewingKey::from_orchard_fvk` (`zcash_keys` feature `unstable-frost`), with no Sapling or transparent components.
5. Every member checks that the resulting UFVK and address match the ones all members sign in the descriptor (§7.2 step 8).

#### 7.4.1 Upstream status (open item V1, researched 2026-09-29)

- **No upstream library implements ZIP 2005 FROST key derivation yet.** `orchard` 0.15.5 on crates.io and `main` (as of 2026-09-28) have no `qsk`/`use_qsk` code. The `unstable-frost` feature only makes `SpendValidatingKey::{to_bytes, from_bytes}` public.
- **frost-tools derives FROST viewing keys the old way.** `zcash-sign/src/generate.rs` uses `FullViewingKey::from_sk_ak_incompatible_with_quantum_recoverability_and_will_be_removed`, from an `orchard` fork (`conradoplg/orchard@42015f1`, the not-yet-merged orchard#475). The comment says ZF "directed us to keep this non-quantum-recoverable constructor for now (frost-tools#591) … Quantum recoverability is deferred." It sets `nk` and `rivk` from `sk` the old way (`rivk = H^rivk(sk)`) with no `qsk`. **Vaults created that way (including by Konclave and other tools built on frost-tools) are spendable today but not recoverable under ZIP 2005.** The Recovery Protocol would check `ak` against `H^ask(sk)`, which doesn't match a DKG-generated `ak`. frost-tools#591 was closed on 2026-08-15 after Ironwood signing merged, but it kept this non-recoverable derivation.
- **ZF has confirmed this is blocked upstream** (frost#1094, reply from conradoplg on 2026-09-21): FVK derivation for FROST under ZIP 2005 *"is blocked on zcash/zips#895 being finished and integrated with ZIP 2005, and then implemented. Yes, if you use `from_sk_ak_incompatible_with_quantum_recoverability_and_will_be_removed` you are giving up on quantum recoverability and would need to migrate to a new wallet then that is supported."* zips#895 ("[ZIP 312] Specify key generation, change randomizer handling", open, last updated 2026-08-28) still lists as missing "how to agree on `sk` in the DKG setting" and whether to adopt COCKTAIL-DKG (§7.5).
- **What this means for Zafe:** the derivations *from* `sk` (`nk`, `qsk`, `qk`, `rivk_ext`) are already normative in ZIP 2005 § 4.2.3, which is deployed with NU6.3. What's still open is *how participants agree on `sk`* and *how the DKG is run*. Zafe's reading is that the Recovery Protocol checks `rivk_ext = H^rivk_ext_qk(ak, nk)`, `qk = H^qk(qsk)`, `nk = H^nk(sk)` and a signature under `ak`, and never checks how `sk` was agreed. If so, a vault built as in §7.4 steps 1–4 is recoverable whatever agreement method #895 eventually standardizes. **ZF hasn't confirmed this reading yet** (question Q1 in `upstream-asks.md`). Until they do, Zafe runs on **testnet only** with this derivation, and mainnet vaults wait for confirmation (M2 gate).
- **What Zafe does:** implement the four derivations above in `zafe-core` (about 40 lines using `blake2b_simd`, `blake3` and `pasta_curves`), then build the FVK with the public `from_bytes`. This is the one place Zafe writes key-derivation code itself, so:
  - **Check the shared building blocks against `orchard`:** for random `sk`, compute `H^nk(sk)` and the legacy `H^rivk(sk)` with Zafe's code and confirm they give byte-identical results to `orchard::keys::FullViewingKey::from(&SpendingKey::from_bytes(sk))`. That confirms the `PRF^expand`, `ToBase` and `ToScalar` code.
  - **The new parts** (`0x0C`, BLAKE3 `qk`, `0x0D` `rivk_ext`) have no official test vectors. Write vectors, propose them to `zcash-test-vectors`, and ask ZF/ECC to cross-check (see frost-tools#591 and orchard#475).
  - **Switch to the upstream API** once `orchard` (or `zcash_keys`) ships a ZIP 2005 constructor. Replace Zafe's code and keep the vectors as regression tests.
- **ZIP 326 wallet rules for `use_qsk = true` accounts** (vault birthday is after NU6.3):
  - Never scan the Orchard pool for the vault; scan only the Ironwood pool, from the birthday height.
  - Never send Orchard-pool funds to the vault.
  - Keep `use_qsk = true` for every key in the account.
  - Record `useQsk: true` in the descriptor and backup. ZIP 2005 says recovery requires knowing it.
- **Form of `sk`:** 32 bytes (the Orchard spending-key length). ZIP 2005 places no other constraint on it for the `use_qsk` path. The contribution hash in step 2 produces 32 uniformly distributed bytes. Don't pass it through `orchard::SpendingKey::from_bytes`, whose rejection of `H^ask(sk) = 0` doesn't apply here.

### 7.5 Future standard DKG: COCKTAIL-DKG

- **COCKTAIL-DKG** (c2sp.org/cocktail-dkg, v0.2.1) is a standalone 3-round DKG for FROST, derived from ChillDKG. It defines a **Pallas / BLAKE2b-512** ciphersuite. Its design matches Zafe's threat model:
  - it treats the coordinator as *"an untrusted facilitator"*
  - it encrypts shares pairwise with ECDH, so the coordinator can relay them
  - its final CertEq round has all participants sign a canonical transcript, which catches a coordinator showing different messages to different participants
  - it identifies which participant misbehaved
- **Status:** zips#895 is weighing whether to specify FROST key generation for Zcash on top of it (nuttycom, 2026-08-24: v0.2 "has now landed … can now be reassessed"). ZF's production implementation (frost#1033) is waiting on C2SP issue #216 and the spec being finalized. There's a WIP PR (frost#1032). **No production Rust crate exists yet.**
- **Zafe v1 decision:** use `frost-core` 3.0.0 DKG (`reddsa` 0.5.2 redpallas `dkg`) with Zafe's own versions of COCKTAIL's protections: HPKE-sealed round 2 (§5.2), the echo check, and all-member transcript signatures (§7.2 steps 5 and 8). Keep the DKG code behind a `VaultKeygen` trait so it can switch to COCKTAIL-DKG, plus the standardized `sk` agreement from zips#895, once a production crate exists. A switch only affects **new** vaults. Existing vaults keep their keys.
- The DKG choice doesn't change the key material a vault ends up with (`ak`, shares, `sk`). It only changes how that material is agreed. So existing vaults stay valid.

---

## 8. Balance and history

- Each device syncs independently from lightwalletd using `zcash_client_backend` (Spend-before-Sync) into a local `zcash_client_sqlite` database.
- **Account type:** the vault UFVK is imported with `AccountPurpose::Spending { derivation: None }`, as Zodl and Vizor do for Keystone accounts. A `ViewOnly` account does not track note witnesses, so it could never build a spend. No spending key is stored; spend authorization always comes from FROST.
- **Birthday height ≥ 2:** sync downloads the chain state at (scan range start − 1), and lightwalletd reads height 0 as "unspecified". This only matters on regtest.
- **Proposals:** built with `propose_transfer` and `create_pczt_from_proposal`, using change to the Ironwood pool and `OvkPolicy::Sender` (the vault's external OVK for payments, internal for change), which §9.3 verification requires.
- Balance, notes, and incoming and outgoing history (with memos) come from the local database, never from the relay.
- The history UI merges chain data with vault log data: who proposed a payment, who approved it, labels.
- lightwalletd sees the device IP and broadcast transactions, but not which notes belong to the vault. Zcash community guidance for Ironwood-era light clients recommends network-layer privacy (Tor or Nym), randomizing the start height, and randomizing broadcast delays. These are planned for M3, with Tor first.

### 8.1 Light-client infrastructure (open item V4, researched 2026-09-29)

| Component | Ironwood status | Notes |
|---|---|---|
| `zcash/lightwalletd` | **Supported since v0.5.x** (PR #567, merged 2026-07-21); latest v0.5.4 (2026-08-27) | Parses v6 transactions, compact blocks, tree states, subtree roots, mempool, `poolTypes` filtering. `CompactTx.actions` was renamed `orchardActions` (same field number); Ironwood actions are field 9. **Requires an NU6.3 backend (Zebra ≥ 6.0.0).** An older lightwalletd stalls at the first v6 transaction. Reports `lightwalletProtocolVersion` in `GetLightdInfo` (PR #589) |
| `zcash_client_backend` 0.24.0 / `zcash_client_sqlite` 0.22.0 (2026-08-19) | **Scans Ironwood** (`IronwoodDomain` in `scanning/compact.rs` and `scanning/full.rs`) | `import_account_ufvk` exists for view-only accounts, which is the Keystone-style pattern in §4.1 |
| `pczt` 0.9.3 (2026-08-07) | **Ironwood signing supported** | `roles::low_level_signer::Signer::{sign_ironwood_with, sign_orchard_with}` (this closes the rest of V3) |
| Zebra 6.3+ | Built-in lightwalletd-compatible server (off by default) | Lets a self-hosted relay tier run one binary instead of Zebra plus lightwalletd |
| Zaino 0.10.0 (Z3 stack indexer) | Not verified | Possible future backend |
| Public mainnet endpoints | **Not confirmed** | zec.rocks ran an Ironwood-compatible branch on testnet before activation. No official list of Ironwood-ready mainnet servers was found |

**Zafe policy:**
- **Check the server on connect.** Call `GetLightdInfo` and refuse servers that don't report a consensus branch ID at or after NU6.3 for the current height, or that report a light-wallet protocol version older than the Ironwood one. Surface this as "server out of date", not as a sync error.
- **Default endpoints.** Zafe runs its own Zebra ≥ 6.x + lightwalletd ≥ 0.5.4 pair (it can share infrastructure with the relay, but it's a separate service) as the default, with zec.rocks and other community servers as fallbacks, and allows custom endpoints (as Vizor does).
- **Cross-check the chain tip.** Optionally compare the tip and tree state against a second server, to catch a lying server (§13.1).
- **Scanning** follows ZIP 326: Ironwood pool only, from the vault birthday height (§7.4.1).

---

## 9. Proposals and signing

### 9.1 Proposal types (v1)

```typescript
type ProposalAction =
  | { kind: "PAYMENT"; payments: Payment[] }           // 1..50 recipients (batch)
  | { kind: "ADDRESS_BOOK"; add: Contact[]; remove: string[] }
  | { kind: "RULES"; rules: VaultRules }
  | { kind: "MEMBERSHIP"; change: MembershipChange }   // §10
  | { kind: "MIGRATE"; toVault: VaultDescriptor }      // §10.3

interface Payment {
  address: string       // unified / Orchard-receiver address; transparent
                        // and TEX recipients allowed with a warning, no memo
  amountZat: bigint     // zatoshis
  memo?: Uint8Array     // ≤ 512 bytes, shielded recipients only
  label?: string        // local label, never on-chain
}

interface Proposal {
  id: string            // random 16 bytes
  vaultId: string
  epoch: number
  author: string        // sigPk
  action: ProposalAction
  pczt?: Uint8Array     // PAYMENT / MIGRATE only
  pcztHash?: Uint8Array
  reservedNotes?: Uint8Array[]   // nullifiers of selected notes
  createdAt: number
  expiryHeight?: number
}
```

Proposals that don't move funds (address book, rules) need t approvals and **no** FROST signature. The approvals are the authorization, and every client checks them when replaying the log.

### 9.2 Building a payment proposal (on the author's device)

1. Check the payments against rules and the address book (§11) and warn or block.
2. Select notes from the local wallet, **excluding notes reserved** by other open proposals.
3. `propose_transfer`, then `create_pczt_from_proposal` (Creator, Constructor, IO Finalizer), then add proofs (Prover, Halo 2 on device).
4. Publish a `PROPOSAL` log entry with the PCZT, `pcztHash` and `reservedNotes`.

**Note reservation:** every client treats notes listed in open proposals as unavailable. If two proposals reserve the same note, the one with the lower log index wins and the other is automatically marked `conflicted`.

### 9.3 Independent verification (every member, before voting)

Each member's app parses the PCZT locally and refuses to show an Approve button unless **all** of these pass:

1. **Network and version:** v6 transaction, correct consensus branch ID for the current height, network matches the vault.
2. **Spends:** every spend that needs a signature belongs to this vault (the spend's `ak` equals the vault's `groupPublicKey`, and the nullifier derives from the vault's `nk`). For each spend, `rk == ak + [α]·G`, where `α` is the spend's randomizer from the PCZT.
3. **Outputs:** every output is exactly one of:
   - a payment in the proposal (same address, amount and memo). **Zafe builds every payment output with the vault's external outgoing viewing key**, and members recover it with that key. Recovery decrypts the real ciphertext and checks it against the note commitment, so the compared recipient, amount and memo are what the recipient actually receives. An output the vault can't recover is rejected.
   - change to **this vault's own address**. The plaintext recipient must belong to the vault, **and** the output must trial-decrypt with the vault's incoming viewing key for that scope. Otherwise a valid commitment with a garbage ciphertext would silently burn the change.
   - a zero-value output (padding), which moves no funds.
   Any other output means rejection. Spends must each be either a vault note (nullifier and `rk` checked against the vault viewing key) or a zero-value padding spend already signed by the IO Finalizer. In v1, transparent, Sapling and Orchard-pool components are rejected outright.
4. **Value:** inputs − outputs = fee, and the fee must **equal** the ZIP 317 conventional fee (5,000 zat × max(2, number of actions)). No tolerance: overpaying the fee is a way to burn funds.
5. **Sighash:** compute the v6 sighash locally from the PCZT, and use only this value as the FROST message.
6. **Rules** (§11) pass, or the proposal carries a rule-override flag that the rules themselves allow.
7. **Hash:** `pcztHash` matches the PCZT bytes.

The approval screen shows every recipient, amount, memo, fee, the change amount, and the address-book name or an "unknown address" warning. Only after this does the member choose Approve or Reject.

### 9.4 Voting and FROST round 1

- **Approve** = a signed `VOTE(approve)` on the log **plus FROST round-1 commitments**, one per spend that needs a signature, generated at that moment.
- **Reject** = a signed `VOTE(reject)`. A proposal fails when `N − t + 1` members reject.
- Secret nonces are held in secure storage marked **this-device-only and excluded from backups** (iOS `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`; Android Keystore-wrapped files in a no-backup directory). Each nonce is keyed by `(proposalId, pcztHash, spendIndex)`.
- **Nonce rule:** a nonce produces at most one signature share. It is deleted *before* the share is sent. A member never produces two shares with the same nonce, under any circumstances.

This makes approval asynchronous. Members approve whenever they open the notification, and the signing round needs them again only for round 2.

### 9.5 Signing round 2

1. **Session leader** = the proposal author. If the author hasn't acted within a timeout (default 1 hour after t approvals), any approving member may take over.
2. The leader picks t approvers with unused commitments. For each spend it builds the signing package (commitments plus the sighash as the message), with the randomizer set to that spend's `α` from the PCZT (§9.5.1). It sends the packages by HPKE to those t members.
3. Each of the t members runs the §9.3 checks again. It checks that the package's message equals its locally computed sighash and that each randomizer equals the corresponding spend's `α` in the PCZT. Then it signs with `frost_rerandomized::sign(signing_package, nonces, key_package, Randomizer::from_scalar(α))`, deletes the nonce, and sends the signature share.
4. The leader verifies each share individually (FROST cheater identification) and aggregates with `RandomizedParams::from_randomizer(ak, α)`. It injects one signature per spend with the pczt `Signer` (`sign_ironwood_with` → `action.apply_signature(sighash, sig)`, which checks the signature against the spend's `rk`). Then it runs the Spend Finalizer, extracts the transaction, broadcasts it through lightwalletd, and logs `TX_BROADCAST`.
5. If a member in the chosen set doesn't respond within a timeout, the leader starts a new session with a different set of approvers whose commitments are still unused.

**Spends to sign:** every action in the Ironwood bundle (and, in v6, the Orchard bundle) where `spend().spend_auth_sig()` is `None`. Don't filter by value. The IO Finalizer already signs true dummy spends during PCZT creation, so anything still unsigned belongs to the vault and needs a FROST signature, including zero-value spends the wallet controls. Konclave had a bug from filtering by value. Each pool is signed through its own `Signer` entry point, and asking the wrong one returns no spends rather than an error, so track `(pool, actionIndex)`. This mirrors `zcash-sign/src/sign.rs` in frost-tools.

#### 9.5.1 Randomizer: use the PCZT's `α` (resolved from open item V2)

**Decision:** the FROST randomizer for each spend is the `α` that the Constructor put in the PCZT when the proposal was built. Neither the leader nor the relay generates a new one.

**Why this is necessary:** `α` determines `rk = ak + [α]G`. `rk` is part of the transaction's effecting data, so it feeds the sighash, which is the FROST message. It is also a secret input to the Halo 2 proof. So `α` has to be fixed before the message exists, and before round 1. ZIP 312's `randomizer_generate(msg, commitments)` (hash fresh randomness with the message and the commitments after round 1) can't be applied as written, because the message already depends on `α`.

**Why it is secure:**
- **Unforgeability:** the Re-Randomized FROST paper (Gouvêa & Komlo, ePrint 2024/436) proves unforgeability in a game where *"signatures are allowed to be generated under an adversarially-chosen randomizer"*. The adversary supplies `α` in each round-2 signing query. A randomizer chosen by the proposer, even a malicious one, and fixed before round 1 is covered by that proof. It can't be used to forge.
- **Privacy / unlinkability:** this depends on `α` being uniformly random. The paper and ZIP 312 both put the coordinator (the transaction builder) in charge of the transaction's privacy: *"the coordinator is trusted with the privacy of the signature … but they still can't forge signatures."* In Zafe the proposer builds the PCZT and already sees everything, and all members are trusted with privacy anyway (§2.3). So a proposer who biases `α` only harms privacy they already control.
- **What hashing in the commitments is for:** the paper and the `frost-rerandomized` docs describe binding the randomizer to the message and commitments as a hedge. It keeps the randomizer unique and uniform "even in case of random number generator failure". It isn't needed for unforgeability. Zafe relies on the PCZT builder's CSPRNG for `α` (the `orchard` builder samples it).

**ZF agrees:** zips#895 changes ZIP 312's randomizer handling *"after I realized that it couldn't possibly be generated using the message as an input"* (conradoplg). In frost#1094 (2026-09-21): *"there will always be a mechanism for feeding an external randomizer since it is required for Zcash."* **API caveat:** in `frost-rerandomized` 3.0.0, the only public way to sign with an externally supplied randomizer is `sign()`, and it is `#[deprecated]` (it points to `sign_with_randomizer_seed`, which can't take a fixed `α`). The `Randomize` trait is private. Zafe calls `sign()` with `#[allow(deprecated)]` in one wrapper function, and switches when ZF either removes the deprecation or adds `sign_with_randomizer` (they've said they might remove it).

**Upstream precedent:** frost-tools does exactly this. `zcash-sign` reads `action.spend().alpha()` for every unsigned spend and prints it as "Randomizer #idx". The operator passes it to `frost-client coordinator --randomizer`. The coordinator then uses it verbatim (`RandomizedParams::from_randomizer`) instead of generating one (`coordinator/round_2.rs`). The `frost-rerandomized` API documents `from_randomizer` / `Randomizer::from_scalar` as intended *"for compatibility reasons with specifications on how the randomizer must be generated"*, and requires the value to be uniformly random.

**What Zafe adds beyond frost-tools:**
1. **Members check the randomizer.** In frost-client, participants receive the message and randomizer from the coordinator and sign without seeing the PCZT. In Zafe every member recomputes the sighash and reads `α` from the PCZT itself, and refuses on any mismatch (§9.3). A wrong `α` can't forge, but it would waste the round or break unlinkability. A wrong sighash is the actual attack.
2. **One session covers many spends.** frost-client handles one message and one randomizer per session (`signing_package.first()`, `randomizer[0]`). Zafe sends one signing package and one randomizer per spend in a single round-2 message, with separate nonces for each spend.
3. **The randomizer travels over HPKE.** ZIP 312 requires a confidential channel for it. In Zafe it's inside the PCZT that every member already holds, and it's never sent unencrypted through the relay.

### 9.6 Proposal status

```text
draft → open → approved (≥ t approvals) → signing → broadcast → mined → confirmed(10)
side exits: rejected | cancelled (by author, before signing) | conflicted
          | expired (expiryHeight passed) | failed (with reason)
```

- **Re-anchoring:** under ZIP 229, anchors are authorizing data, so a proposal whose anchor has gone stale can be re-anchored and re-proved without invalidating the sighash or the collected signatures. **[VERIFY]** the API for this in `pczt` / librustzcash (issue #2529).
- `expiryHeight` defaults to about 7 days of blocks. After that, the proposal must be rebuilt.

---

## 10. Membership changes

Changing members depends on the group, so Zafe offers two operations and explains the trade-off in the UI.

### 10.1 Replace a lost device (same member)

The member keeps their seat. Two ways back in (§12): restore from an encrypted backup, or **repair** (§10.4.2), where at least t other members regenerate the member's share on the new device. For repair, the new device gets a new identity key, which is announced by a `MEMBERSHIP` proposal needing t approvals. Then one member HPKE-sends `sk`, the descriptor and the log key.

**The FROST identifier stays the same** (repair recreates the share at the member's existing identifier), even though the identity key changes. So in this one case the `Identifier` is not re-derived from the new `sigPk` (an exception to §5.1). The descriptor records the mapping.

### 10.2 Rotate (keep the vault address)

Use this for add, remove or replace when every departing member is trusted to delete their data.

| Operation | FROST mechanism | Who must be online | Result |
|---|---|---|---|
| **Add** member | Repair toward a **new** identifier (§10.4.2) | ≥ t existing members (the helpers) + the new member | N+1, same t, same `ak` and address |
| **Remove** member | DKG refresh among the remaining members, excluding the removed one (§10.4.1) | **All** remaining members | N−1, same t (requires N−1 ≥ t), same `ak` and address |
| **Replace** (remove X, add Y) | Remove first, then add | All remaining members, then ≥ t of them + Y | N, same t |
| **Change t** | **Not supported.** `frost-core` rejects a changed `min_signers` (`Error::InvalidMinSigners`) | — | Use **Migrate** (§10.3) |
| **Proactive refresh** (no membership change) | DKG refresh among all members | All members | Same set; old shares made useless once deleted |

Order matters for **Replace**: refresh first, then repair from the refreshed shares. Repairing first would give Y a share on the old polynomial that X's old share still combines with.

Every operation starts as a `MEMBERSHIP` proposal with t approvals, and finishes with a new `VaultDescriptor` (`epoch + 1`) signed by all members of the new set and logged as `MEMBERSHIP_CHANGED`. The log key is rotated on every removal.

**What rotation does NOT do** (the UI shows this before confirming). The FROST book puts it this way: *"the Refresh Shares functionality does not 'restore full security' to a group … the security of the group [depends] on the threshold of all previous set of participants being honest."*
- A removed member **keeps full view access**: they hold `sk` and the UFVK, so they can see all past and future vault activity.
- A removed member keeps `qsk`.
- A removed member's old share still combines with any other member's **pre-refresh** share. If a remaining member kept or backed up an old share, the removed member plus that member (t in total, for t = 2) can spend. Only migration removes this risk.

### 10.4 FROST mechanics (open item V5, researched 2026-09-29 against `frost-core` 3.0.0)

#### 10.4.1 Refresh via DKG (`frost_core::keys::refresh`)

- Uses `refresh_dkg_part1` → `refresh_dkg_part2` → `refresh_dkg_shares`. It has the same 3-part shape and channel requirements as the DKG: part 1 is broadcast, part 2 is sealed point-to-point, and senders are authenticated. The same echo check and HPKE rules from §7.2 apply.
- `max_signers` can be **smaller** than the original, as long as it stays ≥ `min_signers`. Leaving an identifier out removes that participant. `min_signers` must equal the original.
- The group verifying key (`ak`) is unchanged. Each participant's verifying share changes.
- Refresh with a trusted dealer (`compute_refreshing_shares`) also exists. Zafe doesn't use it, because a dealer would briefly be a single point of trust.
- **Delete old shares only after confirming it worked.** The FROST book: *"Applications should first ensure that all participants who refreshed their KeyPackages were actually able to do so successfully, before deleting their old KeyPackages … it might require successfully generating a signature with all of those participants."* So Zafe keeps both old and new key packages until a **test signature** succeeds. The test is a FROST signature over `"Zafe refresh check" || vaultId || epoch` by every member of the new set, in groups of t. Only then does each device delete its old package and attest to the deletion in the log. Backups are re-made afterwards, because old backups contain old shares (§12.2).
- **Refresh can only shrink the group.** It never adds identifiers. Adding a member is a repair (§10.4.2).
- `PublicKeyPackage` and `KeyPackage` must carry `min_signers`, which is set automatically for packages created with `frost-core` ≥ 3.0.0. Zafe vaults are always created on 3.x (§19 V9).

#### 10.4.2 Repair (`frost_core::keys::repairable`, RTS from ePrint 2017/1155)

- **Helpers:** at least `min_signers` existing members (`repair_share_part1` returns an error with fewer).
- **Rounds:**
  1. Each helper sends a `delta` to every other helper (sealed point-to-point).
  2. Each helper sums what it received into a `sigma` and seals it to the member being repaired.
  3. The member being repaired sums the sigmas (`repair_share_part3`) and gets a `KeyPackage` at the target identifier.
- **It works for a new identifier**, which is how adding a member works. The FROST book describes removing a member with refresh (2-of-3 → 2-of-2) and then using repair "to issue a new share and move from 2-of-2 back to 2-of-3".
- **`repair_share_part3` doesn't check the result.** It sums whatever sigmas it receives, so one bad helper can give the member a wrong share. That can't cause theft, but it makes the member useless without anyone noticing. Zafe adds these checks:
  - **Lost-device repair:** the recovered `verifying_share` must equal the member's verifying share in the stored `PublicKeyPackage`.
  - **New member:** the new member publishes its `verifying_share`, and every member adds it to its `PublicKeyPackage`. The test signature from §10.4.1 is then run with the new member plus t−1 others before the change counts as complete. If it fails, the operation is aborted and the helpers are asked to redo it, or cheater identification during the test signature points to the bad share.
- Repair doesn't need or change `sk`. The new member receives `sk` separately (HPKE, §7.4).
- The redpallas even-Y form is preserved: refresh adds shares of a zero polynomial and repair reconstructs points on the existing polynomial, so `ak` and its sign never change. `reddsa` 0.5.2's `post_dkg` hook only runs in the initial DKG, and nothing more is needed afterwards.

#### 10.4.3 Operational constraints

- **Removal needs every remaining member online** (for the refresh). If a remaining member is unreachable, the choice is: wait, remove that member too (if t still fits), or migrate.
- **Refresh and repair messages are asynchronous.** They go through the relay mailboxes like the DKG. Each part only needs the others' messages from the previous part, so members can do their part whenever they open the app. A session times out after 7 days and restarts.
- Signing proposals pause while a refresh is running, because shares are changing.

### 10.3 Migrate (new vault)

Use this for hostile removal, changing t, or when rotation isn't supported.

1. The new member set creates a new vault (§7).
2. A `MIGRATE` proposal in the old vault moves all funds to the new vault's address. It needs t approvals from the old members, may take several transactions if there are many notes, and costs fees.
3. The old vault is marked `archived`. Its history stays viewable.
4. Payers must be told the new address. The UI provides a "share new address" step.

---

## 11. Rules and address book (enforced by members' apps)

### 11.1 Rules

```typescript
interface VaultRules {
  maxPerTxZat?: bigint
  maxPerPeriodZat?: { amount: bigint; periodDays: number }
  allowlistOnly?: boolean               // only address-book recipients
  largePayment?: { overZat: bigint; approvals: number }  // needs k ≥ t approvals
}
```

- Rules are part of the vault state, set at creation or by a `RULES` proposal with t approvals.
- **Every member's app enforces the rules before voting and before signing in round 2.** Period limits are computed from the vault's own chain history, which every member can see.
- `largePayment.approvals` can require **more** than t approvals. That's enforced by the apps: an honest member's app won't produce a share until k approvals are logged.
- **Limitation (show it in the UI):** rules are not on-chain. Any t members acting together can bypass them, for example with modified apps. Rules protect against mistakes, a compromised relay, and a single rogue member, not against t colluding members.

### 11.2 Address book

- `Contact = { name, address, addedBy, addedAt }`. It is changed only by `ADDRESS_BOOK` proposals with t approvals.
- Payments to addresses outside the address book show a prominent warning, or are blocked when `allowlistOnly` is set.

### 11.3 Export

A CSV export of the vault history. Columns: date, txid, direction, counterparty (address and contact name), amount, fee, memo, proposal id, proposer, approvers. It is generated locally from the wallet database and the vault log, never on a server.

---

## 12. Recovery and backup

### 12.1 Threshold reality

- If up to `N − t` members lose their keys, the vault is still fully usable, and lost seats can be repaired (§10.1).
- If more than `N − t` members lose their keys, the funds are **permanently lost**. Zafe can't help. The app shows this when a vault is created and whenever a member's backup status is missing.

### 12.2 Encrypted backup

- Contents: identity keys, FROST key package, `sk`, the vault descriptor, the log key, and `useQsk = true`. **Never** FROST nonces.
- Encryption: a passphrase run through Argon2id (at least 64 MiB, 3 iterations), then XChaCha20-Poly1305. The passphrase must be at least 12 words or pass a zxcvbn score of 4 or more.
- Destinations: iCloud Drive, Google Drive, or a file export.
- The app tracks who has a verified backup (a member attests to it in the log) and shows the vault's "backup health".
- Warning shown when backing up: the backup plus the passphrase give that member's full capability.

### 12.3 Restore

Restore the backup, then sync the vault log from the relay, then resync the wallet from the birthday height. If the log key has rotated since the backup was made, another member must send the current key.

---

## 13. Security model

### 13.1 Guarantees

| Threat | Result |
|---|---|
| Relay compromised | Can censor, delay and analyse metadata. **Cannot** spend, read content, or forge votes. Setup impersonation is blocked by the safety-number check |
| lightwalletd compromised | Can hide or delay chain data and see IP addresses. Can't spend. A wrong balance can't cause a bad signature, because members verify the PCZT itself |
| Fewer than t members compromised | Cannot spend. Can see everything (they hold `sk`) |
| t or more members compromised | Can spend. Rules can be bypassed. This is inherent |
| Author's device compromised | Can propose malicious PCZTs. Other members' checks in §9.3 catch any mismatch with what they are shown |
| Zafe company | Operates only the relay. Same as "relay compromised" |

### 13.2 Assumptions

- Members actually compare safety numbers during setup.
- Members' devices and OS secure storage are not compromised.
- Upstream libraries (`frost-core`, `reddsa`, `orchard`, `pczt`, librustzcash) are correct.
- In a rotation, departing members delete their shares (only migration removes this assumption).

### 13.3 What is impossible (tell users plainly)

- On-chain rules, timelocks, or recovery modules.
- Revoking a former member's view access without migrating.
- Recovering funds with fewer than t keys.
- Showing the public or DAO non-members spending totals without giving someone a viewing key or per-payment disclosures.

---

## 14. Data storage

| Location | Data | Protection |
|---|---|---|
| Device secure storage | Identity keys, FROST key package, `sk`, log key, FROST nonces (this-device-only) | Keychain / Keystore, biometric or passcode gate for signing |
| Device app database | Wallet database (notes, transactions), decrypted vault log cache, descriptors | SQLCipher, key in Keychain/Keystore |
| Relay (Postgres) | Mailboxes, public identity keys, encrypted envelopes (TTL), encrypted vault log, push tokens | Server-side encryption at rest; no content keys |

---

## 15. Repository layout

```text
zafe/
├── app/                    # Flutter app (iOS, Android, desktop)
│   ├── lib/
│   └── rust_bridge/        # flutter_rust_bridge glue
├── crates/
│   ├── zafe-core/          # wallet, pczt, frost, vault, comms, rules
│   ├── zafe-relay/         # relay server (axum)
│   ├── zafe-proto/         # envelope + vault log types (serde, versioned)
│   └── zafe-cli/           # headless member for testing and CI
├── infra/                  # relay deploy (Docker), Postgres migrations
├── tests/
│   └── e2e/                # multi-member scenarios on testnet / regtest
└── docs/
    ├── spec.md
    ├── threat-model.md
    └── protocol.md
```

`zafe-cli` is a full member without a UI. It makes end-to-end tests possible (three CLI members run a 2-of-3 vault in CI) and serves as the first thing built (§16, M0).

---

## 16. Milestones

**M0: Core protocol (CLI, testnet)**
*Status 2026-09-29: **complete.*** `scripts/m0-e2e.sh` runs three separate `zafe` CLI members through the relay: invite and join, matching safety numbers, key generation over sealed envelopes, vault funded on Ironwood regtest (`infra/regtest/`), payment proposed, independently verified and approved, FROST-signed, broadcast and logged. Library-level tests: 51 plus the Docker-gated `regtest_e2e`. **Cross-checked against frost-tools `zcash-sign`** (`tests/zcash_sign_crosscheck.rs`): on zcash-sign's real testnet Ironwood fixture, Zafe reproduces the on-chain sighash; on a Zafe PCZT, zcash-sign prints the same sighash and randomizers, accepts Zafe's FROST signatures, and writes byte-identical spend-auth signatures. The relay is still in-memory, which is fine for M0.
DKG + safety number + `sk` agreement + UFVK/address → sync → PCZT → FROST signing for all spends → broadcast. Three `zafe-cli` members running over a local relay. Resolve every [VERIFY] item. Cross-check signatures and transactions against `zcash-sign` / `zcash-devtool`.

**M1: App v1 (testnet)**
Flutter app with vault creation (invite link and QR), receive, balance, history, payment proposals, independent verification screen, approve/reject, async signing, push notifications, hosted relay.

**M2: v1 feature-complete (mainnet beta)**
Batch payments, address book, rules, CSV export, encrypted backup and restore, repair, backup health. External security audit of zafe-core and the protocol before mainnet funds. **Mainnet gate:** open item U1 resolved (ZF confirms the key derivation gives recoverable vaults).

**M3: Membership**
Rotation (refresh-based, per [VERIFY]), migration flow, desktop builds.

**M4: Platform**
Self-hostable relay packaging and paid hosted tiers, dapp SDK (proposal requests from third-party apps), hardware members (Keystone/Ledger FROST support permitting), payment disclosures.

---

## 17. Business model

- **Open source:** the app, zafe-core and the relay, under Apache-2.0 (or MIT/Apache dual). No AGPL code from the Zodl SDKs.
- **Revenue:** a hosted relay with a free tier (for example up to 3 vaults and N ≤ 5), paid tiers (larger vaults, more history retention, priority support, SLAs), and enterprise support and custom deployments. Self-hosting stays free.
- **Trust story:** the code can be audited, the relay is blind, and a vault can move to a self-hosted relay at any time without moving funds (the relay holds nothing that matters for custody).

---

## 18. Landscape

| Project | What it is | Relation to Zafe |
|---|---|---|
| Konclave (deegalabs, Apache/MIT) | Desktop (Tauri) FROST treasury; browser DKG; blind relay; **helper server holding the UFVK** builds PCZTs; mainnet Ironwood spends | Closest product. Zafe differs by being mobile-first, **building PCZTs only on members' devices** (the server never holds a viewing key), async approvals, signer-enforced rules, and **ZIP 2005 quantum-recoverable vault keys**. Tools built on frost-tools currently use the legacy, non-recoverable derivation (§7.4.1); confirm this for Konclave's own code |
| Quorum / zcash-multisig | Next.js + keyless coordinator; testnet Ironwood 2-of-3 | Hackathon-stage web app |
| ZF frost-tools | `frostd`, `frost-client`, `zcash-sign` CLIs | Reference implementation and test oracle |
| Zodl / Vizor | Single-user wallets with Keystone (PCZT) support | Pattern source: view-only account plus external signer |

---

## 19. Open items

| # | Item | Blocks |
|---|---|---|
| ~~V1~~ | **Resolved (2026-09-29), with follow-up:** no upstream ZIP 2005 FROST constructor exists (frost-tools deliberately uses the legacy, non-recoverable derivation). Zafe implements the ZIP 2005 derivations itself and builds the FVK with the public `FullViewingKey::from_bytes`. DKG output is normalized with `EvenY`. `sk` is 32 bytes. See §7.4.1. **Follow-up:** write test vectors and get them cross-checked upstream; move to the upstream API when it ships | M0 |
| ~~V2~~ | **Resolved (2026-09-29):** use the PCZT's `α` as the FROST randomizer (§9.5.1). Checked against frost-tools @ `06c0dbdb` (`zcash-sign/src/sign.rs`, `frost-client/src/coordinator/round_2.rs`), frost @ `0966bd15` (`frost-rerandomized`), ZIP 312, and ePrint 2024/436 | — |
| ~~V3~~ | **Resolved:** `pczt` 0.9.3 `roles::low_level_signer::Signer::{sign_ironwood_with, sign_orchard_with}`, then `action.spend().alpha()` and `action.apply_signature(sighash, sig)` (in `orchard` 0.15.5 `pczt/signer.rs`); the v6 sighash is `v6_signature_hash(&pczt.into_effects(), SignableInput::Shielded, …)`; dummy spends are signed by the IO Finalizer | — |
| ~~V4~~ | **Resolved (2026-09-29):** lightwalletd ≥ v0.5 (on Zebra ≥ 6.0) and `zcash_client_backend` 0.24 support Ironwood. Public mainnet endpoints are unconfirmed, so Zafe runs its own and checks servers on connect (§8.1) | — |
| ~~V5~~ | **Resolved (2026-09-29):** `frost-core` 3.0.0 supports DKG refresh (can remove members, N shrinks, t fixed) and repair (lost share or new identifier, ≥ t helpers). Changing t → migrate. Repair output isn't verified, so Zafe adds verifying-share checks and a test signature before old shares are deleted. See §10.2 and §10.4 | — |
| ~~V6~~ | **Resolved (2026-09-29):** `pczt` 0.9.3 `roles::updater::Updater::set_ironwood_spend_witnesses` sets or refreshes spend witnesses (and hence the anchor) **after** signing without changing the sighash, as long as proofs haven't been created yet. Proving happens after signing, at broadcast time. `Builder` also supports deferring anchors until proving. Evidence: `pczt` tests `wallet_can_set_ironwood_witness_after_signing` and `builder_can_defer_anchors_until_proving` | — |
| V7 | Halo 2 proving time on low-end Android; decide whether proving should happen at proposal time or be deferred | M1 |
| V8 | iOS background limits for signing round 2 (the notification must open the app?) | M1 |
| ~~V9~~ | **Resolved (2026-09-29):** all published crates, no git pins (see §4.4) | — |
| D1 | Maximum N (default 15): DKG cost is O(N²) messages; confirm the UX | M1 |
| D2 | Default proposal expiry and timeouts | M1 |
| D3 | Free vs paid tier limits | M4 |
| U1 | **Upstream:** ZF confirmation that the §7.4 derivation gives quantum-recoverable vaults whatever agreement method zips#895 standardizes for `sk`; ZIP 2005 FROST test vectors | **Mainnet (M2)** |
| U2 | **Upstream:** a non-deprecated external-randomizer signing API in `frost-rerandomized` (frost#1094) | Not blocking (deprecation warning only) |
| U3 | **Upstream:** a home for the redpallas ciphersuite (frost#963) and security-fix policy for `reddsa` 0.5.x in the meantime | Not blocking now; affects future upgrades |
| U4 | **Upstream:** COCKTAIL-DKG Pallas implementation and the ZIP 312 key-generation spec (zips#895, frost#1033) | Not blocking (§7.5) |

Questions for ZF are drafted in `upstream-asks.md`.
