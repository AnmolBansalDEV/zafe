import 'package:flutter/painting.dart';

import '../primitives.dart';

/// Icon color hierarchy retained from the OLDSemantic icon tokens.
///
/// * [accent] — Active, selected, primary icons.
/// * [regular] — Standard UI icons. (Named `regular` instead of `default`
///   because `default` is a reserved word in Dart.)
/// * [muted] — Inactive, decorative icons. Theme-invariant.
/// * [disabled] — Icons on disabled controls.
/// * [inverse] — Icons on inverted surfaces.
/// * [onPrimary] — Icons placed inside a primary button.
/// * [warning] — Caution icons (orange warning ladder).
/// * [destructive] — Destructive-state icons.
/// * [destructiveLight] — Softer destructive icon for secondary error affordances.
/// * [value] — Money received and success: Zcash gold, the value colour.
/// * [brand] — Brand-colored icons.
class AppIconColors {
  const AppIconColors({
    required this.accent,
    required this.regular,
    required this.muted,
    required this.disabled,
    required this.inverse,
    required this.onPrimary,
    required this.warning,
    required this.destructive,
    required this.destructiveLight,
    required this.value,
    required this.brand,
  });

  final Color accent;
  final Color regular;
  final Color muted;
  final Color disabled;
  final Color inverse;
  final Color onPrimary;
  final Color warning;
  final Color destructive;
  final Color destructiveLight;
  final Color value;
  final Color brand;

  static const dark = AppIconColors(
    accent: Primitives.p800Dark,
    regular: Primitives.p700Dark,
    muted: Primitives.p500Dark,
    disabled: Primitives.p300Dark,
    inverse: Primitives.p0Dark,
    onPrimary: Primitives.p0Dark,
    warning: OrangePrimitives.p500Dark,
    destructive: RosePrimitives.p400Dark,
    destructiveLight: RosePrimitives.p300Dark,
    value: GoldPrimitives.p500Dark,
    brand: BrandPrimitives.p400Dark,
  );

  static const light = AppIconColors(
    accent: Primitives.p900Light,
    regular: Primitives.p700Light,
    muted: Primitives.p500Light,
    disabled: Primitives.p300Light,
    inverse: Primitives.p0Light,
    onPrimary: Primitives.p0Light,
    warning: OrangePrimitives.p400Light,
    destructive: RosePrimitives.p300Light,
    destructiveLight: RosePrimitives.p200Light,
    value: GoldPrimitives.p400Light,
    brand: BrandPrimitives.p300Light,
  );
}
