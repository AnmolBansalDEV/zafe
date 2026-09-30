import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/config/network_config.dart';
import '../../core/feedback/app_haptics.dart';
import '../../core/formatting/member_label.dart';
import '../../core/formatting/zec_amount.dart';
import '../../core/layout/mobile/mobile_top_nav.dart';
import '../../core/layout/mobile/mobile_top_scroll_fade.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_tappable.dart';
import '../../core/widgets/app_toast.dart';
import '../../notifications/vault_watch.dart';
import '../backup/backup_prompt_screen.dart' show backupStatusProvider;
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
                child: MobileTopScrollFade(
                  child: ListView(
                    padding: const EdgeInsets.fromLTRB(16, 16, 16, 64 + 48),
                    children: [
                      _BalanceCard(
                        spendableZat: vault.balance?.spendableZat,
                        totalZat: vault.balance?.totalZat,
                        hidden: ref.watch(privacyModeProvider),
                        threshold: summary.threshold,
                        members: summary.members.length,
                        onToggle: () {
                          AppHaptics.privacyToggle();
                          ref.read(privacyModeProvider.notifier).toggle();
                        },
                      ),
                      const SizedBox(height: AppSpacing.s),
                      if ((vault.balance?.totalZat ?? BigInt.zero) ==
                          BigInt.zero)
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
                                leading: const AppIcon(
                                  AppIcons.plane,
                                  size: 20,
                                ),
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
                      if (ref.watch(backupStatusProvider).value == false) ...[
                        const _BackupReminder(),
                        const SizedBox(height: AppSpacing.md),
                      ],
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
        color: colors.background.brandAlpha,
        borderRadius: BorderRadius.circular(AppRadii.small),
        border: Border.all(color: colors.border.brandStrong, width: 1.5),
      ),
      alignment: Alignment.center,
      child: Text(
        '$threshold/$members',
        style: AppTypography.labelMedium.copyWith(
          fontFamily: 'Space Grotesk',
          color: colors.text.primary,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

/// Zafe's vault card: the balance on a dark card with a jade glow and safe-dial rings,
/// and the approval rule as signer dots along the bottom.
class _BalanceCard extends StatelessWidget {
  const _BalanceCard({
    required this.spendableZat,
    required this.totalZat,
    required this.hidden,
    required this.onToggle,
    required this.threshold,
    required this.members,
  });

  final BigInt? spendableZat;
  final BigInt? totalZat;
  final bool hidden;
  final VoidCallback onToggle;
  final int threshold;
  final int members;

  static const _ink = Color(0xFF0E131B);
  static const _jade = Color(0xFF2EC4A6);

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final total = totalZat;
    final amount = total == null
        ? '—'
        : ZecAmount.fromZatoshi(total).compactBalance.amountText;
    final pending =
        (total != null && spendableZat != null && total > spendableZat!)
        ? ZecAmount.fromZatoshi(total - spendableZat!).balance.amountText
        : null;
    const homeText = Color(0xFFF2F4F8);

    return Container(
      height: 216,
      decoration: BoxDecoration(
        color: _ink,
        borderRadius: BorderRadius.circular(AppRadii.large),
        border: Border.all(color: const Color(0x1A2EC4A6), width: 1),
        boxShadow: appSurfaceShadow(colors),
      ),
      clipBehavior: Clip.antiAlias,
      child: Stack(
        children: [
          const Positioned.fill(
            child: CustomPaint(painter: _VaultDialPainter()),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 18, 16, 18),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      'VAULT BALANCE',
                      style: AppTypography.labelSmall.copyWith(
                        color: homeText.withValues(alpha: 0.7),
                        letterSpacing: 1.6,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(width: AppSpacing.xs),
                    const AppIcon(
                      AppIcons.shieldKeyhole,
                      size: 14,
                      color: _jade,
                    ),
                    const Spacer(),
                    AppTappable(
                      onTap: onToggle,
                      semanticsLabel: hidden ? 'Show balance' : 'Hide balance',
                      child: Container(
                        width: 32,
                        height: 32,
                        decoration: BoxDecoration(
                          color: const Color(0x14FFFFFF),
                          borderRadius: BorderRadius.circular(AppRadii.xSmall),
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
                Text.rich(
                  TextSpan(
                    children: [
                      TextSpan(
                        text: hidden ? fixedPrivacyMask() : amount,
                        style: const TextStyle(
                          fontFamily: 'Space Grotesk',
                          fontWeight: FontWeight.w500,
                          fontSize: 46,
                          height: 1.05,
                          letterSpacing: -1.8,
                          color: homeText,
                        ),
                      ),
                      const TextSpan(
                        text: ' $kZcashDefaultCurrencyTicker',
                        style: TextStyle(
                          fontFamily: 'Space Grotesk',
                          fontWeight: FontWeight.w500,
                          fontSize: 22,
                          color: _jade,
                        ),
                      ),
                    ],
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                const SizedBox(height: AppSpacing.xs),
                if (pending != null && !hidden)
                  Text(
                    '+$pending $kZcashDefaultCurrencyTicker confirming',
                    style: AppTypography.bodySmall.copyWith(
                      color: homeText.withValues(alpha: 0.7),
                    ),
                  ),
                const SizedBox(height: AppSpacing.sm),
                _ThresholdStrip(threshold: threshold, members: members),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// The approval rule as dots: [threshold] filled jade dots out of [members].
class _ThresholdStrip extends StatelessWidget {
  const _ThresholdStrip({required this.threshold, required this.members});
  final int threshold;
  final int members;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        for (var i = 0; i < members; i++)
          Container(
            width: 10,
            height: 10,
            margin: const EdgeInsets.only(right: 6),
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: i < threshold ? _BalanceCard._jade : null,
              border: Border.all(
                color: i < threshold
                    ? _BalanceCard._jade
                    : const Color(0x66F2F4F8),
                width: 1.5,
              ),
            ),
          ),
        const SizedBox(width: 4),
        Text(
          '$threshold of $members signers to send',
          style: AppTypography.labelSmall.copyWith(
            color: const Color(0xB3F2F4F8),
          ),
        ),
      ],
    );
  }
}

/// Concentric safe-dial rings with tick marks, off the card's top-right corner, over a
/// soft jade glow.
class _VaultDialPainter extends CustomPainter {
  const _VaultDialPainter();

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width - 36, 20);
    canvas.drawCircle(
      center,
      size.width * 0.75,
      Paint()
        ..shader =
            RadialGradient(
              colors: const [Color(0x402EC4A6), Color(0x002EC4A6)],
            ).createShader(
              Rect.fromCircle(center: center, radius: size.width * 0.75),
            ),
    );
    final ring = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1;
    for (var i = 0; i < 6; i++) {
      ring.color = Color.fromRGBO(46, 196, 166, 0.22 - i * 0.03);
      canvas.drawCircle(center, 44.0 + i * 26, ring);
    }
    final tick = Paint()
      ..color = const Color(0x552EC4A6)
      ..strokeWidth = 1.5
      ..strokeCap = StrokeCap.round;
    const outer = 44.0 + 2 * 26;
    for (var i = 0; i < 60; i++) {
      final angle = i * 6 * 3.141592653589793 / 180;
      final len = i % 5 == 0 ? 8.0 : 4.0;
      final dir = Offset.fromDirection(angle);
      canvas.drawLine(center + dir * (outer - len), center + dir * outer, tick);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
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

/// "Recent activity": header with "See all", up to 10 rows 12 apart, and
/// the empty state.
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
          SizedBox(
            height: 24,
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    'Recent activity',
                    style: AppTypography.labelLarge.copyWith(
                      color: colors.text.accent,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                if (items.isNotEmpty)
                  AppTappable(
                    onTap: () => context.push('/activity'),
                    semanticsLabel: 'See all activity',
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          'See all',
                          style: AppTypography.labelLarge.copyWith(
                            color: colors.button.ghost.label,
                          ),
                        ),
                        AppIcon(
                          AppIcons.chevronForward,
                          size: 16,
                          color: colors.button.ghost.label,
                        ),
                      ],
                    ),
                  ),
              ],
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          if (items.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: AppSpacing.md),
              child: Center(
                child: Column(
                  children: [
                    Text(
                      proposals.loaded
                          ? 'No activity, yet...'
                          : 'Loading activity...',
                      style: AppTypography.headlineSmall.copyWith(
                        color: colors.text.accent,
                      ),
                    ),
                    if (proposals.loaded) ...[
                      const SizedBox(height: AppSpacing.xxs),
                      Text(
                        'How about proposing\nyour first payment?',
                        textAlign: TextAlign.center,
                        style: AppTypography.bodyMedium.copyWith(
                          color: colors.text.secondary,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          for (var i = 0; i < items.length; i++) ...[
            if (i > 0) const SizedBox(height: AppSpacing.s),
            ProposalRow(
              proposal: items[i],
              hideAmount: hideAmounts,
              onTap: () => context.push('/proposal/${items[i].id}'),
            ),
          ],
        ],
      ),
    );
  }
}

/// Until this device's copy of the vault is backed up (spec §12.2 "backup health").
class _BackupReminder extends StatelessWidget {
  const _BackupReminder();

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    // Home entry card: ground, radius 24, 1.5px
    // white @ 7% border, icon + title/chevron + body.
    return AppTappable(
      onTap: () => context.push('/export'),
      semanticsLabel: 'Back up this vault',
      child: Container(
        constraints: const BoxConstraints(minHeight: 77),
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.sm,
          vertical: AppSpacing.s,
        ),
        decoration: BoxDecoration(
          color: colors.background.ground,
          borderRadius: BorderRadius.circular(AppRadii.large),
        ),
        foregroundDecoration: BoxDecoration(
          borderRadius: BorderRadius.circular(AppRadii.large),
          border: Border.all(color: const Color(0x12FFFFFF), width: 1.5),
        ),
        child: Padding(
          padding: const EdgeInsets.all(AppSpacing.xxs),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              AppIcon(AppIcons.warning, size: 20, color: colors.icon.warning),
              const SizedBox(width: AppSpacing.s),
              Expanded(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            'Back up this vault',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: AppTypography.labelLarge.copyWith(
                              color: colors.text.accent,
                            ),
                          ),
                        ),
                        AppIcon(
                          AppIcons.chevronForward,
                          size: 20,
                          color: colors.icon.accent,
                        ),
                      ],
                    ),
                    const SizedBox(height: AppSpacing.xs),
                    Text(
                      'Your key share lives only on this phone',
                      maxLines: 2,
                      style: AppTypography.bodyMedium.copyWith(
                        color: colors.text.secondary,
                        height: 17 / 16,
                        letterSpacing: -0.04,
                      ),
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
