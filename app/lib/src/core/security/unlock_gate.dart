import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/device_lock_provider.dart';
import '../widgets/app_icon.dart';
import '../widgets/app_toast.dart';
import 'device_auth.dart';

/// Asks the owner to unlock before a sensitive action. Returns true when the action
/// may go ahead. On cancel or failure it shows a neutral toast and returns false,
/// so callers just `if (!await confirmUnlock(...)) return;`.
///
/// [always] ignores the "Require unlock to approve" setting (used to turn it off).
Future<bool> confirmUnlock(
  BuildContext context,
  WidgetRef ref, {
  required String reason,
  bool always = false,
}) async {
  final required = always || ref.read(requireUnlockProvider);
  final outcome = await ref
      .read(deviceAuthProvider)
      .unlock(required: required, reason: reason);
  if (!context.mounted) return false;
  final message = switch (outcome) {
    UnlockOutcome.unlocked || UnlockOutcome.notRequired => null,
    UnlockOutcome.noScreenLock =>
      await takeNoScreenLockWarning()
          ? 'This phone has no screen lock. Set one to protect approvals'
          : null,
    UnlockOutcome.cancelled => 'Unlock cancelled. Nothing was done',
    UnlockOutcome.lockedOut => 'Too many attempts. Try again later',
    UnlockOutcome.failed => 'Couldn\'t confirm it\'s you. Nothing was done',
  };
  if (!context.mounted) return false;
  if (message != null) {
    showAppToast(
      context,
      message,
      iconName: outcome == UnlockOutcome.noScreenLock
          ? AppIcons.warningCircle
          : AppIcons.lock,
      duration: outcome == UnlockOutcome.noScreenLock
          ? const Duration(seconds: 4)
          : AppToast.defaultDuration,
    );
  }
  return outcome.allowed;
}
