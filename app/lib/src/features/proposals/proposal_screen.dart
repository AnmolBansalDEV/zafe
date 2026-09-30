import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/errors/zafe_error_copy.dart';
import '../../core/layout/mobile/zafe_screen.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_loading_icon.dart';
import '../../core/widgets/app_toast.dart';
import '../../core/widgets/mobile/mobile_surface_card.dart';
import '../../providers/proposals_provider.dart';
import '../../core/security/unlock_gate.dart';
import '../../providers/vault_provider.dart';
import '../../rust/api/proposals.dart' as rust;
import 'proposal_parts.dart';
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
  Timer? _poll;

  @override
  void initState() {
    super.initState();
    Future.microtask(() => ref.read(proposalsProvider.notifier).refresh());
    _poll = Timer.periodic(const Duration(seconds: 5), (_) {
      final send = ref.read(proposalsProvider).sends[widget.id];
      if (!(send?.running ?? false)) {
        ref.read(proposalsProvider.notifier).refresh();
      }
    });
  }

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }

  Future<void> _vote(bool approve) async {
    if (approve &&
        !await confirmUnlock(
          context,
          ref,
          reason: 'Unlock to approve and sign this payment',
        )) {
      return;
    }
    setState(() => _voting = true);
    final notifier = ref.read(proposalsProvider.notifier);
    try {
      if (approve) {
        final r = await notifier.approve(widget.id);
        if (mounted && r.completed && r.autoSend) {
          // This approval completed the signatures: watch it go out.
          context.push('/proposal/${widget.id}/send');
        } else if (mounted) {
          showAppToast(
            context,
            r.completed
                ? (r.autoSend
                      ? 'Approved. Sending now'
                      : 'Approved. Ready to send')
                : (r.signed ? 'Approved and signed' : 'Approved'),
          );
        }
      } else {
        await notifier.reject(widget.id);
        if (mounted) showAppToast(context, 'Rejected');
      }
    } catch (e) {
      debugPrint('vote failed: ${describeError(e)}');
      if (mounted) {
        showAppToast(
          context,
          zafeErrorMessage(e),
          iconName: AppIcons.warningCircle,
          tone: AppToastTone.destructive,
        );
      }
    } finally {
      if (mounted) setState(() => _voting = false);
    }
  }

  Future<void> _startSend() async {
    if (!await confirmUnlock(
      context,
      ref,
      reason: 'Unlock to send this payment',
    )) {
      return;
    }
    if (mounted) context.push('/proposal/${widget.id}/send');
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

    final send = proposals.sends[p.id];
    final progress = send?.progress;
    final sending = send?.running ?? false;
    final sent =
        p.stage == rust.ProposalStage.sent ||
        progress?.stage == rust.SendStage.sent;

    return PopScope(
      canPop: !sending,
      child: ZafeScreen(
        title: sending ? 'Sending...' : _screenTitle(p),
        showBack: !sending,
        children: [
          ProposalBody(
            proposal: p,
            members: vault.summary!.members,
            me: me,
            send: send,
          ),
          const SizedBox(height: AppSpacing.lg),
          ..._actions(p, sent: sent, send: send),
        ],
      ),
    );
  }

  List<Widget> _actions(
    rust.ProposalInfo p, {
    required bool sent,
    required SendState? send,
  }) {
    final colors = context.colors;
    if (sent) return const [];

    final sending = send?.running ?? false;
    if (send != null) {
      final progress = send.progress;
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
                          ? p.ready
                                ? 'Sending...'
                                : progress == null
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
                    ? p.ready
                          ? 'Every signature is in. Building the private transaction proof on this phone; keep Zafe open.'
                          : 'The private transaction proof is built on this phone meanwhile. Keep Zafe open.'
                    : send.error!,
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
          final signed = p.oneTap && p.myVote == rust.MyVote.approved;
          return [
            Text(
              'You ${p.myVote == rust.MyVote.approved ? (signed ? 'approved and signed' : 'approved') : 'rejected'} this payment. '
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
            child: Text(
              _voting
                  ? (p.oneTap
                        ? 'Checking and signing...'
                        : 'Checking and approving...')
                  : (p.oneTap ? 'Approve and sign' : 'Approve'),
            ),
          ),
          if (p.oneTap) ...[
            const SizedBox(height: AppSpacing.xs),
            Text(
              'Approving signs the payment on this device. It can\'t be withdrawn afterwards.',
              textAlign: TextAlign.center,
              style: AppTypography.bodySmall.copyWith(
                color: colors.text.secondary,
              ),
            ),
          ],
          const SizedBox(height: AppSpacing.s),
          AppButton(
            expand: true,
            variant: AppButtonVariant.ghost,
            onPressed: _voting ? null : () => _vote(false),
            child: const Text('Reject'),
          ),
        ];
      case rust.ProposalStage.approved when p.ready:
        return [
          AppButton(
            expand: true,
            leading: const AppIcon(AppIcons.plane, size: 20),
            onPressed: _startSend,
            child: const Text('Send now'),
          ),
          const SizedBox(height: AppSpacing.xs),
          Text(
            p.autoSend
                ? 'Every signature is in. It is sent automatically by the signer who approved last; you can also send it yourself.'
                : 'Every signature is in. Any signer can send it now; no one else needs to be online.',
            textAlign: TextAlign.center,
            style: AppTypography.bodySmall.copyWith(
              color: colors.text.secondary,
            ),
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

/// Short page titles (the nav title has room for about 20 characters).
String _screenTitle(rust.ProposalInfo p) => switch (p.stage) {
  rust.ProposalStage.open =>
    p.myVote == rust.MyVote.none ? 'Review payment' : 'Awaiting votes',
  _ => proposalTitle(p),
};
