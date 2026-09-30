import 'package:flutter/painting.dart';

import '../primitives.dart';

/// The sync status in the top nav.
///
/// * [text] — "Synced" label: the value colour's pale step (Zcash gold).
/// * [textError] — Label when a sync failed.
/// * [glow] — The status indicator's glow.
class AppSyncColors {
  const AppSyncColors({
    required this.text,
    required this.textError,
    required this.glow,
  });

  final Color text;
  final Color textError;
  final Color glow;

  static const dark = AppSyncColors(
    text: GoldPrimitives.p800Dark,
    textError: Primitives.p900Alpha50Dark,
    glow: Primitives.p500Dark,
  );

  static const light = AppSyncColors(
    text: GoldPrimitives.p500Light,
    textError: Primitives.p900Alpha50Light,
    glow: GoldPrimitives.p300Light,
  );
}
