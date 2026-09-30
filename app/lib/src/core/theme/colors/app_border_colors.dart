import 'package:flutter/painting.dart';

import '../primitives.dart';

/// Border / divider weights.
///
/// * [subtle] — Hairline dividers, row separators.
/// * [subtleOpacity] — Alpha border used on strong filled controls.
/// * [inverseOpacity] — Alpha border used over inverted / strong fills.
/// * [regular] — Default field/card/chip border. (Named `regular` instead of
///   Figma's `default` because `default` is a reserved word in Dart.)
/// * [medium] — Active/filled field border.
/// * [strong] — Max-contrast border.
/// * [destructive] — Validation / destructive emphasis.
/// * [destructiveSubtle] — Soft destructive border.
/// * [brandStrong] — Brand feedback / accent border.
class AppBorderColors {
  const AppBorderColors({
    required this.subtle,
    required this.subtleOpacity,
    required this.inverseOpacity,
    required this.regular,
    required this.medium,
    required this.strong,
    required this.destructive,
    required this.destructiveSubtle,
    required this.brandStrong,
  });

  final Color subtle;
  final Color subtleOpacity;
  final Color inverseOpacity;
  final Color regular;
  final Color medium;
  final Color strong;
  final Color destructive;
  final Color destructiveSubtle;
  final Color brandStrong;

  static const dark = AppBorderColors(
    subtle: Primitives.p150Dark,
    subtleOpacity: Primitives.p900Alpha10Dark,
    inverseOpacity: Primitives.p150Alpha15Dark,
    regular: Primitives.p300Dark,
    medium: Primitives.p400Dark,
    strong: Primitives.p800Dark,
    destructive: RosePrimitives.p400Dark,
    destructiveSubtle: RosePrimitives.p100Dark,
    brandStrong: BrandPrimitives.p400Dark,
  );

  static const light = AppBorderColors(
    subtle: Primitives.p150Light,
    subtleOpacity: Primitives.p900Alpha5Light,
    inverseOpacity: Primitives.p0Alpha10Light,
    regular: Primitives.p200Light,
    medium: Primitives.p300Light,
    strong: Primitives.p900Light,
    destructive: RosePrimitives.p300Light,
    destructiveSubtle: RosePrimitives.p100Light,
    brandStrong: BrandPrimitives.p300Light,
  );
}
