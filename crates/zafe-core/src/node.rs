//! Member orchestration over the relay (spec §7, §9): what one member's device does.
//!
//! Transport-level steps are async functions over a [`RelayClient`]. They are written so a
//! CLI can run each step as a separate command, and so the mobile app can reuse them.

use std::{
    collections::BTreeMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use orchard::keys::Scope;
use rand_core::{CryptoRng, RngCore};
use reddsa::frost::redpallas::{
    keys::{dkg, KeyPackage, PublicKeyPackage},
    Identifier,
};
use serde::{Deserialize, Serialize};
use zafe_proto::{
    relay::AppendResult, safety_number, Chain, Envelope, Identity, IdentityPublic, Kind, LogEntry,
    LogKey, MailboxId,
};
use zcash_keys::address::UnifiedAddress;
use zcash_protocol::consensus::Parameters;

use crate::{
    keygen::{member_identifier, KeygenParams, Round1, SkContribution},
    keys::VaultSecret,
    relay_client::{RelayClient, RelayClientError},
    vault::{MemberInfo, VaultDescriptor, VaultEvent, VaultState},
};

#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error(transparent)]
    Relay(#[from] RelayClientError),
    #[error("invalid invite")]
    BadInvite,
    #[error("membership is not ready: {0}")]
    NotReady(String),
    #[error("safety number mismatch: relay shows {actual}, you confirmed {confirmed}")]
    SafetyNumberMismatch { actual: String, confirmed: String },
    #[error("echo mismatch from a member: the relay showed members different round-1 packages")]
    EchoMismatch,
    #[error("timed out waiting for {0}")]
    Timeout(&'static str),
    #[error("protocol: {0}")]
    Protocol(String),
}

fn proto(e: impl core::fmt::Debug) -> NodeError {
    NodeError::Protocol(format!("{e:?}"))
}

/// Everything a new member needs to join (shared out of band as a string or QR code).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invite {
    pub mailbox: MailboxId,
    pub join_token: [u8; 32],
    /// Creator's Ed25519 key, so joiners know whose round-1 message carries the birthday.
    pub creator: [u8; 32],
    pub threshold: u16,
    pub members: u16,
    pub name: String,
}

impl Invite {
    const PREFIX: &'static str = "zafe-invite-v1:";

    pub fn encode(&self) -> String {
        format!(
            "{}{}",
            Self::PREFIX,
            hex::encode(postcard::to_allocvec(self).expect("encodable"))
        )
    }

    pub fn decode(s: &str) -> Result<Self, NodeError> {
        let body = s
            .trim()
            .strip_prefix(Self::PREFIX)
            .ok_or(NodeError::BadInvite)?;
        postcard::from_bytes(&hex::decode(body).map_err(|_| NodeError::BadInvite)?)
            .map_err(|_| NodeError::BadInvite)
    }
}

/// Per-sender envelope sequence numbers: strictly increasing and clock-based, so they stay
/// increasing across separate processes without persistence.
#[derive(Default)]
pub struct SeqCounter(u64);

impl SeqCounter {
    pub fn next_seq(&mut self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);
        self.0 = self.0.saturating_add(1).max(now);
        self.0
    }
}

/// A member's long-lived vault material after creation. `vault_secret`, `key_package` and
/// `log_key` are secrets: on devices they belong in secure storage (spec §14).
#[derive(Clone, Serialize, Deserialize)]
pub struct VaultMaterial {
    pub descriptor: VaultDescriptor,
    pub key_package: Vec<u8>,
    pub public_key_package: Vec<u8>,
    pub vault_secret: [u8; 32],
    pub log_key_epoch: u32,
    pub log_key: [u8; 32],
}

impl core::fmt::Debug for VaultMaterial {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VaultMaterial")
            .field("vault", &self.descriptor.name)
            .field("address", &self.descriptor.address)
            .finish_non_exhaustive()
    }
}

impl VaultMaterial {
    pub fn key_package(&self) -> Result<KeyPackage, NodeError> {
        KeyPackage::deserialize(&self.key_package).map_err(proto)
    }

    pub fn public_key_package(&self) -> Result<PublicKeyPackage, NodeError> {
        PublicKeyPackage::deserialize(&self.public_key_package).map_err(proto)
    }

    pub fn log_key(&self) -> LogKey {
        LogKey::from_bytes(self.log_key_epoch, self.log_key)
    }

    pub fn vault_keys(&self) -> Result<crate::keys::VaultKeys, NodeError> {
        crate::keys::VaultKeys::derive(
            &VaultSecret::from_bytes(self.vault_secret),
            &self.descriptor.group_public_key,
        )
        .map_err(proto)
    }

    pub fn member_identities(&self) -> Vec<IdentityPublic> {
        self.descriptor.members.iter().map(|m| m.identity).collect()
    }
}

// --- Setup: create, join, seal, safety number ------------------------------------------

pub async fn create_vault<R: RngCore + CryptoRng>(
    relay: &RelayClient,
    creator: &Identity,
    name: &str,
    threshold: u16,
    members: u16,
    rng: &mut R,
) -> Result<Invite, NodeError> {
    let mut mailbox = [0u8; 16];
    let mut join_token = [0u8; 32];
    rng.fill_bytes(&mut mailbox);
    rng.fill_bytes(&mut join_token);
    KeygenParams::new(mailbox, threshold, members).map_err(proto)?;
    relay
        .create_mailbox(creator, mailbox, &join_token, members)
        .await?;
    Ok(Invite {
        mailbox,
        join_token,
        creator: creator.public().sig_pk,
        threshold,
        members,
        name: name.to_owned(),
    })
}

pub async fn join_vault(
    relay: &RelayClient,
    member: &Identity,
    invite: &Invite,
) -> Result<(), NodeError> {
    relay
        .join(member, invite.mailbox, invite.join_token)
        .await?;
    Ok(())
}

/// Current members (as the relay reports them), whether membership is sealed, and the
/// safety number to compare out of band.
pub async fn membership(
    relay: &RelayClient,
    who: &Identity,
    invite: &Invite,
) -> Result<(Vec<IdentityPublic>, bool, String), NodeError> {
    let response = relay.members(who, invite.mailbox).await?;
    let number = safety_number(&invite.mailbox, &response.members);
    Ok((response.members, response.sealed, number))
}

/// Creator only: freezes membership once all `n` members have joined.
pub async fn seal(
    relay: &RelayClient,
    creator: &Identity,
    invite: &Invite,
) -> Result<(), NodeError> {
    let (members, _, _) = membership(relay, creator, invite).await?;
    if members.len() != usize::from(invite.members) {
        return Err(NodeError::NotReady(format!(
            "{} of {} members joined",
            members.len(),
            invite.members
        )));
    }
    relay
        .seal(
            creator,
            invite.mailbox,
            members.iter().map(|m| m.sig_pk).collect(),
        )
        .await?;
    Ok(())
}

// --- Key generation over the relay -----------------------------------------------------

#[derive(Serialize, Deserialize)]
struct Round1Msg {
    package: Vec<u8>,
    /// Set by the creator only: the vault birthday height all descriptors use.
    birthday_height: Option<u32>,
}

/// Collects opened envelopes by (kind, sender) from the inbox, keeping the cursor.
struct Inbox<'a> {
    relay: &'a RelayClient,
    me: &'a Identity,
    mailbox: MailboxId,
    members: BTreeMap<[u8; 32], IdentityPublic>,
    cursor: u64,
    received: BTreeMap<(u8, [u8; 32]), Vec<u8>>,
}

fn kind_tag(kind: Kind) -> u8 {
    kind as u8
}

impl Inbox<'_> {
    async fn poll(&mut self) -> Result<(), NodeError> {
        for (cursor, sender, envelope) in read_inbox(
            self.relay,
            self.me,
            self.mailbox,
            &self.members,
            self.cursor,
        )
        .await?
        {
            self.cursor = cursor;
            if let Ok(payload) = envelope.open(self.me, &sender) {
                // First message of each kind from each sender wins; later copies are ignored.
                self.received
                    .entry((kind_tag(envelope.header.kind), envelope.header.from))
                    .or_insert(payload);
            }
        }
        Ok(())
    }

    /// Waits until a message of `kind` has arrived from every sender in `from`.
    async fn wait_all(
        &mut self,
        kind: Kind,
        from: &[[u8; 32]],
        deadline: Instant,
        what: &'static str,
    ) -> Result<BTreeMap<[u8; 32], Vec<u8>>, NodeError> {
        loop {
            self.poll().await?;
            let got: BTreeMap<_, _> = from
                .iter()
                .filter_map(|pk| {
                    self.received
                        .get(&(kind_tag(kind), *pk))
                        .map(|p| (*pk, p.clone()))
                })
                .collect();
            if got.len() == from.len() {
                return Ok(got);
            }
            if Instant::now() > deadline {
                return Err(NodeError::Timeout(what));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

/// Runs the whole key-generation ceremony for this member (spec §7.2), blocking until done.
///
/// `confirmed_safety_number` is what the user compared out of band; the ceremony refuses to
/// start if the relay's member set produces a different one. The creator passes the vault
/// birthday height; other members take it from the creator's round-1 message.
#[allow(clippy::too_many_arguments)]
pub async fn run_keygen<P: Parameters, R: RngCore + CryptoRng>(
    relay: &RelayClient,
    me: &Identity,
    invite: &Invite,
    confirmed_safety_number: &str,
    network: &P,
    network_name: &str,
    creator_birthday_height: Option<u32>,
    rng: &mut R,
    timeout: Duration,
) -> Result<VaultMaterial, NodeError> {
    let deadline = Instant::now() + timeout;
    let mut seq = SeqCounter::default();
    let is_creator = me.public().sig_pk == invite.creator;

    // Membership must be sealed and match what the user confirmed.
    let (members, sealed, number) = membership(relay, me, invite).await?;
    if !sealed || members.len() != usize::from(invite.members) {
        return Err(NodeError::NotReady("membership is not sealed yet".into()));
    }
    if number != confirmed_safety_number.trim() {
        return Err(NodeError::SafetyNumberMismatch {
            actual: number,
            confirmed: confirmed_safety_number.into(),
        });
    }
    let params =
        KeygenParams::new(invite.mailbox, invite.threshold, invite.members).map_err(proto)?;
    let by_pk: BTreeMap<[u8; 32], IdentityPublic> =
        members.iter().map(|m| (m.sig_pk, *m)).collect();
    let others: Vec<[u8; 32]> = by_pk
        .keys()
        .filter(|pk| **pk != me.public().sig_pk)
        .copied()
        .collect();
    let frost_id = |pk: &[u8; 32]| member_identifier(pk, &invite.mailbox).map_err(proto);
    let id_to_pk: BTreeMap<Identifier, [u8; 32]> = by_pk
        .keys()
        .map(|pk| Ok((frost_id(pk)?, *pk)))
        .collect::<Result<_, NodeError>>()?;

    let mut inbox = Inbox {
        relay,
        me,
        mailbox: invite.mailbox,
        members: by_pk.clone(),
        cursor: 0,
        received: BTreeMap::new(),
    };

    // Round 1: broadcast.
    let round1 = Round1::start(params, frost_id(&me.public().sig_pk)?, rng).map_err(proto)?;
    let msg = Round1Msg {
        package: round1.package().serialize().map_err(proto)?,
        birthday_height: if is_creator {
            creator_birthday_height
        } else {
            None
        },
    };
    let payload = postcard::to_allocvec(&msg).map_err(proto)?;
    relay
        .send(
            &Envelope::public(
                me,
                invite.mailbox,
                seq.next_seq(),
                Kind::DkgRound1,
                &payload,
            )
            .map_err(proto)?,
        )
        .await?;

    let r1 = inbox
        .wait_all(Kind::DkgRound1, &others, deadline, "round-1 packages")
        .await?;
    let mut birthday_height = if is_creator {
        creator_birthday_height
    } else {
        None
    };
    let mut received1 = BTreeMap::new();
    for (pk, bytes) in &r1 {
        let m: Round1Msg = postcard::from_bytes(bytes).map_err(proto)?;
        if *pk == invite.creator {
            birthday_height = m.birthday_height;
        }
        received1.insert(
            frost_id(pk)?,
            dkg::round1::Package::deserialize(&m.package).map_err(proto)?,
        );
    }
    let birthday_height =
        birthday_height.ok_or_else(|| NodeError::Protocol("creator sent no birthday".into()))?;

    // Round 2: echo hash (broadcast), round-2 packages and sk contributions (sealed).
    let (round2, outgoing) = round1.advance(received1).map_err(proto)?;
    let echo = round2.echo();
    relay
        .send(
            &Envelope::public(me, invite.mailbox, seq.next_seq(), Kind::DkgEcho, &echo)
                .map_err(proto)?,
        )
        .await?;
    let contribution = SkContribution::generate(rng);
    for (to_id, package) in outgoing {
        let to = by_pk[&id_to_pk[&to_id]];
        let bytes = package.serialize().map_err(proto)?;
        relay
            .send(
                &Envelope::sealed(
                    me,
                    &to,
                    invite.mailbox,
                    seq.next_seq(),
                    Kind::DkgRound2,
                    &bytes,
                    rng,
                )
                .map_err(proto)?,
            )
            .await?;
        relay
            .send(
                &Envelope::sealed(
                    me,
                    &to,
                    invite.mailbox,
                    seq.next_seq(),
                    Kind::SkContribution,
                    contribution.as_bytes(),
                    rng,
                )
                .map_err(proto)?,
            )
            .await?;
    }

    for (_, their_echo) in inbox
        .wait_all(Kind::DkgEcho, &others, deadline, "echo hashes")
        .await?
    {
        if their_echo.as_slice() != echo.as_slice() {
            return Err(NodeError::EchoMismatch);
        }
    }
    let r2 = inbox
        .wait_all(Kind::DkgRound2, &others, deadline, "round-2 packages")
        .await?;
    let received2 = r2
        .iter()
        .map(|(pk, b)| {
            Ok((
                frost_id(pk)?,
                dkg::round2::Package::deserialize(b).map_err(proto)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, NodeError>>()?;
    let dkg_result = round2.finish(received2).map_err(proto)?;

    let sk_msgs = inbox
        .wait_all(Kind::SkContribution, &others, deadline, "sk contributions")
        .await?;
    let mut contributions = BTreeMap::new();
    contributions.insert(frost_id(&me.public().sig_pk)?, contribution);
    for (pk, bytes) in sk_msgs {
        let arr: [u8; 32] = bytes.as_slice().try_into().map_err(proto)?;
        contributions.insert(frost_id(&pk)?, SkContribution::from_bytes(arr));
    }
    let output = dkg_result.finish(&contributions).map_err(proto)?;

    // Descriptor: identical on every member, then signed by all.
    let fvk = output.vault_keys.fvk();
    let address =
        UnifiedAddress::from_receivers(Some(fvk.address_at(0u32, Scope::External)), None, None)
            .ok_or_else(|| NodeError::Protocol("cannot build unified address".into()))?
            .encode(network);
    let descriptor = VaultDescriptor {
        vault_id: invite.mailbox,
        version: 1,
        name: invite.name.clone(),
        network: network_name.to_owned(),
        threshold: invite.threshold,
        members: by_pk
            .values()
            .map(|m| {
                Ok(MemberInfo {
                    identity: *m,
                    frost_id: frost_id(&m.sig_pk)?.serialize(),
                    name: hex::encode(&m.sig_pk[..4]),
                })
            })
            .collect::<Result<_, NodeError>>()?,
        group_public_key: *output.vault_keys.ak(),
        ufvk: output.vault_keys.ufvk().map_err(proto)?.encode(network),
        address,
        use_qsk: true,
        birthday_height,
        epoch: 0,
        transcript_hash: output.transcript_hash,
    };
    let my_sig = me
        .sign(&descriptor.signing_message().map_err(proto)?)
        .to_vec();
    relay
        .send(
            &Envelope::public(
                me,
                invite.mailbox,
                seq.next_seq(),
                Kind::DescriptorSignature,
                &my_sig,
            )
            .map_err(proto)?,
        )
        .await?;
    let mut signatures = vec![(me.public().sig_pk, my_sig)];
    let message = descriptor.signing_message().map_err(proto)?;
    for (pk, sig) in inbox
        .wait_all(
            Kind::DescriptorSignature,
            &others,
            deadline,
            "descriptor signatures",
        )
        .await?
    {
        by_pk[&pk]
            .verify(&message, &sig)
            .map_err(|_| NodeError::Protocol("bad descriptor signature".into()))?;
        signatures.push((pk, sig));
    }

    // Log key: the creator generates it and seals it to every member, then writes the
    // VaultCreated entry. Others wait for both.
    let log_key = if is_creator {
        let key = LogKey::generate(0, rng);
        for pk in &others {
            let mut bytes = 0u32.to_le_bytes().to_vec();
            bytes.extend_from_slice(key.as_bytes());
            relay
                .send(
                    &Envelope::sealed(
                        me,
                        &by_pk[pk],
                        invite.mailbox,
                        seq.next_seq(),
                        Kind::LogKey,
                        &bytes,
                        rng,
                    )
                    .map_err(proto)?,
                )
                .await?;
        }
        let event = VaultEvent::Created {
            descriptor: descriptor.clone(),
            signatures,
        };
        let entry = LogEntry::create(
            me,
            &key,
            invite.mailbox,
            0,
            [0; 32],
            &event.to_bytes().map_err(proto)?,
            rng,
        )
        .map_err(proto)?;
        match relay.append_log(&entry).await? {
            AppendResult::Appended { .. } => {}
            AppendResult::Conflict { len } => {
                return Err(NodeError::Protocol(format!(
                    "log already has {len} entries"
                )))
            }
        }
        key
    } else {
        let msgs = inbox
            .wait_all(Kind::LogKey, &[invite.creator], deadline, "log key")
            .await?;
        let bytes = &msgs[&invite.creator];
        if bytes.len() != 36 {
            return Err(NodeError::Protocol("bad log key".into()));
        }
        let epoch = u32::from_le_bytes(bytes[..4].try_into().expect("4 bytes"));
        LogKey::from_bytes(epoch, bytes[4..].try_into().expect("32 bytes"))
    };

    // Everyone checks the log's first entry is the descriptor they signed.
    let state = loop {
        let entries = relay.read_log(me, invite.mailbox, 0).await?;
        if !entries.is_empty() {
            let mut chain = Chain::new(invite.mailbox);
            for e in entries {
                chain.append(e, &members).map_err(proto)?;
            }
            break VaultState::replay(chain.entries(), &log_key).map_err(proto)?;
        }
        if Instant::now() > deadline {
            return Err(NodeError::Timeout("VaultCreated log entry"));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    };
    if state.descriptor != descriptor {
        return Err(NodeError::Protocol(
            "logged descriptor differs from ours".into(),
        ));
    }

    Ok(VaultMaterial {
        descriptor,
        key_package: output.key_package.serialize().map_err(proto)?,
        public_key_package: output.public_key_package.serialize().map_err(proto)?,
        vault_secret: *output.vault_secret.as_bytes(),
        log_key_epoch: log_key.epoch,
        log_key: *log_key.as_bytes(),
    })
}

// --- Relay reads -------------------------------------------------------------------------

/// Reads every envelope delivered to `me` after `after`, following pagination. Envelopes
/// for another mailbox, from non-members, with bad signatures, or replayed/reordered
/// (non-increasing seq per sender) are dropped. Returns `(cursor, sender, envelope)`.
pub async fn read_inbox(
    relay: &RelayClient,
    me: &Identity,
    mailbox: MailboxId,
    members: &BTreeMap<[u8; 32], IdentityPublic>,
    after: u64,
) -> Result<Vec<(u64, IdentityPublic, Envelope)>, NodeError> {
    let mut out = Vec::new();
    let mut cursor = after;
    let mut guard = ReplayGuard::default();
    loop {
        let page = relay.inbox(me, mailbox, cursor).await?;
        let Some((last, _)) = page.last() else { break };
        cursor = *last;
        for (c, envelope) in page {
            if envelope.header.mailbox != mailbox {
                continue;
            }
            let Some(sender) = members.get(&envelope.header.from) else {
                continue;
            };
            if envelope.verify(sender).is_err() || guard.check_and_record(&envelope.header).is_err()
            {
                continue;
            }
            out.push((c, *sender, envelope));
        }
    }
    Ok(out)
}

// --- Vault log helpers -------------------------------------------------------------------

/// Reads new log entries after the chain's head (following pagination), verifying the chain
/// and applying them to `state`.
async fn catch_up(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
    chain: &mut Chain,
    state: &mut VaultState,
) -> Result<(), NodeError> {
    let members = material.member_identities();
    let key = material.log_key();
    loop {
        let batch = relay
            .read_log(me, material.descriptor.vault_id, chain.len())
            .await?;
        if batch.is_empty() {
            return Ok(());
        }
        for entry in batch {
            chain.append(entry.clone(), &members).map_err(proto)?;
            state.apply_entry(&entry, &key);
        }
    }
}

/// Reads and verifies the whole log, returning the chain and the replayed state.
pub async fn load_state(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
) -> Result<(Chain, VaultState), NodeError> {
    let mailbox = material.descriptor.vault_id;
    let members = material.member_identities();
    let mut chain = Chain::new(mailbox);
    // The first page must contain the Created entry.
    for entry in relay.read_log(me, mailbox, 0).await? {
        chain.append(entry, &members).map_err(proto)?;
    }
    let mut state = VaultState::replay(chain.entries(), &material.log_key()).map_err(proto)?;
    catch_up(relay, me, material, &mut chain, &mut state).await?;
    Ok((chain, state))
}

/// Appends `event` on top of an already-loaded chain and state. The event is checked
/// against the current state first (so members don't write entries everyone will ignore);
/// on a conflict, only the new entries are fetched, the check is repeated, and the append
/// is retried.
pub async fn append_event<R: RngCore + CryptoRng>(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
    chain: &mut Chain,
    state: &mut VaultState,
    event: &VaultEvent,
    rng: &mut R,
) -> Result<u64, NodeError> {
    let bytes = event.to_bytes().map_err(proto)?;
    let key = material.log_key();
    for _ in 0..10 {
        state
            .check(me.public().sig_pk, event)
            .map_err(|e| NodeError::Protocol(format!("event no longer valid: {e}")))?;
        let entry = LogEntry::create(
            me,
            &key,
            material.descriptor.vault_id,
            chain.len(),
            chain.head(),
            &bytes,
            rng,
        )
        .map_err(proto)?;
        match relay.append_log(&entry).await? {
            AppendResult::Appended { index } => {
                chain
                    .append(entry.clone(), &material.member_identities())
                    .map_err(proto)?;
                state.apply_entry(&entry, &key);
                return Ok(index);
            }
            AppendResult::Conflict { .. } => catch_up(relay, me, material, chain, state).await?,
        }
    }
    Err(NodeError::Protocol(
        "could not append after 10 conflicts".into(),
    ))
}

// --- Proposals and signing over the relay ----------------------------------------------

use std::collections::BTreeSet;

use pczt::{
    roles::{prover::Prover, tx_extractor::TransactionExtractor},
    Pczt,
};
use reddsa::frost::redpallas::{
    round1::SigningCommitments, round2::SignatureShare, SigningPackage,
};
use zafe_proto::ReplayGuard;

use crate::{
    session::{
        aggregate_request, pczt_hash, Leader, Member, NonceStore, ProposalId, SigningRequest,
    },
    tx,
    vault::{ProposalStatus, ProposedPayment},
    verify::{verify_pczt, Expectations, Payment, VerifiedTx},
    wallet::{Client, PaymentRequest, VaultWallet},
};
use zcash_protocol::consensus::{BlockHeight, BranchId};

/// Default acceptable distance between the tip and a proposal's expiry height.
pub const MAX_EXPIRY_DELTA: u32 = 100;

#[derive(Serialize, Deserialize)]
struct SigningRequestMsg {
    proposal: ProposalId,
    pczt_hash: [u8; 32],
    signers: Vec<Vec<u8>>,
    packages: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct SharesMsg {
    proposal: ProposalId,
    /// Hash of the encoded signing request these shares answer (binds shares to one round).
    request_hash: [u8; 32],
    shares: Vec<Vec<u8>>,
}

fn hash_request_bytes(bytes: &[u8]) -> [u8; 32] {
    blake2b_simd::Params::new()
        .hash_length(32)
        .personal(b"Zafe_SignRequest")
        .hash(bytes)
        .as_bytes()
        .try_into()
        .expect("32 bytes")
}

/// Hash identifying a signing request (and the round its shares belong to).
pub fn request_hash(request: &SigningRequest) -> Result<[u8; 32], NodeError> {
    Ok(hash_request_bytes(&encode_request(request)?))
}

/// Hash identifying one member's set of round-1 commitments (to avoid reusing them).
pub fn commitments_hash(commitments: &[Vec<u8>]) -> [u8; 32] {
    let mut state = blake2b_simd::Params::new()
        .hash_length(32)
        .personal(b"Zafe_Commitments")
        .to_state();
    for c in commitments {
        state.update(&(c.len() as u32).to_le_bytes());
        state.update(c);
    }
    state.finalize().as_bytes().try_into().expect("32 bytes")
}

/// Decodes a unified (or Orchard-receiver-bearing) address to its Orchard receiver.
pub fn orchard_receiver<P: Parameters>(
    network: &P,
    address: &str,
) -> Result<orchard::Address, NodeError> {
    match zcash_keys::address::Address::decode(network, address) {
        Some(zcash_keys::address::Address::Unified(ua)) => ua
            .orchard()
            .copied()
            .ok_or_else(|| NodeError::Protocol("address has no Orchard receiver".into())),
        _ => Err(NodeError::Protocol(format!(
            "unsupported recipient address {address}"
        ))),
    }
}

/// What this member expects for a proposal, from the log (payments) and its own wallet
/// (chain tip), never from the proposer.
pub fn expectations<P: Parameters>(
    network: &P,
    payments: &[ProposedPayment],
    tip_height: u32,
) -> Result<Expectations, NodeError> {
    Ok(Expectations {
        payments: payments
            .iter()
            .map(|p| {
                Ok(Payment {
                    recipient: orchard_receiver(network, &p.address)?,
                    amount_zat: p.amount_zat,
                    memo: p
                        .memo
                        .as_slice()
                        .try_into()
                        .map_err(|_| NodeError::Protocol("memo must be 512 bytes".into()))?,
                })
            })
            .collect::<Result<_, NodeError>>()?,
        consensus_branch_id: BranchId::for_height(network, BlockHeight::from_u32(tip_height + 1))
            .into(),
        tip_height,
        max_expiry_delta: MAX_EXPIRY_DELTA,
    })
}

fn members_by_pk(material: &VaultMaterial) -> BTreeMap<[u8; 32], IdentityPublic> {
    material
        .member_identities()
        .into_iter()
        .map(|m| (m.sig_pk, m))
        .collect()
}

/// Builds a PCZT for `payments` from this member's wallet and logs it as a proposal.
pub async fn propose<P: Parameters + Clone + Send + Sync + 'static, R: RngCore + CryptoRng>(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
    wallet: &mut VaultWallet<P>,
    payments: &[PaymentRequest],
    rng: &mut R,
) -> Result<ProposalId, NodeError> {
    let pczt = wallet.propose(payments).map_err(proto)?;
    let tip = wallet
        .chain_height()
        .map_err(proto)?
        .ok_or_else(|| NodeError::NotReady("wallet not synced".into()))?;
    let mut id = [0u8; 16];
    rng.fill_bytes(&mut id);
    let event = VaultEvent::Proposal {
        id,
        payments: payments
            .iter()
            .map(|p| ProposedPayment {
                address: p.address.clone(),
                amount_zat: p.amount_zat,
                memo: p
                    .memo
                    .clone()
                    .unwrap_or_else(zcash_protocol::memo::MemoBytes::empty)
                    .as_array()
                    .to_vec(),
            })
            .collect(),
        pczt_hash: pczt_hash(&pczt).map_err(proto)?,
        pczt: pczt.serialize().map_err(proto)?,
        tip_height: tip,
    };
    let (mut chain, mut state) = load_state(relay, me, material).await?;
    append_event(relay, me, material, &mut chain, &mut state, &event, rng).await?;
    Ok(id)
}

fn proposal_pczt(
    state: &VaultState,
    id: &ProposalId,
) -> Result<(Pczt, Vec<ProposedPayment>), NodeError> {
    let p = state
        .proposals
        .get(id)
        .ok_or_else(|| NodeError::Protocol("unknown proposal".into()))?;
    let pczt = Pczt::parse(&p.pczt).map_err(proto)?;
    if pczt_hash(&pczt).map_err(proto)? != p.pczt_hash {
        return Err(NodeError::Protocol(
            "logged PCZT does not match its hash".into(),
        ));
    }
    Ok((pczt, p.payments.clone()))
}

/// Verifies a proposal independently and, if it passes, votes Approve with fresh round-1
/// commitments. Returns what was verified (for display).
#[allow(clippy::too_many_arguments)]
pub async fn approve<P: Parameters, R: RngCore + CryptoRng>(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
    network: &P,
    tip_height: u32,
    proposal: ProposalId,
    store: &mut impl NonceStore,
    rng: &mut R,
) -> Result<VerifiedTx, NodeError> {
    let (mut chain, mut state) = load_state(relay, me, material).await?;
    let (pczt, payments) = proposal_pczt(&state, &proposal)?;
    let expected = expectations(network, &payments, tip_height)?;
    let key_package = material.key_package()?;
    let keys = material.vault_keys()?;
    let member = Member {
        identifier: *key_package.identifier(),
        key_package: &key_package,
        vault_fvk: keys.fvk(),
    };
    let (approval, verified) = member
        .approve(proposal, &pczt, &expected, store, rng)
        .map_err(proto)?;
    let commitments = approval
        .commitments
        .iter()
        .map(|c| c.serialize().map_err(proto))
        .collect::<Result<Vec<_>, _>>()?;
    let event = VaultEvent::Vote {
        proposal,
        pczt_hash: approval.pczt_hash,
        approve: true,
        commitments,
    };
    append_event(relay, me, material, &mut chain, &mut state, &event, rng).await?;
    Ok(verified)
}

/// Votes Reject.
pub async fn reject<R: RngCore + CryptoRng>(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
    proposal: ProposalId,
    rng: &mut R,
) -> Result<(), NodeError> {
    let (mut chain, mut state) = load_state(relay, me, material).await?;
    let pczt_hash = state
        .proposals
        .get(&proposal)
        .ok_or_else(|| NodeError::Protocol("unknown proposal".into()))?
        .pczt_hash;
    let event = VaultEvent::Vote {
        proposal,
        pczt_hash,
        approve: false,
        commitments: vec![],
    };
    append_event(relay, me, material, &mut chain, &mut state, &event, rng).await?;
    Ok(())
}

fn member_by_frost_id(
    material: &VaultMaterial,
    id: &Identifier,
) -> Result<IdentityPublic, NodeError> {
    let bytes = id.serialize();
    material
        .descriptor
        .members
        .iter()
        .find(|m| m.frost_id == bytes)
        .map(|m| m.identity)
        .ok_or_else(|| NodeError::Protocol("unknown FROST identifier".into()))
}

/// A signing request sent by the leader, plus the commitment sets it consumed (the leader
/// must not put them in another request).
pub struct SentRequest {
    pub request: SigningRequest,
    pub used_commitments: Vec<[u8; 32]>,
}

/// Leader: once the proposal is approved, sends sealed signing requests to `threshold`
/// approvers whose current commitments are not in `used` (commitment sets already put in an
/// earlier request). Members re-approve to provide fresh commitments after a failed round.
#[allow(clippy::too_many_arguments)]
pub async fn request_signatures<P: Parameters, R: RngCore + CryptoRng>(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
    network: &P,
    tip_height: u32,
    proposal: ProposalId,
    used: &BTreeSet<[u8; 32]>,
    rng: &mut R,
) -> Result<SentRequest, NodeError> {
    let (_, state) = load_state(relay, me, material).await?;
    let p = state
        .proposals
        .get(&proposal)
        .ok_or_else(|| NodeError::Protocol("unknown proposal".into()))?;
    if p.status != ProposalStatus::Approved {
        return Err(NodeError::NotReady(format!("proposal is {:?}", p.status)));
    }
    let (pczt, payments) = proposal_pczt(&state, &proposal)?;
    let keys = material.vault_keys()?;
    let verified = verify_pczt(
        &pczt,
        keys.fvk(),
        &expectations(network, &payments, tip_height)?,
    )
    .map_err(proto)?;
    let mut leader =
        Leader::new(proposal, &pczt, &verified, material.descriptor.threshold).map_err(proto)?;

    let mut hash_of = BTreeMap::new();
    for (author, commitments) in &p.approvals {
        let hash = commitments_hash(commitments);
        if used.contains(&hash) {
            continue;
        }
        let info = material
            .descriptor
            .member(author)
            .ok_or_else(|| NodeError::Protocol("approval from non-member".into()))?;
        let member = Identifier::deserialize(&info.frost_id).map_err(proto)?;
        let parsed = commitments
            .iter()
            .map(|c| SigningCommitments::deserialize(c).map_err(proto))
            .collect::<Result<Vec<_>, _>>()?;
        leader.add_approval(crate::session::Approval {
            member,
            proposal,
            pczt_hash: p.pczt_hash,
            commitments: parsed,
        });
        hash_of.insert(member, hash);
    }
    let threshold = usize::from(material.descriptor.threshold);
    let chosen: Vec<Identifier> = leader
        .available_signers()
        .into_iter()
        .take(threshold)
        .collect();
    if chosen.len() < threshold {
        return Err(NodeError::NotReady(format!(
            "only {} approver(s) have unused commitments; ask members to approve again",
            chosen.len()
        )));
    }
    let request = leader.request(&chosen).map_err(proto)?;

    let bytes = encode_request(&request)?;
    let mut seq = SeqCounter::default();
    for id in &request.signers {
        let to = member_by_frost_id(material, id)?;
        let env = Envelope::sealed(
            me,
            &to,
            material.descriptor.vault_id,
            seq.next_seq(),
            Kind::SigningRequest,
            &bytes,
            rng,
        )
        .map_err(proto)?;
        relay.send(&env).await?;
    }
    let used_commitments = chosen.iter().map(|id| hash_of[id]).collect();
    Ok(SentRequest {
        request,
        used_commitments,
    })
}

/// Outcome of answering signing requests.
#[derive(Debug, Default)]
pub struct RespondReport {
    pub answered: Vec<ProposalId>,
    /// Requests that were not answered, with the reason (stale, expired, forged, ...).
    pub skipped: Vec<(ProposalId, String)>,
}

/// Member: answers every pending signing request addressed to this member. Each request is
/// re-verified from the log; a bad or stale request is skipped and reported, never fatal.
#[allow(clippy::too_many_arguments)]
pub async fn respond<P: Parameters, R: RngCore + CryptoRng>(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
    network: &P,
    tip_height: u32,
    store: &mut impl NonceStore,
    rng: &mut R,
) -> Result<RespondReport, NodeError> {
    let (_, state) = load_state(relay, me, material).await?;
    let members = members_by_pk(material);
    let key_package = material.key_package()?;
    let keys = material.vault_keys()?;
    let member = Member {
        identifier: *key_package.identifier(),
        key_package: &key_package,
        vault_fvk: keys.fvk(),
    };
    let mut seq = SeqCounter::default();
    let mut report = RespondReport::default();

    for (_, leader, env) in read_inbox(relay, me, material.descriptor.vault_id, &members, 0).await?
    {
        if env.header.kind != Kind::SigningRequest {
            continue;
        }
        let Ok(bytes) = env.open(me, &leader) else {
            continue;
        };
        let Ok(msg) = postcard::from_bytes::<SigningRequestMsg>(&bytes) else {
            continue;
        };
        if !store.contains(&msg.proposal, &msg.pczt_hash) {
            continue; // already answered, or never approved
        }
        let result: Result<Vec<u8>, NodeError> = (|| {
            let (pczt, payments) = proposal_pczt(&state, &msg.proposal)?;
            let request = decode_request(&bytes)?;
            let shares = member
                .sign(
                    &request,
                    &pczt,
                    &expectations(network, &payments, tip_height)?,
                    store,
                )
                .map_err(proto)?;
            let reply = SharesMsg {
                proposal: msg.proposal,
                request_hash: hash_request_bytes(&bytes),
                shares: shares.iter().map(|s| s.serialize()).collect(),
            };
            postcard::to_allocvec(&reply).map_err(proto)
        })();
        match result {
            Ok(reply) => {
                let env = Envelope::sealed(
                    me,
                    &leader,
                    material.descriptor.vault_id,
                    seq.next_seq(),
                    Kind::SignatureShares,
                    &reply,
                    rng,
                )
                .map_err(proto)?;
                relay.send(&env).await?;
                report.answered.push(msg.proposal);
            }
            Err(e) => report.skipped.push((msg.proposal, e.to_string())),
        }
    }
    Ok(report)
}

/// Leader: waits for the shares answering exactly `request`, aggregates (verifying every
/// share), proves, extracts (fully verified), broadcasts via lightwalletd, and logs the
/// broadcast. Returns the txid.
#[allow(clippy::too_many_arguments)]
pub async fn finalize<R: RngCore + CryptoRng>(
    relay: &RelayClient,
    me: &Identity,
    material: &VaultMaterial,
    request: &SigningRequest,
    lightwalletd: &mut Client,
    proving_key: &orchard::circuit::ProvingKey,
    verifying_key: &orchard::circuit::VerifyingKey,
    timeout: Duration,
    rng: &mut R,
) -> Result<[u8; 32], NodeError> {
    let deadline = Instant::now() + timeout;
    let (mut chain, mut state) = load_state(relay, me, material).await?;
    let (pczt, _) = proposal_pczt(&state, &request.proposal)?;
    let members = members_by_pk(material);
    let wanted = request_hash(request)?;

    let mut shares: BTreeMap<Identifier, Vec<SignatureShare>> = BTreeMap::new();
    let mut cursor = 0;
    while shares.len() < request.signers.len() {
        for (c, from, env) in
            read_inbox(relay, me, material.descriptor.vault_id, &members, cursor).await?
        {
            cursor = c;
            if env.header.kind != Kind::SignatureShares {
                continue;
            }
            let Ok(bytes) = env.open(me, &from) else {
                continue;
            };
            let Ok(msg) = postcard::from_bytes::<SharesMsg>(&bytes) else {
                continue;
            };
            if msg.proposal != request.proposal || msg.request_hash != wanted {
                continue; // a share from another proposal or an earlier round
            }
            let Some(info) = material.descriptor.member(&from.sig_pk) else {
                continue;
            };
            let id = Identifier::deserialize(&info.frost_id).map_err(proto)?;
            if !request.signers.contains(&id) {
                continue;
            }
            let parsed = msg
                .shares
                .iter()
                .map(|s| SignatureShare::deserialize(s).map_err(proto))
                .collect::<Result<_, _>>()?;
            shares.insert(id, parsed);
        }
        if shares.len() < request.signers.len() {
            if Instant::now() > deadline {
                return Err(NodeError::Timeout("signature shares"));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    let keys = material.vault_keys()?;
    let spends = tx::spends_to_sign(&pczt, keys.fvk()).map_err(proto)?;
    let signatures = aggregate_request(request, &spends, &shares, &material.public_key_package()?)
        .map_err(proto)?;

    let signed = tx::apply_signatures(pczt, &signatures).map_err(proto)?;
    let proved = Prover::new(signed)
        .create_ironwood_proof(proving_key)
        .map_err(proto)?
        .finish();
    let transaction = TransactionExtractor::new(proved)
        .with_orchard(verifying_key)
        .extract()
        .map_err(proto)?;
    let mut raw = Vec::new();
    transaction.write(&mut raw).map_err(proto)?;
    let reply = lightwalletd
        .send_transaction(zcash_client_backend::proto::service::RawTransaction {
            data: raw,
            height: 0,
        })
        .await
        .map_err(|e| NodeError::Protocol(e.to_string()))?
        .into_inner();
    if reply.error_code != 0 {
        return Err(NodeError::Protocol(format!(
            "broadcast rejected: {}",
            reply.error_message
        )));
    }
    let txid: [u8; 32] = *transaction.txid().as_ref();
    // The transaction is on its way regardless; if the log entry is no longer valid (e.g.
    // the author cancelled meanwhile), report the txid anyway.
    let event = VaultEvent::Broadcast {
        proposal: request.proposal,
        txid,
    };
    if let Err(e) = append_event(relay, me, material, &mut chain, &mut state, &event, rng).await {
        return Err(NodeError::Protocol(format!(
            "broadcast {} but could not log it: {e}",
            hex::encode(txid)
        )));
    }
    Ok(txid)
}

/// Serializes a signing request (e.g. for the leader to keep it between steps).
pub fn encode_request(request: &SigningRequest) -> Result<Vec<u8>, NodeError> {
    let msg = SigningRequestMsg {
        proposal: request.proposal,
        pczt_hash: request.pczt_hash,
        signers: request.signers.iter().map(|id| id.serialize()).collect(),
        packages: request
            .packages
            .iter()
            .map(|p| p.serialize().map_err(proto))
            .collect::<Result<_, _>>()?,
    };
    postcard::to_allocvec(&msg).map_err(proto)
}

pub fn decode_request(bytes: &[u8]) -> Result<SigningRequest, NodeError> {
    let msg: SigningRequestMsg = postcard::from_bytes(bytes).map_err(proto)?;
    Ok(SigningRequest {
        proposal: msg.proposal,
        pczt_hash: msg.pczt_hash,
        signers: msg
            .signers
            .iter()
            .map(|b| Identifier::deserialize(b).map_err(proto))
            .collect::<Result<_, _>>()?,
        packages: msg
            .packages
            .iter()
            .map(|b| SigningPackage::deserialize(b).map_err(proto))
            .collect::<Result<_, _>>()?,
    })
}
