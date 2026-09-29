//! FROST signing primitives for vault spends (spec §9.4–§9.5).
//!
//! One FROST signature per spend, each re-randomized with that spend's `alpha` from the
//! PCZT (spec §9.5.1). Session orchestration (leader, timeouts, nonce storage) lives above
//! this module; here are the per-member and per-leader operations.

use std::collections::BTreeMap;

use pasta_curves::pallas;
use rand_core::{CryptoRng, RngCore};
use reddsa::frost::redpallas::{
    self,
    keys::{KeyPackage, PublicKeyPackage},
    rerandomized::{RandomizedParams, Randomizer},
    round1::{SigningCommitments, SigningNonces},
    round2::SignatureShare,
    Identifier, PallasBlake2b512, SigningPackage,
};

#[derive(Debug, thiserror::Error)]
pub enum SigningError {
    #[error("FROST: {0}")]
    Frost(#[from] redpallas::Error),
    #[error("signature encoding")]
    Encoding,
}

/// Round 1 for one spend: fresh nonces and their public commitments.
///
/// Each nonce must be used for at most one signature share and deleted before that share
/// is sent (spec §9.4).
pub fn commit<R: RngCore + CryptoRng>(
    key_package: &KeyPackage,
    rng: &mut R,
) -> (SigningNonces, SigningCommitments) {
    redpallas::round1::commit(key_package.signing_share(), rng)
}

/// Builds the signing package for one spend (leader side).
pub fn signing_package(
    commitments: BTreeMap<Identifier, SigningCommitments>,
    sighash: &[u8; 32],
) -> SigningPackage {
    SigningPackage::new(commitments, sighash)
}

/// Round 2 for one spend: this member's signature share.
///
/// The caller must already have checked that the package's message equals the locally
/// computed sighash and that `alpha` comes from the PCZT (spec §9.3). Consumes the nonces.
pub fn sign(
    signing_package: &SigningPackage,
    nonces: SigningNonces,
    key_package: &KeyPackage,
    alpha: pallas::Scalar,
) -> Result<SignatureShare, SigningError> {
    // `frost_rerandomized::sign` is deprecated, but it is the only public API that accepts
    // an externally fixed randomizer, which Zcash requires (frost#1094, spec §9.5.1).
    #[allow(deprecated)]
    let share = frost_rerandomized::sign::<PallasBlake2b512>(
        signing_package,
        &nonces,
        key_package,
        Randomizer::from_scalar(alpha),
    )?;
    Ok(share)
}

/// Verifies every share and aggregates them into a RedPallas spend authorization
/// signature valid under `rk = ak + [alpha]G` (leader side).
///
/// On an invalid share, the returned FROST error identifies the culprit.
pub fn aggregate(
    signing_package: &SigningPackage,
    shares: &BTreeMap<Identifier, SignatureShare>,
    public_key_package: &PublicKeyPackage,
    alpha: pallas::Scalar,
) -> Result<[u8; 64], SigningError> {
    let params = RandomizedParams::from_randomizer(
        public_key_package.verifying_key(),
        Randomizer::from_scalar(alpha),
    );
    let signature =
        redpallas::rerandomized::aggregate(signing_package, shares, public_key_package, &params)?;
    signature
        .serialize()?
        .try_into()
        .map_err(|_| SigningError::Encoding)
}
