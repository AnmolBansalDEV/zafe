//! Vault-log replay rules (spec §6.3, §9.6).

use rand::{rngs::StdRng, SeedableRng};
use zafe_core::vault::{
    MemberInfo, ProposalStatus, ProposedPayment, VaultDescriptor, VaultError, VaultEvent,
    VaultState,
};
use zafe_proto::{Chain, Identity, LogEntry, LogKey};

const MAILBOX: [u8; 16] = [4; 16];

struct Log {
    rng: StdRng,
    ids: Vec<Identity>,
    key: LogKey,
    chain: Chain,
}

impl Log {
    fn new() -> Self {
        let mut rng = StdRng::seed_from_u64(8);
        let ids = (0..4).map(|_| Identity::generate(&mut rng)).collect();
        let key = LogKey::generate(0, &mut rng);
        Self {
            rng,
            ids,
            key,
            chain: Chain::new(MAILBOX),
        }
    }

    fn descriptor(&self) -> VaultDescriptor {
        VaultDescriptor {
            vault_id: MAILBOX,
            version: 1,
            name: "Grants".into(),
            network: "regtest".into(),
            threshold: 2,
            members: self.ids[..3]
                .iter()
                .enumerate()
                .map(|(i, id)| MemberInfo {
                    identity: *id.public(),
                    frost_id: vec![i as u8],
                    name: format!("m{i}"),
                })
                .collect(),
            group_public_key: [1; 32],
            ufvk: "uviewregtest1...".into(),
            address: "uregtest1...".into(),
            use_qsk: true,
            birthday_height: 2,
            proposal_expiry_blocks: 8064,
            epoch: 0,
            transcript_hash: [2; 32],
        }
    }

    fn created(&self, signers: &[usize]) -> VaultEvent {
        let d = self.descriptor();
        let msg = d.signing_message().unwrap();
        let signatures = signers
            .iter()
            .map(|i| {
                (
                    self.ids[*i].public().sig_pk,
                    self.ids[*i].sign(&msg).to_vec(),
                )
            })
            .collect();
        VaultEvent::Created {
            descriptor: d,
            signatures,
        }
    }

    /// Appends an event authored by member `author` (the relay only accepts members, but
    /// replay must not rely on that, so the chain here is checked against all 4 ids).
    fn push(&mut self, author: usize, event: &VaultEvent) {
        let entry = LogEntry::create(
            &self.ids[author],
            &self.key,
            MAILBOX,
            self.chain.len(),
            self.chain.head(),
            &event.to_bytes().unwrap(),
            &mut self.rng,
        )
        .unwrap();
        let all: Vec<_> = self.ids.iter().map(|i| *i.public()).collect();
        self.chain.append(entry, &all).unwrap();
    }

    fn replay(&self) -> Result<VaultState, VaultError> {
        VaultState::replay(self.chain.entries(), &self.key)
    }
}

fn proposal(id: u8) -> VaultEvent {
    VaultEvent::Proposal {
        id: [id; 16],
        payments: vec![ProposedPayment {
            address: "uregtest1payee".into(),
            amount_zat: 1,
            memo: vec![0xF6],
        }],
        pczt: vec![1, 2, 3],
        pczt_hash: [id; 32],
        tip_height: 100,
        created_at: 1_700_000_000,
        signing_spends: 1,
        auto_send: false,
    }
}

fn vote(id: u8, approve: bool) -> VaultEvent {
    VaultEvent::Vote {
        proposal: [id; 16],
        pczt_hash: [id; 32],
        approve,
        commitments: vec![],
        shares: vec![],
    }
}

#[test]
fn proposal_lifecycle() {
    let mut log = Log::new();
    log.push(0, &log.created(&[0, 1, 2]));
    log.push(0, &proposal(1));
    log.push(1, &vote(1, true));
    assert_eq!(
        log.replay().unwrap().proposals[&[1; 16]].status,
        ProposalStatus::Open
    );
    log.push(2, &vote(1, true));
    assert_eq!(
        log.replay().unwrap().proposals[&[1; 16]].status,
        ProposalStatus::Approved
    );
    log.push(
        0,
        &VaultEvent::Broadcast {
            proposal: [1; 16],
            txid: [9; 32],
        },
    );
    let state = log.replay().unwrap();
    assert_eq!(state.proposals[&[1; 16]].status, ProposalStatus::Broadcast);
    assert_eq!(state.applied, 5);
}

#[test]
fn rejection_threshold_is_n_minus_t_plus_one() {
    let mut log = Log::new();
    log.push(0, &log.created(&[0, 1, 2]));
    log.push(0, &proposal(1));
    log.push(1, &vote(1, false));
    assert_eq!(
        log.replay().unwrap().proposals[&[1; 16]].status,
        ProposalStatus::Open,
        "1 reject of 3 is not enough"
    );
    log.push(2, &vote(1, false));
    assert_eq!(
        log.replay().unwrap().proposals[&[1; 16]].status,
        ProposalStatus::Rejected
    );
}

#[test]
fn a_member_changing_their_vote_counts_once() {
    let mut log = Log::new();
    log.push(0, &log.created(&[0, 1, 2]));
    log.push(0, &proposal(1));
    log.push(1, &vote(1, true));
    log.push(1, &vote(1, true)); // re-approval with fresh commitments
    assert_eq!(log.replay().unwrap().proposals[&[1; 16]].approvals.len(), 1);
    log.push(1, &vote(1, false));
    let p = &log.replay().unwrap().proposals[&[1; 16]];
    assert!(p.approvals.is_empty());
    assert_eq!(p.rejections.len(), 1);
}

#[test]
fn creation_needs_every_member_signature() {
    let mut log = Log::new();
    log.push(0, &log.created(&[0, 1]));
    assert_eq!(
        log.replay().unwrap_err(),
        VaultError::MissingDescriptorSignature
    );
}

#[test]
fn log_must_start_with_creation() {
    let mut log = Log::new();
    log.push(0, &proposal(1));
    assert_eq!(log.replay().unwrap_err(), VaultError::NotCreatedFirst);
}

/// Invalid entries are skipped by every member instead of making the log unreadable.
#[test]
fn invalid_entries_are_ignored_not_fatal() {
    let cases: Vec<(usize, VaultEvent, VaultError)> = vec![
        (3, proposal(2), VaultError::NotAMember(2)), // id 3 is not in the descriptor
        (
            1,
            VaultEvent::Vote {
                proposal: [1; 16],
                pczt_hash: [0xEE; 32],
                approve: true,
                commitments: vec![],
                shares: vec![],
            },
            VaultError::PcztMismatch(2),
        ),
        (
            1,
            VaultEvent::Cancelled { proposal: [1; 16] },
            VaultError::NotAuthor(2),
        ),
        (
            0,
            VaultEvent::Broadcast {
                proposal: [1; 16],
                txid: [0; 32],
            },
            VaultError::ProposalClosed(2),
        ),
        (1, vote(9, true), VaultError::UnknownProposal(2)),
    ];
    for (author, bad, expected) in cases {
        let mut log = Log::new();
        log.push(0, &log.created(&[0, 1, 2]));
        log.push(0, &proposal(1));
        log.push(author, &bad);
        log.push(1, &vote(1, true)); // entries after the bad one still apply
        let state = log.replay().expect("log stays readable");
        assert_eq!(state.ignored, vec![(2, expected)]);
        assert_eq!(state.proposals.len(), 1);
        assert_eq!(state.proposals[&[1; 16]].approvals.len(), 1);
        assert_eq!(state.applied, 4);
    }
}

/// The race from the code review: the author cancels while the leader broadcasts.
#[test]
fn broadcast_racing_a_cancel_does_not_brick_the_log() {
    let mut log = Log::new();
    log.push(0, &log.created(&[0, 1, 2]));
    log.push(0, &proposal(1));
    log.push(1, &vote(1, true));
    log.push(2, &vote(1, true));
    log.push(0, &VaultEvent::Cancelled { proposal: [1; 16] });
    log.push(
        1,
        &VaultEvent::Broadcast {
            proposal: [1; 16],
            txid: [5; 32],
        },
    );
    let state = log.replay().unwrap();
    assert_eq!(state.proposals[&[1; 16]].status, ProposalStatus::Cancelled);
    assert_eq!(state.ignored, vec![(5, VaultError::ProposalClosed(5))]);

    // `check` lets a writer see this before appending.
    let err = state.check(
        log.ids[1].public().sig_pk,
        &VaultEvent::Broadcast {
            proposal: [1; 16],
            txid: [5; 32],
        },
    );
    assert_eq!(err, Err(VaultError::ProposalClosed(6)));
}

// --- One-tap signing: commitment pools and share votes ---------------------------------

fn commitments(rng: &mut StdRng, n: usize) -> VaultEvent {
    use reddsa::frost::redpallas::{keys::SigningShare, round1};
    // Any valid signing share will do: replay only checks that commitments decode.
    let mut scalar = [0u8; 32];
    scalar[0] = 7;
    let share = SigningShare::deserialize(&scalar).unwrap();
    VaultEvent::Commitments {
        batch: (0..n)
            .map(|_| round1::commit(&share, rng).1.serialize().unwrap())
            .collect(),
    }
}

fn share_vote(id: u8, groups: &[u16]) -> VaultEvent {
    VaultEvent::Vote {
        proposal: [id; 16],
        pczt_hash: [id; 32],
        approve: true,
        commitments: vec![],
        shares: groups
            .iter()
            .map(|g| zafe_core::vault::GroupShares {
                group: *g,
                shares: vec![vec![0xAB; 32]],
            })
            .collect(),
    }
}

/// A 2-of-3 vault whose three members each published `per_member` commitments.
fn pooled_log(per_member: usize) -> Log {
    let mut log = Log::new();
    log.push(0, &log.created(&[0, 1, 2]));
    let mut rng = StdRng::seed_from_u64(99);
    for m in 0..3 {
        log.push(m, &commitments(&mut rng, per_member));
    }
    log
}

#[test]
fn proposals_get_disjoint_commitments_in_log_order() {
    let mut log = pooled_log(4);
    log.push(0, &proposal(1));
    log.push(1, &proposal(2));
    let s = log.replay().unwrap();
    let (p1, p2) = (
        s.proposals[&[1; 16]].preprocessed.as_ref().unwrap(),
        s.proposals[&[2; 16]].preprocessed.as_ref().unwrap(),
    );
    // 2-of-3: three groups, each with one commitment per member per spend.
    assert_eq!(p1.subsets.len(), 3);
    let all: Vec<&Vec<u8>> = [p1, p2]
        .iter()
        .flat_map(|p| p.commitments.iter().flatten().flatten())
        .collect();
    let unique: std::collections::BTreeSet<_> = all.iter().collect();
    assert_eq!(all.len(), 12);
    assert_eq!(unique.len(), 12, "a commitment is never assigned twice");
    // Each member used 2 per proposal: 4 published, 0 left.
    for m in 0..3 {
        assert_eq!(s.pools[&log.ids[m].public().sig_pk].available(), 0);
    }
    // Replay is deterministic.
    let again = log.replay().unwrap();
    assert_eq!(
        again.proposals[&[2; 16]].preprocessed,
        s.proposals[&[2; 16]].preprocessed
    );
}

#[test]
fn short_pools_fall_back_to_interactive() {
    let mut log = pooled_log(1); // each member needs 2 per proposal
    log.push(0, &proposal(1));
    let s = log.replay().unwrap();
    assert!(s.proposals[&[1; 16]].preprocessed.is_none());
    for m in 0..3 {
        assert_eq!(s.pools[&log.ids[m].public().sig_pk].available(), 1);
    }
}

#[test]
fn share_votes_complete_a_group_and_are_final() {
    let mut log = pooled_log(2);
    log.push(0, &proposal(1));
    // Groups: 0 = {m0,m1}, 1 = {m0,m2}, 2 = {m1,m2} (descriptor order).
    log.push(0, &share_vote(1, &[0, 1]));
    let s = log.replay().unwrap();
    assert_eq!(s.proposals[&[1; 16]].ready_group, None);
    log.push(2, &share_vote(1, &[1, 2]));
    let s = log.replay().unwrap();
    let p = &s.proposals[&[1; 16]];
    assert_eq!(p.status, ProposalStatus::Approved);
    assert_eq!(p.ready_group, Some(1));
    assert_eq!(p.completed_by, Some(log.ids[2].public().sig_pk));
    // Member 2 can no longer withdraw: the shares are out.
    log.push(2, &vote(1, false));
    let s = log.replay().unwrap();
    assert!(s
        .ignored
        .iter()
        .any(|(_, e)| matches!(e, VaultError::VoteFinal(_))));
    assert_eq!(s.proposals[&[1; 16]].status, ProposalStatus::Approved);
}

#[test]
fn invalid_share_votes_are_ignored() {
    let mut log = pooled_log(2);
    log.push(0, &proposal(1));
    log.push(0, &share_vote(1, &[2])); // member 0 is not in group 2 = {m1,m2}
    log.push(0, &share_vote(1, &[0, 0])); // the same group twice
    log.push(0, &share_vote(1, &[7])); // no such group
    let s = log.replay().unwrap();
    assert_eq!(
        s.ignored
            .iter()
            .filter(|(_, e)| matches!(e, VaultError::BadShares(_)))
            .count(),
        3
    );
    assert!(s.proposals[&[1; 16]].shares.is_empty());
}

#[test]
fn garbage_commitments_are_rejected() {
    let mut log = Log::new();
    log.push(0, &log.created(&[0, 1, 2]));
    log.push(
        1,
        &VaultEvent::Commitments {
            batch: vec![vec![1, 2, 3]],
        },
    );
    let s = log.replay().unwrap();
    assert!(matches!(s.ignored[0].1, VaultError::BadCommitments(_)));
    assert!(s.pools.is_empty() || s.pools.values().all(|p| p.commitments.is_empty()));
}
