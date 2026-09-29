import '../core/formatting/member_label.dart';
import '../core/formatting/zec_amount.dart';
import '../core/privacy/amount_display.dart';
import '../rust/api/proposals.dart' as rust;

/// A notification to show for a change in the vault.
class VaultUpdate {
  const VaultUpdate({
    required this.proposalId,
    required this.title,
    required this.body,
  });

  final String proposalId;
  final String title;
  final String body;

  @override
  String toString() => 'VaultUpdate($proposalId, $title, $body)';
}

/// What this device last saw of each proposal: `stage/myVote/ready`.
typedef SeenSnapshot = Map<String, String>;

String seenKey(rust.ProposalInfo p) =>
    '${p.stage.name}/${p.myVote.name}/${p.ready}';

/// The notifications worth showing between the last snapshot and now. Pure, so it is the
/// same in the app and in background checks, and testable.
///
/// Without an earlier snapshot nothing is announced (a fresh install shouldn't replay the
/// vault's history). Amounts are left out when `hideAmounts` (privacy mode) is on.
List<VaultUpdate> vaultUpdates({
  required SeenSnapshot? previous,
  required List<rust.ProposalInfo> proposals,
  required String vaultName,
  required bool hideAmounts,
}) {
  if (previous == null) return const [];
  final out = <VaultUpdate>[];
  for (final p in proposals) {
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
          out.add(update('payment needs your approval', what));
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
        out.add(update('payment rejected', what));
      case rust.ProposalStage.cancelled:
        out.add(update('payment cancelled', what));
    }
  }
  return out;
}

SeenSnapshot snapshotOf(List<rust.ProposalInfo> proposals) => {
  for (final p in proposals) p.id: seenKey(p),
};

/// Payments waiting for this member: a vote, or (once every signature is in and nobody is
/// auto-sending) a send.
int actionableCount(List<rust.ProposalInfo> proposals) => proposals
    .where(
      (p) =>
          (p.stage == rust.ProposalStage.open &&
              p.myVote == rust.MyVote.none) ||
          (p.stage == rust.ProposalStage.approved && !(p.ready && p.autoSend)),
    )
    .length;
