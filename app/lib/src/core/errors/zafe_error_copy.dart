import '../../rust/api/error.dart';
import '../config/network_config.dart';

/// Friendly copy for errors from Rust. Switches on the typed `ZafeErrorKind` (no
/// substring matching); `fallback` covers `other`.
String zafeErrorMessage(
  Object error, {
  String fallback = 'Something went wrong. Try again.',
}) {
  if (error is! ZafeError) return fallback;
  return switch (error.kind) {
    ZafeErrorKind.network =>
      'Network error. Check your connection and try again.',
    ZafeErrorKind.notReady => _sentence(error.message),
    ZafeErrorKind.timeout =>
      'Other signers haven\'t answered yet. Ask them to open Zafe, then try again.',
    ZafeErrorKind.verification =>
      'This payment failed the check on your device. Don\'t approve it. (${error.message})',
    ZafeErrorKind.insufficientFunds =>
      'Not enough $kZcashDefaultCurrencyTicker in the vault to cover the amount and the fee.',
    ZafeErrorKind.invalidInput => _sentence(error.message),
    ZafeErrorKind.other => fallback,
  };
}

String _sentence(String message) {
  final m = message.replaceFirst(RegExp(r'^membership is not ready: '), '');
  if (m.isEmpty) return m;
  final s = m[0].toUpperCase() + m.substring(1);
  return s.endsWith('.') ? s : '$s.';
}

/// For logs: the generated `ZafeError` has no useful `toString`.
String describeError(Object error) =>
    error is ZafeError ? '${error.kind.name}: ${error.message}' : '$error';
