import '../core/formatting/member_label.dart';
import '../core/formatting/zec_amount.dart';
import '../core/privacy/amount_display.dart';
import '../features/proposals/proposal_status.dart' show proposalExpired;
import '../rust/api/proposals.dart' as rust;
import '../rust/api/received.dart' as rust;

/// A notification to show for a change in the vault.
class VaultUpdate {
  const VaultUpdate({
    required this.proposalId,
    required this.title,
    required this.body,
  });

  /// The proposal id, or `rx:<txid>` for a received payment (see [receivedKey]).
  final String proposalId;
  final String title;
  final String body;

  @override
  String toString() => 'VaultUpdate($proposalId, $title, $body)';
}

/// What this device last saw of each proposal (`stage/myVote/ready`), plus one
/// `rx:<txid>` entry per received payment and the [kReceivedMarker].
typedef SeenSnapshot = Map<String, String>;

/// Snapshot keys of received payments start with this.
const kReceivedPrefix = 'rx:';

/// Present once received payments are part of the snapshot. Older snapshots lack it, and
/// then the vault's past receipts must not be announced as new.
const kReceivedMarker = 'rx:*';

String receivedKey(String txid) => '$kReceivedPrefix$txid';

/// Present while this member has to approve a proposal again (its signature went into an
/// unfinished signing round), so the request is announced once.
String reapprovalKey(String proposalId) => 're:$proposalId';

String seenKey(rust.ProposalInfo p) =>
    '${p.stage.name}/${p.myVote.name}/${p.ready}';

/// The notifications worth showing between the last snapshot and now. Pure, so it is the
/// same in the app and in background checks, and testable.
///
/// Without an earlier snapshot nothing is announced (a fresh install shouldn't replay the
/// vault's history). Amounts are left out when `hideAmounts` (privacy mode) is on.
/// `names` are this device's local signer names (key hex → name): a signer it named is
/// mentioned by name (who proposed, rejected or cancelled); unnamed signers aren't
/// mentioned (a short key means little in a notification).
List<VaultUpdate> vaultUpdates({
  required SeenSnapshot? previous,
  required List<rust.ProposalInfo> proposals,
  required String vaultName,
  required bool hideAmounts,
  List<rust.ReceivedInfo> received = const [],
  Map<String, String> names = const {},
}) {
  String? nameOf(String keyHex) {
    final n = names[keyHex]?.trim();
    return n == null || n.isEmpty ? null : n;
  }

  if (previous == null) return const [];
  final out = <VaultUpdate>[];
  if (previous.containsKey(kReceivedMarker)) {
    for (final r in received) {
      if (previous.containsKey(receivedKey(r.txid))) continue;
      final amount = amountWithTicker(
        ZecAmount.fromZatoshi(r.amountZat).activity.amountText,
        hide: hideAmounts,
      );
      out.add(
        VaultUpdate(
          proposalId: receivedKey(r.txid),
          title:
              '$vaultName: ${r.isCoinbase ? 'mining reward' : 'payment'} received',
          body: hideAmounts ? 'Open Zafe to see it.' : '+$amount',
        ),
      );
    }
  }
  for (final p in proposals) {
    if (p.needsReapproval && !previous.containsKey(reapprovalKey(p.id))) {
      out.add(
        VaultUpdate(
          proposalId: p.id,
          title: '$vaultName: approve a payment again',
          body:
              'A signing round didn\'t finish. Approve again so the payment can be sent.',
        ),
      );
    }
    final before = previous[p.id];
    if (before == seenKey(p)) continue;
    final beforeStage = before?.split('/').first;
    final amount = amountWithTicker(
      ZecAmount.fromZatoshi(p.totalZat).activity.amountText,
      hide: hideAmounts,
    );
    final to = p.payments.length == 1
        ? compactAddress(p.payments.first.address)
        : '${p.payments.length} recipients';
    final what = hideAmounts ? 'A payment' : '$amount to $to';
    VaultUpdate update(String title, String body) =>
        VaultUpdate(proposalId: p.id, title: '$vaultName: $title', body: body);

    switch (p.stage) {
      case rust.ProposalStage.open:
        // New proposals that need this member's vote (not their own).
        if (before == null && !p.isMine && p.myVote == rust.MyVote.none) {
          final by = nameOf(p.author);
          out.add(
            update(
              'payment needs your approval',
              by == null
                  ? what
                  : '$by proposed ${hideAmounts ? 'a payment' : what}.',
            ),
          );
        }
      case rust.ProposalStage.approved:
        if (beforeStage == rust.ProposalStage.approved.name &&
            before!.endsWith('/${p.ready}')) {
          break;
        }
        if (p.ready && p.autoSend) break; // it is being sent; "sent" follows
        out.add(
          update(
            p.ready ? 'payment ready to send' : 'payment approved',
            p.ready
                ? '$what. Every signature is in; any signer can send it.'
                : '$what. Open Zafe to collect signatures and send.',
          ),
        );
      case rust.ProposalStage.sent:
        out.add(update('payment sent', what));
      case rust.ProposalStage.rejected:
        final by = [
          for (final k in p.rejections)
            if (nameOf(k) != null) nameOf(k)!,
        ];
        out.add(
          update(
            'payment rejected',
            by.isEmpty ? what : '$what. Rejected by ${by.join(', ')}.',
          ),
        );
      case rust.ProposalStage.cancelled:
        final by = p.isMine ? null : nameOf(p.author);
        out.add(
          update(
            'payment cancelled',
            by == null ? what : '$what. Cancelled by $by.',
          ),
        );
    }
  }
  return out;
}

/// The snapshot for `proposals` and/or `received`; a `null` list keeps that kind's
/// entries from `previous`.
SeenSnapshot snapshotOf(
  List<rust.ProposalInfo>? proposals, {
  List<rust.ReceivedInfo>? received,
  SeenSnapshot previous = const {},
}) => {
  if (proposals == null)
    for (final e in previous.entries)
      if (!e.key.startsWith(kReceivedPrefix)) e.key: e.value,
  if (proposals != null)
    for (final p in proposals) p.id: seenKey(p),
  if (proposals != null)
    for (final p in proposals)
      if (p.needsReapproval) reapprovalKey(p.id): '',
  if (received == null)
    for (final e in previous.entries)
      if (e.key.startsWith(kReceivedPrefix)) e.key: e.value,
  if (received != null) ...{
    kReceivedMarker: '',
    for (final r in received) receivedKey(r.txid): '',
  },
};

/// Payments waiting for this member: a vote, or (once every signature is in and nobody is
/// auto-sending) a send.
int actionableCount(List<rust.ProposalInfo> proposals, {int? height}) =>
    proposals
        .where(
          (p) =>
              !proposalExpired(p, height) &&
              (p.needsReapproval ||
                  (p.stage == rust.ProposalStage.open &&
                      p.myVote == rust.MyVote.none) ||
                  (p.stage == rust.ProposalStage.approved &&
                      !(p.ready && p.autoSend))),
        )
        .length;
