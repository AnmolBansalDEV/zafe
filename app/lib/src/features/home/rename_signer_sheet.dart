import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/formatting/member_label.dart';
import '../../core/layout/mobile/app_mobile_sheet.dart';
import '../../core/storage/member_names.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/mobile_text_field.dart';
import '../../providers/member_names_provider.dart';

/// Names a signer on this device (other members never see it).
Future<void> showRenameSignerSheet(
  BuildContext context,
  WidgetRef ref,
  String keyHex,
) async {
  final current = ref.read(memberNamesProvider)[keyHex] ?? '';
  final name = await showAppMobileSheet<String>(
    context: context,
    builder: (sheet) => _RenameSheet(keyHex: keyHex, current: current),
  );
  if (name != null) {
    await ref.read(memberNamesProvider.notifier).rename(keyHex, name);
  }
}

class _RenameSheet extends StatefulWidget {
  const _RenameSheet({required this.keyHex, required this.current});
  final String keyHex;
  final String current;

  @override
  State<_RenameSheet> createState() => _RenameSheetState();
}

class _RenameSheetState extends State<_RenameSheet> {
  late final _name = TextEditingController(text: widget.current);
  final _focus = FocusNode();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => _focus.requestFocus());
  }

  @override
  void dispose() {
    _name.dispose();
    _focus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return MobileModalScaffold(
      title: 'Name this signer',
      onClose: () => Navigator.of(context).pop(),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Key ${shortKey(widget.keyHex)}. The name stays on this phone; other '
            'members don\'t see it.',
            style: AppTypography.bodyMedium.copyWith(
              color: colors.text.secondary,
            ),
          ),
          const SizedBox(height: AppSpacing.sm),
          MobileTextField(
            controller: _name,
            focusNode: _focus,
            hintText: 'e.g. Alice (treasurer)',
            textInputAction: TextInputAction.done,
            inputFormatters: [
              LengthLimitingTextInputFormatter(MemberNames.maxLength),
            ],
            onSubmitted: (v) => Navigator.of(context).pop(v),
          ),
          const SizedBox(height: AppSpacing.md),
          AppButton(
            expand: true,
            onPressed: () => Navigator.of(context).pop(_name.text),
            child: const Text('Save'),
          ),
          if (widget.current.isNotEmpty) ...[
            const SizedBox(height: AppSpacing.xs),
            AppButton(
              expand: true,
              variant: AppButtonVariant.ghost,
              onPressed: () => Navigator.of(context).pop(''),
              child: const Text('Remove name'),
            ),
          ],
        ],
      ),
    );
  }
}
