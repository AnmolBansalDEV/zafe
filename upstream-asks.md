# Upstream asks: Zcash Foundation (FROST)

**Where to send:** one comment on **frost#1094** (the open issue about external randomizers
and FVK derivation, answered by conradoplg on 2026-09-21), plus a short pointer in `#frost`
on the Zcash R&D Discord. conradoplg co-wrote the Re-Randomized FROST paper, so U5 is his.

**Status (re-checked 2026-10-01 against upstream):** U5 and U1 block mainnet; U3 is minor.
Ids are the open items in `spec.md` §19. Write each question so it can be answered
without opening links.

---

## U5 (blocks one-tap signing on mainnet): a builder-chosen randomizer with pre-published commitments

**Context checked:**
- ZIP 312, both the published text and the zips#895 draft (head `be04583`), says the
  Coordinator **MUST** derive the randomizer after round 1: `randomizer_generate()` =
  `HR(random_bytes(Ns) ‖ encode_group_commitment_list(commitment_list))` (the published
  version also hashes the message, which the draft drops because `α` feeds the sighash).
  The rationale calls the commitment binding a hedge: it "prevents the Coordinator from
  fully influencing the randomizer, reducing its trust assumptions".
- Zafe instead uses each spend's PCZT `α` (picked by the builder like `RedDSA.GenRandom`),
  as does frost-tools' `zcash-sign` @ `06c0dbdb` (`collect_randomizers` reads
  `action.spend().alpha()`). So Zafe **deviates from a MUST in ZIP 312** (spec §9.5.1).
- ePrint 2024/436 (checked against the PDF): the unforgeability game allows signatures
  "under a adversarially-chosen randomizer" (Fig. 4, `OSign′` after the honest
  commitments); the extraction works "because this value is public"; §5.1: the party that
  chooses the randomizer "is not trusted with security; even if this party were to act
  maliciously, then the scheme remains secure".
- Zafe could comply if required: `VaultState::assign_commitments` depends only on the pool
  state and the spend count, so a proposer knows each spend's commitments before building
  the PCZT and could set `α = HR(seed ‖ those commitments)`. Unchecked: whether the PCZT
  builder (`create_pczt_from_proposal`) lets us choose `α`; a log race would mean a rebuild.

**Asks:** (a) is unforgeability preserved with a builder-chosen `α`, including several
concurrent signing packages over the same sighash and `α` (one per signer group, disjoint
commitments)? (b) will ZIP 312 allow it (what any PCZT signer gets), or should wallets bind
`α` to the commitments, and would the variant above count?

## U1 (blocks mainnet): are vaults created today recoverable?

**Context checked:**
- ZIP 2005 (status Proposed; changes since 2026-07 are wording) has a normative "Usage with
  FROST": participants "MUST privately agree on a value `sk`", then derive `nk`, `qsk`,
  `qk`, `rivk_ext` per § 4.2.3 with `use_qsk = true`; all must get the same `ak`, `nk`,
  `rivk_ext`; `qsk` kept as securely as `ask`. No `sk`-agreement method is required. The
  Recovery Protocol checks the derivations, a `SoK^qsk` and a RedDSA signature under `ak`.
- Zafe's constants match § 4.2.3 exactly (`0x07`, `0x0C`, `0x0D`, BLAKE3
  "Zcash ZIP 2005 qk-derivation v1"); FVK via `FullViewingKey::from_bytes`, never the
  `from_sk_ak_incompatible…` constructor (orchard#475, still open).
- conradoplg on frost#1094: FROST FVK derivation is "blocked on zips#895 being finished and
  integrated with ZIP 2005, and then implemented". ZIP 2005 already has the FROST section,
  so it's unclear whether that still applies to wallets deriving keys themselves.
- No official `use_qsk` vectors: zcash-test-vectors#129 (open) adds only QR note
  commitments. Zafe has its own (`crates/zafe-core/test-vectors/zip2005_use_qsk.json`, from
  `tests/zip2005_vectors.rs`, checked independently by `scripts/check_zip2005_vectors.py`).

**Asks:** does "blocked on zips#895" apply to keys derived this way, or will such vaults pass
the Recovery Protocol as they are? Is anything in § 4.2.3 still expected to change? Offer:
PR our vectors to zcash-test-vectors.

## U3 (minor): reddsa 0.5.x until the ciphersuite moves

reddsa 0.6.0/0.6.1 (2026-09-25) dropped the FROST ciphersuites; frost#963 (move them to the
`frost` repo) is open and `frost` has no redpallas crate. Zafe stays on reddsa 0.5.2 +
frost-core 3.0.0 (orchard 0.15.5 uses reddsa 0.5). **Ask:** security fixes for 0.5.x until
the move, and will it keep the frost-core 3.x `KeyPackage` serialization?

---

## Not asking (re-checked 2026-10-01)

- **Old Q2, external-randomizer API:** answered on frost#1094 ("there will always a
  mechanism for feeding an external randomizer"; `sign()` may be un-deprecated). We keep
  the one `#[allow(deprecated)]` wrapper.
- **Old Q4, DKG and `sk` agreement:** the zips#895 draft specifies it ("Contributory
  Generation of sk": each member's `sk_i = random_bytes(32)` as a COCKTAIL-DKG payload,
  `sk = H(n ‖ len(payload_1) ‖ payload_1 ‖ …)`). Zafe's own scheme differs; recoverability
  doesn't depend on it (U1). Align when COCKTAIL-DKG ships with Pallas (frost#1033 open;
  the WIP frost#1032 has no Pallas ciphersuite; C2SP#216 closed for C2SP#299).
- **Old Q5, FROST book:** `book/src/zcash/technical-details.md` still suggests throwing
  `sk` away and says the key share is the only secret, against ZIP 2005. Send a docs PR
  rather than a question.
- **Old Q6, frostd store-and-forward/push and repair (frost-tools#311):** not a question;
  revisit when Zafe implements repair.

## Discord pointer (paste after the comment is up)

> hi FROST folks 👋 I'm building Zafe, a mobile shielded multisig on Ironwood with
> re-randomized FROST (frost-core 3.0, reddsa 0.5.2, PCZT). Two questions block our
> mainnet, written up on frost#1094: <comment link>
> 1. our randomizer is the PCZT's `α`, chosen before round 1 (like `zcash-sign`), which
>    ZIP 312 says MUST come after. OK for pre-published commitments?
> 2. ZIP 2005 keys derived from an agreed `sk` per § 4.2.3: recoverable as is?
> thanks 🙏

---

# Upstream ask: zcashlabs/thus-spoke-zakura (local regtest environment)

**Where to send:** an issue on `zcashlabs/thus-spoke-zakura`.

**Finding (2026-09-29, `ths` at `b24101f`, node `zakuracore/zakura:1.4.0`):** the environment runs a **pre-Ironwood** regtest chain:
- `crates/tsz-server/src/main.rs` `zakura_config()` sets only `"NU6" = 1` under `[network.testnet_parameters.activation_heights]`.
- `crates/tsz-server/src/wallet.rs` `regtest_network()` and `db.rs` `local_network()` hardcode `nu6_1`, `nu6_2` and `nu6_3` to `None`.

The **Zakura node itself supports NU6.3 on regtest**. Started with `"NU5"`, `"NU6"`, `"NU6.1"`, `"NU6.2"` and `"NU6.3"` all set to `1`, `getblockchaininfo` reports every upgrade through NU6.3 at height 1, with next-block consensus branch `37a5165b` (NU6.3).

**Ask:** activate every NU6.x (including NU6.3) at height 1 by default, or behind a flag (e.g. `ths start --ironwood`). Make the faucet and "send" route through the Ironwood pool, since Orchard stops accepting deposits after NU6.3. That would make `ths` usable for testing Ironwood apps like Zafe. Zebra's own regtest default already activates every NU6.x at height 1.

**Zafe's workaround until then:** run `zakuracore/zakura:1.4.0` and the `ths` lightwalletd image directly with our own NU6.3 config (`infra/regtest/`).
