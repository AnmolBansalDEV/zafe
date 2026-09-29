//! The app's whole payment flow through the Flutter bridge API (what the Dart side calls),
//! for three members on a live Ironwood regtest chain: create/join/seal, keygen, fund,
//! sync, propose, review, approve, answer signing requests, and send.
//!
//! Needs Docker. `cargo test -p rust_lib_zafe --test bridge_e2e -- --ignored --nocapture`

use std::{path::PathBuf, process::Command, thread, time::Duration};

use rust_lib_zafe::api::{
    proposals::{self, MyVote, PaymentInput, ProposalStage, SendStage},
    vault,
};

const NAME: &str = "zafe-bridge";
const RPC_PORT: u16 = 48332;
const LWD_PORT: u16 = 49167;
const RELAY_PORT: u16 = 48887;

struct Regtest;

impl Regtest {
    fn script(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../infra/regtest")
            .join(name)
    }

    fn start(miner_address: &str) -> Self {
        let status = Command::new(Self::script("up.sh"))
            .arg(miner_address)
            .env("ZAFE_REGTEST_NAME", NAME)
            .env("ZAFE_REGTEST_RPC_PORT", RPC_PORT.to_string())
            .env("ZAFE_REGTEST_LWD_PORT", LWD_PORT.to_string())
            .status()
            .expect("run up.sh");
        assert!(status.success(), "regtest failed to start");
        Self
    }

    fn mine(&self, blocks: u32) {
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
                r#"{{"jsonrpc":"2.0","id":1,"method":"generate","params":[{blocks}]}}"#
            ))
            .arg(format!("http://127.0.0.1:{RPC_PORT}"))
            .output()
            .expect("curl");
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("\"result\""),
            "generate failed"
        );
    }
}

impl Drop for Regtest {
    fn drop(&mut self) {
        let _ = Command::new(Self::script("down.sh"))
            .env("ZAFE_REGTEST_NAME", NAME)
            .status();
    }
}

/// A throwaway regtest unified address that is not the vault's (as examples/vault_address).
fn outside_address() -> String {
    use orchard::keys::{FullViewingKey, Scope, SpendingKey};
    use zafe_core::keys::{VaultKeys, VaultSecret};
    let ak: [u8; 32] = FullViewingKey::from(&SpendingKey::from_bytes([42; 32]).unwrap()).to_bytes()
        [..32]
        .try_into()
        .unwrap();
    let keys = VaultKeys::derive(&VaultSecret::from_bytes([1; 32]), &ak).unwrap();
    let address = keys.fvk().address_at(0u32, Scope::External);
    zcash_keys::address::UnifiedAddress::from_receivers(Some(address), None, None)
        .unwrap()
        .encode(&zafe_core::wallet::regtest_network())
}

struct Member {
    seeds: Vec<u8>,
    material: Vec<u8>,
    db_dir: String,
    state_dir: String,
}

fn start_relay() {
    thread::spawn(|| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", RELAY_PORT))
                .await
                .unwrap();
            axum::serve(listener, zafe_relay::Relay::new().router())
                .await
                .unwrap();
        })
    });
    thread::sleep(Duration::from_millis(300));
}

#[test]
#[ignore = "needs Docker (Ironwood regtest)"]
fn payment_flow_through_bridge() {
    let relay = format!("http://127.0.0.1:{RELAY_PORT}");
    let lwd = format!("http://127.0.0.1:{LWD_PORT}");
    let tmp = std::env::temp_dir().join(format!("zafe-bridge-{}", std::process::id()));
    start_relay();

    // Setup: A creates a 2-of-3 vault, B and C join, A seals.
    let seeds: Vec<Vec<u8>> = (0..3).map(|_| vault::generate_identity().seeds).collect();
    let invite =
        vault::create_vault(relay.clone(), seeds[0].clone(), "Grants".into(), 2, 3).unwrap();
    for s in &seeds[1..] {
        vault::join_vault(relay.clone(), s.clone(), invite.clone()).unwrap();
    }
    vault::seal_vault(relay.clone(), seeds[0].clone(), invite.clone()).unwrap();
    let safety: Vec<String> = seeds
        .iter()
        .map(|s| {
            vault::vault_membership(relay.clone(), s.clone(), invite.clone())
                .unwrap()
                .safety_number
        })
        .collect();
    assert!(safety.iter().all(|n| n == &safety[0]));

    // Keygen: all three at once (A picks birthday 2: regtest isn't up yet).
    let handles: Vec<_> = seeds
        .iter()
        .map(|s| {
            let (relay, lwd, s, invite, sn) = (
                relay.clone(),
                lwd.clone(),
                s.clone(),
                invite.clone(),
                safety[0].clone(),
            );
            thread::spawn(move || {
                vault::run_keygen(relay, lwd, "regtest".into(), s, invite, sn, 120, Some(2))
                    .unwrap()
            })
        })
        .collect();
    let materials: Vec<Vec<u8>> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let summary = vault::vault_summary(materials[0].clone()).unwrap();
    for m in &materials[1..] {
        assert_eq!(
            vault::vault_summary(m.clone()).unwrap().address,
            summary.address
        );
    }
    let members: Vec<Member> = seeds
        .into_iter()
        .zip(materials)
        .enumerate()
        .map(|(i, (seeds, material))| {
            let dir = tmp.join(format!("m{i}"));
            let state = dir.join("signing");
            std::fs::create_dir_all(&state).unwrap();
            Member {
                seeds,
                material,
                db_dir: dir.to_string_lossy().into(),
                state_dir: state.to_string_lossy().into(),
            }
        })
        .collect();

    // Fund the vault by mining to it; coinbase matures after 100 blocks.
    let chain = Regtest::start(&summary.address);
    chain.mine(120);
    let sync =
        |m: &Member| vault::sync_vault(m.db_dir.clone(), lwd.clone(), m.material.clone()).unwrap();
    let mut balance = sync(&members[0]);
    for _ in 0..60 {
        if balance.height >= 121 && balance.spendable_zat > 0 {
            break;
        }
        thread::sleep(Duration::from_secs(1));
        balance = sync(&members[0]);
    }
    assert!(
        balance.spendable_zat > 100_000_000,
        "vault not funded: {}",
        balance.spendable_zat
    );
    for m in &members[1..] {
        sync(m);
    }

    // Inputs as the Dart screens validate them.
    let payee = outside_address();
    assert!(proposals::check_address("regtest".into(), payee.clone()).valid);
    assert_eq!(proposals::parse_zec("1.5".into()), Some(150_000_000));
    assert_eq!(proposals::parse_zec("0,00000001".into()), Some(1));
    assert_eq!(proposals::parse_zec("1.000000001".into()), None);

    // A proposes 1 ZEC.
    let a = &members[0];
    let id = proposals::propose_payment(
        relay.clone(),
        lwd.clone(),
        a.db_dir.clone(),
        a.seeds.clone(),
        a.material.clone(),
        vec![PaymentInput {
            address: payee.clone(),
            amount_zat: 100_000_000,
            memo: "grant #1".into(),
        }],
    )
    .unwrap();
    let list = |m: &Member| {
        proposals::list_proposals(
            relay.clone(),
            m.state_dir.clone(),
            m.seeds.clone(),
            m.material.clone(),
        )
        .unwrap()
    };
    let p = &list(&members[1])[0];
    assert_eq!(p.id, id);
    assert_eq!(p.stage, ProposalStage::Open);
    assert_eq!(p.payments[0].memo, "grant #1");
    assert!(!p.is_mine);

    // A (the proposer) and B review independently, then approve; C stays out. So the
    // leader is one of the two signers and must sign its own part locally.
    for m in &members[..2] {
        let r = proposals::review_proposal(
            relay.clone(),
            lwd.clone(),
            m.db_dir.clone(),
            m.seeds.clone(),
            m.material.clone(),
            id.clone(),
        )
        .unwrap();
        assert!(r.verified, "review failed: {}", r.problem);
        assert_eq!(r.fee_zat, 10_000);
        proposals::approve_proposal(
            relay.clone(),
            lwd.clone(),
            m.db_dir.clone(),
            m.state_dir.clone(),
            m.seeds.clone(),
            m.material.clone(),
            id.clone(),
        )
        .unwrap();
    }
    let p = &list(&members[1])[0];
    assert_eq!(p.stage, ProposalStage::Approved);
    assert_eq!(p.my_vote, MyVote::Approved);

    // Approving twice is a NotReady error, not a crash.
    let again = proposals::approve_proposal(
        relay.clone(),
        lwd.clone(),
        members[1].db_dir.clone(),
        members[1].state_dir.clone(),
        members[1].seeds.clone(),
        members[1].material.clone(),
        id.clone(),
    );
    assert!(
        matches!(again, Err(e) if e.kind == rust_lib_zafe::api::error::ZafeErrorKind::NotReady)
    );

    // A leads and signs its own part; B answers on its next poll; A aggregates and
    // broadcasts.
    let leader = {
        let (relay, lwd, db, st, s, m, id) = (
            relay.clone(),
            lwd.clone(),
            a.db_dir.clone(),
            a.state_dir.clone(),
            a.seeds.clone(),
            a.material.clone(),
            id.clone(),
        );
        thread::spawn(move || {
            let mut events = Vec::new();
            proposals::send_with_progress(relay, lwd, db, st, s, m, id, |p| {
                println!("progress {:?} {}/{}", p.stage, p.received, p.needed);
                events.push((p.stage, p.txid));
            })
            .map(|_| events)
        })
    };
    let mut answered = 0;
    for _ in 0..60 {
        for m in &members[1..2] {
            answered += proposals::answer_signing_requests(
                relay.clone(),
                lwd.clone(),
                m.db_dir.clone(),
                m.state_dir.clone(),
                m.seeds.clone(),
                m.material.clone(),
            )
            .unwrap();
        }
        if answered >= 1 || leader.is_finished() {
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
    let events = leader.join().unwrap().expect("send failed");
    let (stage, txid) = events.last().cloned().unwrap();
    assert_eq!(stage, SendStage::Sent);
    let txid = txid.unwrap();
    println!("broadcast {txid}");

    let p = &list(&members[2])[0];
    assert_eq!(p.stage, ProposalStage::Sent);
    assert_eq!(p.txid.as_deref(), Some(txid.as_str()));
    assert!(!p.signing_started);

    // Mined: the vault's balance drops by amount + fee (unless it paid itself).
    chain.mine(1);
    thread::sleep(Duration::from_secs(3));
    let after = sync(&members[2]);
    println!("after: height {} total {}", after.height, after.total_zat);
    let _ = std::fs::remove_dir_all(&tmp);
}
