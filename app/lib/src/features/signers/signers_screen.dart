import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/layout/mobile/mobile_top_nav.dart';
import '../../core/layout/mobile/mobile_top_scroll_fade.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/mobile/mobile_surface_card.dart';
import '../../core/widgets/mobile/zafe_detail.dart';
import '../../providers/member_names_provider.dart';
import '../../providers/vault_provider.dart';
import '../home/rename_sheet.dart';

/// The vault's signers and its approval rule (a tab). Signer management, such as
/// rotating a key, will live here.
class SignersScreen extends ConsumerWidget {
  const SignersScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.colors;
    final vault = ref.watch(vaultProvider);
    final summary = vault.summary;
    if (summary == null) return const SizedBox.shrink();
    final names = ref.watch(memberNamesProvider);
    final me = vault.myKeyHex;
    final threshold = summary.threshold;
    final members = summary.members;

    return SafeArea(
      bottom: false,
      child: Column(
        children: [
          const MobileTopNav.back(title: 'Signers'),
          Expanded(
            child: MobileTopScrollFade(
              child: ListView(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 112),
                children: [
                  MobileSurfaceCard(
                    cornerRadius: AppRadii.large,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          '$threshold of ${members.length} to send',
                          style: AppTypography.headlineSmall.copyWith(
                            color: colors.text.accent,
                          ),
                        ),
                        const SizedBox(height: AppSpacing.xxs),
                        Text(
                          threshold == 1
                              ? 'Any one signer can send a payment on their own.'
                              : 'Every payment needs approvals from $threshold '
                                    'of these signers before it can be sent.',
                          style: AppTypography.bodyMedium.copyWith(
                            color: colors.text.secondary,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: AppSpacing.md),
                  MobileSurfaceCard(
                    cornerRadius: AppRadii.large,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        for (final m in members)
                          SignerRow(
                            keyHex: m,
                            me: me,
                            name: names[m],
                            onTap: m == me
                                ? null
                                : () => showRenameSignerSheet(context, ref, m),
                          ),
                      ],
                    ),
                  ),
                  const SizedBox(height: AppSpacing.s),
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: AppSpacing.xxs,
                    ),
                    child: Text(
                      'Tap a signer to name them on this phone. Names stay on this '
                      'phone; other members don\'t see them.',
                      style: AppTypography.bodySmall.copyWith(
                        color: colors.text.muted,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
