import '../config/network_config.dart';
import 'privacy_mask.dart';

/// "1.5 TAZ", or "****** TAZ" in privacy mode. `hideAmountIfPrivacyMode` only adds the
/// unit to the mask, so the unit must be part of the visible text: this keeps both.
String amountWithTicker(
  String amountText, {
  required bool hide,
  int maskLength = kDefaultPrivacyMaskLength,
}) => hideAmountIfPrivacyMode(
  '$amountText $kZcashDefaultCurrencyTicker',
  privacyModeEnabled: hide,
  maskLength: maskLength,
);
