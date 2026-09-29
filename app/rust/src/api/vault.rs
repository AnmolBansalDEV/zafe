//! Vault setup and wallet API for the Flutter app.
//!
//! Keep this surface to primitives and flat structs (the Vizor rule); all Zcash and FROST
//! types stay inside `zafe-core`. Secrets (identity seeds, vault material) cross as bytes so
//! the Dart side can keep them in platform secure storage; nothing here persists secrets.
//! Calls run on FRB's worker threads; async work uses one shared tokio runtime.

use std::{path::PathBuf, sync::OnceLock, time::Duration};

use rand::rngs::OsRng;
use zafe_core::{
    node::{self, Invite, VaultMaterial},
    relay_client::RelayClient,
    wallet::{connect, latest_height, VaultWallet, ZafeNetwork},
};
use zafe_proto::{Identity, IdentitySeeds};

use super::error::ZafeError;

type Result<T, E = ZafeError> = std::result::Result<T, E>;

pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .expect("tokio runtime")
    })
}

pub(crate) fn network(name: &str) -> Result<ZafeNetwork, ZafeError> {
    ZafeNetwork::from_name(name)
        .ok_or_else(|| ZafeError::invalid(format!("unknown network {name}")))
}

pub(crate) fn identity(seeds: &[u8]) -> Result<Identity, ZafeError> {
    if seeds.len() != 64 {
        return Err(ZafeError::invalid("identity seeds must be 64 bytes"));
    }
    Ok(Identity::from_seeds(IdentitySeeds {
        sig_seed: seeds[..32].try_into().expect("32"),
        enc_seed: seeds[32..].try_into().expect("32"),
    }))
}

pub(crate) fn material(bytes: &[u8]) -> Result<VaultMaterial, ZafeError> {
    postcard::from_bytes(bytes).map_err(|_| ZafeError::invalid("invalid vault material"))
}

/// A new member identity. `seeds` (64 bytes) is secret: store it in secure storage.
pub struct IdentityInfo {
    pub seeds: Vec<u8>,
    pub public_key_hex: String,
}

#[flutter_rust_bridge::frb(sync)]
pub fn generate_identity() -> IdentityInfo {
    let id = Identity::generate(&mut OsRng);
    let mut seeds = id.seeds().sig_seed.to_vec();
    seeds.extend_from_slice(&id.seeds().enc_seed);
    IdentityInfo {
        seeds,
        public_key_hex: hex::encode(id.public().sig_pk),
    }
}

/// The public key (hex) of an identity, e.g. to mark "You" in member lists.
#[flutter_rust_bridge::frb(sync)]
pub fn identity_public_key(seeds: Vec<u8>) -> Result<String, ZafeError> {
    Ok(hex::encode(identity(&seeds)?.public().sig_pk))
}

pub struct InviteInfo {
    pub name: String,
    pub threshold: u16,
    pub members: u16,
    pub creator_hex: String,
}

#[flutter_rust_bridge::frb(sync)]
pub fn parse_invite(invite: String) -> Result<InviteInfo, ZafeError> {
    let i = Invite::decode(&invite)?;
    Ok(InviteInfo {
        name: i.name,
        threshold: i.threshold,
        members: i.members,
        creator_hex: hex::encode(i.creator),
    })
}

/// Creates a vault mailbox on the relay and returns the invite string to share.
pub fn create_vault(
    relay_url: String,
    seeds: Vec<u8>,
    name: String,
    threshold: u16,
    members: u16,
) -> Result<String, ZafeError> {
    let me = identity(&seeds)?;
    let relay = RelayClient::new(relay_url);
    let invite = runtime().block_on(node::create_vault(
        &relay, &me, &name, threshold, members, &mut OsRng,
    ))?;
    Ok(invite.encode())
}

pub fn join_vault(relay_url: String, seeds: Vec<u8>, invite: String) -> Result<(), ZafeError> {
    let me = identity(&seeds)?;
    let invite = Invite::decode(&invite)?;
    runtime().block_on(node::join_vault(&RelayClient::new(relay_url), &me, &invite))?;
    Ok(())
}

pub struct MembershipInfo {
    pub members: Vec<String>,
    pub sealed: bool,
    pub safety_number: String,
    pub is_creator: bool,
}

pub fn vault_membership(
    relay_url: String,
    seeds: Vec<u8>,
    invite: String,
) -> Result<MembershipInfo, ZafeError> {
    let me = identity(&seeds)?;
    let invite = Invite::decode(&invite)?;
    let (members, sealed, safety_number) =
        runtime().block_on(node::membership(&RelayClient::new(relay_url), &me, &invite))?;
    Ok(MembershipInfo {
        members: members.iter().map(|m| hex::encode(m.sig_pk)).collect(),
        sealed,
        safety_number,
        is_creator: me.public().sig_pk == invite.creator,
    })
}

/// Creator only: freezes membership once everyone has joined.
pub fn seal_vault(relay_url: String, seeds: Vec<u8>, invite: String) -> Result<(), ZafeError> {
    let me = identity(&seeds)?;
    let invite = Invite::decode(&invite)?;
    runtime().block_on(node::seal(&RelayClient::new(relay_url), &me, &invite))?;
    Ok(())
}

/// Runs key generation. Blocks until every member finishes (or `timeout_secs`). Returns the
/// vault material (secret: store it in secure storage). The creator picks the birthday:
/// `birthday_height`, or lightwalletd's tip + 1 when `None` (the app passes `None`).
#[allow(clippy::too_many_arguments)]
pub fn run_keygen(
    relay_url: String,
    lightwalletd_url: String,
    network_name: String,
    seeds: Vec<u8>,
    invite: String,
    confirmed_safety_number: String,
    timeout_secs: u32,
    birthday_height: Option<u32>,
) -> Result<Vec<u8>, ZafeError> {
    let me = identity(&seeds)?;
    let invite = Invite::decode(&invite)?;
    let net = network(&network_name)?;
    let relay = RelayClient::new(relay_url);
    let material = runtime().block_on(async {
        let birthday = if me.public().sig_pk != invite.creator {
            None
        } else if let Some(h) = birthday_height {
            Some(h.max(2))
        } else {
            let tip = latest_height(&mut connect(&lightwalletd_url).await?).await?;
            Some((tip + 1).max(2))
        };
        node::run_keygen(
            &relay,
            &me,
            &invite,
            &confirmed_safety_number,
            &net,
            net.name(),
            birthday,
            &mut OsRng,
            Duration::from_secs(u64::from(timeout_secs)),
        )
        .await
        .map_err(anyhow::Error::from)
    })?;
    Ok(postcard::to_allocvec(&material).map_err(anyhow::Error::from)?)
}

pub struct VaultSummary {
    pub name: String,
    pub network: String,
    pub address: String,
    pub threshold: u16,
    pub members: Vec<String>,
    pub birthday_height: u32,
}

#[flutter_rust_bridge::frb(sync)]
pub fn vault_summary(material: Vec<u8>) -> Result<VaultSummary, ZafeError> {
    let m = self::material(&material)?;
    let d = &m.descriptor;
    Ok(VaultSummary {
        name: d.name.clone(),
        network: d.network.clone(),
        address: d.address.clone(),
        threshold: d.threshold,
        members: d
            .members
            .iter()
            .map(|x| hex::encode(x.identity.sig_pk))
            .collect(),
        birthday_height: d.birthday_height,
    })
}

pub struct Balance {
    pub height: u32,
    pub spendable_zat: u64,
    pub total_zat: u64,
}

/// Serializes use of the wallet database (sync, propose and reads race otherwise).
pub(crate) fn wallet_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

pub(crate) fn wallet_path(db_dir: &str, m: &VaultMaterial) -> PathBuf {
    PathBuf::from(db_dir).join(format!(
        "vault-{}.sqlite",
        hex::encode(m.descriptor.vault_id)
    ))
}

/// Opens the vault wallet, creating it on first use. Hold `wallet_lock()` while using it.
pub(crate) async fn open_wallet(
    db_dir: &str,
    lightwalletd_url: &str,
    m: &VaultMaterial,
) -> Result<VaultWallet<ZafeNetwork>, ZafeError> {
    let net = network(&m.descriptor.network)?;
    let path = wallet_path(db_dir, m);
    if VaultWallet::exists(&path, net) {
        return Ok(VaultWallet::open(&path, net)?);
    }
    let _ = std::fs::remove_file(&path); // a leftover without its account
    let mut client = connect(lightwalletd_url).await?;
    let ufvk = m.vault_keys()?.ufvk().map_err(anyhow::Error::from)?;
    Ok(VaultWallet::create(
        &path,
        net,
        &m.descriptor.name,
        &ufvk,
        m.descriptor.birthday_height,
        &mut client,
    )
    .await?)
}

/// Syncs the vault wallet (creating its database under `db_dir` on first use).
pub fn sync_vault(
    db_dir: String,
    lightwalletd_url: String,
    material: Vec<u8>,
) -> Result<Balance, ZafeError> {
    let m = self::material(&material)?;
    let _guard = wallet_lock();
    runtime().block_on(async {
        let mut wallet = open_wallet(&db_dir, &lightwalletd_url, &m).await?;
        wallet.sync(&mut connect(&lightwalletd_url).await?).await?;
        let b = wallet.balance()?;
        Ok(Balance {
            height: wallet.chain_height()?.unwrap_or(0),
            spendable_zat: b.ironwood_spendable,
            total_zat: b.total,
        })
    })
}
