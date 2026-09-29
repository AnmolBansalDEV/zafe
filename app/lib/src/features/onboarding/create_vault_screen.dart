import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/layout/mobile/zafe_screen.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_toast.dart';
import '../../core/widgets/mobile/mobile_surface_card.dart';
import '../../core/widgets/mobile_text_field.dart';
import '../../providers/vault_provider.dart';

class CreateVaultScreen extends ConsumerStatefulWidget {
  const CreateVaultScreen({super.key});

  @override
  ConsumerState<CreateVaultScreen> createState() => _CreateVaultScreenState();
}

class _CreateVaultScreenState extends ConsumerState<CreateVaultScreen> {
  final _name = TextEditingController();
  final _nameFocus = FocusNode();
  int _members = 3;
  int _threshold = 2;
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    _nameFocus.dispose();
    super.dispose();
  }

  Future<void> _create() async {
    setState(() => _busy = true);
    try {
      await ref
          .read(vaultProvider.notifier)
          .createVault(name: _name.text.trim(), threshold: _threshold, members: _members);
      if (mounted) context.go('/setup');
    } catch (e) {
      if (mounted) showAppToast(context, 'Couldn\'t create the vault. $e', iconName: AppIcons.warning);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final valid = _name.text.trim().isNotEmpty && _threshold >= 2 && _threshold <= _members;
    return ZafeScreen(
      title: 'New vault',
      bottom: AppButton(
        expand: true,
        onPressed: valid && !_busy ? _create : null,
        leading: _busy ? const AppIcon(AppIcons.loader, size: 20) : null,
        child: Text(_busy ? 'Creating...' : 'Create vault'),
      ),
      children: [
        Text('Vault name', style: AppTypography.labelMedium.copyWith(color: colors.text.secondary)),
        const SizedBox(height: AppSpacing.xs),
        MobileTextField(
          controller: _name,
          focusNode: _nameFocus,
          hintText: 'e.g. Grants committee',
          onChanged: (_) => setState(() {}),
        ),
        const SizedBox(height: AppSpacing.md),
        MobileSurfaceCard(
          cornerRadius: AppRadii.large,
          child: Column(
            children: [
              _Stepper(
                label: 'Members',
                value: _members,
                min: 2,
                max: 15,
                onChanged: (v) => setState(() {
                  _members = v;
                  if (_threshold > v) _threshold = v;
                }),
              ),
              const SizedBox(height: AppSpacing.sm),
              _Stepper(
                label: 'Signatures needed',
                value: _threshold,
                min: 2,
                max: _members,
                onChanged: (v) => setState(() => _threshold = v),
              ),
            ],
          ),
        ),
        const SizedBox(height: AppSpacing.sm),
        Text(
          'Payments need $_threshold of $_members members to sign. If more than '
          '${_members - _threshold} member${_members - _threshold == 1 ? '' : 's'} lose their keys, '
          'the funds can\'t be recovered.',
          style: AppTypography.bodySmall.copyWith(color: colors.text.secondary),
        ),
        if (_threshold == _members) ...[
          const SizedBox(height: AppSpacing.xs),
          Text(
            'Every member must sign every payment. One lost phone locks the vault.',
            style: AppTypography.bodySmall.copyWith(color: colors.text.warning),
          ),
        ],
      ],
    );
  }
}

class _Stepper extends StatelessWidget {
  const _Stepper({
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.onChanged,
  });

  final String label;
  final int value;
  final int min;
  final int max;
  final ValueChanged<int> onChanged;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    Widget button(String icon, bool enabled, int next) => AppButton(
      variant: AppButtonVariant.secondary,
      size: AppButtonSize.mediumLarge,
      onPressed: enabled ? () => onChanged(next) : null,
      child: AppIcon(icon, size: 16),
    );
    return Row(
      children: [
        Expanded(child: Text(label, style: AppTypography.bodyMedium.copyWith(color: colors.text.accent))),
        button(AppIcons.cancel, value > min, value - 1),
        SizedBox(
          width: 44,
          child: Text(
            '$value',
            textAlign: TextAlign.center,
            style: AppTypography.headlineSmall.copyWith(color: colors.text.accent),
          ),
        ),
        button(AppIcons.plus, value < max, value + 1),
      ],
    );
  }
}
