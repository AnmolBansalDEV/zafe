//! Zafe relay (spec §6): a blind store-and-forward server.
//!
//! It holds mailboxes, member public keys, signed envelopes (sealed ones are opaque), and
//! the encrypted vault log. It checks signatures, membership, sequence numbers and log
//! chaining, and never has keys that decrypt anything. M0 keeps state in memory.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use zafe_proto::{
    log::Chain,
    relay::{
        decode_body, encode_body, join_token_hash, AppendResult, CreateMailbox, InboxRead,
        InboxResponse, Join, LogRead, LogResponse, MembersRead, MembersResponse, Seal, Signed,
        MAX_REQUEST_SKEW_SECS,
    },
    Envelope, IdentityPublic, LogEntry, MailboxId, Recipient, ReplayGuard,
};

/// Largest number of items returned by one read.
const PAGE: usize = 500;

type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

#[derive(Clone)]
pub struct Relay {
    mailboxes: Arc<Mutex<HashMap<MailboxId, Mailbox>>>,
    clock: Clock,
}

struct Mailbox {
    creator: [u8; 32],
    join_token_hash: [u8; 32],
    sealed: bool,
    members: BTreeMap<[u8; 32], IdentityPublic>,
    /// `(cursor, recipient, envelope)`.
    deliveries: Vec<(u64, [u8; 32], Envelope)>,
    next_cursor: u64,
    replay: ReplayGuard,
    log: Chain,
}

#[derive(Debug, thiserror::Error)]
pub enum RelayError {
    #[error("malformed request")]
    BadRequest,
    #[error("invalid signature")]
    Unauthenticated,
    #[error("not allowed")]
    Forbidden,
    #[error("unknown mailbox")]
    NotFound,
    #[error("mailbox already exists")]
    Exists,
    #[error("request timestamp outside the allowed window")]
    Stale,
    #[error("replayed or reordered envelope")]
    Replay,
}

impl IntoResponse for RelayError {
    fn into_response(self) -> Response {
        let status = match self {
            RelayError::BadRequest => StatusCode::BAD_REQUEST,
            RelayError::Unauthenticated => StatusCode::UNAUTHORIZED,
            RelayError::Forbidden => StatusCode::FORBIDDEN,
            RelayError::NotFound => StatusCode::NOT_FOUND,
            RelayError::Exists | RelayError::Replay => StatusCode::CONFLICT,
            RelayError::Stale => StatusCode::BAD_REQUEST,
        };
        (status, self.to_string()).into_response()
    }
}

type RelayResult = Result<Vec<u8>, RelayError>;

impl Default for Relay {
    fn default() -> Self {
        Self::new()
    }
}

impl Relay {
    pub fn new() -> Self {
        Self::with_clock(Arc::new(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        }))
    }

    /// For tests: a controllable clock (seconds since the Unix epoch).
    pub fn with_clock(clock: Clock) -> Self {
        Self {
            mailboxes: Arc::default(),
            clock,
        }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/v1/mailbox/create", post(create))
            .route("/v1/mailbox/join", post(join))
            .route("/v1/mailbox/seal", post(seal))
            .route("/v1/mailbox/members", post(members))
            .route("/v1/envelope", post(post_envelope))
            .route("/v1/inbox", post(inbox))
            .route("/v1/log/append", post(log_append))
            .route("/v1/log/read", post(log_read))
            .with_state(self)
    }

    fn check_fresh(&self, timestamp: u64) -> Result<(), RelayError> {
        let now = (self.clock)();
        if timestamp.abs_diff(now) > MAX_REQUEST_SKEW_SECS {
            return Err(RelayError::Stale);
        }
        Ok(())
    }
}

fn verified<T>(body: &Bytes) -> Result<Signed<T>, RelayError>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let signed = Signed::<T>::from_bytes(body).map_err(|_| RelayError::BadRequest)?;
    signed.verify().map_err(|_| RelayError::Unauthenticated)?;
    Ok(signed)
}

fn ok<T: serde::Serialize>(value: &T) -> RelayResult {
    encode_body(value).map_err(|_| RelayError::BadRequest)
}

async fn create(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<CreateMailbox>(&body)?;
    let mut boxes = relay.mailboxes.lock().expect("lock");
    if boxes.contains_key(&req.payload.mailbox) {
        return Err(RelayError::Exists);
    }
    let creator = req.signer;
    boxes.insert(
        req.payload.mailbox,
        Mailbox {
            creator: creator.sig_pk,
            join_token_hash: req.payload.join_token_hash,
            sealed: false,
            members: BTreeMap::from([(creator.sig_pk, creator)]),
            deliveries: Vec::new(),
            next_cursor: 1,
            replay: ReplayGuard::default(),
            log: Chain::new(req.payload.mailbox),
        },
    );
    ok(&())
}

async fn join(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<Join>(&body)?;
    let mut boxes = relay.mailboxes.lock().expect("lock");
    let mb = boxes
        .get_mut(&req.payload.mailbox)
        .ok_or(RelayError::NotFound)?;
    if mb.sealed || join_token_hash(&req.payload.join_token) != mb.join_token_hash {
        return Err(RelayError::Forbidden);
    }
    mb.members.insert(req.signer.sig_pk, req.signer);
    ok(&())
}

async fn seal(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<Seal>(&body)?;
    let mut boxes = relay.mailboxes.lock().expect("lock");
    let mb = boxes
        .get_mut(&req.payload.mailbox)
        .ok_or(RelayError::NotFound)?;
    if req.signer.sig_pk != mb.creator || mb.sealed {
        return Err(RelayError::Forbidden);
    }
    let mut wanted = req.payload.members.clone();
    wanted.sort();
    wanted.dedup();
    let current: Vec<[u8; 32]> = mb.members.keys().copied().collect();
    if wanted != current {
        return Err(RelayError::Forbidden);
    }
    mb.sealed = true;
    ok(&())
}

async fn members(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<MembersRead>(&body)?;
    relay.check_fresh(req.payload.timestamp)?;
    let boxes = relay.mailboxes.lock().expect("lock");
    let mb = boxes
        .get(&req.payload.mailbox)
        .ok_or(RelayError::NotFound)?;
    if !mb.members.contains_key(&req.signer.sig_pk) {
        return Err(RelayError::Forbidden);
    }
    ok(&MembersResponse {
        members: mb.members.values().copied().collect(),
        sealed: mb.sealed,
    })
}

async fn post_envelope(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let envelope = Envelope::from_bytes(&body).map_err(|_| RelayError::BadRequest)?;
    let mut boxes = relay.mailboxes.lock().expect("lock");
    let mb = boxes
        .get_mut(&envelope.header.mailbox)
        .ok_or(RelayError::NotFound)?;
    let sender = *mb
        .members
        .get(&envelope.header.from)
        .ok_or(RelayError::Forbidden)?;
    envelope
        .verify(&sender)
        .map_err(|_| RelayError::Unauthenticated)?;

    let recipients: Vec<[u8; 32]> = match envelope.header.to {
        Recipient::One(pk) if mb.members.contains_key(&pk) && pk != sender.sig_pk => vec![pk],
        Recipient::One(_) => return Err(RelayError::Forbidden),
        Recipient::All => mb
            .members
            .keys()
            .filter(|pk| **pk != sender.sig_pk)
            .copied()
            .collect(),
    };
    mb.replay
        .check_and_record(&envelope.header)
        .map_err(|_| RelayError::Replay)?;
    for recipient in recipients {
        let cursor = mb.next_cursor;
        mb.next_cursor += 1;
        mb.deliveries.push((cursor, recipient, envelope.clone()));
    }
    ok(&())
}

async fn inbox(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<InboxRead>(&body)?;
    relay.check_fresh(req.payload.timestamp)?;
    let boxes = relay.mailboxes.lock().expect("lock");
    let mb = boxes
        .get(&req.payload.mailbox)
        .ok_or(RelayError::NotFound)?;
    if !mb.members.contains_key(&req.signer.sig_pk) {
        return Err(RelayError::Forbidden);
    }
    let envelopes = mb
        .deliveries
        .iter()
        .filter(|(cursor, to, _)| *cursor > req.payload.after && *to == req.signer.sig_pk)
        .take(PAGE)
        .map(|(cursor, _, env)| (*cursor, env.clone()))
        .collect();
    ok(&InboxResponse { envelopes })
}

async fn log_append(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let entry: LogEntry = decode_body(&body).map_err(|_| RelayError::BadRequest)?;
    let mut boxes = relay.mailboxes.lock().expect("lock");
    let mb = boxes
        .get_mut(&entry.header.mailbox)
        .ok_or(RelayError::NotFound)?;
    if !mb.sealed {
        return Err(RelayError::Forbidden);
    }
    let members: Vec<IdentityPublic> = mb.members.values().copied().collect();
    if !mb.members.contains_key(&entry.header.author) {
        return Err(RelayError::Forbidden);
    }
    let index = entry.header.index;
    if index != mb.log.len() || entry.header.prev_hash != mb.log.head() {
        return ok(&AppendResult::Conflict { len: mb.log.len() });
    }
    mb.log
        .append(entry, &members)
        .map_err(|_| RelayError::Unauthenticated)?;
    ok(&AppendResult::Appended { index })
}

async fn log_read(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<LogRead>(&body)?;
    relay.check_fresh(req.payload.timestamp)?;
    let boxes = relay.mailboxes.lock().expect("lock");
    let mb = boxes
        .get(&req.payload.mailbox)
        .ok_or(RelayError::NotFound)?;
    if !mb.members.contains_key(&req.signer.sig_pk) {
        return Err(RelayError::Forbidden);
    }
    let entries = mb
        .log
        .entries()
        .iter()
        .skip(usize::try_from(req.payload.from).unwrap_or(usize::MAX))
        .take(PAGE)
        .cloned()
        .collect();
    ok(&LogResponse { entries })
}
