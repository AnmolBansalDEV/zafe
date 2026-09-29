import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:pretty_qr_code/pretty_qr_code.dart';
import 'package:share_plus/share_plus.dart';

import '../../core/layout/mobile/zafe_screen.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_copy_feedback.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_tappable.dart';
import '../../core/widgets/app_toast.dart';
import '../../core/widgets/dot_qr_shape.dart';
import '../../core/widgets/mobile/mobile_surface_card.dart';
import '../../providers/vault_provider.dart';
import '../../rust/api/vault.dart' as rust;

/// Vault setup: invite, members joining, locking membership, comparing the safety
/// number, and key generation (spec §7.2).
class SetupScreen extends ConsumerStatefulWidget {
  const SetupScreen({super.key});

  @override
  ConsumerState<SetupScreen> createState() => _SetupScreenState();
}

class _SetupScreenState extends ConsumerState<SetupScreen> {
  Timer? _poll;
  bool _compared = false;
  bool _busy = false;
  bool _creatingKeys = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _refresh();
    _poll = Timer.periodic(const Duration(seconds: 2), (_) => _refresh());
  }

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }

  Future<void> _refresh() async {
    if (_creatingKeys) return;
    try {
      await ref.read(vaultProvider.notifier).refreshMembership();
      if (mounted && _error != null) setState(() => _error = null);
    } catch (e) {
      if (mounted) setState(() => _error = 'Can\'t reach the relay. Retrying...');
    }
  }

  Future<void> _seal() async {
    setState(() => _busy = true);
    try {
      await ref.read(vaultProvider.notifier).seal();
    } catch (e) {
      if (mounted) showAppToast(context, 'Couldn\'t lock membership. $e', iconName: AppIcons.warning);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _createKeys(String safetyNumber) async {
    setState(() => _creatingKeys = true);
    _poll?.cancel();
    try {
      await ref.read(vaultProvider.notifier).createKeys(safetyNumber);
      if (mounted) context.go('/home');
    } catch (e) {
      if (mounted) {
        setState(() => _creatingKeys = false);
        showAppToast(context, 'Key generation failed. $e', iconName: AppIcons.warning);
        _poll = Timer.periodic(const Duration(seconds: 2), (_) => _refresh());
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final vault = ref.watch(vaultProvider);
    final info = vault.inviteInfo;
    final membership = vault.membership;
    if (info == null) return const SizedBox.shrink();

    final joined = membership?.members.length ?? 1;
    final everyoneJoined = joined == info.members;
    final sealed = membership?.sealed ?? false;
    final isCreator = membership?.isCreator ?? false;

    if (_creatingKeys) return const _CreatingKeys();

    Widget? bottom;
    if (sealed) {
      bottom = AppButton(
        expand: true,
        onPressed: _compared ? () => _createKeys(membership!.safetyNumber) : null,
        leading: const AppIcon(AppIcons.key, size: 20),
        child: const Text('Create vault keys'),
      );
    } else if (isCreator) {
      bottom = AppButton(
        expand: true,
        onPressed: everyoneJoined && !_busy ? _seal : null,
        leading: const AppIcon(AppIcons.lock, size: 20),
        child: Text(everyoneJoined ? 'Lock membership' : 'Waiting for members...'),
      );
    }

    return ZafeScreen(
      title: info.name,
      showBack: false,
      bottom: bottom,
      children: [
        if (!sealed && isCreator) ...[
          _InviteCard(invite: vault.invite!),
          const SizedBox(height: AppSpacing.md),
        ],
        _MembersCard(
          members: membership?.members ?? const [],
          expected: info.members,
          threshold: info.threshold,
          myKeyHex: _myKeyHex(vault),
        ),
        const SizedBox(height: AppSpacing.md),
        if (_error != null)
          Text(_error!, style: AppTypography.bodySmall.copyWith(color: context.colors.text.warning)),
        if (!sealed && !isCreator)
          _Hint(
            text: everyoneJoined
                ? 'Everyone has joined. Waiting for the creator to lock membership.'
                : 'Waiting for the other members to join with the invite.',
          ),
        if (sealed && membership != null) ...[
          _SafetyNumberCard(number: membership.safetyNumber),
          const SizedBox(height: AppSpacing.sm),
          _ConfirmOption(
            selected: _compared,
            label: 'I compared this number with every member',
            onTap: () => setState(() => _compared = !_compared),
          ),
        ],
      ],
    );
  }

  String? _myKeyHex(VaultState vault) {
    final id = vault.identity;
    return id == null ? null : rust.identityPublicKey(seeds: id);
  }
}

class _InviteCard extends StatelessWidget {
  const _InviteCard({required this.invite});
  final String invite;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return MobileSurfaceCard(
      cornerRadius: AppRadii.large,
      padding: const EdgeInsets.fromLTRB(16, 24, 16, 16),
      child: Column(
        children: [
          Text('Invite members', style: AppTypography.bodyLarge.copyWith(color: colors.text.accent, fontWeight: FontWeight.w600)),
          const SizedBox(height: AppSpacing.xxs),
          Text(
            'Share this with the people who will co-sign. Anyone with it can join until you lock membership.',
            textAlign: TextAlign.center,
            style: AppTypography.bodySmall.copyWith(color: colors.text.secondary),
          ),
          const SizedBox(height: AppSpacing.sm),
          Container(
            width: 220,
            height: 220,
            padding: const EdgeInsets.all(AppSpacing.sm),
            decoration: BoxDecoration(
              color: colors.surface.qrCode,
              borderRadius: BorderRadius.circular(AppRadii.large),
            ),
            child: PrettyQrView.data(
              data: invite,
              decoration: const PrettyQrDecoration(shape: DotQrShape(color: Color(0xFF141818))),
            ),
          ),
          const SizedBox(height: AppSpacing.sm),
          Row(
            children: [
              Expanded(
                child: AppButton(
                  variant: AppButtonVariant.secondary,
                  onPressed: () => SharePlus.instance.share(ShareParams(text: invite)),
                  leading: const AppIcon(AppIcons.share, size: 20),
                  child: const Text('Share'),
                ),
              ),
              const SizedBox(width: AppSpacing.xs),
              Expanded(
                child: AppButton(
                  variant: AppButtonVariant.secondary,
                  onPressed: () => copyTextWithToast(context, text: invite, toastMessage: 'Invite copied'),
                  leading: const AppIcon(AppIcons.copy, size: 20),
                  child: const Text('Copy'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _MembersCard extends StatelessWidget {
  const _MembersCard({
    required this.members,
    required this.expected,
    required this.threshold,
    required this.myKeyHex,
  });

  final List<String> members;
  final int expected;
  final int threshold;
  final String? myKeyHex;

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
                  'Members',
                  style: AppTypography.labelLarge.copyWith(color: colors.text.accent, fontWeight: FontWeight.w600),
                ),
              ),
              Text(
                '${members.length} of $expected joined',
                style: AppTypography.labelMedium.copyWith(color: colors.text.secondary),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.xs),
          for (final m in members)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: AppSpacing.xs),
              child: Row(
                children: [
                  Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      color: colors.background.neutralSubtleOpacity,
                      shape: BoxShape.circle,
                    ),
                    alignment: Alignment.center,
                    child: AppIcon(AppIcons.user, size: 16, color: colors.icon.regular),
                  ),
                  const SizedBox(width: AppSpacing.s),
                  Text(
                    '${m.substring(0, 8)}...${m.substring(m.length - 6)}',
                    style: AppTypography.codeSmall.copyWith(color: colors.text.primary),
                  ),
                  if (m == myKeyHex) ...[
                    const SizedBox(width: AppSpacing.xs),
                    Text('You', style: AppTypography.labelMedium.copyWith(color: colors.text.secondary)),
                  ],
                ],
              ),
            ),
          for (var i = members.length; i < expected; i++)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: AppSpacing.xs),
              child: Row(
                children: [
                  Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(color: colors.border.regular),
                    ),
                  ),
                  const SizedBox(width: AppSpacing.s),
                  Text('Waiting...', style: AppTypography.bodySmall.copyWith(color: colors.text.muted)),
                ],
              ),
            ),
          const SizedBox(height: AppSpacing.xs),
          Text(
            '$threshold signatures needed for each payment.',
            style: AppTypography.bodySmall.copyWith(color: colors.text.secondary),
          ),
        ],
      ),
    );
  }
}

class _SafetyNumberCard extends StatelessWidget {
  const _SafetyNumberCard({required this.number});
  final String number;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return Container(
      padding: const EdgeInsets.fromLTRB(16, 24, 16, 24),
      decoration: BoxDecoration(
        color: colors.background.homeCard,
        borderRadius: BorderRadius.circular(AppRadii.large),
        border: Border.all(color: const Color(0x12FFFFFF), width: 1.5),
      ),
      child: Column(
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              AppIcon(AppIcons.shieldKeyhole, size: 20, color: colors.text.homeCard),
              const SizedBox(width: AppSpacing.xs),
              Text('Safety number', style: AppTypography.labelMedium.copyWith(color: colors.text.homeCard)),
            ],
          ),
          const SizedBox(height: AppSpacing.sm),
          Text(
            number,
            style: AppTypography.headlineLarge.copyWith(color: colors.text.homeCard),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: AppSpacing.sm),
          Text(
            'Read it to each member over a call, or compare in person. Every member must see the same number. '
            'If any member sees a different one, stop: someone may be impersonating a member.',
            textAlign: TextAlign.center,
            style: AppTypography.bodySmall.copyWith(color: colors.text.homeCard.withValues(alpha: 0.7)),
          ),
        ],
      ),
    );
  }
}

class _ConfirmOption extends StatelessWidget {
  const _ConfirmOption({required this.selected, required this.label, required this.onTap});
  final bool selected;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return AppTappable(
      onTap: onTap,
      child: Container(
        height: 64,
        padding: const EdgeInsets.symmetric(horizontal: AppSpacing.sm),
        decoration: BoxDecoration(
          color: colors.background.ground,
          borderRadius: BorderRadius.circular(AppRadii.medium),
          border: Border.all(
            color: selected ? colors.border.strong : colors.border.subtle,
            width: selected ? 1.5 : 1,
          ),
        ),
        child: Row(
          children: [
            Expanded(child: Text(label, style: AppTypography.bodyMediumStrong.copyWith(color: colors.text.accent))),
            Container(
              width: 24,
              height: 24,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: selected ? colors.background.inverse : colors.background.neutralSubtleOpacity,
              ),
              alignment: Alignment.center,
              child: selected ? AppIcon(AppIcons.check, size: 14, color: colors.icon.inverse) : null,
            ),
          ],
        ),
      ),
    );
  }
}

class _Hint extends StatelessWidget {
  const _Hint({required this.text});
  final String text;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return Row(
      children: [
        AppIcon(AppIcons.loader, size: 16, color: colors.icon.muted),
        const SizedBox(width: AppSpacing.xs),
        Expanded(child: Text(text, style: AppTypography.bodySmall.copyWith(color: colors.text.secondary))),
      ],
    );
  }
}

class _CreatingKeys extends StatelessWidget {
  const _CreatingKeys();

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return Scaffold(
      backgroundColor: colors.background.window,
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(AppSpacing.sm),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.end,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              AppIcon(AppIcons.loader, size: 24, color: colors.icon.accent),
              const SizedBox(height: AppSpacing.sm),
              Text(
                'Creating your vault keys...',
                style: TextStyle(fontFamily: 'Young Serif', fontSize: 24, height: 28 / 24, color: colors.text.accent),
              ),
              const SizedBox(height: AppSpacing.xs),
              Text(
                'Keep Zafe open. Your share is being created together with the other members; '
                'the full key never exists anywhere.',
                style: AppTypography.bodyMedium.copyWith(color: colors.text.secondary),
              ),
              const SizedBox(height: AppSpacing.lg),
            ],
          ),
        ),
      ),
    );
  }
}
