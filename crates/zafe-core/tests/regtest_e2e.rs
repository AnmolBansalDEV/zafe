//! End-to-end on a local Ironwood regtest network (Zakura + lightwalletd in Docker):
//! a 2-of-3 vault is funded by mining to its address, every member syncs its own wallet,
//! one member proposes a payment, members approve and sign, and the transaction is
//! proven, broadcast, mined and seen by the recipient.
//!
//! Needs Docker. Run with:
//!   cargo test -p zafe-core --test regtest_e2e -- --ignored --nocapture

use std::{collections::BTreeMap, path::PathBuf, process::Command};

use orchard::{
    circuit::{OrchardCircuitVersion, ProvingKey, VerifyingKey},
    keys::{FullViewingKey, Scope, SpendingKey},
};
use pczt::roles::{prover::Prover, tx_extractor::TransactionExtractor};
use rand::{rngs::StdRng, SeedableRng};
use zafe_core::{
    session::{Leader, Member, MemoryNonceStore},
    tx,
    verify::{verify_pczt, Expectations, Payment},
    wallet::{connect, regtest_network, PaymentRequest, VaultWallet},
};
use zcash_client_backend::proto::service::RawTransaction;
use zcash_keys::{address::UnifiedAddress, keys::UnifiedFullViewingKey};
use zcash_protocol::{
    consensus::{BlockHeight, BranchId},
    memo::Memo,
};

mod common;
use common::{params, run_keygen};

const NAME: &str = "zafe-e2e";
const RPC_PORT: u16 = 38232;
const LWD_PORT: u16 = 39067;

struct Regtest;

impl Regtest {
    fn start(miner_address: &str) -> Self {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../infra/regtest/up.sh");
        let status = Command::new(script)
            .arg(miner_address)
            .env("ZAFE_REGTEST_NAME", NAME)
            .env("ZAFE_REGTEST_RPC_PORT", RPC_PORT.to_string())
            .env("ZAFE_REGTEST_LWD_PORT", LWD_PORT.to_string())
            .status()
            .expect("run up.sh");
        assert!(status.success(), "regtest failed to start");
        Self
    }

    fn rpc(&self, method: &str, params: &str) -> String {
        let out = Command::new("curl")
            .args([
                "-s",
                "-X",
                "POST",
                "-H",
                "content-type: application/json",
                "--data",
            ])
            .arg(format!(
                r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":{params}}}"#
            ))
            .arg(format!("http://127.0.0.1:{RPC_PORT}"))
            .output()
            .expect("curl");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn mine(&self, blocks: u32) {
        let reply = self.rpc("generate", &format!("[{blocks}]"));
        assert!(reply.contains("\"result\""), "generate failed: {reply}");
    }
}

impl Drop for Regtest {
    fn drop(&mut self) {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../infra/regtest/down.sh");
        let _ = Command::new(script).env("ZAFE_REGTEST_NAME", NAME).status();
    }
}

async fn wait_for_lightwalletd_height(client: &mut zafe_core::wallet::Client, height: u64) {
    for _ in 0..120 {
        let tip = client
            .get_latest_block(zcash_client_backend::proto::service::ChainSpec::default())
            .await
            .map(|r| r.into_inner().height)
            .unwrap_or(0);
        if tip >= height {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    panic!("lightwalletd did not reach height {height}");
}

#[tokio::test]
#[ignore = "needs Docker; see module docs"]
async fn vault_pays_on_regtest() {
    let network = regtest_network();
    let mut rng = StdRng::seed_from_u64(99);

    // 1. Create a 2-of-3 vault.
    let members = run_keygen(params(2, 3), &mut rng);
    let vault_keys = &members[0].1.vault_keys;
    let vault_fvk = vault_keys.fvk().clone();
    let vault_ufvk = vault_keys.ufvk().unwrap();
    let vault_ua = UnifiedAddress::from_receivers(
        Some(vault_fvk.address_at(0u32, Scope::External)),
        None,
        None,
    )
    .unwrap()
    .encode(&network);
    println!("vault address: {vault_ua}");

    // 2. Start regtest paying coinbase to the vault; mine past coinbase maturity.
    let chain = Regtest::start(&vault_ua);
    chain.mine(120);
    let lwd = format!("http://127.0.0.1:{LWD_PORT}");
    let mut client = connect(&lwd).await.expect("lightwalletd");
    wait_for_lightwalletd_height(&mut client, 121).await;

    // 3. Each member keeps its own wallet database and syncs independently.
    let dir = tempdir();
    let mut wallets = Vec::new();
    for (i, _) in members.iter().enumerate() {
        let path = dir.join(format!("member{i}.sqlite"));
        let mut w = VaultWallet::create(&path, network, "vault", &vault_ufvk, 2, &mut client)
            .await
            .unwrap();
        w.sync(&mut client).await.unwrap();
        wallets.push(w);
    }
    let before = wallets[0].balance().unwrap();
    println!("vault balance after funding: {before:?}");
    assert!(
        before.ironwood_spendable > 0,
        "vault has spendable Ironwood funds"
    );
    for w in &wallets[1..] {
        assert_eq!(
            w.balance().unwrap(),
            before,
            "every member sees the same balance"
        );
    }

    // 4. Member 0 proposes paying 1 ZEC with a memo to an outside recipient.
    let recipient_fvk = FullViewingKey::from(&SpendingKey::from_bytes([77; 32]).unwrap());
    let recipient = recipient_fvk.address_at(0u32, Scope::External);
    let recipient_ua = UnifiedAddress::from_receivers(Some(recipient), None, None)
        .unwrap()
        .encode(&network);
    let memo = Memo::from_bytes(b"Grant milestone 1").unwrap().encode();
    let amount = 100_000_000;
    let pczt = wallets[0]
        .propose(
            &[PaymentRequest {
                address: recipient_ua,
                amount_zat: amount,
                memo: Some(memo.clone()),
            }],
            8064,
        )
        .expect("proposal");

    // 5. Members verify independently, approve, and sign (2 of 3).
    let tip = wallets[0].chain_height().unwrap().unwrap();
    let expected = Expectations {
        payments: vec![Payment {
            recipient,
            amount_zat: amount,
            memo: *memo.as_array(),
        }],
        consensus_branch_id: BranchId::for_height(&network, BlockHeight::from_u32(tip + 1)).into(),
        tip_height: tip,
        max_expiry_delta: 8064 + 1 + 96,
    };
    let verified = verify_pczt(&pczt, &vault_fvk, &expected).expect("member verification");
    println!(
        "verified: fee {} zat, change {} zat",
        verified.fee_zat, verified.change_total_zat
    );

    let proposal_id = [1u8; 16];
    let mut leader = Leader::new(proposal_id, &pczt, &verified, 2).unwrap();
    let mut stores: Vec<MemoryNonceStore> = (0..3).map(|_| MemoryNonceStore::default()).collect();
    for i in [1, 2] {
        let (id, out) = &members[i];
        let m = Member {
            identifier: *id,
            key_package: &out.key_package,
            vault_fvk: out.vault_keys.fvk(),
        };
        let (approval, _) = m
            .approve(proposal_id, &pczt, &expected, &mut stores[i], &mut rng)
            .unwrap();
        assert!(leader.add_approval(approval));
    }
    let request = leader.request(&[members[1].0, members[2].0]).unwrap();
    let mut shares = BTreeMap::new();
    for i in [1, 2] {
        let (id, out) = &members[i];
        let m = Member {
            identifier: *id,
            key_package: &out.key_package,
            vault_fvk: out.vault_keys.fvk(),
        };
        shares.insert(
            *id,
            m.sign(&request, &pczt, &expected, &mut stores[i]).unwrap(),
        );
    }
    let signatures = leader
        .aggregate(&request, &shares, &members[0].1.public_key_package)
        .unwrap();

    // 6. Inject, prove, extract (fully verified), broadcast, mine.
    let signed = tx::apply_signatures(pczt, &signatures).unwrap();
    let pk = ProvingKey::build(OrchardCircuitVersion::PostNu6_3);
    let vk = VerifyingKey::build(OrchardCircuitVersion::PostNu6_3);
    let proved = Prover::new(signed)
        .create_ironwood_proof(&pk)
        .unwrap()
        .finish();
    let transaction = TransactionExtractor::new(proved)
        .with_orchard(&vk)
        .extract()
        .expect("valid transaction");
    let mut raw = Vec::new();
    transaction.write(&mut raw).unwrap();
    let reply = client
        .send_transaction(RawTransaction {
            data: raw,
            height: 0,
        })
        .await
        .expect("send")
        .into_inner();
    println!(
        "broadcast: code {} {}",
        reply.error_code, reply.error_message
    );
    assert_eq!(
        reply.error_code, 0,
        "node rejected transaction: {}",
        reply.error_message
    );
    println!("txid {}", transaction.txid());

    chain.mine(2);
    wait_for_lightwalletd_height(&mut client, u64::from(tip) + 2).await;

    // 7. The vault sees the spend; the recipient sees the payment.
    wallets[1].sync(&mut client).await.unwrap();
    let after = wallets[1].balance().unwrap();
    println!("vault balance after payment: {after:?}");

    let recipient_ufvk = UnifiedFullViewingKey::from_orchard_fvk(recipient_fvk).unwrap();
    let mut recipient_wallet = VaultWallet::create(
        &dir.join("recipient.sqlite"),
        network,
        "recipient",
        &recipient_ufvk,
        2,
        &mut client,
    )
    .await
    .unwrap();
    recipient_wallet.sync(&mut client).await.unwrap();
    let received = recipient_wallet.balance().unwrap();
    println!("recipient balance: {received:?}");
    assert_eq!(
        received.ironwood_total, amount,
        "recipient received the payment in Ironwood"
    );
}

fn tempdir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("zafe-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
