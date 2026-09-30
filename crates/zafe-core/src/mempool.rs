//! Pending vault transactions from lightwalletd's mempool.
//!
//! Block sync only learns of a transaction once it is mined. While the app is open, this
//! watcher reads lightwalletd's `GetMempoolStream`, trial-decrypts every transaction with
//! the vault's viewing key and hands the vault's own to a `store` callback (the app stores
//! them unmined in the wallet database, so an incoming payment shows as pending right
//! away). Privacy: the whole mempool is streamed; no transaction is ever looked up by id.
//!
//! lightwalletd closes the stream whenever a block is mined: that is normal, and the
//! watcher reconnects after [`BLOCK_RECONNECT`]. Real failures back off along
//! [`Backoff`] (1 s doubling to 30 s). Cancellation is polled every [`CANCEL_POLL`], also
//! while connecting, reading or waiting, so a stop takes effect within that time.
//!
//! Trial decryption needs only the viewing key, so the wallet database is touched only
//! for the vault's own transactions: callers take their wallet lock inside `store`, never
//! while the stream is idle.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};

use zcash_client_backend::{decrypt_transaction, proto::service};
use zcash_keys::keys::UnifiedFullViewingKey;
pub use zcash_primitives::transaction::{Transaction, TxId};
use zcash_protocol::consensus::{BlockHeight, BranchId, Parameters};

use crate::wallet::{connect, latest_height, WalletError};

/// Delay before reconnecting after lightwalletd closed the stream (a new block).
pub const BLOCK_RECONNECT: Duration = Duration::from_secs(1);
/// How often a running watch checks whether it was cancelled.
pub const CANCEL_POLL: Duration = Duration::from_millis(100);
/// Transactions remembered as already handled (the stream resends the whole mempool after
/// every reconnect).
const SEEN_CAPACITY: usize = 4096;

#[derive(Debug)]
pub enum MempoolEvent {
    /// The stream is open (after every (re)connect).
    Connected,
    /// A vault transaction from the mempool was handed to `store` successfully.
    Stored { txid: TxId },
    /// `store` failed for a vault transaction; it is tried again after the next reconnect.
    StoreFailed { txid: TxId, error: WalletError },
    /// Connecting or reading failed; the watch retries after `retry_in`.
    Disconnected {
        error: WalletError,
        retry_in: Duration,
    },
}

/// Delays between retries after consecutive failures: 1 s, doubling, capped at 30 s.
#[derive(Debug, Default)]
pub struct Backoff {
    failures: u32,
}

impl Backoff {
    pub const FIRST: Duration = Duration::from_secs(1);
    pub const MAX: Duration = Duration::from_secs(30);

    /// Records a failure and returns how long to wait before the next attempt.
    pub fn failed(&mut self) -> Duration {
        let delay = Self::FIRST
            .saturating_mul(1u32 << self.failures.min(16))
            .min(Self::MAX);
        self.failures = self.failures.saturating_add(1);
        delay
    }

    /// The connection worked: the next failure starts the ladder again.
    pub fn reset(&mut self) {
        self.failures = 0;
    }
}

/// A bounded set of txids, forgetting the oldest first.
#[derive(Debug)]
struct SeenTxids {
    capacity: usize,
    order: VecDeque<TxId>,
    set: HashSet<TxId>,
}

impl SeenTxids {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: VecDeque::new(),
            set: HashSet::new(),
        }
    }

    fn contains(&self, txid: &TxId) -> bool {
        self.set.contains(txid)
    }

    fn insert(&mut self, txid: TxId) {
        if self.capacity == 0 || !self.set.insert(txid) {
            return;
        }
        self.order.push_back(txid);
        while self.order.len() > self.capacity {
            if let Some(old) = self.order.pop_front() {
                self.set.remove(&old);
            }
        }
    }
}

/// Parses a mempool transaction as it would be mined in the block after `tip`.
pub fn parse_mempool_tx<P: Parameters>(params: &P, tip: u32, data: &[u8]) -> Option<Transaction> {
    let branch = BranchId::for_height(params, BlockHeight::from_u32(tip.saturating_add(1)));
    Transaction::read(data, branch).ok()
}

/// Whether any output of `tx` decrypts with the vault's viewing key (a payment to the
/// vault, or one the vault sent, recovered with its outgoing key).
pub fn is_vault_tx<P: Parameters>(
    params: &P,
    tip: u32,
    tx: &Transaction,
    ufvk: &UnifiedFullViewingKey,
) -> bool {
    let ufvks = HashMap::from([((), ufvk.clone())]);
    decrypt_transaction(params, None, Some(BlockHeight::from_u32(tip)), tx, &ufvks)
        .has_decrypted_outputs()
}

/// Resolves once `cancelled()` returns true.
async fn until_cancelled(cancelled: &impl Fn() -> bool) {
    while !cancelled() {
        tokio::time::sleep(CANCEL_POLL).await;
    }
}

/// Sleeps for `delay`; false if cancelled meanwhile.
async fn sleep_unless_cancelled(delay: Duration, cancelled: &impl Fn() -> bool) -> bool {
    tokio::select! {
        biased;
        _ = until_cancelled(cancelled) => false,
        _ = tokio::time::sleep(delay) => true,
    }
}

/// Watches lightwalletd's mempool at `endpoint` until `cancelled()` returns true, calling
/// `store` (on a blocking thread) for each new transaction of the vault with viewing key
/// `ufvk`, and `emit` for every [`MempoolEvent`]. Network errors never end the watch.
pub async fn watch<P, C, S, E>(
    endpoint: &str,
    params: &P,
    ufvk: &UnifiedFullViewingKey,
    cancelled: C,
    store: S,
    mut emit: E,
) where
    P: Parameters,
    C: Fn() -> bool,
    S: Fn(Transaction) -> Result<(), WalletError> + Send + Sync + 'static,
    E: FnMut(MempoolEvent),
{
    let store = Arc::new(store);
    let mut backoff = Backoff::default();
    let mut seen = SeenTxids::new(SEEN_CAPACITY);
    loop {
        if cancelled() {
            return;
        }
        let open = async {
            let mut client = connect(endpoint).await?;
            // Mempool transactions are decrypted as if mined in the next block.
            let tip = latest_height(&mut client).await?;
            let stream = client
                .get_mempool_stream(service::Empty {})
                .await
                .map_err(|s| crate::wallet::status_error(&s))?
                .into_inner();
            Ok::<_, WalletError>((tip, stream))
        };
        let opened = tokio::select! {
            biased;
            _ = until_cancelled(&cancelled) => return,
            r = open => r,
        };
        let (tip, mut stream) = match opened {
            Ok(x) => x,
            Err(error) => {
                let retry_in = backoff.failed();
                emit(MempoolEvent::Disconnected { error, retry_in });
                if !sleep_unless_cancelled(retry_in, &cancelled).await {
                    return;
                }
                continue;
            }
        };
        emit(MempoolEvent::Connected);

        let ended = loop {
            let message = tokio::select! {
                biased;
                _ = until_cancelled(&cancelled) => return,
                m = stream.message() => m,
            };
            let raw = match message {
                Ok(Some(raw)) => raw,
                Ok(None) => break Ok(()),
                Err(s) => break Err(crate::wallet::status_error(&s)),
            };
            // A stream that delivers works: only failures in a row back off further.
            backoff.reset();
            let Some(tx) = parse_mempool_tx(params, tip, &raw.data) else {
                continue;
            };
            let txid = tx.txid();
            if seen.contains(&txid) {
                continue;
            }
            if !is_vault_tx(params, tip, &tx, ufvk) {
                seen.insert(txid);
                continue;
            }
            let store = store.clone();
            let stored = tokio::select! {
                biased;
                _ = until_cancelled(&cancelled) => return,
                r = tokio::task::spawn_blocking(move || store(tx)) => r,
            };
            match stored {
                Ok(Ok(())) => {
                    seen.insert(txid);
                    emit(MempoolEvent::Stored { txid });
                }
                // Not remembered: the stream resends it after the next reconnect.
                Ok(Err(error)) => emit(MempoolEvent::StoreFailed { txid, error }),
                Err(e) => emit(MempoolEvent::StoreFailed {
                    txid,
                    error: WalletError::Db(e.to_string()),
                }),
            }
        };
        drop(stream);
        let delay = match ended {
            // Closed by lightwalletd: a new block was mined.
            Ok(()) => {
                backoff.reset();
                BLOCK_RECONNECT
            }
            Err(error) => {
                let retry_in = backoff.failed();
                emit(MempoolEvent::Disconnected { error, retry_in });
                retry_in
            }
        };
        if !sleep_unless_cancelled(delay, &cancelled).await {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::Instant,
    };

    use super::*;
    use crate::keys::{VaultKeys, VaultSecret};

    #[test]
    fn backoff_doubles_to_thirty_seconds_and_resets() {
        let mut b = Backoff::default();
        let secs: Vec<u64> = (0..8).map(|_| b.failed().as_secs()).collect();
        assert_eq!(secs, [1, 2, 4, 8, 16, 30, 30, 30]);
        b.reset();
        assert_eq!(b.failed(), Backoff::FIRST);
        // Many failures never overflow.
        for _ in 0..100 {
            assert!(b.failed() <= Backoff::MAX);
        }
    }

    #[test]
    fn seen_txids_forget_the_oldest() {
        let id = |b: u8| TxId::from_bytes([b; 32]);
        let mut seen = SeenTxids::new(2);
        seen.insert(id(1));
        seen.insert(id(2));
        seen.insert(id(2));
        assert!(seen.contains(&id(1)) && seen.contains(&id(2)));
        seen.insert(id(3));
        assert!(!seen.contains(&id(1)));
        assert!(seen.contains(&id(2)) && seen.contains(&id(3)));
    }

    fn ufvk() -> UnifiedFullViewingKey {
        use orchard::keys::{FullViewingKey, SpendingKey};
        let ak: [u8; 32] = FullViewingKey::from(&SpendingKey::from_bytes([3u8; 32]).unwrap())
            .to_bytes()[..32]
            .try_into()
            .unwrap();
        VaultKeys::derive(&VaultSecret::from_bytes([9u8; 32]), &ak)
            .unwrap()
            .ufvk()
            .unwrap()
    }

    #[tokio::test]
    async fn cancelled_before_start_returns_without_events() {
        let mut events = 0;
        watch(
            "http://127.0.0.1:1",
            &zcash_protocol::consensus::Network::TestNetwork,
            &ufvk(),
            || true,
            |_| Ok(()),
            |_| events += 1,
        )
        .await;
        assert_eq!(events, 0);
    }

    #[tokio::test]
    async fn unreachable_server_backs_off_and_stops_on_cancel() {
        let cancel = AtomicBool::new(false);
        let mut retries = Vec::new();
        let started = Instant::now();
        // Nothing listens on port 1: the connection is refused at once.
        watch(
            "http://127.0.0.1:1",
            &zcash_protocol::consensus::Network::TestNetwork,
            &ufvk(),
            || cancel.load(Ordering::Relaxed),
            |_| Ok(()),
            |e| match e {
                MempoolEvent::Disconnected { retry_in, .. } => {
                    retries.push(retry_in);
                    // Stop during the second wait (2 s): it must not be waited out.
                    if retries.len() == 2 {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
                other => panic!("unexpected {other:?}"),
            },
        )
        .await;
        assert_eq!(retries, [Duration::from_secs(1), Duration::from_secs(2)]);
        // 1 s of backoff plus two refused connections; the 2 s wait was cut short.
        assert!(started.elapsed() < Duration::from_millis(2500));
    }
}
