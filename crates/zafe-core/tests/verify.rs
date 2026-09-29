//! Member verification of proposal PCZTs (spec §9.3): the honest case, and each way a
//! malicious proposer, leader or relay could try to get a bad transaction signed.

use orchard::{
    keys::{FullViewingKey, Scope},
    Address,
};
use pczt::Pczt;
use rand::{rngs::StdRng, SeedableRng};
use zafe_core::verify::{verify_pczt, Expectations, Payment, VerifyError};
use zcash_protocol::memo::{Memo, MemoBytes};

mod common;
use common::*;

const FUNDS: u64 = 1_000_000;
const FEE_2_ACTIONS: u64 = 10_000;
const FEE_3_ACTIONS: u64 = 15_000;

struct Vault {
    fvk: FullViewingKey,
}

impl Vault {
    fn new(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let members = run_keygen(params(2, 3), &mut rng);
        Self {
            fvk: members[0].1.vault_keys.fvk().clone(),
        }
    }

    fn change_address(&self) -> Address {
        self.fvk.address_at(0u32, Scope::Internal)
    }

    /// Builds a PCZT spending a freshly received vault note to `outputs`.
    fn pczt(&self, outputs: Vec<Out>) -> Pczt {
        let note =
            receive_ironwood_note(&self.fvk, self.fvk.address_at(0u32, Scope::External), FUNDS);
        let (anchor, path) = witness(&note);
        build_pczt(&self.fvk, note, path, anchor, outputs)
    }

    /// An output recoverable by the vault (how Zafe builds payments).
    fn pay(&self, recipient: Address, value: u64, memo: &MemoBytes) -> Out {
        Out {
            ovk: Some(self.fvk.to_ovk(Scope::External)),
            recipient,
            value,
            memo: memo.clone(),
        }
    }

    fn change(&self, value: u64) -> Out {
        Out {
            ovk: Some(self.fvk.to_ovk(Scope::Internal)),
            recipient: self.change_address(),
            value,
            memo: MemoBytes::empty(),
        }
    }
}

fn memo(text: &str) -> MemoBytes {
    Memo::from_bytes(text.as_bytes()).unwrap().encode()
}

fn payment(recipient: Address, amount_zat: u64, memo: &MemoBytes) -> Payment {
    Payment {
        recipient,
        amount_zat,
        memo: *memo.as_array(),
    }
}

fn expect(payments: Vec<Payment>) -> Expectations {
    Expectations {
        payments,
        consensus_branch_id: branch_id(),
        tip_height: TARGET_HEIGHT - 1,
        max_expiry_delta: 100,
    }
}

#[test]
fn honest_payment_with_change_verifies() {
    let v = Vault::new(30);
    let payee = outside_address(7);
    let m = memo("Invoice #123");
    let pczt = v.pczt(vec![
        v.pay(payee, 400_000, &m),
        v.change(FUNDS - 400_000 - FEE_2_ACTIONS),
    ]);

    let verified = verify_pczt(&pczt, &v.fvk, &expect(vec![payment(payee, 400_000, &m)])).unwrap();
    assert_eq!(verified.input_total_zat, FUNDS);
    assert_eq!(verified.change_total_zat, FUNDS - 400_000 - FEE_2_ACTIONS);
    assert_eq!(verified.fee_zat, FEE_2_ACTIONS);
    assert_eq!(verified.spends_to_sign.len(), 1);
    assert_eq!(
        verified.sighash,
        zafe_core::tx::shielded_sighash(&pczt).unwrap()
    );
}

#[test]
fn batch_payment_verifies() {
    let v = Vault::new(31);
    let (a, b) = (outside_address(7), outside_address(8));
    let (ma, mb) = (memo("grant A"), memo("grant B"));
    let pczt = v.pczt(vec![
        v.pay(a, 300_000, &ma),
        v.pay(b, 200_000, &mb),
        v.change(FUNDS - 500_000 - FEE_3_ACTIONS),
    ]);
    let ok = verify_pczt(
        &pczt,
        &v.fvk,
        &expect(vec![payment(b, 200_000, &mb), payment(a, 300_000, &ma)]),
    );
    assert!(ok.is_ok(), "{ok:?}");
}

#[test]
fn rejects_hidden_output_to_attacker() {
    // Pays the invoice, but "change" goes to an attacker instead of the vault.
    let v = Vault::new(32);
    let payee = outside_address(7);
    let m = memo("Invoice #123");
    let pczt = v.pczt(vec![
        v.pay(payee, 400_000, &m),
        v.pay(
            outside_address(66),
            FUNDS - 400_000 - FEE_2_ACTIONS,
            &MemoBytes::empty(),
        ),
    ]);
    // The builder shuffles action order, so match the error kind, not the index.
    let err = verify_pczt(&pczt, &v.fvk, &expect(vec![payment(payee, 400_000, &m)])).unwrap_err();
    assert!(matches!(err, VerifyError::UnexpectedOutput(_)), "{err:?}");
}

#[test]
fn rejects_wrong_amount_recipient_or_memo() {
    let v = Vault::new(33);
    let payee = outside_address(7);
    let m = memo("Invoice #123");
    let pczt = v.pczt(vec![
        v.pay(payee, 400_000, &m),
        v.change(FUNDS - 400_000 - FEE_2_ACTIONS),
    ]);

    for claimed in [
        payment(payee, 300_000, &m),                    // amount differs
        payment(outside_address(9), 400_000, &m),       // recipient differs
        payment(payee, 400_000, &memo("Invoice #999")), // memo differs
    ] {
        let err = verify_pczt(&pczt, &v.fvk, &expect(vec![claimed])).unwrap_err();
        assert!(matches!(err, VerifyError::UnexpectedOutput(_)), "{err:?}");
    }
}

#[test]
fn rejects_missing_payment() {
    let v = Vault::new(34);
    let payee = outside_address(7);
    let m = memo("x");
    let pczt = v.pczt(vec![
        v.pay(payee, 400_000, &m),
        v.change(FUNDS - 400_000 - FEE_2_ACTIONS),
    ]);
    let err = verify_pczt(
        &pczt,
        &v.fvk,
        &expect(vec![
            payment(payee, 400_000, &m),
            payment(outside_address(8), 1, &m),
        ]),
    )
    .unwrap_err();
    assert_eq!(err, VerifyError::MissingPayments(1));
}

#[test]
fn rejects_excess_fee() {
    // Under-reported change silently burns value as fee.
    let v = Vault::new(35);
    let payee = outside_address(7);
    let m = memo("x");
    let note = receive_ironwood_note(&v.fvk, v.fvk.address_at(0u32, Scope::External), FUNDS);
    let (anchor, path) = witness(&note);
    let outputs = vec![
        v.pay(payee, 400_000, &m),
        v.change(FUNDS - 400_000 - 50_000),
    ];
    let pczt = build_pczt_with_fee(&v.fvk, note, path, anchor, outputs, Some(50_000));
    let err = verify_pczt(&pczt, &v.fvk, &expect(vec![payment(payee, 400_000, &m)])).unwrap_err();
    assert_eq!(
        err,
        VerifyError::WrongFee {
            expected: FEE_2_ACTIONS,
            actual: 50_000
        }
    );
}

#[test]
fn rejects_unrecoverable_payment_output() {
    // Built without the vault OVK: the memo and recipient cannot be independently checked.
    let v = Vault::new(36);
    let payee = outside_address(7);
    let m = memo("x");
    let hidden = Out {
        ovk: None,
        recipient: payee,
        value: 400_000,
        memo: m.clone(),
    };
    let pczt = v.pczt(vec![hidden, v.change(FUNDS - 400_000 - FEE_2_ACTIONS)]);
    let err = verify_pczt(&pczt, &v.fvk, &expect(vec![payment(payee, 400_000, &m)])).unwrap_err();
    assert!(
        matches!(err, VerifyError::UnrecoverableOutput(_)),
        "{err:?}"
    );
}

#[test]
fn rejects_wrong_network_or_expiry() {
    let v = Vault::new(37);
    let payee = outside_address(7);
    let m = memo("x");
    let pczt = v.pczt(vec![
        v.pay(payee, 400_000, &m),
        v.change(FUNDS - 400_000 - FEE_2_ACTIONS),
    ]);
    let payments = vec![payment(payee, 400_000, &m)];

    let mut wrong_branch = expect(payments.clone());
    wrong_branch.consensus_branch_id ^= 1;
    assert!(matches!(
        verify_pczt(&pczt, &v.fvk, &wrong_branch).unwrap_err(),
        VerifyError::WrongBranchId { .. }
    ));

    let mut stale = expect(payments);
    stale.tip_height = TARGET_HEIGHT + 1_000; // the transaction's expiry has passed
    assert!(matches!(
        verify_pczt(&pczt, &v.fvk, &stale).unwrap_err(),
        VerifyError::BadExpiry { .. }
    ));
}

#[test]
fn rejects_spend_from_another_wallet() {
    let v = Vault::new(38);
    let other = FullViewingKey::from(&orchard::keys::SpendingKey::from_bytes([5; 32]).unwrap());
    let note = receive_ironwood_note(&other, other.address_at(0u32, Scope::External), FUNDS);
    let (anchor, path) = witness(&note);
    let payee = outside_address(7);
    let m = memo("x");
    let out = Out {
        ovk: Some(v.fvk.to_ovk(Scope::External)),
        recipient: payee,
        value: 400_000,
        memo: m.clone(),
    };
    let other_change = Out {
        ovk: None,
        recipient: other.address_at(0u32, Scope::Internal),
        value: FUNDS - 400_000 - FEE_2_ACTIONS,
        memo: MemoBytes::empty(),
    };
    let pczt = build_pczt(&other, note, path, anchor, vec![out, other_change]);
    let err = verify_pczt(&pczt, &v.fvk, &expect(vec![payment(payee, 400_000, &m)])).unwrap_err();
    assert!(matches!(err, VerifyError::ForeignSpend(_)), "{err:?}");
}
