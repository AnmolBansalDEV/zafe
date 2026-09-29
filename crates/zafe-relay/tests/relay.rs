//! Relay behavior over its HTTP API, driven in-process.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use tower::ServiceExt;
use zafe_proto::{
    relay::{
        decode_body, encode_body, join_token_hash, AppendResult, CreateMailbox, InboxRead,
        InboxResponse, Join, LogRead, LogResponse, MembersRead, MembersResponse, Remove, Seal,
        Signed,
    },
    Chain, Envelope, Identity, Kind, LogEntry, LogKey,
};
use zafe_relay::Relay;

const MAILBOX: [u8; 16] = [7; 16];
const NOW: u64 = 1_790_000_000;

async fn call(app: &Router, path: &str, body: Vec<u8>) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(Request::post(path).body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    (
        status,
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
}

async fn signed<T: serde::Serialize + serde::de::DeserializeOwned>(
    app: &Router,
    path: &str,
    who: &Identity,
    payload: T,
) -> (StatusCode, Vec<u8>) {
    call(
        app,
        path,
        Signed::new(who, payload).unwrap().to_bytes().unwrap(),
    )
    .await
}

struct World {
    app: Router,
    rng: StdRng,
    ids: Vec<Identity>,
    token: [u8; 32],
}

impl World {
    /// Creator (ids[0]) opens the mailbox; ids[1], ids[2] join; ids[3] is an outsider.
    async fn new(seal: bool) -> Self {
        let app = Relay::with_clock(Arc::new(|| NOW)).router();
        let mut rng = StdRng::seed_from_u64(5);
        let ids: Vec<Identity> = (0..4).map(|_| Identity::generate(&mut rng)).collect();
        let mut token = [0u8; 32];
        rng.fill_bytes(&mut token);

        let create = CreateMailbox {
            mailbox: MAILBOX,
            join_token_hash: join_token_hash(&token),
            max_members: 3,
        };
        assert_eq!(
            signed(&app, "/v1/mailbox/create", &ids[0], create).await.0,
            StatusCode::OK
        );
        for id in &ids[1..3] {
            let join = Join {
                mailbox: MAILBOX,
                join_token: token,
            };
            assert_eq!(
                signed(&app, "/v1/mailbox/join", id, join).await.0,
                StatusCode::OK
            );
        }
        let w = Self {
            app,
            rng,
            ids,
            token,
        };
        if seal {
            let members = w.ids[..3].iter().map(|i| i.public().sig_pk).collect();
            assert_eq!(
                signed(
                    &w.app,
                    "/v1/mailbox/seal",
                    &w.ids[0],
                    Seal {
                        mailbox: MAILBOX,
                        members
                    }
                )
                .await
                .0,
                StatusCode::OK
            );
        }
        w
    }

    async fn inbox(&self, who: usize) -> Vec<(u64, Envelope)> {
        let (status, body) = signed(
            &self.app,
            "/v1/inbox",
            &self.ids[who],
            InboxRead {
                mailbox: MAILBOX,
                after: 0,
                timestamp: NOW,
            },
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        decode_body::<InboxResponse>(&body).unwrap().envelopes
    }
}

#[tokio::test]
async fn membership_lifecycle() {
    let w = World::new(false).await;

    // Wrong token is refused.
    let mut bad = w.token;
    bad[0] ^= 1;
    let (status, _) = signed(
        &w.app,
        "/v1/mailbox/join",
        &w.ids[3],
        Join {
            mailbox: MAILBOX,
            join_token: bad,
        },
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Only the creator can seal, and only with exactly the current member set.
    let three: Vec<_> = w.ids[..3].iter().map(|i| i.public().sig_pk).collect();
    let (status, _) = signed(
        &w.app,
        "/v1/mailbox/seal",
        &w.ids[1],
        Seal {
            mailbox: MAILBOX,
            members: three.clone(),
        },
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = signed(
        &w.app,
        "/v1/mailbox/seal",
        &w.ids[0],
        Seal {
            mailbox: MAILBOX,
            members: three[..2].to_vec(),
        },
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = signed(
        &w.app,
        "/v1/mailbox/seal",
        &w.ids[0],
        Seal {
            mailbox: MAILBOX,
            members: three,
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // After sealing, even the right token cannot add members.
    let (status, _) = signed(
        &w.app,
        "/v1/mailbox/join",
        &w.ids[3],
        Join {
            mailbox: MAILBOX,
            join_token: w.token,
        },
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = signed(
        &w.app,
        "/v1/mailbox/members",
        &w.ids[2],
        MembersRead {
            mailbox: MAILBOX,
            timestamp: NOW,
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let members = decode_body::<MembersResponse>(&body).unwrap();
    assert!(members.sealed);
    assert_eq!(members.members.len(), 3);

    // Outsiders cannot list members; stale requests are refused.
    let outsider = signed(
        &w.app,
        "/v1/mailbox/members",
        &w.ids[3],
        MembersRead {
            mailbox: MAILBOX,
            timestamp: NOW,
        },
    )
    .await;
    assert_eq!(outsider.0, StatusCode::FORBIDDEN);
    let stale = signed(
        &w.app,
        "/v1/mailbox/members",
        &w.ids[1],
        MembersRead {
            mailbox: MAILBOX,
            timestamp: NOW - 3600,
        },
    )
    .await;
    assert_eq!(stale.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn envelopes_are_routed_to_the_right_inboxes() {
    let mut w = World::new(true).await;
    let sealed = Envelope::sealed(
        &w.ids[0],
        w.ids[1].public(),
        MAILBOX,
        1,
        Kind::DkgRound2,
        b"for 1",
        &mut w.rng,
    )
    .unwrap();
    let public = Envelope::public(&w.ids[0], MAILBOX, 2, Kind::DkgRound1, b"for all").unwrap();
    for env in [&sealed, &public] {
        assert_eq!(
            call(&w.app, "/v1/envelope", env.to_bytes().unwrap())
                .await
                .0,
            StatusCode::OK
        );
    }

    let inbox1 = w.inbox(1).await;
    let inbox2 = w.inbox(2).await;
    assert_eq!(inbox1.len(), 2);
    assert_eq!(
        inbox2.len(),
        1,
        "sealed envelope only goes to its recipient"
    );
    assert!(
        w.inbox(0).await.is_empty(),
        "sender does not receive its own broadcast"
    );
    assert_eq!(
        inbox1[0].1.open(&w.ids[1], w.ids[0].public()).unwrap(),
        b"for 1"
    );

    // Replay of an already-delivered envelope is refused.
    assert_eq!(
        call(&w.app, "/v1/envelope", sealed.to_bytes().unwrap())
            .await
            .0,
        StatusCode::CONFLICT
    );
    // Outsiders cannot post or read.
    let intruder = Envelope::public(&w.ids[3], MAILBOX, 1, Kind::Approval, b"x").unwrap();
    assert_eq!(
        call(&w.app, "/v1/envelope", intruder.to_bytes().unwrap())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let read = signed(
        &w.app,
        "/v1/inbox",
        &w.ids[3],
        InboxRead {
            mailbox: MAILBOX,
            after: 0,
            timestamp: NOW,
        },
    )
    .await;
    assert_eq!(read.0, StatusCode::FORBIDDEN);
    // A forged envelope claiming a member as sender fails verification.
    let mut forged = Envelope::public(&w.ids[3], MAILBOX, 9, Kind::Approval, b"x").unwrap();
    forged.header.from = w.ids[1].public().sig_pk;
    assert_eq!(
        call(&w.app, "/v1/envelope", forged.to_bytes().unwrap())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn log_appends_must_extend_the_head() {
    let mut w = World::new(false).await;
    let key = LogKey::generate(0, &mut w.rng);
    let first = LogEntry::create(
        &w.ids[0],
        &key,
        MAILBOX,
        0,
        [0; 32],
        b"vault created",
        &mut w.rng,
    )
    .unwrap();
    // No log before membership is sealed.
    assert_eq!(
        call(&w.app, "/v1/log/append", encode_body(&first).unwrap())
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    let members: Vec<_> = w.ids[..3].iter().map(|i| i.public().sig_pk).collect();
    signed(
        &w.app,
        "/v1/mailbox/seal",
        &w.ids[0],
        Seal {
            mailbox: MAILBOX,
            members,
        },
    )
    .await;

    let (_, body) = call(&w.app, "/v1/log/append", encode_body(&first).unwrap()).await;
    assert_eq!(
        decode_body::<AppendResult>(&body).unwrap(),
        AppendResult::Appended { index: 0 }
    );

    // Two members race to append index 1: the second gets a conflict.
    let head = first.hash().unwrap();
    let a = LogEntry::create(&w.ids[1], &key, MAILBOX, 1, head, b"proposal A", &mut w.rng).unwrap();
    let b = LogEntry::create(&w.ids[2], &key, MAILBOX, 1, head, b"proposal B", &mut w.rng).unwrap();
    let (_, ra) = call(&w.app, "/v1/log/append", encode_body(&a).unwrap()).await;
    let (_, rb) = call(&w.app, "/v1/log/append", encode_body(&b).unwrap()).await;
    assert_eq!(
        decode_body::<AppendResult>(&ra).unwrap(),
        AppendResult::Appended { index: 1 }
    );
    assert_eq!(
        decode_body::<AppendResult>(&rb).unwrap(),
        AppendResult::Conflict { len: 2 }
    );

    // A member reads the log back and verifies the whole chain.
    let (_, body) = signed(
        &w.app,
        "/v1/log/read",
        &w.ids[2],
        LogRead {
            mailbox: MAILBOX,
            from: 0,
            timestamp: NOW,
        },
    )
    .await;
    let entries = decode_body::<LogResponse>(&body).unwrap().entries;
    let publics: Vec<_> = w.ids[..3].iter().map(|i| *i.public()).collect();
    let mut chain = Chain::new(MAILBOX);
    for e in entries {
        chain.append(e, &publics).unwrap();
    }
    assert_eq!(chain.len(), 2);
    assert_eq!(chain.entries()[1].decrypt(&key).unwrap(), b"proposal A");
}

#[tokio::test]
async fn member_cap_and_creator_removal() {
    let w = World::new(false).await;
    // Mailbox is full (3 of 3): a fourth identity with the right token is refused.
    let full = signed(
        &w.app,
        "/v1/mailbox/join",
        &w.ids[3],
        Join {
            mailbox: MAILBOX,
            join_token: w.token,
        },
    )
    .await;
    assert_eq!(full.0, StatusCode::FORBIDDEN);

    // Only the creator can remove, and only before sealing; the creator cannot be removed.
    let by_member = signed(
        &w.app,
        "/v1/mailbox/remove",
        &w.ids[1],
        Remove {
            mailbox: MAILBOX,
            member: w.ids[2].public().sig_pk,
        },
    )
    .await;
    assert_eq!(by_member.0, StatusCode::FORBIDDEN);
    let creator_self = signed(
        &w.app,
        "/v1/mailbox/remove",
        &w.ids[0],
        Remove {
            mailbox: MAILBOX,
            member: w.ids[0].public().sig_pk,
        },
    )
    .await;
    assert_eq!(creator_self.0, StatusCode::FORBIDDEN);
    let ok = signed(
        &w.app,
        "/v1/mailbox/remove",
        &w.ids[0],
        Remove {
            mailbox: MAILBOX,
            member: w.ids[2].public().sig_pk,
        },
    )
    .await;
    assert_eq!(ok.0, StatusCode::OK);

    // A slot is free again.
    let joined = signed(
        &w.app,
        "/v1/mailbox/join",
        &w.ids[3],
        Join {
            mailbox: MAILBOX,
            join_token: w.token,
        },
    )
    .await;
    assert_eq!(joined.0, StatusCode::OK);
}
