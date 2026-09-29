//! In-process simulation of vault creation (spec §7.2) and a re-randomized FROST
//! signature checked the way Orchard checks spend authorization.

use std::collections::BTreeMap;

use ff::Field;
use pasta_curves::pallas;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use reddsa::frost::redpallas::{
    self,
    keys::dkg,
    rerandomized::{RandomizedParams, Randomizer},
    Identifier, PallasBlake2b512,
};
use zafe_core::keygen::{
    member_identifier, round1_echo, KeygenError, KeygenOutput, KeygenParams, Round1, SkContribution,
};

fn params(t: u16, n: u16) -> KeygenParams {
    KeygenParams::new([0x5a; 16], t, n).unwrap()
}

fn identifiers(params: &KeygenParams) -> Vec<Identifier> {
    (0..params.max_signers)
        .map(|i| member_identifier(&[i as u8 + 1; 32], &params.vault_id).unwrap())
        .collect()
}

/// Runs the full ceremony for every member, as if messages were relayed faithfully.
fn run_keygen(params: KeygenParams, rng: &mut StdRng) -> Vec<(Identifier, KeygenOutput)> {
    let ids = identifiers(&params);

    let round1: Vec<Round1> = ids
        .iter()
        .map(|id| Round1::start(params, *id, rng).unwrap())
        .collect();
    let broadcast: BTreeMap<Identifier, dkg::round1::Package> = round1
        .iter()
        .map(|r| (r.identifier(), r.package().clone()))
        .collect();

    let mut round2 = Vec::new();
    let mut sealed: BTreeMap<(Identifier, Identifier), dkg::round2::Package> = BTreeMap::new();
    for r1 in round1 {
        let me = r1.identifier();
        let others = broadcast
            .iter()
            .filter(|(id, _)| **id != me)
            .map(|(i, p)| (*i, p.clone()))
            .collect();
        let (r2, outgoing) = r1.advance(others).unwrap();
        for (to, pkg) in outgoing {
            sealed.insert((me, to), pkg);
        }
        round2.push((me, r2));
    }

    let echoes: Vec<[u8; 32]> = round2.iter().map(|(_, r2)| r2.echo()).collect();
    assert!(echoes.windows(2).all(|w| w[0] == w[1]), "echo mismatch");

    let dkg_results: Vec<_> = round2
        .into_iter()
        .map(|(me, r2)| {
            let received = sealed
                .iter()
                .filter(|((_, to), _)| *to == me)
                .map(|((from, _), pkg)| (*from, pkg.clone()))
                .collect();
            (me, r2.finish(received).unwrap())
        })
        .collect();

    let contributions: BTreeMap<Identifier, SkContribution> = ids
        .iter()
        .map(|id| (*id, SkContribution::generate(rng)))
        .collect();

    dkg_results
        .into_iter()
        .map(|(me, res)| (me, res.finish(&contributions).unwrap()))
        .collect()
}

#[test]
fn members_agree_on_vault_keys() {
    let mut rng = StdRng::seed_from_u64(10);
    for (t, n) in [(2, 3), (3, 5), (2, 2)] {
        let outputs = run_keygen(params(t, n), &mut rng);
        let (_, first) = &outputs[0];
        let ak = first.vault_keys.ak();
        assert_eq!(ak[31] & 0x80, 0, "ak must have even y");

        for (_, out) in &outputs {
            assert_eq!(out.transcript_hash, first.transcript_hash);
            assert_eq!(out.vault_secret.as_bytes(), first.vault_secret.as_bytes());
            assert_eq!(
                out.vault_keys.fvk().to_bytes(),
                first.vault_keys.fvk().to_bytes()
            );
            assert_eq!(out.public_key_package, first.public_key_package);
            assert_eq!(*out.key_package.min_signers(), t);
        }
        // Shares are distinct.
        let shares: std::collections::BTreeSet<_> = outputs
            .iter()
            .map(|(_, o)| o.key_package.signing_share().serialize())
            .collect();
        assert_eq!(shares.len(), usize::from(n));
    }
}

#[test]
fn rerandomized_signature_verifies_as_orchard_spend_auth() {
    let mut rng = StdRng::seed_from_u64(11);
    let outputs = run_keygen(params(2, 3), &mut rng);
    let pubkeys = outputs[0].1.public_key_package.clone();
    let ak = *outputs[0].1.vault_keys.ak();

    // As in a PCZT: alpha is fixed by the transaction builder before round 1.
    let alpha = pallas::Scalar::random(&mut rng);
    let mut sighash = [0u8; 32];
    rng.fill_bytes(&mut sighash);

    let signers = &outputs[..2]; // any t of n
    let mut nonces = BTreeMap::new();
    let mut commitments = BTreeMap::new();
    for (id, out) in signers {
        let (n, c) = redpallas::round1::commit(out.key_package.signing_share(), &mut rng);
        nonces.insert(*id, n);
        commitments.insert(*id, c);
    }
    let signing_package = redpallas::SigningPackage::new(commitments, &sighash);
    let randomizer = Randomizer::from_scalar(alpha);

    let mut shares = BTreeMap::new();
    for (id, out) in signers {
        #[allow(deprecated)] // the only public API taking an external randomizer (frost#1094)
        let share = frost_rerandomized::sign::<PallasBlake2b512>(
            &signing_package,
            &nonces[id],
            &out.key_package,
            randomizer,
        )
        .unwrap();
        shares.insert(*id, share);
    }
    let params = RandomizedParams::from_randomizer(pubkeys.verifying_key(), randomizer);
    let signature =
        redpallas::rerandomized::aggregate(&signing_package, &shares, &pubkeys, &params).unwrap();

    // Orchard's check: rk = ak.randomize(alpha); RedPallas SpendAuth verify(sighash).
    let sig_bytes: [u8; 64] = signature.serialize().unwrap().try_into().unwrap();
    let rk = reddsa::VerificationKey::<reddsa::orchard::SpendAuth>::try_from(ak)
        .unwrap()
        .randomize(&alpha);
    rk.verify(&sighash, &reddsa::Signature::from(sig_bytes))
        .expect("aggregate signature must verify under rk");

    // A different alpha must not verify.
    let wrong_rk = reddsa::VerificationKey::<reddsa::orchard::SpendAuth>::try_from(ak)
        .unwrap()
        .randomize(&pallas::Scalar::random(&mut rng));
    assert!(wrong_rk
        .verify(&sighash, &reddsa::Signature::from(sig_bytes))
        .is_err());
}

#[test]
fn equivocating_relay_is_detected_by_echo() {
    let mut rng = StdRng::seed_from_u64(12);
    let params = params(2, 3);
    let ids = identifiers(&params);
    let r1: Vec<Round1> = ids
        .iter()
        .map(|id| Round1::start(params, *id, &mut rng).unwrap())
        .collect();
    let honest: BTreeMap<_, _> = r1
        .iter()
        .map(|r| (r.identifier(), r.package().clone()))
        .collect();

    // The relay swaps member 0's package for a forged one when delivering to member 2.
    let forged = Round1::start(params, ids[0], &mut rng).unwrap();
    let mut seen_by_2 = honest.clone();
    seen_by_2.insert(ids[0], forged.package().clone());

    assert_ne!(
        round1_echo(&params, &honest).unwrap(),
        round1_echo(&params, &seen_by_2).unwrap()
    );
}

#[test]
fn rejects_bad_inputs() {
    assert!(matches!(
        KeygenParams::new([0; 16], 1, 3),
        Err(KeygenError::InvalidParams { .. })
    ));
    assert!(matches!(
        KeygenParams::new([0; 16], 4, 3),
        Err(KeygenError::InvalidParams { .. })
    ));
    assert!(matches!(
        KeygenParams::new([0; 16], 2, 16),
        Err(KeygenError::InvalidParams { .. })
    ));

    let mut rng = StdRng::seed_from_u64(13);
    let params = params(2, 3);
    let ids = identifiers(&params);
    let r1: Vec<Round1> = ids
        .iter()
        .map(|id| Round1::start(params, *id, &mut rng).unwrap())
        .collect();
    let all: BTreeMap<_, _> = r1
        .iter()
        .map(|r| (r.identifier(), r.package().clone()))
        .collect();

    let mut it = r1.into_iter();
    let first = it.next().unwrap();
    // Includes own package → rejected.
    assert!(matches!(
        first.advance(all.clone()),
        Err(KeygenError::OwnPackageReceived)
    ));

    let second = it.next().unwrap();
    let only_one: BTreeMap<_, _> = all
        .iter()
        .filter(|(id, _)| **id == ids[0])
        .map(|(i, p)| (*i, p.clone()))
        .collect();
    assert!(matches!(
        second.advance(only_one),
        Err(KeygenError::WrongPackageCount { .. })
    ));
}
