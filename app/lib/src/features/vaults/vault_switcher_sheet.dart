import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/formatting/zec_amount.dart';
import '../../core/layout/mobile/app_mobile_sheet.dart';
import '../../core/privacy/amount_display.dart';
import '../../core/storage/vault_summaries.dart';
import '../../core/storage/zafe_secure_store.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_tappable.dart';
import '../../providers/privacy_mode_provider.dart';
import '../../providers/vault_provider.dart';
import '../../rust/api/vault.dart' as rust;

/// Vizor's account sheet, for vaults: every vault on this device with its rule, balance and
/// payments waiting for you, plus "Add vault" and the current vault's settings.
Future<void> showVaultSwitcher(BuildContext context) =>
    showAppMobileSheet<void>(
      context: context,
      builder: (_) => const _VaultSwitcher(),
    );

class _VaultSwitcher extends ConsumerWidget {
  const _VaultSwitcher();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.colors;
    final state = ref.watch(vaultProvider);
    final hide = ref.watch(privacyModeProvider);
    return MobileModalScaffold(
      title: 'Vaults',
      onClose: () => Navigator.of(context).pop(),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (final v in state.vaults)
            _VaultRow(
              vault: v,
              active: v.id == state.activeId,
              hideAmounts: hide,
              liveBalance: state.balances[v.id]?.totalZat,
              onTap: () async {
                Navigator.of(context).pop();
                await ref.read(vaultProvider.notifier).switchTo(v.id);
                if (context.mounted) context.go(v.ready ? '/home' : '/setup');
              },
            ),
          const SizedBox(height: AppSpacing.xs),
          Container(height: 1, color: colors.border.regular),
          const SizedBox(height: AppSpacing.xs),
          _ActionRow(
            icon: AppIcons.addNew,
            label: 'Add vault',
            onTap: () {
              Navigator.of(context).pop();
              ref.read(vaultProvider.notifier).beginAddVault();
              context.go('/welcome');
            },
          ),
          _ActionRow(
            icon: AppIcons.cog,
            label: 'Vault settings',
            onTap: () {
              Navigator.of(context).pop();
              context.push('/settings');
            },
          ),
        ],
      ),
    );
  }
}

class _VaultRow extends StatelessWidget {
  const _VaultRow({
    required this.vault,
    required this.active,
    required this.hideAmounts,
    required this.liveBalance,
    required this.onTap,
  });

  final StoredVault vault;
  final bool active;
  final bool hideAmounts;
  final BigInt? liveBalance;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final summary = vault.ready
        ? rust.vaultSummary(material: vault.material!)
        : null;
    final invite = summary == null && vault.invite != null
        ? rust.parseInvite(invite: vault.invite!)
        : null;
    final name = summary?.name ?? invite?.name ?? 'Vault';
    final threshold = summary?.threshold ?? invite?.threshold ?? 0;
    final members = summary?.members.length ?? invite?.members ?? 0;
    return AppTappable(
      onTap: onTap,
      semanticsLabel: '$name${active ? ', current vault' : ''}',
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: AppSpacing.xs),
        child: Row(
          children: [
            Container(
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
            ),
            const SizedBox(width: AppSpacing.s),
            Expanded(
              child: FutureBuilder<VaultSummaryInfo>(
                future: VaultSummaries.read(vault.id),
                builder: (context, snap) {
                  final info = snap.data;
                  final balance = liveBalance ?? info?.balanceZat;
                  final detail = !vault.ready
                      ? 'Setting up'
                      : balance == null
                      ? '$threshold of $members to approve'
                      : amountWithTicker(
                          ZecAmount.fromZatoshi(balance).balance.amountText,
                          hide: hideAmounts,
                        );
                  return Row(
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              name,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: AppTypography.labelLarge.copyWith(
                                color: colors.text.accent,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                            const SizedBox(height: AppSpacing.xxs),
                            Text(
                              detail,
                              style: AppTypography.labelMedium.copyWith(
                                color: colors.text.secondary,
                              ),
                            ),
                          ],
                        ),
                      ),
                      if ((info?.actionable ?? 0) > 0)
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: AppSpacing.xs,
                            vertical: 2,
                          ),
                          decoration: BoxDecoration(
                            color: colors.background.brandCrimsonStrong,
                            borderRadius: BorderRadius.circular(AppRadii.full),
                          ),
                          child: Text(
                            '${info!.actionable}',
                            style: AppTypography.labelMedium.copyWith(
                              color: colors.text.inverse,
                            ),
                          ),
                        ),
                    ],
                  );
                },
              ),
            ),
            const SizedBox(width: AppSpacing.xs),
            SizedBox(
              width: 20,
              child: active
                  ? AppIcon(AppIcons.check, size: 20, color: colors.icon.accent)
                  : null,
            ),
          ],
        ),
      ),
    );
  }
}

class _ActionRow extends StatelessWidget {
  const _ActionRow({
    required this.icon,
    required this.label,
    required this.onTap,
  });
  final String icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return AppTappable(
      onTap: onTap,
      semanticsLabel: label,
      child: SizedBox(
        height: 44,
        child: Row(
          children: [
            SizedBox(
              width: 40,
              child: Center(
                child: AppIcon(icon, size: 20, color: colors.icon.muted),
              ),
            ),
            const SizedBox(width: AppSpacing.s),
            Text(
              label,
              style: AppTypography.labelLarge.copyWith(
                color: colors.text.accent,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
