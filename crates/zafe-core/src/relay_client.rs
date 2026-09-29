//! Client for the Zafe relay (spec §6). Signs every request with the member identity.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{de::DeserializeOwned, Serialize};
use zafe_proto::{
    relay::{
        decode_body, encode_body, join_token_hash, AppendResult, CreateMailbox, InboxRead,
        InboxResponse, Join, LogRead, LogResponse, MembersRead, MembersResponse, Remove, Seal,
        Signed,
    },
    Envelope, Identity, LogEntry, MailboxId,
};

#[derive(Debug, thiserror::Error)]
pub enum RelayClientError {
    #[error("relay returned {status}: {body}")]
    Status { status: u16, body: String },
    #[error("transport: {0}")]
    Transport(String),
    #[error("encoding")]
    Encoding,
}

#[derive(Clone)]
pub struct RelayClient {
    base: String,
    http: reqwest::Client,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl RelayClient {
    /// `base` is the relay URL, e.g. `http://127.0.0.1:8787`.
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        }
    }

    async fn post(&self, path: &str, body: Vec<u8>) -> Result<Vec<u8>, RelayClientError> {
        let response = self
            .http
            .post(format!("{}{path}", self.base))
            .header("content-type", "application/octet-stream")
            .body(body)
            .send()
            .await
            .map_err(|e| RelayClientError::Transport(e.to_string()))?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| RelayClientError::Transport(e.to_string()))?;
        if !status.is_success() {
            return Err(RelayClientError::Status {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }
        Ok(bytes.to_vec())
    }

    async fn signed<T, R>(
        &self,
        path: &str,
        who: &Identity,
        payload: T,
    ) -> Result<R, RelayClientError>
    where
        T: Serialize + DeserializeOwned,
        R: DeserializeOwned,
    {
        let body = Signed::new(who, payload)
            .and_then(|s| s.to_bytes())
            .map_err(|_| RelayClientError::Encoding)?;
        decode_body(&self.post(path, body).await?).map_err(|_| RelayClientError::Encoding)
    }

    pub async fn create_mailbox(
        &self,
        creator: &Identity,
        mailbox: MailboxId,
        join_token: &[u8; 32],
        max_members: u16,
    ) -> Result<(), RelayClientError> {
        let payload = CreateMailbox {
            max_members,
            mailbox,
            join_token_hash: join_token_hash(join_token),
        };
        self.signed("/v1/mailbox/create", creator, payload).await
    }

    pub async fn join(
        &self,
        member: &Identity,
        mailbox: MailboxId,
        join_token: [u8; 32],
    ) -> Result<(), RelayClientError> {
        self.signed(
            "/v1/mailbox/join",
            member,
            Join {
                mailbox,
                join_token,
            },
        )
        .await
    }

    pub async fn seal(
        &self,
        creator: &Identity,
        mailbox: MailboxId,
        members: Vec<[u8; 32]>,
    ) -> Result<(), RelayClientError> {
        self.signed("/v1/mailbox/seal", creator, Seal { mailbox, members })
            .await
    }

    /// Creator only, before sealing: removes a joined member.
    pub async fn remove(
        &self,
        creator: &Identity,
        mailbox: MailboxId,
        member: [u8; 32],
    ) -> Result<(), RelayClientError> {
        self.signed("/v1/mailbox/remove", creator, Remove { mailbox, member })
            .await
    }

    pub async fn members(
        &self,
        who: &Identity,
        mailbox: MailboxId,
    ) -> Result<MembersResponse, RelayClientError> {
        self.signed(
            "/v1/mailbox/members",
            who,
            MembersRead {
                mailbox,
                timestamp: now(),
            },
        )
        .await
    }

    pub async fn send(&self, envelope: &Envelope) -> Result<(), RelayClientError> {
        let body = envelope
            .to_bytes()
            .map_err(|_| RelayClientError::Encoding)?;
        decode_body(&self.post("/v1/envelope", body).await?).map_err(|_| RelayClientError::Encoding)
    }

    /// Envelopes delivered to `who` after cursor `after`, oldest first.
    pub async fn inbox(
        &self,
        who: &Identity,
        mailbox: MailboxId,
        after: u64,
    ) -> Result<Vec<(u64, Envelope)>, RelayClientError> {
        let response: InboxResponse = self
            .signed(
                "/v1/inbox",
                who,
                InboxRead {
                    mailbox,
                    after,
                    timestamp: now(),
                },
            )
            .await?;
        Ok(response.envelopes)
    }

    pub async fn append_log(&self, entry: &LogEntry) -> Result<AppendResult, RelayClientError> {
        let body = encode_body(entry).map_err(|_| RelayClientError::Encoding)?;
        decode_body(&self.post("/v1/log/append", body).await?)
            .map_err(|_| RelayClientError::Encoding)
    }

    pub async fn read_log(
        &self,
        who: &Identity,
        mailbox: MailboxId,
        from: u64,
    ) -> Result<Vec<LogEntry>, RelayClientError> {
        let response: LogResponse = self
            .signed(
                "/v1/log/read",
                who,
                LogRead {
                    mailbox,
                    from,
                    timestamp: now(),
                },
            )
            .await?;
        Ok(response.entries)
    }
}
