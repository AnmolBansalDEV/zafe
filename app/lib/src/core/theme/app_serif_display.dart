import 'package:flutter/widgets.dart';

import 'app_typography.dart';

/// Display style: Space Grotesk Medium 32/33 (the display face;
/// the name is historical). Shared by review amounts and display page titles.
TextStyle appSerifDisplayStyle({required Color color}) {
  return AppTypography.displaySmall.copyWith(color: color);
}
