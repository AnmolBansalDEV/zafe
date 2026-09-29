import 'package:flutter/widgets.dart';

import '../../core/formatting/zec_amount.dart';
import '../../core/privacy/amount_display.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_tappable.dart';
import '../../rust/api/proposals.dart' as rust;

/// Title for a proposal from this member's point of view (sentence case, Vizor's tone).
String proposalTitle(rust.ProposalInfo p) => switch (p.stage) {
  rust.ProposalStage.open =>
    p.myVote == rust.MyVote.none
        ? 'Needs your approval'
        : 'Waiting for approvals',
  rust.ProposalStage.approved => 'Ready to send',
  rust.ProposalStage.rejected => 'Rejected',
  rust.ProposalStage.cancelled => 'Cancelled',
  rust.ProposalStage.sent => 'Sent',
};

String _icon(rust.ProposalInfo p) => switch (p.stage) {
  rust.ProposalStage.open =>
    p.myVote == rust.MyVote.none ? AppIcons.editFilled : AppIcons.time,
  rust.ProposalStage.approved => AppIcons.plane,
  rust.ProposalStage.rejected || rust.ProposalStage.cancelled => AppIcons.cross,
  rust.ProposalStage.sent => AppIcons.checkCircle,
};

const _months = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec',
];

/// "May 29, 13:40" (Vizor's activity timestamp).
String formatTimestamp(BigInt unixSeconds) {
  final t = DateTime.fromMillisecondsSinceEpoch(unixSeconds.toInt() * 1000);
  String two(int n) => n.toString().padLeft(2, '0');
  return '${_months[t.month - 1]} ${t.day}, ${two(t.hour)}:${two(t.minute)}';
}

class ProposalStatusChip extends StatelessWidget {
  const ProposalStatusChip({
    super.key,
    required this.proposal,
    this.sending = false,
  });
  final rust.ProposalInfo proposal;
  final bool sending;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final (icon, label, color) = sending
        ? (AppIcons.loader, 'Sending', colors.text.secondary)
        : switch (proposal.stage) {
            rust.ProposalStage.open => (
              AppIcons.time,
              '${proposal.approvals.length} of ${proposal.threshold} approved',
              colors.text.secondary,
            ),
            rust.ProposalStage.approved => (
              AppIcons.check,
              'Approved',
              colors.text.positiveStrong,
            ),
            rust.ProposalStage.rejected => (
              AppIcons.cross,
              'Rejected',
              colors.text.destructive,
            ),
            rust.ProposalStage.cancelled => (
              AppIcons.cross,
              'Cancelled',
              colors.text.muted,
            ),
            rust.ProposalStage.sent => (
              AppIcons.checkCircle,
              'Completed',
              colors.text.positiveStrong,
            ),
          };
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        AppIcon(icon, size: 16, color: color),
        const SizedBox(width: AppSpacing.xxs),
        Text(label, style: AppTypography.labelLarge.copyWith(color: color)),
      ],
    );
  }
}

/// Vizor's 44px activity row, for a proposal.
class ProposalRow extends StatelessWidget {
  const ProposalRow({
    super.key,
    required this.proposal,
    required this.onTap,
    this.hideAmount = false,
  });
  final rust.ProposalInfo proposal;
  final bool hideAmount;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final p = proposal;
    final needsMe =
        p.stage == rust.ProposalStage.open && p.myVote == rust.MyVote.none;
    final subtitle = switch (p.stage) {
      rust.ProposalStage.open =>
        '${p.approvals.length} of ${p.threshold} approved',
      rust.ProposalStage.approved => 'Approved',
      _ => p.createdAt > BigInt.zero ? formatTimestamp(p.createdAt) : '',
    };
    return AppTappable(
      onTap: onTap,
      child: SizedBox(
        height: 44,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: AppSpacing.xxs),
          child: Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: needsMe
                      ? colors.background.homeCard
                      : colors.background.neutralSubtleOpacity,
                  shape: BoxShape.circle,
                ),
                alignment: Alignment.center,
                child: AppIcon(
                  _icon(p),
                  size: 18,
                  color: needsMe ? colors.text.homeCard : colors.icon.regular,
                ),
              ),
              const SizedBox(width: AppSpacing.xs),
              Expanded(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      proposalTitle(p),
                      style: AppTypography.labelLarge.copyWith(
                        color: colors.text.accent,
                      ),
                    ),
                    const SizedBox(height: AppSpacing.xxs),
                    Text(
                      subtitle,
                      style: AppTypography.labelLarge.copyWith(
                        color: colors.text.secondary,
                      ),
                    ),
                  ],
                ),
              ),
              ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 128),
                child: Text(
                  amountWithTicker(
                    ZecAmount.fromZatoshi(p.totalZat).activity.amountText,
                    hide: hideAmount,
                    maskLength: 3, // Vizor's activity-row mask
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: AppTypography.labelLarge.copyWith(
                    color:
                        p.stage == rust.ProposalStage.rejected ||
                            p.stage == rust.ProposalStage.cancelled
                        ? colors.text.muted
                        : colors.text.accent,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
