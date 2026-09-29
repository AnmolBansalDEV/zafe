import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/errors/zafe_error_copy.dart';
import '../../core/formatting/member_label.dart';
import '../../core/formatting/zec_amount.dart';
import '../../core/layout/mobile/zafe_screen.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_copy_feedback.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_loading_icon.dart';
import '../../core/widgets/app_tappable.dart';
import '../../core/widgets/app_toast.dart';
import '../../core/widgets/mobile/mobile_address_verify_sheet.dart';
import '../../core/widgets/mobile/mobile_review_row.dart';
import '../../core/widgets/mobile/mobile_surface_card.dart';
import '../../core/widgets/mobile/zafe_detail.dart';
import '../../providers/privacy_mode_provider.dart';
import '../../providers/proposals_provider.dart';
import '../../core/privacy/amount_display.dart';
import '../../providers/vault_provider.dart';
import '../../rust/api/proposals.dart' as rust;
import 'proposal_status.dart';

/// One payment proposal: what it pays, this device's independent check, the signers'
/// votes, and the next action (approve/reject, or collect signatures and send).
class ProposalScreen extends ConsumerStatefulWidget {
  const ProposalScreen({super.key, required this.id});
  final String id;

  @override
  ConsumerState<ProposalScreen> createState() => _ProposalScreenState();
}

class _ProposalScreenState extends ConsumerState<ProposalScreen> {
  bool _voting = false;
  StreamSubscription<rust.SendProgress>? _send;
  rust.SendProgress? _progress;
  String? _sendError;
  Timer? _poll;

  @override
  void initState() {
    super.initState();
    Future.microtask(() => ref.read(proposalsProvider.notifier).refresh());
    _poll = Timer.periodic(const Duration(seconds: 5), (_) {
      if (_send == null) ref.read(proposalsProvider.notifier).refresh();
    });
  }

  @override
  void dispose() {
    _poll?.cancel();
    _send?.cancel();
    super.dispose();
  }

  Future<void> _vote(bool approve) async {
    setState(() => _voting = true);
    final notifier = ref.read(proposalsProvider.notifier);
    try {
      approve
          ? await notifier.approve(widget.id)
          : await notifier.reject(widget.id);
      if (mounted) showAppToast(context, approve ? 'Approved' : 'Rejected');
    } catch (e) {
      debugPrint('vote failed: ${describeError(e)}');
      if (mounted) {
        showAppToast(context, zafeErrorMessage(e), iconName: AppIcons.warning);
      }
    } finally {
      if (mounted) setState(() => _voting = false);
    }
  }

  void _startSend() {
    setState(() {
      _sendError = null;
      _progress = null;
    });
    _send = ref
        .read(proposalsProvider.notifier)
        .send(widget.id)
        .listen(
          (p) {
            final error = p.error;
            if (p.stage == rust.SendStage.failed && error != null) {
              _fail(error);
            } else {
              setState(() => _progress = p);
            }
          },
          onError: (Object e) => _fail(e),
          onDone: () => setState(() => _send = null),
        );
  }

  void _fail(Object e) {
    debugPrint('send failed: ${describeError(e)}');
    setState(() {
      _sendError = zafeErrorMessage(e, fallback: 'Send failed. Try again.');
      _send = null;
      _progress = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final vault = ref.watch(vaultProvider);
    final proposals = ref.watch(proposalsProvider);
    final p = proposals.byId(widget.id);
    final me = vault.myKeyHex;

    if (p == null) {
      return ZafeScreen(
        title: 'Payment',
        children: [
          const SizedBox(height: 120),
          Center(
            child: proposals.loaded
                ? Text(
                    'Proposal not found',
                    style: AppTypography.bodyMedium.copyWith(
                      color: colors.text.secondary,
                    ),
                  )
                : AppLoadingIcon(size: 24, color: colors.icon.muted),
          ),
        ],
      );
    }

    final payment = p.payments.first;
    final amount = ZecAmount.fromZatoshi(p.totalZat).receipt;
    final sending = _send != null;
    final sent =
        p.stage == rust.ProposalStage.sent ||
        _progress?.stage == rust.SendStage.sent;
    final showReview =
        p.stage == rust.ProposalStage.open ||
        p.stage == rust.ProposalStage.approved;

    return PopScope(
      canPop: !sending,
      child: ZafeScreen(
        title: sending ? 'Sending...' : _screenTitle(p),
        showBack: !sending,
        children: [
          MobileReviewInfoRow(
            label: 'Amount',
            value: amountWithTicker(
              amount.amountText,
              hide: ref.watch(privacyModeProvider),
            ),
            leading: const ZecCoin(),
            strikethrough:
                p.stage == rust.ProposalStage.rejected ||
                p.stage == rust.ProposalStage.cancelled,
          ),
          const Padding(
            padding: EdgeInsets.symmetric(vertical: AppSpacing.xs),
            child: Align(
              alignment: Alignment.centerLeft,
              child: MobileReviewFlowArrow(),
            ),
          ),
          MobileReviewInfoRow(
            label: 'To',
            value: p.payments.length == 1
                ? 'Shielded address'
                : '${p.payments.length} recipients',
            leading: MobileReviewIconBadge(
              child: AppIcon(
                AppIcons.wallet,
                size: 20,
                color: colors.icon.regular,
              ),
            ),
            bottom: Row(
              children: [
                AppIcon(
                  AppIcons.shieldKeyhole,
                  size: 16,
                  color: colors.icon.brandCrimson,
                ),
                const SizedBox(width: AppSpacing.xxs),
                Text(
                  compactAddress(payment.address),
                  style: AppTypography.labelMedium.copyWith(
                    color: colors.text.secondary,
                  ),
                ),
                const Spacer(),
                AppButton(
                  variant: AppButtonVariant.ghost,
                  size: AppButtonSize.small,
                  leading: const AppIcon(AppIcons.eye, size: 16),
                  onPressed: () => showMobileAddressVerifySheet(
                    context,
                    title: 'Full address',
                    address: payment.address,
                  ),
                  child: const Text('Full address'),
                ),
              ],
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          MobileSurfaceCard(
            cornerRadius: AppRadii.large,
            padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.sm,
              vertical: AppSpacing.lg,
            ),
            child: Column(
              children: [
                DetailRow(
                  label: 'Status',
                  value: '',
                  trailing: ProposalStatusChip(proposal: p, sending: sending),
                ),
                const DetailDivider(),
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
                if (showReview) ...[
                  const DetailDivider(),
                  _ReviewRows(id: p.id),
                ],
                if (p.txid != null || _progress?.txid != null) ...[
                  const DetailDivider(),
                  _TxIdRow(txid: (p.txid ?? _progress!.txid)!),
                ],
              ],
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          _Signers(proposal: p, members: vault.summary!.members, me: me),
          const SizedBox(height: AppSpacing.lg),
          ..._actions(p, sent: sent, sending: sending),
        ],
      ),
    );
  }

  List<Widget> _actions(
    rust.ProposalInfo p, {
    required bool sent,
    required bool sending,
  }) {
    final colors = context.colors;
    if (sent) return const [];

    if (sending || _sendError != null) {
      final progress = _progress;
      return [
        MobileSurfaceCard(
          cornerRadius: AppRadii.large,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  if (sending)
                    AppLoadingIcon(size: 20, color: colors.icon.accent),
                  if (!sending)
                    AppIcon(
                      AppIcons.warning,
                      size: 20,
                      color: colors.icon.destructive,
                    ),
                  const SizedBox(width: AppSpacing.xs),
                  Expanded(
                    child: Text(
                      sending
                          ? progress == null
                                ? 'Asking signers for signatures...'
                                : 'Signatures ${progress.received} of ${progress.needed}'
                          : 'Not sent yet',
                      style: AppTypography.labelLarge.copyWith(
                        color: colors.text.accent,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: AppSpacing.xs),
              Text(
                sending
                    ? 'The private transaction proof is built on this phone meanwhile. Keep Zafe open.'
                    : _sendError!,
                style: AppTypography.bodySmall.copyWith(
                  color: sending
                      ? colors.text.secondary
                      : colors.text.destructive,
                ),
              ),
            ],
          ),
        ),
        if (!sending) ...[
          const SizedBox(height: AppSpacing.sm),
          AppButton(
            expand: true,
            leading: const AppIcon(AppIcons.renew, size: 20),
            onPressed: _startSend,
            child: const Text('Try again'),
          ),
        ],
      ];
    }

    switch (p.stage) {
      case rust.ProposalStage.open:
        if (p.myVote != rust.MyVote.none) {
          final missing = p.threshold - p.approvals.length;
          return [
            Text(
              'You ${p.myVote == rust.MyVote.approved ? 'approved' : 'rejected'} this payment. '
              'Waiting for $missing more approval${missing == 1 ? '' : 's'}.',
              textAlign: TextAlign.center,
              style: AppTypography.bodySmall.copyWith(
                color: colors.text.secondary,
              ),
            ),
          ];
        }
        final review = ref.watch(proposalReviewProvider(p.id));
        final verified = review.value?.verified == true;
        return [
          AppButton(
            expand: true,
            leading: _voting ? null : const AppIcon(AppIcons.check, size: 20),
            onPressed: verified && !_voting ? () => _vote(true) : null,
            child: Text(_voting ? 'Checking and approving...' : 'Approve'),
          ),
          const SizedBox(height: AppSpacing.s),
          AppButton(
            expand: true,
            variant: AppButtonVariant.ghost,
            onPressed: _voting ? null : () => _vote(false),
            child: const Text('Reject'),
          ),
        ];
      case rust.ProposalStage.approved:
        return [
          AppButton(
            expand: true,
            leading: const AppIcon(AppIcons.plane, size: 20),
            onPressed: _startSend,
            child: Text(
              p.signingStarted ? 'Resume sending' : 'Collect signatures & send',
            ),
          ),
          const SizedBox(height: AppSpacing.xs),
          Text(
            'Any signer can send once the payment is approved. The approvers sign automatically '
            'when they open Zafe.',
            textAlign: TextAlign.center,
            style: AppTypography.bodySmall.copyWith(
              color: colors.text.secondary,
            ),
          ),
        ];
      case rust.ProposalStage.rejected:
      case rust.ProposalStage.cancelled:
      case rust.ProposalStage.sent:
        return const [];
    }
  }
}

/// Fee, change and the "verified on this device" line, from `review_proposal`.
/// Short page titles (the serif nav title truncates past ~14 characters).
String _screenTitle(rust.ProposalInfo p) => switch (p.stage) {
  rust.ProposalStage.open =>
    p.myVote == rust.MyVote.none ? 'Review payment' : 'Awaiting votes',
  _ => proposalTitle(p),
};

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

class _Signers extends StatelessWidget {
  const _Signers({
    required this.proposal,
    required this.members,
    required this.me,
  });
  final rust.ProposalInfo proposal;
  final List<String> members;
  final String? me;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return MobileSurfaceCard(
      cornerRadius: AppRadii.large,
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
              Text(
                '${proposal.approvals.length} of ${proposal.threshold} needed',
                style: AppTypography.labelMedium.copyWith(
                  color: colors.text.secondary,
                ),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.xs),
          for (final m in members)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: AppSpacing.xs),
              child: Row(
                children: [
                  AppIcon(AppIcons.user, size: 16, color: colors.icon.muted),
                  const SizedBox(width: AppSpacing.s),
                  Expanded(
                    child: Text(
                      memberLabel(m, me: me),
                      style: AppTypography.labelLarge.copyWith(
                        color: colors.text.primary,
                      ),
                    ),
                  ),
                  _VoteTag(
                    approved: proposal.approvals.contains(m),
                    rejected: proposal.rejections.contains(m),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

class _VoteTag extends StatelessWidget {
  const _VoteTag({required this.approved, required this.rejected});
  final bool approved;
  final bool rejected;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final (icon, label, color) = approved
        ? (AppIcons.checkCircle, 'Approved', colors.text.positiveStrong)
        : rejected
        ? (AppIcons.cross, 'Rejected', colors.text.destructive)
        : (AppIcons.time, 'Waiting', colors.text.muted);
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        AppIcon(icon, size: 16, color: color),
        const SizedBox(width: AppSpacing.xxs),
        Text(label, style: AppTypography.labelMedium.copyWith(color: color)),
      ],
    );
  }
}
