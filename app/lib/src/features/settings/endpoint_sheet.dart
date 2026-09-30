import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/config/endpoints.dart';
import '../../core/config/network_config.dart';
import '../../core/errors/sync_failure.dart';
import '../../core/errors/zafe_error_copy.dart';
import '../../core/layout/mobile/app_mobile_sheet.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/mobile_text_field.dart';
import '../../notifications/vault_watch.dart' show reregisterPush;
import '../../providers/endpoints_provider.dart';
import '../../providers/proposals_provider.dart';
import '../../providers/vault_provider.dart';
import '../../rust/api/endpoints.dart' as rust;

/// Edits the relay or lightwalletd URL: checks the format, tries a connection, then saves
/// (for every vault on this phone). "Reset to default" goes back to the build's URL.
Future<void> showEndpointSheet(
  BuildContext context,
  WidgetRef ref,
  EndpointKind kind,
) async {
  final changed = await showAppMobileSheet<bool>(
    context: context,
    builder: (_) => _EndpointSheet(kind: kind),
  );
  if (changed == true) {
    if (kind == EndpointKind.relay) unawaited(reregisterPush());
    // Pick up the new server right away.
    await ref.read(proposalsProvider.notifier).refresh();
    await ref.read(vaultProvider.notifier).sync();
  }
}

/// Tries `url` for `kind`: a relay `GET /health`, or lightwalletd's info (network and
/// tip). Returns null when it answers, else what went wrong (user copy).
Future<String?> tryEndpoint(EndpointKind kind, String url) async {
  try {
    switch (kind) {
      case EndpointKind.relay:
        await rust.checkRelay(relayUrl: url);
      case EndpointKind.lightwalletd:
        await rust.checkLightwalletd(
          lightwalletdUrl: url,
          networkName: kZafeNetwork,
        );
    }
    return null;
  } catch (e) {
    final failure = classifySyncFailure(
      e,
      fallback: kind == EndpointKind.relay
          ? SyncEndpoint.relay
          : SyncEndpoint.lightwalletd,
    );
    return switch (failure.kind) {
      SyncFailureKind.relayUnreachable ||
      SyncFailureKind.lightwalletdUnreachable ||
      SyncFailureKind.offline => 'Couldn\'t connect to this server.',
      SyncFailureKind.tls =>
        'The secure connection failed: the certificate isn\'t trusted, has '
            'expired or names another server.',
      SyncFailureKind.timeout => 'The server didn\'t answer in time.',
      SyncFailureKind.wrongNetwork =>
        'This server is on another network than this app ($kZafeNetwork).',
      SyncFailureKind.updateRequired ||
      SyncFailureKind.relayOutdated => zafeErrorMessage(e),
      _ =>
        kind == EndpointKind.relay
            ? 'This doesn\'t look like a Zafe relay.'
            : 'This doesn\'t look like a lightwalletd server.',
    };
  }
}

class _EndpointSheet extends ConsumerStatefulWidget {
  const _EndpointSheet({required this.kind});
  final EndpointKind kind;

  @override
  ConsumerState<_EndpointSheet> createState() => _EndpointSheetState();
}

class _EndpointSheetState extends ConsumerState<_EndpointSheet> {
  late final _url = TextEditingController(
    text: ref.read(endpointsProvider).url(widget.kind),
  );
  final _focus = FocusNode();
  bool _busy = false;
  String? _error;

  bool get _relay => widget.kind == EndpointKind.relay;

  @override
  void dispose() {
    _url.dispose();
    _focus.dispose();
    super.dispose();
  }

  Future<void> _save(String input) async {
    final check = checkEndpointUrl(input);
    if (check.url == null) {
      setState(() => _error = check.error);
      return;
    }
    final url = check.url!;
    setState(() {
      _busy = true;
      _error = null;
    });
    final problem = await tryEndpoint(widget.kind, url);
    if (!mounted) return;
    if (problem != null) {
      setState(() {
        _busy = false;
        _error = problem;
      });
      return;
    }
    await ref.read(endpointsProvider.notifier).set(widget.kind, url);
    if (mounted) Navigator.of(context).pop(true);
  }

  Future<void> _reset() async {
    await ref.read(endpointsProvider.notifier).reset(widget.kind);
    if (mounted) Navigator.of(context).pop(true);
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final isDefault = ref.watch(endpointsProvider).isDefault(widget.kind);
    final defaultUrl = ZafeEndpoints.defaults.url(widget.kind);
    return MobileModalScaffold(
      title: _relay ? 'Relay' : 'Zcash server',
      onClose: () => Navigator.of(context).pop(),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            _relay
                ? 'Carries proposals, votes and signatures between members, '
                      'encrypted. Every member of a vault must use the same relay.'
                : 'The lightwalletd server this phone reads the $kZafeNetwork '
                      'chain from. It sees which blocks this phone downloads, not '
                      'the vault\'s keys.',
            style: AppTypography.bodyMedium.copyWith(
              color: colors.text.secondary,
            ),
          ),
          const SizedBox(height: AppSpacing.sm),
          MobileTextField(
            controller: _url,
            focusNode: _focus,
            enabled: !_busy,
            hintText: 'https://',
            keyboardType: TextInputType.url,
            textInputAction: TextInputAction.done,
            inputFormatters: [LengthLimitingTextInputFormatter(200)],
            onSubmitted: _busy ? null : _save,
          ),
          if (_error != null) ...[
            const SizedBox(height: AppSpacing.xs),
            Text(
              _error!,
              style: AppTypography.bodySmall.copyWith(
                color: colors.text.destructive,
              ),
            ),
          ],
          const SizedBox(height: AppSpacing.xs),
          Text(
            'Default: $defaultUrl',
            style: AppTypography.bodySmall.copyWith(
              color: colors.text.secondary,
            ),
          ),
          const SizedBox(height: AppSpacing.md),
          AppButton(
            expand: true,
            onPressed: _busy ? null : () => _save(_url.text),
            child: Text(_busy ? 'Connecting...' : 'Test and save'),
          ),
          if (!isDefault) ...[
            const SizedBox(height: AppSpacing.xs),
            AppButton(
              expand: true,
              variant: AppButtonVariant.ghost,
              onPressed: _busy ? null : _reset,
              child: const Text('Reset to default'),
            ),
          ],
        ],
      ),
    );
  }
}
