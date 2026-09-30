//! Vault creation over a real HTTP relay with three concurrent members (spec §7.2).

use std::time::Duration;

use rand::{rngs::StdRng, SeedableRng};
use zafe_core::{
    node::{create_vault, join_vault, membership, run_keygen, seal, Invite, NodeError},
    relay_client::RelayClient,
    wallet::regtest_network,
};
use zafe_proto::Identity;
use zafe_relay::{
    limits::{Limits, Rate},
    quota::Quotas,
};

/// A relay over HTTP with `limits` (the hosted defaults unless a test needs others, so
/// these flows prove they fit in them).
async fn start_relay(limits: Limits) -> String {
    start_relay_with(limits, Quotas::hosted()).await
}

async fn start_relay_with(limits: Limits, quotas: Quotas) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            zafe_relay::Relay::new()
                .with_limits(limits)
                .with_quotas(quotas)
                .router(),
        )
        .await
        .unwrap();
    });
    format!("http://{addr}")
}

async fn setup() -> (RelayClient, Vec<Identity>, Invite, String) {
    let relay = RelayClient::new(start_relay(Limits::hosted()).await);
    let mut rng = StdRng::seed_from_u64(60);
    let ids: Vec<Identity> = (0..3).map(|_| Identity::generate(&mut rng)).collect();
    let invite = create_vault(&relay, &ids[0], "Grants", 2, 3, &mut rng)
        .await
        .unwrap();

    // The invite travels out of band as a string.
    let invite = Invite::decode(&invite.encode()).unwrap();
    for id in &ids[1..] {
        join_vault(&relay, id, &invite).await.unwrap();
    }
    seal(&relay, &ids[0], &invite).await.unwrap();

    // Every member sees the same safety number.
    let mut numbers = Vec::new();
    for id in &ids {
        let (members, sealed, number) = membership(&relay, id, &invite).await.unwrap();
        assert!(sealed);
        assert_eq!(members.len(), 3);
        numbers.push(number);
    }
    assert!(numbers.windows(2).all(|w| w[0] == w[1]));
    let number = numbers.remove(0);
    (relay, ids, invite, number)
}

#[tokio::test]
async fn three_members_create_a_vault_over_the_relay() {
    let (relay, ids, invite, number) = setup().await;
    let net = regtest_network();
    let timeout = Duration::from_secs(60);
    let (mut r0, mut r1, mut r2) = (
        StdRng::seed_from_u64(1),
        StdRng::seed_from_u64(2),
        StdRng::seed_from_u64(3),
    );

    let (a, b, c) = tokio::join!(
        run_keygen(
            &relay,
            &ids[0],
            &invite,
            &number,
            &net,
            "regtest",
            Some(2),
            &mut r0,
            timeout
        ),
        run_keygen(&relay, &ids[1], &invite, &number, &net, "regtest", None, &mut r1, timeout),
        run_keygen(&relay, &ids[2], &invite, &number, &net, "regtest", None, &mut r2, timeout),
    );
    let (a, b, c) = (a.unwrap(), b.unwrap(), c.unwrap());

    assert_eq!(a.descriptor, b.descriptor);
    assert_eq!(a.descriptor, c.descriptor);
    assert_eq!(
        a.descriptor.birthday_height, 2,
        "birthday comes from the creator"
    );
    assert!(a.descriptor.address.starts_with("uregtest1"));
    assert_eq!(a.vault_secret, b.vault_secret);
    assert_eq!(a.log_key, c.log_key);
    assert_ne!(
        a.key_package, b.key_package,
        "each member has its own share"
    );
    // Material round-trips into working keys.
    assert_eq!(
        a.vault_keys().unwrap().fvk().to_bytes(),
        c.vault_keys().unwrap().fvk().to_bytes()
    );
    assert_eq!(*a.key_package().unwrap().min_signers(), 2);
}

#[tokio::test]
async fn keygen_refuses_an_unconfirmed_safety_number() {
    let (relay, ids, invite, _number) = setup().await;
    let net = regtest_network();
    let mut rng = StdRng::seed_from_u64(4);
    let err = run_keygen(
        &relay,
        &ids[1],
        &invite,
        "0000 0000 0000",
        &net,
        "regtest",
        None,
        &mut rng,
        Duration::from_secs(5),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, NodeError::SafetyNumberMismatch { .. }),
        "{err:?}"
    );
}

#[tokio::test]
async fn a_rate_limited_client_gets_a_typed_error() {
    let relay = RelayClient::new(
        start_relay(Limits {
            per_key: Some(Rate::new(1, 1)),
            ..Limits::none()
        })
        .await,
    );
    let mut rng = StdRng::seed_from_u64(61);
    let me = Identity::generate(&mut rng);
    create_vault(&relay, &me, "Grants", 2, 3, &mut rng)
        .await
        .unwrap();
    let again = create_vault(&relay, &me, "Grants", 2, 3, &mut rng).await;
    assert!(
        matches!(
            again,
            Err(NodeError::Relay(
                zafe_core::relay_client::RelayClientError::RateLimited {
                    retry_after_secs: 60
                }
            ))
        ),
        "{again:?}"
    );
}

#[tokio::test]
async fn a_full_relay_gets_a_typed_error() {
    use zafe_core::relay_client::RelayClientError;
    use zafe_proto::{Envelope, Kind};

    let relay = RelayClient::new(
        start_relay_with(
            Limits::hosted(),
            Quotas {
                envelopes_per_recipient: Some(1),
                ..Quotas::none()
            },
        )
        .await,
    );
    let mut rng = StdRng::seed_from_u64(62);
    let a = Identity::generate(&mut rng);
    let b = Identity::generate(&mut rng);
    let mailbox = [7u8; 16];
    relay
        .create_mailbox(&a, mailbox, &[1; 32], 2)
        .await
        .unwrap();
    relay.join(&b, mailbox, [1; 32]).await.unwrap();
    let envelope = |seq| {
        Envelope::sealed(
            &a,
            b.public(),
            mailbox,
            seq,
            Kind::Approval,
            b"hi",
            &mut StdRng::seed_from_u64(seq),
        )
        .unwrap()
    };
    relay.send(&envelope(1)).await.unwrap();
    let err = relay.send(&envelope(2)).await.unwrap_err();
    assert!(
        matches!(&err, RelayClientError::StorageFull { detail } if detail.contains("inbox")),
        "{err:?}"
    );
}
