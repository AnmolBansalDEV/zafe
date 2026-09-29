import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// App-private directories. Android backup and device transfer are disabled for the whole
/// app (AndroidManifest + data_extraction_rules), because `stateDir` holds single-use FROST
/// nonces that must never be restored (spec §9.4).
class ZafePaths {
  const ZafePaths._(this.dbDir, this.stateDir);

  /// Wallet database (chain data, no secrets).
  final String dbDir;

  /// Nonces and the leader's signing rounds.
  final String stateDir;

  static ZafePaths? _cached;

  static Future<ZafePaths> get() async {
    final cached = _cached;
    if (cached != null) return cached;
    final support = await getApplicationSupportDirectory();
    final state = Directory('${support.path}/signing');
    await state.create(recursive: true);
    return _cached = ZafePaths._(support.path, state.path);
  }
}
