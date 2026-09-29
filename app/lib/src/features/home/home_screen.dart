import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/config/network_config.dart';
import '../../core/feedback/app_haptics.dart';
import '../../core/formatting/member_label.dart';
import '../../core/formatting/zec_amount.dart';
import '../../core/layout/mobile/mobile_top_nav.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_tappable.dart';
import '../../core/widgets/app_toast.dart';
import '../../notifications/vault_watch.dart';
import '../vaults/vault_switcher_sheet.dart';
import '../../providers/privacy_mode_provider.dart';
import '../../providers/proposals_provider.dart';
import '../../core/privacy/privacy_mask.dart';
import '../proposals/proposal_status.dart';
import '../../core/widgets/mobile/mobile_surface_card.dart';
import '../../providers/vault_provider.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen>
    with WidgetsBindingObserver {
  Timer? _poll;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    Future.microtask(_refresh);
    _poll = Timer.periodic(const Duration(seconds: 15), (_) => _refresh());
    // A vault exists from here on: notifications, background checks, push.
    unawaited(startVaultWatch());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.paused) {
      // Stop polling while in the background (the process may stay alive): background
      // checks and pushes take over, and only they may announce new activity.
      _poll?.cancel();
      _poll = null;
      unawaited(scheduleSoonCheck());
    }
    if (state == AppLifecycleState.resumed) {
      _poll ??= Timer.periodic(const Duration(seconds: 15), (_) => _refresh());
      unawaited(_refresh());
    }
  }

  Future<void> _refresh() async {
    // Proposals first: answering signing requests needs the synced tip, which the
    // previous sync already stored.
    await ref.read(proposalsProvider.notifier).refresh();
    await ref.read(vaultProvider.notifier).sync();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _poll?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final vault = ref.watch(vaultProvider);
    final summary = vault.summary;
    if (summary == null) return const SizedBox.shrink();

    final String syncLabel;
    if (vault.syncing) {
      syncLabel = 'Syncing...';
    } else if (vault.syncError != null) {
      syncLabel = vault.syncOffline
          ? 'Can\'t reach the network'
          : 'Sync failed, retrying';
    } else if (vault.balance != null) {
      syncLabel = 'Synced at ${vault.balance!.height}';
    } else {
      syncLabel = 'Connecting...';
    }

    return Scaffold(
      backgroundColor: colors.background.window,
      body: AppToastHost(
        child: SafeArea(
          bottom: false,
          child: Column(
            children: [
              MobileTopNav.account(
                accountName: summary.name,
                syncLabel: syncLabel,
                syncLabelColor: vault.syncError != null
                    ? colors.sync.textError
                    : colors.sync.text,
                syncAnimated: vault.syncing,
                onAccountTap: () => showVaultSwitcher(context),
                avatar: _VaultAvatar(
                  threshold: summary.threshold,
                  members: summary.members.length,
                ),
              ),
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.fromLTRB(16, 12, 16, 48),
                  children: [
                    _BalanceCard(
                      spendableZat: vault.balance?.spendableZat,
                      totalZat: vault.balance?.totalZat,
                      hidden: ref.watch(privacyModeProvider),
                      onToggle: () {
                        AppHaptics.privacyToggle();
                        ref.read(privacyModeProvider.notifier).toggle();
                      },
                    ),
                    const SizedBox(height: AppSpacing.s),
                    if ((vault.balance?.totalZat ?? BigInt.zero) == BigInt.zero)
                      AppButton(
                        expand: true,
                        onPressed: () => context.push('/receive'),
                        leading: const AppIcon(AppIcons.addNew, size: 20),
                        child: const Text(
                          'Receive your first $kZcashDefaultCurrencyTicker',
                        ),
                      )
                    else
                      Row(
                        children: [
                          Expanded(
                            child: AppButton(
                              expand: true,
                              onPressed: () => context.push('/send'),
                              leading: const AppIcon(AppIcons.plane, size: 20),
                              child: const Text('New payment'),
                            ),
                          ),
                          const SizedBox(width: AppSpacing.xs),
                          Expanded(
                            child: AppButton(
                              expand: true,
                              variant: AppButtonVariant.secondary,
                              onPressed: () => context.push('/receive'),
                              leading: const AppIcon(
                                AppIcons.arrowDownCircle,
                                size: 20,
                              ),
                              child: const Text('Receive'),
                            ),
                          ),
                        ],
                      ),
                    const SizedBox(height: AppSpacing.md),
                    _Payments(
                      proposals: ref.watch(proposalsProvider),
                      hideAmounts: ref.watch(privacyModeProvider),
                    ),
                    const SizedBox(height: AppSpacing.md),
                    _SignersCard(
                      threshold: summary.threshold,
                      members: summary.members,
                      me: vault.myKeyHex,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _VaultAvatar extends StatelessWidget {
  const _VaultAvatar({required this.threshold, required this.members});
  final int threshold;
  final int members;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return Container(
      width: 40,
      height: 40,
      decoration: BoxDecoration(
        color: colors.background.homeCard,
        shape: BoxShape.circle,
      ),
      alignment: Alignment.center,
      child: Text(
        '$threshold/$members',
        style: AppTypography.labelMedium.copyWith(
          color: colors.text.homeCard,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _BalanceCard extends StatelessWidget {
  const _BalanceCard({
    required this.spendableZat,
    required this.totalZat,
    required this.hidden,
    required this.onToggle,
  });

  final BigInt? spendableZat;
  final BigInt? totalZat;
  final bool hidden;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final total = totalZat;
    final amount = total == null
        ? '—'
        : ZecAmount.fromZatoshi(total).balance.amountText;
    final pending =
        (total != null && spendableZat != null && total > spendableZat!)
        ? ZecAmount.fromZatoshi(total - spendableZat!).balance.amountText
        : null;
    final homeText = colors.text.homeCard;

    return Container(
      decoration: BoxDecoration(
        color: colors.background.ground,
        borderRadius: BorderRadius.circular(AppRadii.large),
        boxShadow: appSurfaceShadow(colors),
      ),
      clipBehavior: Clip.antiAlias,
      child: Container(
        height: 200,
        padding: const EdgeInsets.all(AppSpacing.sm),
        decoration: BoxDecoration(
          color: colors.background.homeCard,
          borderRadius: BorderRadius.circular(AppRadii.large),
          border: Border.all(color: const Color(0x12FFFFFF), width: 1.5),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                AppIcon(AppIcons.shieldKeyhole, size: 20, color: homeText),
                const SizedBox(width: AppSpacing.xs),
                Text(
                  'Shielded vault balance',
                  style: AppTypography.bodySmall.copyWith(color: homeText),
                ),
                const Spacer(),
                AppTappable(
                  onTap: onToggle,
                  semanticsLabel: hidden ? 'Show balance' : 'Hide balance',
                  child: Container(
                    width: 32,
                    height: 32,
                    decoration: const BoxDecoration(
                      color: Color(0x0DFFFFFF),
                      shape: BoxShape.circle,
                    ),
                    alignment: Alignment.center,
                    child: AppIcon(
                      hidden ? AppIcons.eyeClosed : AppIcons.eye,
                      size: 16,
                      color: homeText,
                    ),
                  ),
                ),
              ],
            ),
            const Spacer(),
            if (pending != null && !hidden)
              Padding(
                padding: const EdgeInsets.only(bottom: AppSpacing.xs),
                child: Text(
                  '$pending $kZcashDefaultCurrencyTicker confirming',
                  style: AppTypography.bodySmall.copyWith(
                    color: homeText.withValues(alpha: 0.8),
                  ),
                ),
              ),
            Text.rich(
              TextSpan(
                children: [
                  TextSpan(
                    text: hidden ? fixedPrivacyMask() : amount,
                    style: TextStyle(
                      fontFamily: 'Young Serif',
                      fontSize: 45,
                      height: 48 / 45,
                      letterSpacing: -1.35,
                      color: homeText,
                      fontFeatures: const [FontFeature.liningFigures()],
                    ),
                  ),
                  TextSpan(
                    text: ' $kZcashDefaultCurrencyTicker',
                    style: TextStyle(
                      fontFamily: 'Young Serif',
                      fontSize: 32,
                      height: 33 / 32,
                      color: homeText,
                    ),
                  ),
                ],
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ],
        ),
      ),
    );
  }
}

class _SignersCard extends StatelessWidget {
  const _SignersCard({
    required this.threshold,
    required this.members,
    required this.me,
  });
  final int threshold;
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
                  'Signers',
                  style: AppTypography.labelLarge.copyWith(
                    color: colors.text.accent,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              Text(
                '$threshold of ${members.length} to approve',
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
                  Text(
                    memberLabel(m, me: me),
                    style: AppTypography.labelLarge.copyWith(
                      color: colors.text.primary,
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

class _Payments extends StatelessWidget {
  const _Payments({required this.proposals, required this.hideAmounts});
  final ProposalsState proposals;
  final bool hideAmounts;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final items = proposals.items.take(10).toList();
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.xs,
        vertical: AppSpacing.s,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Payments',
            style: AppTypography.labelLarge.copyWith(
              color: colors.text.accent,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          if (items.isEmpty)
            Text(
              proposals.loaded ? 'No payments yet' : 'Loading...',
              style: AppTypography.bodyMedium.copyWith(
                color: colors.text.secondary,
              ),
            ),
          for (final p in items)
            Padding(
              padding: const EdgeInsets.only(bottom: AppSpacing.s),
              child: ProposalRow(
                proposal: p,
                hideAmount: hideAmounts,
                onTap: () => context.push('/proposal/${p.id}'),
              ),
            ),
        ],
      ),
    );
  }
}
