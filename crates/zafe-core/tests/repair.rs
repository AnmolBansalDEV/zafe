//! Moving a lost member's seat to a new device and repairing its key share over a real HTTP
//! relay (spec §10.1, §10.4.2).

use std::{collections::BTreeMap, time::Duration};

use pasta_curves::pallas;
use rand::{rngs::StdRng, SeedableRng};
use zafe_core::{
    node::{
        create_vault, join_vault, load_state, membership, run_keygen, seal, set_name, NodeError,
        VaultMaterial,
    },
    relay_client::RelayClient,
    repair::{
        approve_replacement, current_material, help_repairs, try_recover, RecoveryRequest,
        RecoveryStatus,
    },
    signing,
    wallet::regtest_network,
};
use zafe_proto::Identity;
use zafe_relay::limits::Limits;

async fn start_relay() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            zafe_relay::Relay::new()
                .with_limits(Limits::hosted())
                .router(),
        )
        .await
        .unwrap();
    });
    format!("http://{addr}")
}

/// A 2-of-3 vault over the relay: the three members' identities and material.
async fn vault(relay: &RelayClient) -> (Vec<Identity>, Vec<VaultMaterial>) {
    let mut rng = StdRng::seed_from_u64(70);
    let ids: Vec<Identity> = (0..3).map(|_| Identity::generate(&mut rng)).collect();
    let invite = create_vault(relay, &ids[0], "Grants", 2, 3, &mut rng)
        .await
        .unwrap();
    for id in &ids[1..] {
        join_vault(relay, id, &invite).await.unwrap();
    }
    seal(relay, &ids[0], &invite).await.unwrap();
    let (_, _, number) = membership(relay, &ids[0], &invite).await.unwrap();
    let net = regtest_network();
    let timeout = Duration::from_secs(60);
    let (mut r0, mut r1, mut r2) = (
        StdRng::seed_from_u64(1),
        StdRng::seed_from_u64(2),
        StdRng::seed_from_u64(3),
    );
    let (a, b, c) = tokio::join!(
        run_keygen(
            relay,
            &ids[0],
            &invite,
            &number,
            &net,
            "regtest",
            Some(2),
            None,
            &mut r0,
            timeout
        ),
        run_keygen(relay, &ids[1], &invite, &number, &net, "regtest", None, None, &mut r1, timeout),
        run_keygen(relay, &ids[2], &invite, &number, &net, "regtest", None, None, &mut r2, timeout),
    );
    (ids, vec![a.unwrap(), b.unwrap(), c.unwrap()])
}

/// Two members sign a message with their key packages; aggregation verifies every share
/// and the signature under the group key.
fn sign_together(a: &VaultMaterial, b: &VaultMaterial, rng: &mut StdRng) {
    let alpha = pallas::Scalar::from(7u64);
    let (ka, kb) = (a.key_package().unwrap(), b.key_package().unwrap());
    let (na, ca) = signing::commit(&ka, rng);
    let (nb, cb) = signing::commit(&kb, rng);
    let package = signing::signing_package(
        BTreeMap::from([(*ka.identifier(), ca), (*kb.identifier(), cb)]),
        &[9; 32],
    );
    let shares = BTreeMap::from([
        (
            *ka.identifier(),
            signing::sign(&package, na, &ka, alpha).unwrap(),
        ),
        (
            *kb.identifier(),
            signing::sign(&package, nb, &kb, alpha).unwrap(),
        ),
    ]);
    signing::aggregate(&package, &shares, &b.public_key_package().unwrap(), alpha)
        .expect("a valid signature from the two shares");
}

/// The creator loses their phone. The other two approve moving the seat to a new device,
/// repair the share, and the new device ends up with the same key share, signs with
/// another member, and reads the vault log.
#[tokio::test]
async fn a_lost_seat_moves_to_a_new_device() {
    let relay = RelayClient::new(start_relay().await);
    let (ids, materials) = vault(&relay).await;
    let (a, b, c) = (&materials[0], &materials[1], &materials[2]);
    let mut rng = StdRng::seed_from_u64(71);
    let dir = std::env::temp_dir().join(format!("zafe-repair-{}", std::process::id()));
    let (dir_b, dir_c) = (dir.join("b"), dir.join("c"));
    set_name(&relay, &ids[0], a, "Alice", &mut rng)
        .await
        .unwrap();

    // The new device makes a fresh identity and shares its request.
    let fresh = Identity::generate(&mut rng);
    let request = RecoveryRequest {
        identity: *fresh.public(),
    };
    let code = request.encode();
    assert!(code.starts_with("zafe-recover-v1:"));
    let request = RecoveryRequest::decode(&code).unwrap();
    let safety = request.safety_code();
    assert_eq!(safety.len(), 9);
    assert!(matches!(
        try_recover(&relay, &fresh).await.unwrap(),
        RecoveryStatus::Waiting
    ));

    // The lost member can't approve its own move, and one approval isn't enough.
    let old = ids[0].public().sig_pk;
    assert!(matches!(
        approve_replacement(&relay, &ids[0], a, old, &request, &mut rng).await,
        Err(NodeError::Invalid(_))
    ));
    assert!(
        !approve_replacement(&relay, &ids[1], b, old, &request, &mut rng)
            .await
            .unwrap()
    );
    // Approving twice changes nothing.
    assert!(matches!(
        approve_replacement(&relay, &ids[1], b, old, &request, &mut rng).await,
        Err(NodeError::Invalid(_))
    ));
    assert!(matches!(
        try_recover(&relay, &fresh).await.unwrap(),
        RecoveryStatus::Waiting
    ));

    // The second approval moves the seat, on the log and on the relay.
    assert!(
        approve_replacement(&relay, &ids[2], c, old, &request, &mut rng)
            .await
            .unwrap()
    );
    let (_, state) = load_state(&relay, &ids[1], b).await.unwrap();
    assert!(state.descriptor.member(&old).is_none());
    assert!(state.descriptor.member(&fresh.public().sig_pk).is_some());
    assert_eq!(
        state.names[&fresh.public().sig_pk],
        "Alice",
        "the name moves too"
    );
    let on_relay = relay.members(&ids[1], a.descriptor.vault_id).await.unwrap();
    assert!(on_relay.members.contains(fresh.public()));
    assert!(!on_relay.members.iter().any(|m| m.sig_pk == old));
    assert!(
        load_state(&relay, &ids[0], a).await.is_err(),
        "the lost device is out"
    );
    assert!(matches!(
        try_recover(&relay, &fresh).await.unwrap(),
        RecoveryStatus::SeatMoved { received: 0, .. }
    ));

    // The two helpers repair the share, whenever each of them runs.
    let first = help_repairs(&relay, &ids[1], b, &state, &dir_b, &mut rng)
        .await
        .unwrap();
    assert_eq!(
        (first.deltas_sent, first.sigmas_sent, first.waiting),
        (1, 0, 1)
    );
    let second = help_repairs(&relay, &ids[2], c, &state, &dir_c, &mut rng)
        .await
        .unwrap();
    assert_eq!((second.deltas_sent, second.sigmas_sent), (1, 1));
    assert!(matches!(
        try_recover(&relay, &fresh).await.unwrap(),
        RecoveryStatus::SeatMoved {
            received: 1,
            needed: 2
        }
    ));
    let third = help_repairs(&relay, &ids[1], b, &state, &dir_b, &mut rng)
        .await
        .unwrap();
    assert_eq!((third.deltas_sent, third.sigmas_sent), (0, 1));
    // Done: nothing more to do.
    let again = help_repairs(&relay, &ids[1], b, &state, &dir_b, &mut rng)
        .await
        .unwrap();
    assert_eq!(again, Default::default());

    let RecoveryStatus::Done { material, invite } = try_recover(&relay, &fresh).await.unwrap()
    else {
        panic!("not recovered");
    };
    assert_eq!(invite.mailbox, a.descriptor.vault_id);
    assert_eq!(invite.threshold, 2);
    let (repaired, original) = (material.key_package().unwrap(), a.key_package().unwrap());
    assert_eq!(repaired.identifier(), original.identifier());
    assert_eq!(repaired.signing_share(), original.signing_share());
    assert_eq!(material.vault_secret, a.vault_secret);
    assert!(material.descriptor.member(&fresh.public().sig_pk).is_some());

    // The new device signs with another member and reads the log like any member.
    sign_together(&material, c, &mut rng);
    let (_, mine) = load_state(&relay, &fresh, &material).await.unwrap();
    assert_eq!(mine.replacements.len(), 1);
    set_name(&relay, &fresh, &material, "Alice (new phone)", &mut rng)
        .await
        .unwrap();
    let (_, state) = load_state(&relay, &ids[2], c).await.unwrap();
    assert_eq!(state.names[&fresh.public().sig_pk], "Alice (new phone)");

    // The other members' stored material picks up the new key.
    let updated = current_material(b, &state).expect("membership changed");
    assert!(updated.descriptor.member(&fresh.public().sig_pk).is_some());
    assert_eq!(updated.key_package, b.key_package);
    assert!(current_material(&updated, &state).is_none());
    load_state(&relay, &ids[1], &updated).await.unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
