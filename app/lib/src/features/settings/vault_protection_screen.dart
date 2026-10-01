import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/layout/mobile/zafe_screen.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_icon.dart';
import '../../providers/vault_provider.dart';

/// What the vault's keys protect against and what they don't (spec §2.3, §12.1): the
/// approval rule, full view access, losing too many keys, and quantum computers.
class VaultProtectionScreen extends ConsumerWidget {
  const VaultProtectionScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final summary = ref.watch(vaultProvider).summary;
    if (summary == null) return const SizedBox.shrink();
    return ZafeScreen(
      title: 'How it\'s protected',
      children: [
        VaultProtectionBody(
          threshold: summary.threshold,
          signers: summary.members.length,
        ),
      ],
    );
  }
}

/// The page's content from plain data, so it can be rendered without Rust.
class VaultProtectionBody extends StatelessWidget {
  const VaultProtectionBody({
    super.key,
    required this.threshold,
    required this.signers,
  });

  final int threshold;
  final int signers;

  @override
  Widget build(BuildContext context) {
    final spare = signers - threshold;
    final loss = spare == 0
        ? 'If any signer loses both their phone and their backup, the funds '
              'are lost for good. Nobody, including Zafe, can recover them.'
        : 'If more than $spare of the $signers signers lose both their phone '
              'and their backup, the funds are lost for good. Nobody, including '
              'Zafe, can recover them.';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Point(
          icon: AppIcons.users,
          title: 'A payment needs $threshold of $signers signers',
          body:
              'Each signer approves on their own phone. Your key share never '
              'leaves this phone, and the full spending key never exists '
              'anywhere, so no single signer, and not Zafe, can move funds.',
        ),
        const _Point(
          icon: AppIcons.eye,
          title: 'Every signer sees everything',
          body:
              'All signers can see every past and future payment, memo and the '
              'balance. This can\'t be taken back from a signer later.',
        ),
        _Point(icon: AppIcons.lock, title: 'Keep a backup', body: loss),
        const _Point(
          icon: AppIcons.warningCircle,
          title: 'Quantum computers',
          body:
              'Vault keys follow Zcash\'s plan for recovering funds if quantum '
              'computers ever break today\'s cryptography. Against such a '
              'computer, though, one signer\'s copy of the vault secret would be '
              'enough to spend. If that day comes, move the funds to a '
              'quantum-safe vault as soon as Zcash offers one.',
          last: true,
        ),
      ],
    );
  }
}

class _Point extends StatelessWidget {
  const _Point({
    required this.icon,
    required this.title,
    required this.body,
    this.last = false,
  });

  final String icon;
  final String title;
  final String body;
  final bool last;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return Padding(
      padding: EdgeInsets.only(bottom: last ? 0 : AppSpacing.md),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          AppIcon(icon, size: 22, color: colors.icon.accent),
          const SizedBox(width: AppSpacing.sm),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: AppTypography.labelLarge.copyWith(
                    color: colors.text.accent,
                  ),
                ),
                const SizedBox(height: AppSpacing.xxs),
                Text(
                  body,
                  style: AppTypography.bodySmall.copyWith(
                    color: colors.text.secondary,
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
