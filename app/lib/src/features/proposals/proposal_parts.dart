import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/errors/zafe_error_copy.dart';
import '../../core/formatting/member_label.dart';
import '../../core/formatting/zec_amount.dart';
import '../../core/privacy/amount_display.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_copy_feedback.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_loading_icon.dart';
import '../../core/widgets/app_tappable.dart';
import '../../core/widgets/mobile/mobile_address_verify_sheet.dart';
import '../../core/widgets/mobile/mobile_surface_card.dart';
import '../../core/widgets/mobile/zafe_detail.dart';
import '../../providers/privacy_mode_provider.dart';
import '../../providers/proposals_provider.dart';
import '../../rust/api/proposals.dart' as rust;
import 'proposal_status.dart';

// The parts of the proposal screen below the nav bar, apart from the action buttons.
// Public so tool/screens/ can render them with fake data.

/// Payment card, details (with this device's independent check) and approvals.
class ProposalBody extends ConsumerWidget {
  const ProposalBody({
    super.key,
    required this.proposal,
    required this.members,
    required this.me,
    this.send,
  });
  final rust.ProposalInfo proposal;
  final List<String> members;
  final String? me;
  final SendState? send;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final p = proposal;
    final payment = p.payments.first;
    final progress = send?.progress;
    final sending = send?.running ?? false;
    final showReview =
        p.stage == rust.ProposalStage.open ||
        p.stage == rust.ProposalStage.approved;
    final txid = p.txid ?? progress?.txid;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        PaymentCard(
          amountText: ZecAmount.fromZatoshi(p.totalZat).receipt.amountText,
          hidden: ref.watch(privacyModeProvider),
          address: payment.address,
          recipients: p.payments.length,
          strikethrough:
              p.stage == rust.ProposalStage.rejected ||
              p.stage == rust.ProposalStage.cancelled,
          onFullAddress: () => showMobileAddressVerifySheet(
            context,
            title: 'Full address',
            address: payment.address,
          ),
        ),
        const SizedBox(height: AppSpacing.md),
        ProposalApprovalsCard(
          proposal: p,
          members: members,
          me: me,
          sending: sending,
        ),
        const SizedBox(height: AppSpacing.md),
        MobileSurfaceCard(
          cornerRadius: AppRadii.large,
          padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.sm,
            vertical: AppSpacing.md,
          ),
          child: Column(
            children: [
              DetailRow(
                label: 'Proposed by',
                value: memberLabel(p.author, me: me),
              ),
              if (p.createdAt > BigInt.zero) ...[
                const DetailDivider(),
                DetailRow(
                  label: 'Created',
                  value: formatTimestamp(p.createdAt),
                ),
              ],
              if (payment.memo.isNotEmpty) ...[
                const DetailDivider(),
                DetailRow(label: 'Message', value: payment.memo),
              ],
              if (showReview) ...[const DetailDivider(), _ReviewRows(id: p.id)],
              if (txid != null) ...[
                const DetailDivider(),
                _TxIdRow(txid: txid),
              ],
            ],
          ),
        ),
      ],
    );
  }
}

/// Status, the approval threshold as signer dots, and one row per signer with their
/// vote.
class ProposalApprovalsCard extends StatelessWidget {
  const ProposalApprovalsCard({
    super.key,
    required this.proposal,
    required this.members,
    required this.me,
    this.sending = false,
  });
  final rust.ProposalInfo proposal;
  final List<String> members;
  final String? me;
  final bool sending;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final p = proposal;
    return MobileSurfaceCard(
      cornerRadius: AppRadii.large,
      padding: const EdgeInsets.fromLTRB(
        AppSpacing.sm,
        AppSpacing.sm,
        AppSpacing.sm,
        AppSpacing.xs,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  'Approvals',
                  style: AppTypography.labelLarge.copyWith(
                    color: colors.text.accent,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              ProposalStatusChip(proposal: p, sending: sending),
            ],
          ),
          const SizedBox(height: AppSpacing.s),
          Row(
            children: [
              Expanded(
                child: ApprovalDots(
                  approved: p.approvals.length,
                  rejected: p.rejections.length,
                  members: members.length,
                  threshold: p.threshold,
                ),
              ),
              Text(
                // Approvals so far, then the vault's rule (t of n), so a 2-of-3 vault never
                // reads as "2 of 2".
                '${p.approvals.length} approved · needs ${p.threshold} of ${members.length}',
                style: AppTypography.labelMedium.copyWith(
                  color: colors.text.secondary,
                ),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.s),
          Container(height: 1, color: colors.border.regular),
          const SizedBox(height: AppSpacing.xs),
          for (final m in members)
            SignerRow(
              keyHex: m,
              me: me,
              trailing: _VoteTag(
                approved: p.approvals.contains(m),
                rejected: p.rejections.contains(m),
                stage: p.stage,
              ),
            ),
        ],
      ),
    );
  }
}

/// Fee, change and the "verified on this device" line, from `review_proposal`.
class _ReviewRows extends ConsumerWidget {
  const _ReviewRows({required this.id});
  final String id;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.colors;
    final review = ref.watch(proposalReviewProvider(id));
    final hide = ref.watch(privacyModeProvider);
    return review.when(
      loading: () => DetailRow(
        label: 'Checking on this device',
        value: '',
        trailing: AppLoadingIcon(size: 16, color: colors.icon.muted),
      ),
      error: (e, _) => AppTappable(
        onTap: () => ref.invalidate(proposalReviewProvider(id)),
        child: DetailRow(
          label: 'Check',
          value: zafeErrorMessage(
            e,
            fallback: 'Couldn\'t check. Tap to retry.',
          ),
          trailing: AppIcon(AppIcons.renew, size: 16, color: colors.icon.muted),
        ),
      ),
      data: (r) => r.verified
          ? Column(
              children: [
                DetailRow(
                  label: 'Tx fee',
                  value: amountWithTicker(
                    ZecAmount.fromZatoshi(r.feeZat).fee.amountText,
                    hide: hide,
                  ),
                ),
                const DetailDivider(),
                DetailRow(
                  label: 'Change to vault',
                  value: amountWithTicker(
                    ZecAmount.fromZatoshi(r.changeZat).receipt.amountText,
                    hide: hide,
                  ),
                ),
                const DetailDivider(),
                DetailRow(
                  label: 'Checked on this device',
                  value: 'Matches',
                  trailing: AppIcon(
                    AppIcons.checkCircle,
                    size: 16,
                    color: colors.icon.success,
                  ),
                ),
              ],
            )
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                DetailRow(
                  label: 'Checked on this device',
                  value: 'Does not match',
                  trailing: AppIcon(
                    AppIcons.warning,
                    size: 16,
                    color: colors.icon.destructive,
                  ),
                ),
                const SizedBox(height: AppSpacing.xs),
                Text(
                  'The transaction does not do exactly what the proposal says. Don\'t approve it. (${r.problem})',
                  style: AppTypography.bodySmall.copyWith(
                    color: colors.text.destructive,
                  ),
                ),
              ],
            ),
    );
  }
}

class _TxIdRow extends StatelessWidget {
  const _TxIdRow({required this.txid});
  final String txid;

  @override
  Widget build(BuildContext context) {
    return AppTappable(
      onTap: () => copyTextWithToast(
        context,
        text: txid,
        toastMessage: 'Transaction ID copied',
      ),
      semanticsLabel: 'Copy transaction ID',
      child: DetailRow(
        label: 'Tx ID',
        value: '${txid.substring(0, 8)}...${txid.substring(txid.length - 8)}',
        trailing: AppIcon(
          AppIcons.copy,
          size: 16,
          color: context.colors.icon.muted,
        ),
      ),
    );
  }
}

class _VoteTag extends StatelessWidget {
  const _VoteTag({
    required this.approved,
    required this.rejected,
    required this.stage,
  });
  final bool approved;
  final bool rejected;
  final rust.ProposalStage stage;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    if (approved) {
      return SignerTag(
        icon: AppIcons.checkCircle,
        iconColor: colors.icon.brand,
        label: 'Approved',
        color: colors.text.secondary,
      );
    }
    if (rejected) {
      return SignerTag(
        icon: AppIcons.cross,
        iconColor: colors.icon.destructive,
        label: 'Rejected',
        color: colors.text.secondary,
      );
    }
    // A member who hasn't voted is only "waiting" while their vote can still matter.
    return switch (stage) {
      rust.ProposalStage.open => SignerTag(
        icon: AppIcons.time,
        label: 'Waiting',
        color: colors.text.secondary,
      ),
      rust.ProposalStage.approved => SignerTag(
        label: 'Not needed',
        color: colors.text.muted,
      ),
      _ => SignerTag(label: 'Didn\'t vote', color: colors.text.muted),
    };
  }
}
