# Upstream asks: Zcash Foundation (FROST)

**Where to send:** the `#frost` channel on the Zcash R&D Discord, as ZF's "State of FROST for Zcash" post asks. Also comment on GitHub. **Q1 and Q2 belong in the existing issue frost#1094**, which another team opened on 2026-09-21 and conradoplg answered. Add Zafe's details there rather than opening a duplicate.

**Priority:** Q1 blocks Zafe's mainnet launch. The rest are non-blocking but affect the plan.

---

## Q1 (blocks mainnet): is the ZIP 2005 FROST key derivation final enough to build on?

**Context:** frost#1094 (reply from conradoplg): FVK derivation for FROST under ZIP 2005 is "blocked on zcash/zips#895 being finished and integrated with ZIP 2005, and then implemented." The only upstream constructor (`from_sk_ak_incompatible_with_quantum_recoverability_and_will_be_removed`, orchard#475) gives up recoverability.

**What Zafe plans to do today:**
1. `ak` comes from the FROST DKG (redpallas, even-Y).
2. Participants agree on a 32-byte `sk` (each participant contributes randomness, sealed to the others; `sk` is a BLAKE2b hash of the DKG transcript and the contributions).
3. `nk = H^nk(sk)`, `qsk = H^qsk(sk)`, `qk = H^qk(qsk)`, `rivk_ext = H^rivk_ext_qk(ak, nk)`, exactly as in ZIP 2005 § 4.2.3 with `use_qsk = true`.
4. `orchard::keys::FullViewingKey::from_bytes(ak || nk || rivk_ext)`, then `UnifiedFullViewingKey::from_orchard_fvk`.

**Questions:**
- (a) Do vaults created this way stay recoverable under ZIP 2005, whatever `sk`-agreement method zips#895 ends up specifying? Our reading is that the Recovery Statement checks `nk = H^nk(sk)`, `qk = H^qk(qsk)`, `rivk_ext = H^rivk_ext_qk(ak, nk)` and a signature under `ak`, and never checks how `sk` was agreed.
- (b) Could any other part of zips#895 change the § 4.2.3 derivations (domain bytes `0x07`, `0x0C`, `0x0D`, the BLAKE3 `qk` context string)?
- (c) Are there, or will there be, **test vectors** for the `use_qsk = true` path (`sk, ak → nk, qsk, qk, rivk_ext, ivk, default address`)? We're happy to contribute vectors to `zcash-test-vectors` if you'll review them.
- (d) Is there a rough timeline for a ZIP 2005 FROST constructor in `orchard` or `zcash_keys`, so we can switch to it?

---

## Q2 (non-blocking): an API for signing with an external randomizer

The same request as frost#1094 part 1. `frost_rerandomized::sign()` (3.0.0) is deprecated, but it's the only public path for a randomizer taken from the PCZT's `alpha`. `sign_with_randomizer_seed` can't reproduce a fixed `alpha`, and the `Randomize` trait is private.

**Ask:** remove the deprecation from `sign()`, or add `sign_with_randomizer(signing_package, nonces, key_package, randomizer)`. Also, please confirm that using the transaction's `alpha`, fixed before round 1 and chosen by the transaction builder, is the intended use under the revised ZIP 312 in zips#895. We're relying on the Re-Randomized FROST unforgeability game, which allows the adversary to choose the randomizer.

---

## Q3 (non-blocking): where the redpallas ciphersuite lives and how it gets security fixes

`reddsa` 0.6.0 removed the `frost` feature ("will be moved to the frost repository"). frost#963 is waiting on `reddsa` internals being exposed. Zafe is pinned to **`reddsa` 0.5.2 + `frost-core`/`frost-rerandomized` 3.0.0**, because `orchard` 0.15.5 uses `reddsa` 0.5 / `pasta_curves` 0.5 and we need `frost-core` 3.x refresh and repair.

**Asks:**
- (a) Will `reddsa` 0.5.x get security fixes until the ciphersuite moves?
- (b) Is there a target release for the ciphersuite in the `frost` repo, and will it keep the 3.x `KeyPackage`/`PublicKeyPackage` serialization, so stored shares keep working?
- (c) Is `reddsa` 0.5.2 + `frost-core` 3.0.0 the combination you'd recommend today for an Ironwood (orchard 0.15.5) wallet?

---

## Q4 (non-blocking): which DKG and how to agree on `sk`

zips#895 is weighing COCKTAIL-DKG. frost#1033 (production COCKTAIL-DKG) is waiting on C2SP#216.

Until then, Zafe uses the `frost-core` 3.0 DKG with a relay that can't read messages, round-2 packages sealed to each recipient, an echo check on round-1 packages, and an all-participant signed transcript, plus the `sk` contribution scheme from Q1.

**Asks:**
- (a) Is there anything wrong with this as an interim approach?
- (b) Do you have a draft of the `sk`-agreement method you plan to specify, so we can match it now and avoid a second way of doing it?
- (c) Would you like us to test the Pallas ciphersuite in the WIP COCKTAIL-DKG PR (frost#1032)?

---

## Q5 (docs, non-blocking): the Zcash section of the FROST book is out of date

`book/src/zcash/technical-details.md` still covers only Sapling and Orchard, and makes two recommendations that conflict with ZIP 2005:
- *"generating `nk` and `rivk` by themselves (or generating them from a random `sk` which is thrown away)"*. ZIP 2005 requires keeping `sk` and treating `qsk` "at least as securely and reliably as `ask`."
- *"The only secret information is the key share."* Under ZIP 2005, `qsk` (from `sk`) is also secret and needed for recovery.

**Ask:** add an Ironwood / ZIP 2005 note, or offer a PR. We can draft one.

---

## Q6 (optional): frostd and repair / refresh in frost-tools

- The "State of FROST" post offers ZF-run production `frostd` servers. Zafe runs its own relay because it needs store-and-forward delivery and mobile push notifications for asynchronous approvals. Is there interest in adding store-and-forward and push to `frostd`, which would let Zafe interoperate with it?
- frost-tools#311 ("Add repair share functionality") is open. Zafe will implement repair and DKG refresh on `frost-core` 3.0. Would you like a PR, or test results?

---

## Short Discord message (paste-ready)

> Hi FROST team, we're building Zafe, a mobile shielded multisig on Ironwood using re-randomized FROST (frost-core 3.0 / reddsa 0.5.2 / pczt 0.9.3). Following up on frost#1094: we want to use ZIP 2005 `use_qsk = true` keys, so we derive nk / qsk / qk / rivk_ext from an agreed `sk` per § 4.2.3 and build the FVK with `FullViewingKey::from_bytes(ak||nk||rivk_ext)`. Can you confirm those vaults stay quantum-recoverable whatever `sk`-agreement method zips#895 settles on, and whether use_qsk test vectors are planned (we can contribute)? That's our mainnet gate. Smaller questions (external-randomizer `sign()` deprecation, reddsa 0.5.x support until the ciphersuite moves, interim DKG approach) are in a comment on #1094. Thanks!
