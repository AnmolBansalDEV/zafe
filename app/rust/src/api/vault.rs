//! Vault setup and wallet API for the Flutter app.
//!
//! Keep this surface to primitives and flat structs (the Vizor rule); all Zcash and FROST
//! types stay inside `zafe-core`. Secrets (identity seeds, vault material) cross as bytes so
//! the Dart side can keep them in platform secure storage; nothing here persists secrets.
//! Calls run on FRB's worker threads; async work uses one shared tokio runtime.

use std::{path::PathBuf, sync::OnceLock, time::Duration};

use anyhow::{anyhow, Context, Result};
use rand::rngs::OsRng;
use zafe_core::{
    node::{self, Invite, VaultMaterial},
    relay_client::RelayClient,
    wallet::{connect, latest_height, VaultWallet, ZafeNetwork},
};
use zafe_proto::{Identity, IdentitySeeds};

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .expect("tokio runtime")
    })
}

fn network(name: &str) -> Result<ZafeNetwork> {
    ZafeNetwork::from_name(name).ok_or_else(|| anyhow!("unknown network {name}"))
}

fn identity(seeds: &[u8]) -> Result<Identity> {
    if seeds.len() != 64 {
        return Err(anyhow!("identity seeds must be 64 bytes"));
    }
    Ok(Identity::from_seeds(IdentitySeeds {
        sig_seed: seeds[..32].try_into().expect("32"),
        enc_seed: seeds[32..].try_into().expect("32"),
    }))
}

fn material(bytes: &[u8]) -> Result<VaultMaterial> {
    postcard::from_bytes(bytes).context("invalid vault material")
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
    IdentityInfo { seeds, public_key_hex: hex::encode(id.public().sig_pk) }
}

/// The public key (hex) of an identity, e.g. to mark "You" in member lists.
#[flutter_rust_bridge::frb(sync)]
pub fn identity_public_key(seeds: Vec<u8>) -> Result<String> {
    Ok(hex::encode(identity(&seeds)?.public().sig_pk))
}

pub struct InviteInfo {
    pub name: String,
    pub threshold: u16,
    pub members: u16,
    pub creator_hex: String,
}

#[flutter_rust_bridge::frb(sync)]
pub fn parse_invite(invite: String) -> Result<InviteInfo> {
    let i = Invite::decode(&invite)?;
    Ok(InviteInfo { name: i.name, threshold: i.threshold, members: i.members, creator_hex: hex::encode(i.creator) })
}

/// Creates a vault mailbox on the relay and returns the invite string to share.
pub fn create_vault(relay_url: String, seeds: Vec<u8>, name: String, threshold: u16, members: u16) -> Result<String> {
    let me = identity(&seeds)?;
    let relay = RelayClient::new(relay_url);
    let invite = runtime().block_on(node::create_vault(&relay, &me, &name, threshold, members, &mut OsRng))?;
    Ok(invite.encode())
}

pub fn join_vault(relay_url: String, seeds: Vec<u8>, invite: String) -> Result<()> {
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

pub fn vault_membership(relay_url: String, seeds: Vec<u8>, invite: String) -> Result<MembershipInfo> {
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
pub fn seal_vault(relay_url: String, seeds: Vec<u8>, invite: String) -> Result<()> {
    let me = identity(&seeds)?;
    let invite = Invite::decode(&invite)?;
    runtime().block_on(node::seal(&RelayClient::new(relay_url), &me, &invite))?;
    Ok(())
}

/// Runs key generation. Blocks until every member finishes (or `timeout_secs`). Returns the
/// vault material (secret: store it in secure storage).
pub fn run_keygen(
    relay_url: String,
    lightwalletd_url: String,
    network_name: String,
    seeds: Vec<u8>,
    invite: String,
    confirmed_safety_number: String,
    timeout_secs: u32,
) -> Result<Vec<u8>> {
    let me = identity(&seeds)?;
    let invite = Invite::decode(&invite)?;
    let net = network(&network_name)?;
    let relay = RelayClient::new(relay_url);
    let material = runtime().block_on(async {
        let birthday = if me.public().sig_pk == invite.creator {
            let tip = latest_height(&mut connect(&lightwalletd_url).await?).await?;
            Some((tip + 1).max(2))
        } else {
            None
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
    Ok(postcard::to_allocvec(&material)?)
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
pub fn vault_summary(material: Vec<u8>) -> Result<VaultSummary> {
    let m = self::material(&material)?;
    let d = &m.descriptor;
    Ok(VaultSummary {
        name: d.name.clone(),
        network: d.network.clone(),
        address: d.address.clone(),
        threshold: d.threshold,
        members: d.members.iter().map(|x| hex::encode(x.identity.sig_pk)).collect(),
        birthday_height: d.birthday_height,
    })
}

pub struct Balance {
    pub height: u32,
    pub spendable_zat: u64,
    pub total_zat: u64,
}

/// Syncs the vault wallet (creating its database under `db_dir` on first use).
pub fn sync_vault(db_dir: String, lightwalletd_url: String, material: Vec<u8>) -> Result<Balance> {
    let m = self::material(&material)?;
    let net = network(&m.descriptor.network)?;
    let path = PathBuf::from(db_dir).join(format!("vault-{}.sqlite", hex::encode(m.descriptor.vault_id)));
    runtime().block_on(async {
        let mut client = connect(&lightwalletd_url).await?;
        let mut wallet = if path.exists() {
            VaultWallet::open(&path, net)?
        } else {
            let ufvk = m.vault_keys()?.ufvk()?;
            VaultWallet::create(&path, net, &m.descriptor.name, &ufvk, m.descriptor.birthday_height, &mut client).await?
        };
        wallet.sync(&mut client).await?;
        let b = wallet.balance()?;
        Ok(Balance {
            height: wallet.chain_height()?.unwrap_or(0),
            spendable_zat: b.ironwood_spendable,
            total_zat: b.total,
        })
    })
}
