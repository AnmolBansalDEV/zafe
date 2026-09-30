//! Vault history as CSV (spec §11.3), built on the device from the vault log (payments
//! the vault sent) and the wallet database (payments it received). Nothing is sent to a
//! server.

use std::collections::BTreeMap;

use crate::{
    vault::{ProposalStatus, VaultState},
    wallet::{memo_text, ReceivedPayment},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Received,
    Sent,
}

/// One CSV row: a received transaction, or one payment of a sent proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryRow {
    /// Unix seconds: the mining block's time when known, else the proposer's clock (sent)
    /// or unknown (unmined receipts).
    pub date: Option<u64>,
    /// Display order (byte-reversed hex), like block explorers.
    pub txid: String,
    pub direction: Direction,
    /// Recipient address for sent payments; empty for receipts (a shielded sender is
    /// unknowable).
    pub counterparty: String,
    /// Contact name for the counterparty (no address book yet: always empty).
    pub contact: String,
    /// Positive for received, negative for sent.
    pub amount_zat: i128,
    /// The transaction fee, on the first row of a sent transaction only.
    pub fee_zat: Option<u64>,
    pub memo: String,
    /// Hex proposal id (sent only).
    pub proposal: String,
    /// The proposer (sent only): hex signing key, or "Name (hex)" when this device named
    /// that signer.
    pub proposer: String,
    /// The approvers (sent only), each like `proposer`.
    pub approvers: Vec<String>,
}

/// ZIP 317 fee of a transaction with `actions` Ironwood actions (what members enforce).
fn zip317_fee(actions: usize) -> u64 {
    5_000 * actions.max(2) as u64
}

fn display_txid(txid: &[u8; 32]) -> String {
    let mut t = *txid;
    t.reverse();
    hex::encode(t)
}

/// "Name (hex)" when `names` (signing key hex → local name) has a name, else the hex.
fn member_label(key: &[u8; 32], names: &BTreeMap<String, String>) -> String {
    let hex = hex::encode(key);
    match names.get(&hex).map(|n| n.trim()) {
        Some(name) if !name.is_empty() => format!("{name} ({hex})"),
        _ => hex,
    }
}

/// Rows for every proposal the vault broadcast, one per payment. `mined_time` gives the
/// mining block's time for a txid (protocol byte order) when the wallet has it; `names`
/// are this device's local signer names (signing key hex → name).
pub fn sent_rows(
    state: &VaultState,
    names: &BTreeMap<String, String>,
    mined_time: impl Fn(&[u8; 32]) -> Option<u64>,
) -> Vec<HistoryRow> {
    let mut rows = Vec::new();
    for p in state.proposals.values() {
        let (ProposalStatus::Broadcast, Some(txid)) = (p.status, p.txid) else {
            continue;
        };
        let date = mined_time(&txid).or((p.created_at > 0).then_some(p.created_at));
        for (i, payment) in p.payments.iter().enumerate() {
            rows.push(HistoryRow {
                date,
                txid: display_txid(&txid),
                direction: Direction::Sent,
                counterparty: payment.address.clone(),
                contact: String::new(),
                amount_zat: -i128::from(payment.amount_zat),
                fee_zat: (i == 0).then(|| zip317_fee(p.nullifiers.len())),
                memo: memo_text(&payment.memo).unwrap_or_default(),
                proposal: hex::encode(p.id),
                proposer: member_label(&p.author, names),
                approvers: p.approvals.keys().map(|k| member_label(k, names)).collect(),
            });
        }
    }
    rows
}

/// Rows for received payments (from `VaultWallet::received_payments`).
pub fn received_rows(received: &[ReceivedPayment]) -> Vec<HistoryRow> {
    received
        .iter()
        .map(|r| HistoryRow {
            date: r.block_time.map(u64::from),
            txid: r.txid.clone(),
            direction: Direction::Received,
            counterparty: String::new(),
            contact: String::new(),
            amount_zat: i128::from(r.amount_zat),
            fee_zat: None,
            memo: r.memos.join("\n\n"),
            proposal: String::new(),
            proposer: String::new(),
            approvers: vec![],
        })
        .collect()
}

pub const CSV_HEADER: &str =
    "date,txid,direction,counterparty,contact,amount_zec,fee_zec,memo,proposal_id,proposer,approvers";

/// The CSV text: a header, then rows oldest first (undated rows last), RFC 4180 quoting.
pub fn to_csv(mut rows: Vec<HistoryRow>) -> String {
    rows.sort_by_key(|r| (r.date.is_none(), r.date));
    let mut out = String::from(CSV_HEADER);
    out.push_str("\r\n");
    for r in rows {
        let fields = [
            r.date.map(iso8601).unwrap_or_default(),
            r.txid,
            match r.direction {
                Direction::Received => "received".into(),
                Direction::Sent => "sent".into(),
            },
            r.counterparty,
            r.contact,
            zec(r.amount_zat),
            r.fee_zat.map(|f| zec(i128::from(f))).unwrap_or_default(),
            r.memo,
            r.proposal,
            r.proposer,
            r.approvers.join("; "),
        ];
        let line: Vec<String> = fields.iter().map(|f| quote(f)).collect();
        out.push_str(&line.join(","));
        out.push_str("\r\n");
    }
    out
}

/// "-1.50000000": exact decimal ZEC (8 places), so spreadsheets don't round.
fn zec(zat: i128) -> String {
    let sign = if zat < 0 { "-" } else { "" };
    let abs = zat.unsigned_abs();
    format!("{sign}{}.{:08}", abs / 100_000_000, abs % 100_000_000)
}

/// Quotes a field when it holds a comma, quote or line break. A leading `=`, `+`, `-` or
/// `@` in text (not our numbers) is prefixed with `'` so spreadsheets don't run it as a
/// formula: memos are written by other people.
fn quote(field: &str) -> String {
    let is_number = field.parse::<f64>().is_ok();
    let field = if !is_number && field.starts_with(['=', '+', '-', '@']) {
        format!("'{field}")
    } else {
        field.to_owned()
    };
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field
    }
}

/// "2026-09-30T16:11:05Z" (UTC).
fn iso8601(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil from days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(date: Option<u64>, amount_zat: i128, memo: &str) -> HistoryRow {
        HistoryRow {
            date,
            txid: "ab".into(),
            direction: if amount_zat < 0 {
                Direction::Sent
            } else {
                Direction::Received
            },
            counterparty: String::new(),
            contact: String::new(),
            amount_zat,
            fee_zat: None,
            memo: memo.into(),
            proposal: String::new(),
            proposer: String::new(),
            approvers: vec![],
        }
    }

    #[test]
    fn named_members_show_name_and_key() {
        let names: BTreeMap<String, String> = [
            (hex::encode([1u8; 32]), "Alice".to_string()),
            (hex::encode([2u8; 32]), "  ".to_string()),
        ]
        .into();
        assert_eq!(
            member_label(&[1; 32], &names),
            format!("Alice ({})", hex::encode([1u8; 32]))
        );
        assert_eq!(member_label(&[2; 32], &names), hex::encode([2u8; 32]));
        assert_eq!(member_label(&[3; 32], &names), hex::encode([3u8; 32]));
    }

    #[test]
    fn dates_are_utc_iso8601() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso8601(1_790_726_400 + 58_265), "2026-09-30T16:11:05Z");
    }

    #[test]
    fn amounts_are_exact_decimals() {
        assert_eq!(zec(150_000_000), "1.50000000");
        assert_eq!(zec(-1), "-0.00000001");
        assert_eq!(zec(0), "0.00000000");
    }

    #[test]
    fn fields_are_quoted_and_formulas_defused() {
        let csv = to_csv(vec![
            row(Some(10), -5, "=HYPERLINK(\"x\")"),
            row(None, 7, "a, \"b\"\nc"),
            row(Some(5), 1, "plain"),
        ]);
        let lines: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(lines[0], CSV_HEADER);
        // Oldest first, undated last.
        assert!(lines[1].starts_with("1970-01-01T00:00:05Z,ab,received,,,0.00000001,,plain"));
        assert!(lines[2].contains(",-0.00000005,,\"'=HYPERLINK(\"\"x\"\")\","));
        assert!(csv.contains(",0.00000007,,\"a, \"\"b\"\"\nc\","));
    }
}
