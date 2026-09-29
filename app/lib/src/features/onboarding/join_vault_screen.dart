import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/layout/mobile/zafe_screen.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_icon.dart';
import '../../core/widgets/app_toast.dart';
import '../../core/widgets/mobile_text_field.dart';
import '../../providers/vault_provider.dart';
import '../../rust/api/vault.dart' as rust;

class JoinVaultScreen extends ConsumerStatefulWidget {
  const JoinVaultScreen({super.key});

  @override
  ConsumerState<JoinVaultScreen> createState() => _JoinVaultScreenState();
}

class _JoinVaultScreenState extends ConsumerState<JoinVaultScreen> {
  final _invite = TextEditingController();
  final _focus = FocusNode();
  rust.InviteInfo? _info;
  bool _busy = false;

  @override
  void dispose() {
    _invite.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _parse() {
    try {
      _info = rust.parseInvite(invite: _invite.text.trim());
    } catch (_) {
      _info = null;
    }
    setState(() {});
  }

  Future<void> _paste() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    if (data?.text != null) {
      _invite.text = data!.text!.trim();
      _parse();
    }
  }

  Future<void> _join() async {
    setState(() => _busy = true);
    try {
      await ref.read(vaultProvider.notifier).joinVault(_invite.text);
      if (mounted) context.go('/setup');
    } catch (e) {
      if (mounted) showAppToast(context, 'Couldn\'t join. $e', iconName: AppIcons.warning);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final info = _info;
    return ZafeScreen(
      title: 'Join a vault',
      bottom: AppButton(
        expand: true,
        onPressed: info != null && !_busy ? _join : null,
        leading: _busy ? const AppIcon(AppIcons.loader, size: 20) : null,
        child: Text(_busy ? 'Joining...' : 'Join vault'),
      ),
      children: [
        Text('Invite', style: AppTypography.labelMedium.copyWith(color: colors.text.secondary)),
        const SizedBox(height: AppSpacing.xs),
        MobileTextField(
          controller: _invite,
          focusNode: _focus,
          hintText: 'Paste the invite you received',
          onChanged: (_) => _parse(),
          trailing: AppButton(
            variant: AppButtonVariant.secondary,
            size: AppButtonSize.mediumLarge,
            onPressed: _paste,
            child: const Text('Paste'),
          ),
        ),
        const SizedBox(height: AppSpacing.sm),
        if (info != null)
          Text(
            '"${info.name}": ${info.threshold} of ${info.members} signatures needed.',
            style: AppTypography.bodyMedium.copyWith(color: colors.text.accent),
          )
        else if (_invite.text.trim().isNotEmpty)
          Text(
            'This doesn\'t look like a Zafe invite.',
            style: AppTypography.bodySmall.copyWith(color: colors.text.destructive),
          ),
      ],
    );
  }
}
