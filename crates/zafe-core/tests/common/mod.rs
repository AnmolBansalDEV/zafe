//! Shared test helpers: an in-process simulation of vault creation.
#![allow(dead_code)]

use std::collections::BTreeMap;

use rand::rngs::StdRng;
use reddsa::frost::redpallas::{keys::dkg, Identifier};
use zafe_core::keygen::{member_identifier, KeygenOutput, KeygenParams, Round1, SkContribution};

pub fn params(t: u16, n: u16) -> KeygenParams {
    KeygenParams::new([0x5a; 16], t, n).unwrap()
}

pub fn identifiers(params: &KeygenParams) -> Vec<Identifier> {
    (0..params.max_signers)
        .map(|i| member_identifier(&[i as u8 + 1; 32], &params.vault_id).unwrap())
        .collect()
}

/// Runs the full ceremony for every member, as if messages were relayed faithfully.
pub fn run_keygen(params: KeygenParams, rng: &mut StdRng) -> Vec<(Identifier, KeygenOutput)> {
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
