import 'dart:convert';
import 'dart:io';

import 'zafe_paths.dart';

/// What the vault switcher shows for each vault (including ones not on screen): last known
/// balance and how many payments wait for this member. A small file per vault, written by
/// the app and by background checks (files are shared across isolates, prefs caches aren't).
class VaultSummaryInfo {
  const VaultSummaryInfo({
    this.balanceZat,
    this.actionable = 0,
    this.backedUp = false,
  });
  final BigInt? balanceZat;
  final int actionable;

  /// A backup of this device's copy exists (exported here, or restored from one).
  final bool backedUp;
}

class VaultSummaries {
  static Future<File> _file(String vaultId) async =>
      File('${(await ZafePaths.get()).vaultDir(vaultId)}/summary.json');

  static Future<VaultSummaryInfo> read(String vaultId) async {
    try {
      final j = jsonDecode(await (await _file(vaultId)).readAsString()) as Map;
      return VaultSummaryInfo(
        balanceZat: j['balance'] == null
            ? null
            : BigInt.parse(j['balance'] as String),
        actionable: (j['actionable'] as int?) ?? 0,
        backedUp: (j['backedUp'] as bool?) ?? false,
      );
    } catch (_) {
      return const VaultSummaryInfo();
    }
  }

  // Writes are read-modify-write: serialize them (sync and refresh both write).
  static Future<void> _queue = Future.value();

  /// Updates the given fields, keeping the others. Never throws (the summary is a cache).
  static Future<void> write(
    String vaultId, {
    BigInt? balanceZat,
    int? actionable,
    bool? backedUp,
  }) {
    final next = _queue.then((_) async {
      try {
        final old = await read(vaultId);
        final f = await _file(vaultId);
        await f.parent.create(recursive: true);
        // A unique temp name: background checks in another isolate may write too.
        final tmp = File(
          '${f.path}.${DateTime.now().microsecondsSinceEpoch}.tmp',
        );
        await tmp.writeAsString(
          jsonEncode({
            'balance': (balanceZat ?? old.balanceZat)?.toString(),
            'actionable': actionable ?? old.actionable,
            'backedUp': backedUp ?? old.backedUp,
          }),
        );
        await tmp.rename(f.path);
      } catch (_) {}
    });
    _queue = next;
    return next;
  }
}
