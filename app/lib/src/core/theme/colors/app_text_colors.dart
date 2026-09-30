import 'package:flutter/painting.dart';

import '../primitives.dart';

/// Text color hierarchy.
///
/// * [accent] — Titles, headings; max contrast.
/// * [primary] — Default body text, paragraphs.
/// * [secondary] — Subtitles, timestamps, metadata.
/// * [muted] — Descriptions. Theme-invariant.
/// * [disabled] — Inactive, unavailable labels.
/// * [inverse] — Text placed on inverted surfaces (e.g. dark text on a light
///   chip inside dark mode).
/// * [warning] — Inline caution copy (orange warning ladder).
/// * [value] — Positive-state copy and money received: Zcash gold
///   (the value colour; teal and green are too close to tell apart).
/// * [destructive] — Destructive utility copy.
/// * [destructiveLight] — Softer destructive copy for secondary error text.
/// * [brand] — Brand-colored inline text accent.
/// * [darkCard] — Text and QR dots on `background.darkCard`.
class AppTextColors {
  const AppTextColors({
    required this.accent,
    required this.primary,
    required this.secondary,
    required this.muted,
    required this.disabled,
    required this.inverse,
    required this.warning,
    required this.value,
    required this.destructive,
    required this.destructiveLight,
    required this.brand,
    required this.darkCard,
  });

  final Color accent;
  final Color primary;
  final Color secondary;
  final Color muted;
  final Color disabled;
  final Color inverse;
  final Color warning;
  final Color value;
  final Color destructive;
  final Color destructiveLight;
  final Color brand;
  final Color darkCard;

  static const dark = AppTextColors(
    accent: Primitives.p900Dark,
    primary: Primitives.p700Dark,
    secondary: Primitives.p600Dark,
    muted: Primitives.p500Dark,
    disabled: Primitives.p400Dark,
    inverse: Primitives.p0Dark,
    warning: OrangePrimitives.p500Dark,
    value: GoldPrimitives.p500Dark,
    destructive: RosePrimitives.p500Dark,
    destructiveLight: RosePrimitives.p400Dark,
    brand: BrandPrimitives.p400Dark,
    darkCard: Primitives.p800Dark,
  );

  static const light = AppTextColors(
    // Accent in light mode reaches the *opposite* extreme of the ladder
    // (p900Light = near-black) rather than mirroring p800Dark's step.
    accent: Primitives.p900Light,
    primary: Primitives.p700Light,
    secondary: Primitives.p600Light,
    muted: Primitives.p500Light,
    disabled: Primitives.p400Light,
    inverse: Primitives.p0Light,
    warning: OrangePrimitives.p400Light,
    value: GoldPrimitives.p400Light,
    destructive: RosePrimitives.p300Light,
    destructiveLight: RosePrimitives.p150Light,
    brand: BrandPrimitives.p300Light,
    darkCard: Primitives.p0Light,
  );
}
