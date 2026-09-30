import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:go_router/go_router.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:permission_handler/permission_handler.dart'
    show openAppSettings;

import '../../core/layout/mobile/mobile_top_nav.dart';
import '../../core/theme/app_theme.dart';
import '../../core/widgets/app_button.dart';
import '../../core/widgets/app_icon.dart';
import '../../rust/api/vault.dart' as rust;
import 'invite_link.dart';

/// Camera scanner for a vault invite QR (raw invite or invite link). Pops with the raw
/// invite once a valid one is seen; the Join screen still asks before joining.
class ScanInviteScreen extends StatefulWidget {
  const ScanInviteScreen({super.key});

  @override
  State<ScanInviteScreen> createState() => _ScanInviteScreenState();
}

class _ScanInviteScreenState extends State<ScanInviteScreen>
    with WidgetsBindingObserver {
  final _controller = MobileScannerController(
    formats: const [BarcodeFormat.qrCode],
  );
  bool _done = false;
  bool _rejected = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    unawaited(_controller.dispose());
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    switch (state) {
      case AppLifecycleState.resumed:
        // Also picks up a permission granted in system settings meanwhile.
        unawaited(_start());
      case AppLifecycleState.inactive:
        // The permission prompt itself makes the app inactive: leave that alone.
        if (_controller.value.isRunning) unawaited(_controller.stop());
      case AppLifecycleState.detached:
      case AppLifecycleState.hidden:
      case AppLifecycleState.paused:
        break;
    }
  }

  Future<void> _start() async {
    if (_done) return;
    try {
      await _controller.start();
    } catch (_) {
      // Already starting, or failed: the error shows through the controller's state.
    }
  }

  void _onDetect(BarcodeCapture capture) {
    if (_done) return;
    for (final code in capture.barcodes) {
      final raw = code.rawValue;
      if (raw == null) continue;
      final invite = extractInvite(raw);
      if (invite != null && _valid(invite)) {
        _done = true;
        unawaited(HapticFeedback.selectionClick());
        context.pop(invite);
        return;
      }
    }
    final sawCode = capture.barcodes.any((c) => c.rawValue != null);
    if (sawCode && !_rejected) setState(() => _rejected = true);
  }

  static bool _valid(String invite) {
    try {
      rust.parseInvite(invite: invite);
      return true;
    } catch (_) {
      return false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return Scaffold(
      backgroundColor: colors.background.window,
      body: SafeArea(
        child: Column(
          children: [
            MobileTopNav.back(
              title: 'Scan invite',
              onBack: () => context.pop(),
              trailing: _TorchButton(controller: _controller),
            ),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
                child: Column(
                  children: [
                    Expanded(
                      child: ClipRRect(
                        borderRadius: BorderRadius.circular(AppRadii.xLarge),
                        child: ColoredBox(
                          color: const Color(0xFF0B0E14),
                          child: MobileScanner(
                            controller: _controller,
                            onDetect: _onDetect,
                            errorBuilder: (context, error) =>
                                _ScannerError(error: error, onRetry: _start),
                            overlayBuilder: (_, _) => const _Frame(),
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: AppSpacing.md),
                    Text(
                      _rejected
                          ? 'That QR code isn\'t a Zafe invite. Scan the one on the '
                                'vault creator\'s setup screen.'
                          : 'Point the camera at the invite QR code on the vault '
                                'creator\'s phone.',
                      textAlign: TextAlign.center,
                      style: AppTypography.bodyMedium.copyWith(
                        color: _rejected
                            ? colors.text.destructive
                            : colors.text.secondary,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Rounded-rect guide in the middle of the camera view.
class _Frame extends StatelessWidget {
  const _Frame();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: FractionallySizedBox(
        widthFactor: 0.7,
        child: AspectRatio(
          aspectRatio: 1,
          child: DecoratedBox(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(AppRadii.large),
              border: Border.all(
                color: Colors.white.withValues(alpha: 0.85),
                width: 3,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _TorchButton extends StatelessWidget {
  const _TorchButton({required this.controller});
  final MobileScannerController controller;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<MobileScannerState>(
      valueListenable: controller,
      builder: (context, state, _) {
        final torch = state.torchState;
        if (torch == TorchState.unavailable || !state.isRunning) {
          return const SizedBox.shrink();
        }
        final on = torch == TorchState.on;
        return IconButton(
          tooltip: on ? 'Turn off flashlight' : 'Turn on flashlight',
          onPressed: () => unawaited(controller.toggleTorch()),
          icon: Icon(
            on ? Icons.flashlight_off_rounded : Icons.flashlight_on_rounded,
            color: context.colors.text.primary,
          ),
        );
      },
    );
  }
}

class _ScannerError extends StatelessWidget {
  const _ScannerError({required this.error, required this.onRetry});
  final MobileScannerException error;
  final Future<void> Function() onRetry;

  @override
  Widget build(BuildContext context) {
    final denied = error.errorCode == MobileScannerErrorCode.permissionDenied;
    const light = Color(0xFFE8ECF2);
    return Padding(
      padding: const EdgeInsets.all(AppSpacing.md),
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            AppIcon(
              denied ? AppIcons.cameraDenied : AppIcons.warningCircle,
              size: 40,
              color: light,
            ),
            const SizedBox(height: AppSpacing.sm),
            Text(
              denied ? 'Camera access is off' : 'The camera didn\'t start',
              textAlign: TextAlign.center,
              style: AppTypography.bodyLarge.copyWith(
                color: light,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: AppSpacing.xxs),
            Text(
              denied
                  ? 'Allow camera access for Zafe in your phone\'s settings to scan '
                        'invites, or go back and paste the invite instead.'
                  : 'Try again, or go back and paste the invite instead.',
              textAlign: TextAlign.center,
              style: AppTypography.bodySmall.copyWith(
                color: light.withValues(alpha: 0.75),
              ),
            ),
            const SizedBox(height: AppSpacing.md),
            if (denied)
              AppButton(
                onPressed: () => unawaited(openAppSettings()),
                leading: const AppIcon(AppIcons.cog, size: 20),
                child: const Text('Open settings'),
              ),
            const SizedBox(height: AppSpacing.xs),
            AppButton(
              variant: AppButtonVariant.secondary,
              onPressed: () => unawaited(onRetry()),
              child: const Text('Try again'),
            ),
          ],
        ),
      ),
    );
  }
}
