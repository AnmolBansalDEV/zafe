//! The vault wallet on a member's device (spec §4.1, §8, §9.2).
//!
//! Like a Keystone account in Zodl or Vizor: the device imports the vault UFVK, syncs from
//! lightwalletd, shows balance and history, and builds PCZTs. Spend authorization comes
//! from FROST, never from a local key. The account is imported as `Spending` with no ZIP 32
//! derivation, so the wallet tracks note witnesses (a `ViewOnly` account would not).

use std::{
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use pczt::Pczt;
use rand::rngs::OsRng;
use tonic::transport::Channel;
use zcash_address::ZcashAddress;
use zcash_client_backend::{
    data_api::{
        chain::{error as chain_error, BlockCache, BlockSource},
        scanning::ScanRange,
        wallet::{
            create_pczt_from_proposal,
            input_selection::{GreedyInputSelector, GreedyInputSelectorError, SpendPolicy},
            propose_transfer, ConfirmationsPolicy,
        },
        Account as _, AccountBirthday, AccountPurpose, WalletRead, WalletWrite,
    },
    fees::{standard::MultiOutputChangeStrategy, DustOutputPolicy, SplitPolicy, StandardFeeRule},
    proto::{
        compact_formats::CompactBlock,
        service::{self, compact_tx_streamer_client::CompactTxStreamerClient},
    },
    sync,
    wallet::OvkPolicy,
};
use zcash_client_sqlite::{util::SystemClock, wallet::init::init_wallet_db, AccountUuid, WalletDb};
use zcash_keys::keys::UnifiedFullViewingKey;
use zcash_primitives::transaction::builder::BundlePadding;
use zcash_protocol::{
    consensus::{BlockHeight, Parameters},
    local_consensus::LocalNetwork,
    memo::MemoBytes,
    value::Zatoshis,
    ShieldedPool,
};
use zip321::{Payment, TransactionRequest};

pub type Client = CompactTxStreamerClient<Channel>;

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("wallet database: {0}")]
    Db(String),
    #[error("lightwalletd: {0}")]
    Remote(String),
    #[error("sync: {0}")]
    Sync(String),
    #[error("proposal: {0}")]
    Proposal(String),
    #[error("invalid payment: {0}")]
    Payment(String),
}

fn db_err(e: impl core::fmt::Debug) -> WalletError {
    WalletError::Db(format!("{e:?}"))
}

/// A regtest network with every upgrade through NU6.3 active at height 1, matching
/// `infra/regtest/zakurad.toml.template`.
pub fn regtest_network() -> LocalNetwork {
    let one = Some(BlockHeight::from_u32(1));
    LocalNetwork {
        overwinter: one,
        sapling: one,
        blossom: one,
        heartwood: one,
        canopy: one,
        nu5: one,
        nu6: one,
        nu6_1: one,
        nu6_2: one,
        nu6_3: one,
    }
}

/// Connects to a lightwalletd gRPC endpoint (e.g. `http://127.0.0.1:9067`).
pub async fn connect(endpoint: &str) -> Result<Client, WalletError> {
    let channel = Channel::from_shared(endpoint.to_owned())
        .map_err(|e| WalletError::Remote(e.to_string()))?
        .connect()
        .await
        .map_err(|e| WalletError::Remote(e.to_string()))?;
    Ok(CompactTxStreamerClient::new(channel))
}

/// The chain tip height lightwalletd reports.
pub async fn latest_height(client: &mut Client) -> Result<u32, WalletError> {
    client
        .get_latest_block(service::ChainSpec::default())
        .await
        .map_err(|e| WalletError::Remote(e.to_string()))?
        .into_inner()
        .height
        .try_into()
        .map_err(|_| WalletError::Remote("height out of range".into()))
}

/// A payment in a proposal.
#[derive(Clone, Debug)]
pub struct PaymentRequest {
    /// Encoded Zcash address (unified, or Orchard-receiver-bearing).
    pub address: String,
    pub amount_zat: u64,
    pub memo: Option<MemoBytes>,
}

/// Balances of the vault account, in zatoshis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultBalance {
    pub ironwood_spendable: u64,
    pub ironwood_total: u64,
    pub total: u64,
}

pub struct VaultWallet<P: Parameters + Clone + Send + 'static> {
    db: WalletDb<rusqlite::Connection, P, SystemClock, OsRng>,
    params: P,
    account: AccountUuid,
    cache: MemBlockCache,
}

impl<P: Parameters + Clone + Send + Sync + 'static> VaultWallet<P> {
    /// Creates a wallet database at `path` and imports the vault UFVK with the given
    /// birthday height (the vault's creation height).
    pub async fn create(
        path: &Path,
        params: P,
        name: &str,
        ufvk: &UnifiedFullViewingKey,
        birthday_height: u32,
        client: &mut Client,
    ) -> Result<Self, WalletError> {
        let mut db =
            WalletDb::for_path(path, params.clone(), SystemClock, OsRng).map_err(db_err)?;
        init_wallet_db(&mut db, None).map_err(db_err)?;

        let tip: u32 = client
            .get_latest_block(service::ChainSpec::default())
            .await
            .map_err(|e| WalletError::Remote(e.to_string()))?
            .into_inner()
            .height
            .try_into()
            .map_err(|_| WalletError::Remote("height out of range".into()))?;
        // Sync downloads the chain state at (range start - 1), and lightwalletd reads a
        // height of 0 as "unspecified", so a wallet cannot be born at height 1.
        if birthday_height < 2 {
            return Err(WalletError::Remote(
                "birthday height must be at least 2".into(),
            ));
        }
        let treestate = client
            .get_tree_state(service::BlockId {
                height: u64::from(birthday_height - 1),
                ..Default::default()
            })
            .await
            .map_err(|e| WalletError::Remote(e.to_string()))?
            .into_inner();
        let birthday = AccountBirthday::from_treestate(treestate, Some(BlockHeight::from_u32(tip)))
            .map_err(|e| WalletError::Remote(format!("birthday: {e:?}")))?;

        let account = db
            .import_account_ufvk(
                name,
                ufvk,
                &birthday,
                AccountPurpose::Spending { derivation: None },
                None,
            )
            .map_err(db_err)?
            .id();
        Ok(Self {
            db,
            params,
            account,
            cache: MemBlockCache::default(),
        })
    }

    /// Opens an existing wallet database holding exactly one vault account.
    pub fn open(path: &Path, params: P) -> Result<Self, WalletError> {
        let db = WalletDb::for_path(path, params.clone(), SystemClock, OsRng).map_err(db_err)?;
        let ids = db.get_account_ids().map_err(db_err)?;
        let [account] = ids.as_slice() else {
            return Err(WalletError::Db(format!(
                "expected one account, found {}",
                ids.len()
            )));
        };
        Ok(Self {
            db,
            params,
            account: *account,
            cache: MemBlockCache::default(),
        })
    }

    /// Scans until the wallet is at the chain tip.
    pub async fn sync(&mut self, client: &mut Client) -> Result<(), WalletError> {
        sync::run(client, &self.params, &self.cache, &mut self.db, 1000)
            .await
            .map_err(|e| WalletError::Sync(format!("{e:?}")))
    }

    pub fn chain_height(&self) -> Result<Option<u32>, WalletError> {
        Ok(self.db.chain_height().map_err(db_err)?.map(u32::from))
    }

    pub fn balance(&self) -> Result<VaultBalance, WalletError> {
        let summary = self
            .db
            .get_wallet_summary(ConfirmationsPolicy::default())
            .map_err(db_err)?
            .ok_or_else(|| WalletError::Db("wallet not synced".into()))?;
        let balance = summary
            .account_balances()
            .get(&self.account)
            .ok_or_else(|| WalletError::Db("account missing from summary".into()))?;
        Ok(VaultBalance {
            ironwood_spendable: balance.ironwood_balance().spendable_value().into_u64(),
            ironwood_total: balance.ironwood_balance().total().into_u64(),
            total: balance.total().into_u64(),
        })
    }

    /// Selects notes, builds and IO-finalizes a PCZT paying `payments`, with change back to
    /// the vault in the Ironwood pool. Payment outputs are encrypted with the vault's
    /// outgoing viewing key (`OvkPolicy::Sender`), which member verification requires.
    pub fn propose(&mut self, payments: &[PaymentRequest]) -> Result<Pczt, WalletError> {
        let request = TransactionRequest::new(
            payments
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    Payment::new(
                        ZcashAddress::from_str(&p.address)
                            .map_err(|e| WalletError::Payment(format!("#{i}: {e}")))?,
                        Some(
                            Zatoshis::from_u64(p.amount_zat)
                                .map_err(|e| WalletError::Payment(format!("#{i}: {e:?}")))?,
                        ),
                        p.memo.clone(),
                        None,
                        None,
                        vec![],
                    )
                    .map_err(|e| WalletError::Payment(format!("#{i}: {e:?}")))
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
        .map_err(|e| WalletError::Payment(format!("{e:?}")))?;

        let change = MultiOutputChangeStrategy::new(
            StandardFeeRule::Zip317,
            None,
            ShieldedPool::Ironwood,
            DustOutputPolicy::default(),
            SplitPolicy::single_output(),
        );
        let proposal =
            propose_transfer::<_, _, _, _, zcash_client_sqlite::wallet::commitment_tree::Error>(
                &mut self.db,
                &self.params,
                self.account,
                &GreedyInputSelector::new(),
                &change,
                request,
                ConfirmationsPolicy::default(),
                &SpendPolicy::default(),
                None,
                None,
            )
            .map_err(|e| WalletError::Proposal(format!("{e:?}")))?;

        create_pczt_from_proposal::<
            _,
            _,
            GreedyInputSelectorError,
            _,
            zcash_primitives::transaction::fees::zip317::FeeError,
            _,
        >(
            &mut self.db,
            &self.params,
            self.account,
            OvkPolicy::Sender,
            &proposal,
            None,
            BundlePadding::DEFAULT,
        )
        .map_err(|e| WalletError::Proposal(format!("{e:?}")))
    }
}

/// In-memory compact block cache for `sync::run`.
#[derive(Clone, Default)]
pub struct MemBlockCache(Arc<Mutex<Vec<CompactBlock>>>);

fn height(block: &CompactBlock) -> BlockHeight {
    BlockHeight::from_u32(u32::try_from(block.height).unwrap_or(u32::MAX))
}

#[derive(Debug, thiserror::Error)]
#[error("block cache: {0}")]
pub struct CacheError(String);

impl BlockSource for MemBlockCache {
    type Error = CacheError;

    fn with_blocks<F, WalletErrT>(
        &self,
        from_height: Option<BlockHeight>,
        limit: Option<usize>,
        mut with_block: F,
    ) -> Result<(), chain_error::Error<WalletErrT, Self::Error>>
    where
        F: FnMut(CompactBlock) -> Result<(), chain_error::Error<WalletErrT, Self::Error>>,
    {
        let mut blocks: Vec<CompactBlock> = self
            .0
            .lock()
            .expect("lock")
            .iter()
            .filter(|b| from_height.is_none_or(|h| height(b) >= h))
            .cloned()
            .collect();
        blocks.sort_by_key(|b| b.height);
        for block in blocks.into_iter().take(limit.unwrap_or(usize::MAX)) {
            with_block(block)?;
        }
        Ok(())
    }
}

#[async_trait]
impl BlockCache for MemBlockCache {
    fn get_tip_height(
        &self,
        range: Option<&ScanRange>,
    ) -> Result<Option<BlockHeight>, Self::Error> {
        Ok(self
            .0
            .lock()
            .expect("lock")
            .iter()
            .map(height)
            .filter(|h| range.is_none_or(|r| r.block_range().contains(h)))
            .max())
    }

    async fn read(&self, range: &ScanRange) -> Result<Vec<CompactBlock>, Self::Error> {
        let mut blocks: Vec<CompactBlock> = self
            .0
            .lock()
            .expect("lock")
            .iter()
            .filter(|b| range.block_range().contains(&height(b)))
            .cloned()
            .collect();
        blocks.sort_by_key(|b| b.height);
        Ok(blocks)
    }

    async fn insert(&self, mut compact_blocks: Vec<CompactBlock>) -> Result<(), Self::Error> {
        self.0.lock().expect("lock").append(&mut compact_blocks);
        Ok(())
    }

    async fn delete(&self, range: ScanRange) -> Result<(), Self::Error> {
        self.0
            .lock()
            .expect("lock")
            .retain(|b| !range.block_range().contains(&height(b)));
        Ok(())
    }
}

/// The networks Zafe runs on, selectable at runtime (e.g. from the app's settings).
#[derive(Clone, Copy, Debug)]
pub enum ZafeNetwork {
    Main,
    Test,
    /// Local regtest with every upgrade through NU6.3 at height 1 (`infra/regtest`).
    Regtest(LocalNetwork),
}

impl ZafeNetwork {
    /// "main", "test" or "regtest".
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "main" => Some(Self::Main),
            "test" => Some(Self::Test),
            "regtest" => Some(Self::Regtest(regtest_network())),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Test => "test",
            Self::Regtest(_) => "regtest",
        }
    }
}

impl Parameters for ZafeNetwork {
    fn network_type(&self) -> zcash_protocol::consensus::NetworkType {
        match self {
            Self::Main => zcash_protocol::consensus::MainNetwork.network_type(),
            Self::Test => zcash_protocol::consensus::TestNetwork.network_type(),
            Self::Regtest(n) => n.network_type(),
        }
    }

    fn activation_height(&self, nu: zcash_protocol::consensus::NetworkUpgrade) -> Option<BlockHeight> {
        match self {
            Self::Main => zcash_protocol::consensus::MainNetwork.activation_height(nu),
            Self::Test => zcash_protocol::consensus::TestNetwork.activation_height(nu),
            Self::Regtest(n) => n.activation_height(nu),
        }
    }
}
