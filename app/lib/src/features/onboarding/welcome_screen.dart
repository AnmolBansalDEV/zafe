import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_tappable.dart';
import '../../providers/vault_provider.dart';
import 'onboarding_art.dart';

class WelcomeScreen extends ConsumerWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.colors;
    // Adding another vault: offer the way back to the one that was on screen.
    final adding = ref.watch(vaultProvider.select((v) => v.isAdding));
    Future<void> cancel() async {
      await ref.read(vaultProvider.notifier).cancelAddVault();
      if (context.mounted) context.go('/home');
    }

    final scaffold = Scaffold(
      backgroundColor: colors.background.window,
      body: Stack(
        fit: StackFit.expand,
        children: [
          const OnboardingHero('welcome_vault'),
          SafeArea(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Spacer(flex: 3),
                  Text(
                    'Zafe',
                    style: TextStyle(
                      fontFamily: 'Space Grotesk',
                      fontSize: 48,
                      height: 1.1,
                      letterSpacing: -1.35,
                      color: colors.text.accent,
                    ),
                  ),
                  const SizedBox(height: AppSpacing.s),
                  Text(
                    'A shielded multisig for Zcash. Every payment needs your co-signers to agree.',
                    style: AppTypography.bodyMedium.copyWith(
                      color: colors.text.secondary,
                    ),
                  ),
                  const Spacer(),
                  const _InfoCard(),
                  const SizedBox(height: AppSpacing.md),
                  AppButton(
                    expand: true,
                    onPressed: () => context.push('/create'),
                    leading: const AppIcon(AppIcons.addNew, size: 20),
                    child: const Text('Create a vault'),
                  ),
                  const SizedBox(height: AppSpacing.s),
                  AppButton(
                    expand: true,
                    variant: AppButtonVariant.secondary,
                    onPressed: () => context.push('/join'),
                    leading: const AppIcon(AppIcons.link, size: 20),
                    child: const Text('Join with an invite'),
                  ),
                  const SizedBox(height: AppSpacing.xs),
                  AppButton(
                    expand: true,
                    variant: AppButtonVariant.ghost,
                    onPressed: () => context.push('/restore'),
                    child: const Text('Restore from backup'),
                  ),
                ],
              ),
            ),
          ),
          if (adding)
            SafeArea(
              child: Align(
                alignment: Alignment.topLeft,
                child: Padding(
                  padding: const EdgeInsets.all(AppSpacing.sm),
                  child: AppTappable(
                    onTap: cancel,
                    semanticsLabel: 'Cancel adding a vault',
                    child: Container(
                      width: 40,
                      height: 40,
                      decoration: BoxDecoration(
                        color: colors.background.ground,
                        shape: BoxShape.circle,
                      ),
                      alignment: Alignment.center,
                      child: AppIcon(
                        AppIcons.cross,
                        size: 20,
                        color: colors.icon.accent,
                      ),
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
    if (!adding) return scaffold;
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) cancel();
      },
      child: scaffold,
    );
  }
}

class _InfoCard extends StatelessWidget {
  const _InfoCard();

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    Widget row(String icon, String title, String body) => Padding(
      padding: const EdgeInsets.symmetric(vertical: AppSpacing.xs),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          AppIcon(icon, size: 20, color: colors.text.darkCard),
          const SizedBox(width: AppSpacing.s),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: AppTypography.labelLarge.copyWith(
                    color: colors.text.darkCard,
                  ),
                ),
                const SizedBox(height: AppSpacing.xxs),
                Text(
                  body,
                  style: AppTypography.bodySmall.copyWith(
                    color: colors.text.darkCard.withValues(alpha: 0.7),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
    return Container(
      padding: const EdgeInsets.all(AppSpacing.sm),
      decoration: BoxDecoration(
        color: colors.background.darkCard,
        borderRadius: BorderRadius.circular(AppRadii.large),
        border: Border.all(color: const Color(0x12FFFFFF), width: 1.5),
      ),
      child: Column(
        children: [
          row(
            AppIcons.shieldKeyhole,
            'Private by default',
            'Balances, payments and memos stay shielded.',
          ),
          row(
            AppIcons.users,
            'Shared control',
            'Payments go out only when enough members sign.',
          ),
          row(
            AppIcons.key,
            'No single key',
            'Each member holds one share. The full key never exists.',
          ),
        ],
      ),
    );
  }
}
