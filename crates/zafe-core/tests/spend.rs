//! End-to-end offline spend from a 2-of-3 vault on a local NU6.3 network:
//! vault keygen → Ironwood note to the vault → PCZT → find spends → local sighash →
//! FROST sign each spend with its alpha → inject → prove → extract and verify.
//!
//! Modeled on `pczt`'s own `wallet_can_set_ironwood_witness_after_signing` test.

use std::{collections::BTreeMap, sync::OnceLock};

use orchard::{
    builder::BundleType,
    bundle::BundleVersion,
    circuit::{OrchardCircuitVersion, ProvingKey, VerifyingKey},
    keys::{FullViewingKey, Scope, SpendingKey},
    note::Note,
    note_encryption::IronwoodDomain,
    tree::{MerkleHashOrchard, MerklePath},
    Address, Anchor,
};
use pczt::{
    roles::{
        creator::Creator, io_finalizer::IoFinalizer, prover::Prover,
        tx_extractor::TransactionExtractor,
    },
    Pczt,
};
use rand::{rngs::StdRng, SeedableRng};
use rand_core::OsRng;
use shardtree::{store::memory::MemoryShardStore, ShardTree};
use zafe_core::{signing, tx};
use zcash_note_encryption::try_note_decryption;
use zcash_primitives::transaction::{
    builder::{BuildConfig, Builder, BundlePadding, PcztResult},
    fees::zip317,
};
use zcash_protocol::{
    consensus::BlockHeight,
    local_consensus::LocalNetwork,
    memo::{Memo, MemoBytes},
    value::Zatoshis,
};

mod common;
use common::{params, run_keygen};

fn proving_key() -> &'static ProvingKey {
    static PK: OnceLock<ProvingKey> = OnceLock::new();
    PK.get_or_init(|| ProvingKey::build(OrchardCircuitVersion::PostNu6_3))
}

fn verifying_key() -> &'static VerifyingKey {
    static VK: OnceLock<VerifyingKey> = OnceLock::new();
    VK.get_or_init(|| VerifyingKey::build(OrchardCircuitVersion::PostNu6_3))
}

fn nu6_3_network() -> LocalNetwork {
    LocalNetwork {
        overwinter: Some(BlockHeight::from_u32(1)),
        sapling: Some(BlockHeight::from_u32(2)),
        blossom: Some(BlockHeight::from_u32(3)),
        heartwood: Some(BlockHeight::from_u32(4)),
        canopy: Some(BlockHeight::from_u32(5)),
        nu5: Some(BlockHeight::from_u32(6)),
        nu6: Some(BlockHeight::from_u32(7)),
        nu6_1: Some(BlockHeight::from_u32(8)),
        nu6_2: Some(BlockHeight::from_u32(9)),
        nu6_3: Some(BlockHeight::from_u32(10)),
    }
}

/// Simulates receiving an Ironwood note at `recipient`, decrypted with `fvk`.
fn receive_ironwood_note(fvk: &FullViewingKey, recipient: Address, value: u64) -> Note {
    let version = BundleVersion::ironwood_v3();
    let mut builder = orchard::builder::Builder::new(
        BundleType::DEFAULT,
        version,
        version.default_flags(),
        Anchor::empty_tree(),
    )
    .unwrap();
    builder
        .add_output(
            None,
            recipient,
            orchard::value::NoteValue::from_raw(value),
            Memo::Empty.encode().into_bytes(),
        )
        .unwrap();
    let (bundle, meta) = builder.build::<i64>(&mut OsRng).unwrap().unwrap();
    let action = &bundle.actions()[meta.output_action_index(0).unwrap()];
    let ivk = fvk.to_ivk(Scope::External).prepare();
    let (note, _, _) =
        try_note_decryption(&IronwoodDomain::for_action(action), &ivk, action).unwrap();
    note
}

/// A single-leaf Ironwood tree containing `note`.
fn witness(note: &Note) -> (Anchor, MerklePath) {
    let cmx: orchard::note::ExtractedNoteCommitment = note.commitment().into();
    let leaf = MerkleHashOrchard::from_cmx(&cmx);
    let mut tree =
        ShardTree::<_, 32, 16>::new(MemoryShardStore::<MerkleHashOrchard, u32>::empty(), 100);
    tree.append(leaf, incrementalmerkletree::Retention::Marked)
        .unwrap();
    tree.checkpoint(9_999_999).unwrap();
    let path = tree
        .witness_at_checkpoint_depth(0.into(), 0)
        .unwrap()
        .unwrap();
    (path.root(leaf).into(), path.into())
}

/// Builds a PCZT spending `note` (owned by `spend_fvk`) to an external recipient.
fn build_pczt(spend_fvk: &FullViewingKey, note: Note, path: MerklePath, anchor: Anchor) -> Pczt {
    let recipient = FullViewingKey::from(&SpendingKey::from_bytes([7; 32]).unwrap())
        .address_at(0u32, Scope::External);
    let mut builder = Builder::new(
        nu6_3_network(),
        10_000_000.into(),
        BuildConfig::Standard {
            sapling_anchor: None,
            orchard_anchor: None,
            ironwood_anchor: Some(anchor),
            orchard_padding: BundlePadding::DEFAULT,
            ironwood_padding: BundlePadding::DEFAULT,
        },
    );
    builder
        .add_ironwood_spend::<zip317::FeeRule>(spend_fvk.clone(), note, path)
        .unwrap();
    builder
        .add_ironwood_output::<zip317::FeeRule>(
            None,
            recipient,
            Zatoshis::const_from_u64(990_000),
            MemoBytes::empty(),
        )
        .unwrap();
    let PcztResult { pczt_parts, .. } = builder
        .build_for_pczt(OsRng, &zip317::FeeRule::standard())
        .unwrap();
    IoFinalizer::new(Creator::build_from_parts(pczt_parts).unwrap())
        .finalize_io()
        .unwrap()
}

#[test]
fn vault_spends_ironwood_note_with_frost() {
    let mut rng = StdRng::seed_from_u64(20);
    let members = run_keygen(params(2, 3), &mut rng);
    let vault = &members[0].1;
    let vault_fvk = vault.vault_keys.fvk().clone();
    let vault_address = vault_fvk.address_at(0u32, Scope::External);

    // The vault receives 0.01 ZEC in the Ironwood pool.
    let note = receive_ironwood_note(&vault_fvk, vault_address, 1_000_000);
    let (anchor, path) = witness(&note);
    let pczt = build_pczt(&vault_fvk, note, path, anchor);

    // Every member independently finds the spends to sign and computes the sighash.
    let spends = tx::spends_to_sign(&pczt, &vault_fvk).unwrap();
    assert_eq!(
        spends.len(),
        1,
        "one real vault spend; the padding dummy is already signed"
    );
    let sighash = tx::shielded_sighash(&pczt).unwrap();

    // Two members sign: round 1 (commitments), leader builds packages, round 2 (shares).
    let signers = &members[1..3];
    let mut signatures = Vec::new();
    for spend in &spends {
        let mut nonces = BTreeMap::new();
        let mut commitments = BTreeMap::new();
        for (id, m) in signers {
            let (n, c) = signing::commit(&m.key_package, &mut rng);
            nonces.insert(*id, n);
            commitments.insert(*id, c);
        }
        let package = signing::signing_package(commitments, &sighash);
        let shares: BTreeMap<_, _> = signers
            .iter()
            .map(|(id, m)| {
                let share = signing::sign(
                    &package,
                    nonces.remove(id).unwrap(),
                    &m.key_package,
                    spend.alpha,
                )
                .unwrap();
                (*id, share)
            })
            .collect();
        let sig =
            signing::aggregate(&package, &shares, &vault.public_key_package, spend.alpha).unwrap();
        signatures.push((spend.action_index, sig));
    }

    // Inject (checked against rk), prove, and extract; extraction verifies the whole
    // Ironwood bundle: spend auth signatures, binding signature and the Halo 2 proof.
    let signed = tx::apply_signatures(pczt, &signatures).unwrap();
    assert!(tx::spends_to_sign(&signed, &vault_fvk).unwrap().is_empty());
    let proved = Prover::new(signed)
        .create_ironwood_proof(proving_key())
        .unwrap()
        .finish();
    let transaction = TransactionExtractor::new(proved)
        .with_orchard(verifying_key())
        .extract()
        .expect("fully authorized, valid Ironwood transaction");
    assert!(transaction.ironwood_bundle().is_some());
}

#[test]
fn rejects_spend_of_foreign_note() {
    let mut rng = StdRng::seed_from_u64(21);
    let members = run_keygen(params(2, 3), &mut rng);
    let vault_fvk = members[0].1.vault_keys.fvk().clone();

    // A PCZT that spends someone else's note.
    let other_fvk = FullViewingKey::from(&SpendingKey::from_bytes([3; 32]).unwrap());
    let note = receive_ironwood_note(
        &other_fvk,
        other_fvk.address_at(0u32, Scope::External),
        1_000_000,
    );
    let (anchor, path) = witness(&note);
    let pczt = build_pczt(&other_fvk, note, path, anchor);

    assert!(matches!(
        tx::spends_to_sign(&pczt, &vault_fvk),
        Err(tx::TxError::NotVaultSpend(_))
    ));
}

#[test]
fn signature_with_wrong_alpha_is_rejected_on_injection() {
    let mut rng = StdRng::seed_from_u64(22);
    let members = run_keygen(params(2, 3), &mut rng);
    let vault = &members[0].1;
    let vault_fvk = vault.vault_keys.fvk().clone();
    let note = receive_ironwood_note(
        &vault_fvk,
        vault_fvk.address_at(0u32, Scope::External),
        1_000_000,
    );
    let (anchor, path) = witness(&note);
    let pczt = build_pczt(&vault_fvk, note, path, anchor);

    let spend = &tx::spends_to_sign(&pczt, &vault_fvk).unwrap()[0];
    let sighash = tx::shielded_sighash(&pczt).unwrap();
    let wrong_alpha = <pasta_curves::pallas::Scalar as ff::Field>::random(&mut rng);

    let signers = &members[..2];
    let mut nonces = BTreeMap::new();
    let mut commitments = BTreeMap::new();
    for (id, m) in signers {
        let (n, c) = signing::commit(&m.key_package, &mut rng);
        nonces.insert(*id, n);
        commitments.insert(*id, c);
    }
    let package = signing::signing_package(commitments, &sighash);
    let shares: BTreeMap<_, _> = signers
        .iter()
        .map(|(id, m)| {
            (
                *id,
                signing::sign(
                    &package,
                    nonces.remove(id).unwrap(),
                    &m.key_package,
                    wrong_alpha,
                )
                .unwrap(),
            )
        })
        .collect();
    let sig =
        signing::aggregate(&package, &shares, &vault.public_key_package, wrong_alpha).unwrap();

    assert!(matches!(
        tx::apply_signatures(pczt, &[(spend.action_index, sig)]),
        Err(tx::TxError::Sign(_))
    ));
}
