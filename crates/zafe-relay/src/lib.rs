//! Zafe relay (spec §6): a blind store-and-forward server.
//!
//! It holds mailboxes, member public keys, signed envelopes (sealed ones are opaque), the
//! encrypted vault log and push tokens. It checks signatures, membership, sequence numbers
//! and log chaining, and never has keys that decrypt anything.
//!
//! State is stored in SQLite (a file, or in memory for tests). That keeps the relay a
//! single self-contained binary, which suits self-hosting; the hosted tier can move the
//! same queries to Postgres.
//!
//! Versions: request bodies are tagged (`zafe_proto::version`); a body in a version this
//! relay doesn't speak gets HTTP 426 with `zafe-supported-version`. The database schema
//! version is `PRAGMA user_version` ([`version::RELAY_DB`]); a newer database is refused.

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use rusqlite::{params, Connection, OptionalExtension};
pub mod fcm;

use zafe_proto::{
    log::GENESIS_PREV_HASH,
    relay::{
        encode_body, join_token_hash, AppendResult, CreateMailbox, InboxRead, InboxResponse, Join,
        LogRead, LogResponse, MembersRead, MembersResponse, PushPlatform, RegisterPush, Remove,
        Seal, Signed, MAX_REQUEST_SKEW_SECS, UNSUPPORTED_VERSION_HEADER,
    },
    version::{self, UnsupportedVersion},
    Envelope, IdentityPublic, LogEntry, MailboxId, ProtoError, Recipient,
};

/// Largest number of items returned by one read.
const PAGE: i64 = 500;

/// Undelivered envelopes older than this are pruned (spec §6.1).
pub const DELIVERY_RETENTION_SECS: u64 = 30 * 24 * 3600;

type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// Sends content-free "vault activity" pushes. Real APNs/FCM senders plug in here (M1).
pub trait Notifier: Send + Sync {
    fn notify(&self, platform: PushPlatform, token: &str);
}

/// Default notifier: logs that a push would be sent.
pub struct LogNotifier;

impl Notifier for LogNotifier {
    fn notify(&self, platform: PushPlatform, _token: &str) {
        tracing::debug!(platform = platform.as_str(), "push: vault activity");
    }
}

#[derive(Clone)]
pub struct Relay {
    db: Arc<Mutex<Connection>>,
    clock: Clock,
    notifier: Arc<dyn Notifier>,
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
    #[error("storage error")]
    Storage,
    /// A request in a version this relay doesn't speak (HTTP 426), or a database written
    /// by a newer relay.
    #[error(transparent)]
    UnsupportedVersion(#[from] UnsupportedVersion),
}

/// Keeps version errors apart from malformed requests.
fn bad_request(e: ProtoError) -> RelayError {
    match e {
        ProtoError::UnsupportedVersion(v) => RelayError::UnsupportedVersion(v),
        _ => RelayError::BadRequest,
    }
}

impl From<rusqlite::Error> for RelayError {
    fn from(e: rusqlite::Error) -> Self {
        tracing::error!("storage: {e}");
        RelayError::Storage
    }
}

impl IntoResponse for RelayError {
    fn into_response(self) -> Response {
        if let RelayError::UnsupportedVersion(v) = self {
            return (
                StatusCode::UPGRADE_REQUIRED,
                [(UNSUPPORTED_VERSION_HEADER, v.supported.to_string())],
                self.to_string(),
            )
                .into_response();
        }
        let status = match self {
            RelayError::BadRequest | RelayError::Stale => StatusCode::BAD_REQUEST,
            RelayError::Unauthenticated => StatusCode::UNAUTHORIZED,
            RelayError::Forbidden => StatusCode::FORBIDDEN,
            RelayError::NotFound => StatusCode::NOT_FOUND,
            RelayError::Exists | RelayError::Replay => StatusCode::CONFLICT,
            RelayError::Storage | RelayError::UnsupportedVersion(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        (status, self.to_string()).into_response()
    }
}

type RelayResult = Result<Vec<u8>, RelayError>;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS mailboxes (
    id BLOB PRIMARY KEY,
    creator BLOB NOT NULL,
    join_token_hash BLOB NOT NULL,
    max_members INTEGER NOT NULL,
    sealed INTEGER NOT NULL DEFAULT 0,
    next_cursor INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS members (
    mailbox BLOB NOT NULL,
    sig_pk BLOB NOT NULL,
    enc_pk BLOB NOT NULL,
    PRIMARY KEY (mailbox, sig_pk)
);
CREATE TABLE IF NOT EXISTS deliveries (
    mailbox BLOB NOT NULL,
    cursor INTEGER NOT NULL,
    recipient BLOB NOT NULL,
    envelope BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (mailbox, cursor)
);
CREATE INDEX IF NOT EXISTS deliveries_by_recipient ON deliveries (mailbox, recipient, cursor);
CREATE TABLE IF NOT EXISTS sender_seqs (
    mailbox BLOB NOT NULL,
    sender BLOB NOT NULL,
    last_seq INTEGER NOT NULL,
    PRIMARY KEY (mailbox, sender)
);
CREATE TABLE IF NOT EXISTS log_entries (
    mailbox BLOB NOT NULL,
    idx INTEGER NOT NULL,
    entry BLOB NOT NULL,
    hash BLOB NOT NULL,
    PRIMARY KEY (mailbox, idx)
);
CREATE TABLE IF NOT EXISTS push_tokens (
    mailbox BLOB NOT NULL,
    member BLOB NOT NULL,
    platform TEXT NOT NULL,
    token TEXT NOT NULL,
    PRIMARY KEY (mailbox, member)
);
";

fn system_clock() -> Clock {
    Arc::new(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    })
}

impl Default for Relay {
    fn default() -> Self {
        Self::new()
    }
}

impl Relay {
    /// An in-memory relay (tests, development).
    pub fn new() -> Self {
        Self::with_clock(system_clock())
    }

    /// An in-memory relay with a controllable clock (seconds since the Unix epoch).
    pub fn with_clock(clock: Clock) -> Self {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        Self::from_connection(conn, clock).expect("schema")
    }

    /// A relay persisted in the SQLite file at `path`.
    pub fn open(path: &Path) -> Result<Self, RelayError> {
        Self::open_with_clock(path, system_clock())
    }

    /// A persisted relay with a controllable clock (tests).
    pub fn open_with_clock(path: &Path, clock: Clock) -> Result<Self, RelayError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::from_connection(conn, clock)
    }

    fn from_connection(conn: Connection, clock: Clock) -> Result<Self, RelayError> {
        let found: u16 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let has_tables: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table')",
            [],
            |r| r.get(0),
        )?;
        // Version 0 with tables: written before versioning (unversioned envelopes and log
        // entries), refused; delete it. Future schema bumps migrate from `found` here.
        if found != 0 || has_tables {
            version::check(version::Format::RelayDb, found)?;
        }
        conn.execute_batch(SCHEMA)?;
        conn.pragma_update(None, "user_version", version::RELAY_DB)?;
        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            clock,
            notifier: Arc::new(LogNotifier),
        })
    }

    /// Replaces the push notifier.
    pub fn with_notifier(mut self, notifier: Arc<dyn Notifier>) -> Self {
        self.notifier = notifier;
        self
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/v1/mailbox/create", post(create))
            .route("/v1/mailbox/join", post(join))
            .route("/v1/mailbox/seal", post(seal))
            .route("/v1/mailbox/remove", post(remove))
            .route("/v1/mailbox/members", post(members))
            .route("/v1/push/register", post(register_push))
            .route("/v1/envelope", post(post_envelope))
            .route("/v1/inbox", post(inbox))
            .route("/v1/log/append", post(log_append))
            .route("/v1/log/read", post(log_read))
            .with_state(self)
    }

    /// Deletes deliveries older than [`DELIVERY_RETENTION_SECS`]. Returns how many.
    pub fn prune(&self) -> Result<usize, RelayError> {
        let cutoff = self.now().saturating_sub(DELIVERY_RETENTION_SECS);
        let db = self.db.lock().expect("lock");
        Ok(db.execute(
            "DELETE FROM deliveries WHERE created_at < ?1",
            params![cutoff as i64],
        )?)
    }

    fn now(&self) -> u64 {
        (self.clock)()
    }

    fn check_fresh(&self, timestamp: u64) -> Result<(), RelayError> {
        if timestamp.abs_diff(self.now()) > MAX_REQUEST_SKEW_SECS {
            return Err(RelayError::Stale);
        }
        Ok(())
    }
}

// --- Storage helpers ---------------------------------------------------------------------

struct MailboxRow {
    creator: [u8; 32],
    join_token_hash: [u8; 32],
    max_members: u16,
    sealed: bool,
}

fn arr32(v: Vec<u8>) -> Result<[u8; 32], RelayError> {
    v.try_into().map_err(|_| RelayError::Storage)
}

fn mailbox(db: &Connection, id: &MailboxId) -> Result<MailboxRow, RelayError> {
    let row = db
        .query_row(
            "SELECT creator, join_token_hash, max_members, sealed FROM mailboxes WHERE id = ?1",
            params![&id[..]],
            |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or(RelayError::NotFound)?;
    Ok(MailboxRow {
        creator: arr32(row.0)?,
        join_token_hash: arr32(row.1)?,
        max_members: u16::try_from(row.2).map_err(|_| RelayError::Storage)?,
        sealed: row.3 != 0,
    })
}

fn members_of(db: &Connection, id: &MailboxId) -> Result<Vec<IdentityPublic>, RelayError> {
    let mut stmt =
        db.prepare("SELECT sig_pk, enc_pk FROM members WHERE mailbox = ?1 ORDER BY sig_pk")?;
    let rows = stmt.query_map(params![&id[..]], |r| {
        Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
    })?;
    rows.map(|r| {
        let (sig, enc) = r?;
        Ok(IdentityPublic {
            sig_pk: arr32(sig)?,
            enc_pk: arr32(enc)?,
        })
    })
    .collect()
}

fn member(
    db: &Connection,
    id: &MailboxId,
    pk: &[u8; 32],
) -> Result<Option<IdentityPublic>, RelayError> {
    Ok(members_of(db, id)?.into_iter().find(|m| &m.sig_pk == pk))
}

fn verified<T>(body: &Bytes) -> Result<Signed<T>, RelayError>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let signed = Signed::<T>::from_bytes(body).map_err(bad_request)?;
    signed.verify().map_err(|_| RelayError::Unauthenticated)?;
    Ok(signed)
}

fn ok<T: serde::Serialize>(value: &T) -> RelayResult {
    encode_body(value).map_err(|_| RelayError::BadRequest)
}

// --- Handlers ----------------------------------------------------------------------------

async fn create(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<CreateMailbox>(&body)?;
    let p = &req.payload;
    if p.max_members < 2 {
        return Err(RelayError::BadRequest);
    }
    let mut db = relay.db.lock().expect("lock");
    let tx = db.transaction()?;
    let inserted = tx.execute(
        "INSERT OR IGNORE INTO mailboxes (id, creator, join_token_hash, max_members, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            &p.mailbox[..],
            &req.signer.sig_pk[..],
            &p.join_token_hash[..],
            p.max_members,
            relay.now() as i64
        ],
    )?;
    if inserted == 0 {
        return Err(RelayError::Exists);
    }
    tx.execute(
        "INSERT INTO members (mailbox, sig_pk, enc_pk) VALUES (?1, ?2, ?3)",
        params![
            &p.mailbox[..],
            &req.signer.sig_pk[..],
            &req.signer.enc_pk[..]
        ],
    )?;
    tx.commit()?;
    ok(&())
}

async fn join(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<Join>(&body)?;
    let mut db = relay.db.lock().expect("lock");
    let tx = db.transaction()?;
    let mb = mailbox(&tx, &req.payload.mailbox)?;
    if mb.sealed || join_token_hash(&req.payload.join_token) != mb.join_token_hash {
        return Err(RelayError::Forbidden);
    }
    let current = members_of(&tx, &req.payload.mailbox)?;
    if !current.iter().any(|m| m.sig_pk == req.signer.sig_pk)
        && current.len() >= usize::from(mb.max_members)
    {
        return Err(RelayError::Forbidden);
    }
    tx.execute(
        "INSERT OR REPLACE INTO members (mailbox, sig_pk, enc_pk) VALUES (?1, ?2, ?3)",
        params![
            &req.payload.mailbox[..],
            &req.signer.sig_pk[..],
            &req.signer.enc_pk[..]
        ],
    )?;
    tx.commit()?;
    ok(&())
}

async fn seal(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<Seal>(&body)?;
    let db = relay.db.lock().expect("lock");
    let mb = mailbox(&db, &req.payload.mailbox)?;
    if req.signer.sig_pk != mb.creator || mb.sealed {
        return Err(RelayError::Forbidden);
    }
    let mut wanted = req.payload.members.clone();
    wanted.sort();
    wanted.dedup();
    let current: Vec<[u8; 32]> = members_of(&db, &req.payload.mailbox)?
        .iter()
        .map(|m| m.sig_pk)
        .collect();
    if wanted != current {
        return Err(RelayError::Forbidden);
    }
    db.execute(
        "UPDATE mailboxes SET sealed = 1 WHERE id = ?1",
        params![&req.payload.mailbox[..]],
    )?;
    ok(&())
}

async fn remove(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<Remove>(&body)?;
    let db = relay.db.lock().expect("lock");
    let mb = mailbox(&db, &req.payload.mailbox)?;
    if req.signer.sig_pk != mb.creator || mb.sealed || req.payload.member == mb.creator {
        return Err(RelayError::Forbidden);
    }
    db.execute(
        "DELETE FROM members WHERE mailbox = ?1 AND sig_pk = ?2",
        params![&req.payload.mailbox[..], &req.payload.member[..]],
    )?;
    ok(&())
}

async fn members(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<MembersRead>(&body)?;
    relay.check_fresh(req.payload.timestamp)?;
    let db = relay.db.lock().expect("lock");
    let mb = mailbox(&db, &req.payload.mailbox)?;
    let members = members_of(&db, &req.payload.mailbox)?;
    if !members.iter().any(|m| m.sig_pk == req.signer.sig_pk) {
        return Err(RelayError::Forbidden);
    }
    ok(&MembersResponse {
        members,
        sealed: mb.sealed,
    })
}

async fn register_push(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<RegisterPush>(&body)?;
    if req.payload.token.is_empty() || req.payload.token.len() > 4096 {
        return Err(RelayError::BadRequest);
    }
    let db = relay.db.lock().expect("lock");
    mailbox(&db, &req.payload.mailbox)?;
    if member(&db, &req.payload.mailbox, &req.signer.sig_pk)?.is_none() {
        return Err(RelayError::Forbidden);
    }
    db.execute(
        "INSERT OR REPLACE INTO push_tokens (mailbox, member, platform, token) VALUES (?1, ?2, ?3, ?4)",
        params![&req.payload.mailbox[..], &req.signer.sig_pk[..], req.payload.platform.as_str(), &req.payload.token],
    )?;
    ok(&())
}

async fn post_envelope(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let envelope = Envelope::from_bytes(&body).map_err(bad_request)?;
    let h = &envelope.header;
    let mut db = relay.db.lock().expect("lock");
    let tx = db.transaction()?;
    mailbox(&tx, &h.mailbox)?;
    let members = members_of(&tx, &h.mailbox)?;
    let sender = members
        .iter()
        .find(|m| m.sig_pk == h.from)
        .ok_or(RelayError::Forbidden)?;
    envelope
        .verify(sender)
        .map_err(|_| RelayError::Unauthenticated)?;

    let recipients: Vec<[u8; 32]> = match h.to {
        Recipient::One(pk) if pk != sender.sig_pk && members.iter().any(|m| m.sig_pk == pk) => {
            vec![pk]
        }
        Recipient::One(_) => return Err(RelayError::Forbidden),
        Recipient::All => members
            .iter()
            .map(|m| m.sig_pk)
            .filter(|pk| *pk != sender.sig_pk)
            .collect(),
    };

    // Replay guard: sequence numbers strictly increase per (mailbox, sender).
    let last: Option<i64> = tx
        .query_row(
            "SELECT last_seq FROM sender_seqs WHERE mailbox = ?1 AND sender = ?2",
            params![&h.mailbox[..], &h.from[..]],
            |r| r.get(0),
        )
        .optional()?;
    let seq = i64::try_from(h.seq).map_err(|_| RelayError::BadRequest)?;
    if last.is_some_and(|l| seq <= l) {
        return Err(RelayError::Replay);
    }
    tx.execute(
        "INSERT OR REPLACE INTO sender_seqs (mailbox, sender, last_seq) VALUES (?1, ?2, ?3)",
        params![&h.mailbox[..], &h.from[..], seq],
    )?;

    let now = relay.now() as i64;
    for recipient in &recipients {
        let cursor: i64 = tx.query_row(
            "SELECT next_cursor FROM mailboxes WHERE id = ?1",
            params![&h.mailbox[..]],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO deliveries (mailbox, cursor, recipient, envelope, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&h.mailbox[..], cursor, &recipient[..], &body[..], now],
        )?;
        tx.execute(
            "UPDATE mailboxes SET next_cursor = next_cursor + 1 WHERE id = ?1",
            params![&h.mailbox[..]],
        )?;
    }

    // Collect push tokens for the recipients before releasing the database.
    let pushes = push_tokens(&tx, &h.mailbox, recipients.iter())?;
    tx.commit()?;
    drop(db);
    for (platform, token) in pushes {
        relay.notifier.notify(platform, &token);
    }
    ok(&())
}

/// Registered push tokens of `members` in `mailbox`.
fn push_tokens<'a>(
    tx: &rusqlite::Transaction<'_>,
    mailbox: &MailboxId,
    members: impl Iterator<Item = &'a [u8; 32]>,
) -> Result<Vec<(PushPlatform, String)>, RelayError> {
    let mut stmt =
        tx.prepare("SELECT platform, token FROM push_tokens WHERE mailbox = ?1 AND member = ?2")?;
    let mut out = Vec::new();
    for member in members {
        if let Some((platform, token)) = stmt
            .query_row(params![&mailbox[..], &member[..]], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .optional()?
        {
            let platform = if platform == "apns" {
                PushPlatform::Apns
            } else {
                PushPlatform::Fcm
            };
            out.push((platform, token));
        }
    }
    Ok(out)
}

async fn inbox(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<InboxRead>(&body)?;
    relay.check_fresh(req.payload.timestamp)?;
    let db = relay.db.lock().expect("lock");
    mailbox(&db, &req.payload.mailbox)?;
    if member(&db, &req.payload.mailbox, &req.signer.sig_pk)?.is_none() {
        return Err(RelayError::Forbidden);
    }
    let after = i64::try_from(req.payload.after).unwrap_or(i64::MAX);
    let mut stmt = db.prepare(
        "SELECT cursor, envelope FROM deliveries
         WHERE mailbox = ?1 AND recipient = ?2 AND cursor > ?3 ORDER BY cursor LIMIT ?4",
    )?;
    let rows = stmt.query_map(
        params![
            &req.payload.mailbox[..],
            &req.signer.sig_pk[..],
            after,
            PAGE
        ],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?)),
    )?;
    // Stored as received (`Envelope::to_bytes`); clients decode each one.
    let envelopes = rows
        .map(|row| row.map(|(cursor, bytes)| (cursor as u64, bytes)))
        .collect::<Result<Vec<_>, _>>()?;
    ok(&InboxResponse { envelopes })
}

async fn log_append(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let entry = LogEntry::from_bytes(&body).map_err(bad_request)?;
    let h = &entry.header;
    let mut db = relay.db.lock().expect("lock");
    let tx = db.transaction()?;
    let mb = mailbox(&tx, &h.mailbox)?;
    if !mb.sealed {
        return Err(RelayError::Forbidden);
    }
    let author = member(&tx, &h.mailbox, &h.author)?.ok_or(RelayError::Forbidden)?;

    let len: i64 = tx.query_row(
        "SELECT COUNT(*) FROM log_entries WHERE mailbox = ?1",
        params![&h.mailbox[..]],
        |r| r.get(0),
    )?;
    let head: Option<Vec<u8>> = tx
        .query_row(
            "SELECT hash FROM log_entries WHERE mailbox = ?1 ORDER BY idx DESC LIMIT 1",
            params![&h.mailbox[..]],
            |r| r.get(0),
        )
        .optional()?;
    let head = match head {
        Some(bytes) => arr32(bytes)?,
        None => GENESIS_PREV_HASH,
    };
    if h.index != len as u64 || h.prev_hash != head {
        return ok(&AppendResult::Conflict { len: len as u64 });
    }
    entry
        .verify_signature(&author)
        .map_err(|_| RelayError::Unauthenticated)?;
    let hash = entry.hash().map_err(|_| RelayError::BadRequest)?;
    tx.execute(
        "INSERT INTO log_entries (mailbox, idx, entry, hash) VALUES (?1, ?2, ?3, ?4)",
        params![&h.mailbox[..], len, &body[..], &hash[..]],
    )?;
    // Every other member learns there is vault activity (a proposal, a vote, a send...);
    // the push carries nothing else, the app reads the encrypted log itself.
    let others: Vec<[u8; 32]> = members_of(&tx, &h.mailbox)?
        .iter()
        .map(|m| m.sig_pk)
        .filter(|pk| *pk != h.author)
        .collect();
    let pushes = push_tokens(&tx, &h.mailbox, others.iter())?;
    tx.commit()?;
    drop(db);
    for (platform, token) in pushes {
        relay.notifier.notify(platform, &token);
    }
    ok(&AppendResult::Appended { index: len as u64 })
}

/// Liveness and readiness for load balancers and uptime checks: 200 `ok` when the
/// database answers a trivial query, 503 otherwise. Reveals nothing about mailboxes.
async fn health(State(relay): State<Relay>) -> Response {
    let alive = match relay.db.lock() {
        Ok(db) => db.query_row("SELECT 1", [], |r| r.get::<_, i64>(0)).is_ok(),
        Err(_) => false,
    };
    if alive {
        (StatusCode::OK, "ok").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "unavailable").into_response()
    }
}

async fn log_read(State(relay): State<Relay>, body: Bytes) -> RelayResult {
    let req = verified::<LogRead>(&body)?;
    relay.check_fresh(req.payload.timestamp)?;
    let db = relay.db.lock().expect("lock");
    mailbox(&db, &req.payload.mailbox)?;
    if member(&db, &req.payload.mailbox, &req.signer.sig_pk)?.is_none() {
        return Err(RelayError::Forbidden);
    }
    let from = i64::try_from(req.payload.from).unwrap_or(i64::MAX);
    let mut stmt = db.prepare(
        "SELECT entry FROM log_entries WHERE mailbox = ?1 AND idx >= ?2 ORDER BY idx LIMIT ?3",
    )?;
    let rows = stmt.query_map(params![&req.payload.mailbox[..], from, PAGE], |r| {
        r.get::<_, Vec<u8>>(0)
    })?;
    // Stored as appended (`LogEntry::to_bytes`); clients decode and verify the chain.
    let entries = rows.collect::<Result<Vec<_>, _>>()?;
    ok(&LogResponse { entries })
}
