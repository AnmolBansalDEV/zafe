//! `zafe`: a headless Zafe member (spec §15, milestone M0).
//!
//! State lives in `--home` as plain files. That is for development only: identity seeds,
//! the FROST share, the vault secret and nonces belong in platform secure storage on real
//! devices (spec §14).

use std::{fs, path::PathBuf, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use rand::rngs::OsRng;
use reddsa::frost::redpallas::round1::{SigningCommitments, SigningNonces};
use zafe_core::{
    node::{self, Invite, VaultMaterial},
    relay_client::RelayClient,
    session::{NonceStore, ProposalId},
    wallet::{connect, latest_height, regtest_network, PaymentRequest, VaultWallet},
};
use zafe_proto::{Identity, IdentitySeeds};
use zcash_protocol::{local_consensus::LocalNetwork, memo::Memo};

#[derive(Parser)]
#[command(
    name = "zafe",
    about = "Headless Zafe multisig member (development only)"
)]
struct Cli {
    /// Directory holding this member's state.
    #[arg(long, env = "ZAFE_HOME", default_value = "./zafe-home")]
    home: PathBuf,
    #[arg(long, env = "ZAFE_RELAY", default_value = "http://127.0.0.1:8787")]
    relay: String,
    #[arg(
        long,
        env = "ZAFE_LIGHTWALLETD",
        default_value = "http://127.0.0.1:9067"
    )]
    lightwalletd: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create this member's identity.
    Init,
    /// Vault setup.
    #[command(subcommand)]
    Vault(VaultCmd),
    /// Sync the vault wallet and print the balance.
    Sync,
    /// Propose a payment.
    Propose {
        #[arg(long)]
        to: String,
        /// Amount in zatoshis.
        #[arg(long)]
        amount: u64,
        #[arg(long)]
        memo: Option<String>,
    },
    /// List proposals.
    Proposals,
    /// Verify a proposal independently and approve it.
    Approve { proposal: String },
    /// Reject a proposal.
    Reject { proposal: String },
    /// Leader: send signing requests for an approved proposal.
    Request { proposal: String },
    /// Answer pending signing requests.
    Respond,
    /// Leader: aggregate shares, prove, broadcast.
    Finalize { proposal: String },
}

#[derive(Subcommand)]
enum VaultCmd {
    /// Create a vault and print the invite.
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        threshold: u16,
        #[arg(long)]
        members: u16,
    },
    /// Join a vault from an invite string.
    Join { invite: String },
    /// Show members, whether membership is sealed, and the safety number.
    Members,
    /// Creator: freeze membership once everyone has joined.
    Seal,
    /// Run key generation. Requires the safety number you compared with the others.
    Keygen {
        #[arg(long)]
        safety_number: String,
        #[arg(long, default_value_t = 300)]
        timeout_secs: u64,
        /// Creator only: vault birthday height (default: lightwalletd tip + 1).
        #[arg(long)]
        birthday: Option<u32>,
    },
    /// Show the vault address and details.
    Show,
}

struct Home(PathBuf);

impl Home {
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn identity(&self) -> Result<Identity> {
        let bytes = fs::read(self.path("identity.bin")).context("no identity; run `zafe init`")?;
        let (sig_seed, enc_seed): ([u8; 32], [u8; 32]) = postcard::from_bytes(&bytes)?;
        Ok(Identity::from_seeds(IdentitySeeds { sig_seed, enc_seed }))
    }

    fn invite(&self) -> Result<Invite> {
        let s = fs::read_to_string(self.path("invite.txt"))
            .context("no invite; create or join a vault")?;
        Ok(Invite::decode(&s)?)
    }

    fn material(&self) -> Result<VaultMaterial> {
        let bytes = fs::read(self.path("vault.bin"))
            .context("vault not created yet; run `zafe vault keygen`")?;
        Ok(postcard::from_bytes(&bytes)?)
    }
}

/// Nonces on disk, one file per (proposal, PCZT hash). `take` deletes before returning.
struct FileNonceStore(PathBuf);

impl FileNonceStore {
    fn file(&self, proposal: &ProposalId, hash: &[u8; 32]) -> PathBuf {
        self.0.join(format!(
            "{}-{}.bin",
            hex::encode(proposal),
            hex::encode(hash)
        ))
    }
}

impl NonceStore for FileNonceStore {
    fn contains(&self, proposal: &ProposalId, hash: &[u8; 32]) -> bool {
        self.file(proposal, hash).exists()
    }

    fn commitments(
        &self,
        proposal: &ProposalId,
        hash: &[u8; 32],
    ) -> Option<Vec<SigningCommitments>> {
        let encoded: Vec<Vec<u8>> =
            postcard::from_bytes(&fs::read(self.file(proposal, hash)).ok()?).ok()?;
        encoded
            .iter()
            .map(|b| SigningNonces::deserialize(b).ok().map(|n| *n.commitments()))
            .collect()
    }

    fn put(&mut self, proposal: ProposalId, hash: [u8; 32], nonces: Vec<SigningNonces>) {
        let encoded: Vec<Vec<u8>> = nonces
            .iter()
            .map(|n| n.serialize().expect("serializable"))
            .collect();
        fs::create_dir_all(&self.0).expect("nonce dir");
        fs::write(
            self.file(&proposal, &hash),
            postcard::to_allocvec(&encoded).expect("encodable"),
        )
        .expect("write nonces");
    }

    fn take(&mut self, proposal: &ProposalId, hash: &[u8; 32]) -> Option<Vec<SigningNonces>> {
        let path = self.file(proposal, hash);
        let bytes = fs::read(&path).ok()?;
        fs::remove_file(&path).ok()?; // delete before use: never reusable
        let encoded: Vec<Vec<u8>> = postcard::from_bytes(&bytes).ok()?;
        encoded
            .iter()
            .map(|b| SigningNonces::deserialize(b).ok())
            .collect()
    }
}

fn network() -> LocalNetwork {
    regtest_network()
}

fn parse_proposal(s: &str) -> Result<ProposalId> {
    hex::decode(s)?
        .try_into()
        .map_err(|_| anyhow!("proposal id must be 16 bytes hex"))
}

async fn open_wallet(
    home: &Home,
    material: &VaultMaterial,
    lwd: &str,
) -> Result<VaultWallet<LocalNetwork>> {
    let mut client = connect(lwd).await?;
    let path = home.path("wallet.sqlite");
    let ufvk = material.vault_keys()?.ufvk()?;
    let mut wallet = if path.exists() {
        VaultWallet::open(&path, network())?
    } else {
        VaultWallet::create(
            &path,
            network(),
            &material.descriptor.name,
            &ufvk,
            material.descriptor.birthday_height,
            &mut client,
        )
        .await?
    };
    wallet.sync(&mut client).await?;
    Ok(wallet)
}

async fn tip(home: &Home, material: &VaultMaterial, lwd: &str) -> Result<u32> {
    open_wallet(home, material, lwd)
        .await?
        .chain_height()?
        .ok_or_else(|| anyhow!("wallet not synced"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let home = Home(cli.home.clone());
    fs::create_dir_all(&home.0)?;
    let relay = RelayClient::new(&cli.relay);
    let mut rng = OsRng;

    match cli.command {
        Command::Init => {
            if home.path("identity.bin").exists() {
                bail!("identity already exists in {}", home.0.display());
            }
            let id = Identity::generate(&mut rng);
            let seeds = id.seeds();
            fs::write(
                home.path("identity.bin"),
                postcard::to_allocvec(&(seeds.sig_seed, seeds.enc_seed))?,
            )?;
            println!("identity {}", hex::encode(id.public().sig_pk));
        }
        Command::Vault(cmd) => vault(cmd, &home, &relay, &cli.lightwalletd, &mut rng).await?,
        Command::Sync => {
            let material = home.material()?;
            let wallet = open_wallet(&home, &material, &cli.lightwalletd).await?;
            let b = wallet.balance()?;
            println!(
                "height {} ironwood_spendable {} ironwood_total {} total {}",
                wallet.chain_height()?.unwrap_or(0),
                b.ironwood_spendable,
                b.ironwood_total,
                b.total
            );
        }
        Command::Propose { to, amount, memo } => {
            let material = home.material()?;
            let mut wallet = open_wallet(&home, &material, &cli.lightwalletd).await?;
            let memo = memo
                .map(|m| Memo::from_bytes(m.as_bytes()).map(|m| m.encode()))
                .transpose()?;
            let payments = [PaymentRequest {
                address: to,
                amount_zat: amount,
                memo,
            }];
            let id = node::propose(
                &relay,
                &home.identity()?,
                &material,
                &mut wallet,
                &payments,
                &mut rng,
            )
            .await?;
            println!("proposal {}", hex::encode(id));
        }
        Command::Proposals => {
            let material = home.material()?;
            let (_, state) = node::load_state(&relay, &home.identity()?, &material).await?;
            for p in state.proposals.values() {
                let total: u64 = p.payments.iter().map(|x| x.amount_zat).sum();
                println!(
                    "{} {:?} {} zat to {} payee(s), approvals {}/{}, rejections {}{}",
                    hex::encode(p.id),
                    p.status,
                    total,
                    p.payments.len(),
                    p.approvals.len(),
                    material.descriptor.threshold,
                    p.rejections.len(),
                    p.txid
                        .map(|t| format!(", txid {}", hex_txid(&t)))
                        .unwrap_or_default()
                );
            }
        }
        Command::Approve { proposal } => {
            let material = home.material()?;
            let tip = tip(&home, &material, &cli.lightwalletd).await?;
            let mut store = FileNonceStore(home.path("nonces"));
            let verified = node::approve(
                &relay,
                &home.identity()?,
                &material,
                &network(),
                tip,
                parse_proposal(&proposal)?,
                &mut store,
                &mut rng,
            )
            .await?;
            println!(
                "verified and approved: {} payment(s), fee {} zat, change {} zat, {} spend(s) to sign",
                verified.payments.len(),
                verified.fee_zat,
                verified.change_total_zat,
                verified.spends_to_sign.len()
            );
        }
        Command::Reject { proposal } => {
            node::reject(
                &relay,
                &home.identity()?,
                &home.material()?,
                parse_proposal(&proposal)?,
                &mut rng,
            )
            .await?;
            println!("rejected");
        }
        Command::Request { proposal } => {
            let material = home.material()?;
            let tip = tip(&home, &material, &cli.lightwalletd).await?;
            let id = parse_proposal(&proposal)?;
            // Commitment sets already put in a request must never be reused.
            let used_path = home.path("requests/used_commitments.bin");
            let mut used: std::collections::BTreeSet<[u8; 32]> = fs::read(&used_path)
                .ok()
                .and_then(|b| postcard::from_bytes(&b).ok())
                .unwrap_or_default();
            let sent = node::request_signatures(
                &relay,
                &home.identity()?,
                &material,
                &network(),
                tip,
                id,
                &used,
                &mut rng,
            )
            .await?;
            used.extend(sent.used_commitments);
            fs::create_dir_all(home.path("requests"))?;
            fs::write(&used_path, postcard::to_allocvec(&used)?)?;
            fs::write(
                home.path(&format!("requests/{proposal}.bin")),
                node::encode_request(&sent.request)?,
            )?;
            println!(
                "signing requests sent to {} member(s)",
                sent.request.signers.len()
            );
        }
        Command::Respond => {
            let material = home.material()?;
            let tip = tip(&home, &material, &cli.lightwalletd).await?;
            let mut store = FileNonceStore(home.path("nonces"));
            let report = node::respond(
                &relay,
                &home.identity()?,
                &material,
                &network(),
                tip,
                &mut store,
                &mut rng,
            )
            .await?;
            for (proposal, reason) in &report.skipped {
                eprintln!("skipped request for {}: {reason}", hex::encode(proposal));
            }
            println!("answered {} signing request(s)", report.answered.len());
        }
        Command::Finalize { proposal } => {
            let material = home.material()?;
            let request = node::decode_request(
                &fs::read(home.path(&format!("requests/{proposal}.bin")))
                    .context("run `zafe request` first")?,
            )?;
            let mut client = connect(&cli.lightwalletd).await?;
            eprintln!("building proving key...");
            let pk = orchard::circuit::ProvingKey::build(
                orchard::circuit::OrchardCircuitVersion::PostNu6_3,
            );
            let vk = orchard::circuit::VerifyingKey::build(
                orchard::circuit::OrchardCircuitVersion::PostNu6_3,
            );
            let txid = node::finalize(
                &relay,
                &home.identity()?,
                &material,
                &request,
                &mut client,
                &pk,
                &vk,
                Duration::from_secs(120),
                &mut rng,
            )
            .await?;
            println!("broadcast txid {}", hex_txid(&txid));
        }
    }
    Ok(())
}

async fn vault(
    cmd: VaultCmd,
    home: &Home,
    relay: &RelayClient,
    lwd: &str,
    rng: &mut OsRng,
) -> Result<()> {
    match cmd {
        VaultCmd::Create {
            name,
            threshold,
            members,
        } => {
            let invite =
                node::create_vault(relay, &home.identity()?, &name, threshold, members, rng)
                    .await?;
            fs::write(home.path("invite.txt"), invite.encode())?;
            println!("{}", invite.encode());
        }
        VaultCmd::Join { invite } => {
            let parsed = Invite::decode(&invite)?;
            node::join_vault(relay, &home.identity()?, &parsed).await?;
            fs::write(home.path("invite.txt"), parsed.encode())?;
            println!("joined vault {}", parsed.name);
        }
        VaultCmd::Members => {
            let (members, sealed, number) =
                node::membership(relay, &home.identity()?, &home.invite()?).await?;
            for m in &members {
                println!("member {}", hex::encode(m.sig_pk));
            }
            println!("sealed {sealed}");
            println!("safety number: {number}");
        }
        VaultCmd::Seal => {
            node::seal(relay, &home.identity()?, &home.invite()?).await?;
            println!("membership sealed");
        }
        VaultCmd::Keygen {
            safety_number,
            timeout_secs,
            birthday,
        } => {
            let invite = home.invite()?;
            let me = home.identity()?;
            let birthday = match (me.public().sig_pk == invite.creator, birthday) {
                (true, Some(height)) => Some(height.max(2)),
                (true, None) => Some((latest_height(&mut connect(lwd).await?).await? + 1).max(2)),
                (false, _) => None,
            };
            let material = node::run_keygen(
                relay,
                &me,
                &invite,
                &safety_number,
                &network(),
                "regtest",
                birthday,
                rng,
                Duration::from_secs(timeout_secs),
            )
            .await?;
            fs::write(home.path("vault.bin"), postcard::to_allocvec(&material)?)?;
            println!("vault created: {}", material.descriptor.address);
        }
        VaultCmd::Show => {
            let m = home.material()?;
            println!("name {}", m.descriptor.name);
            println!(
                "threshold {} of {}",
                m.descriptor.threshold,
                m.descriptor.members.len()
            );
            println!("address {}", m.descriptor.address);
            println!("ufvk {}", m.descriptor.ufvk);
            println!("birthday {}", m.descriptor.birthday_height);
        }
    }
    Ok(())
}

fn hex_txid(txid: &[u8; 32]) -> String {
    let mut display = *txid;
    display.reverse(); // txids display in reversed byte order
    hex::encode(display)
}
