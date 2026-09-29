import 'dart:convert';
import 'dart:io';

import 'zafe_paths.dart';

/// What the vault switcher shows for each vault (including ones not on screen): last known
/// balance and how many payments wait for this member. A small file per vault, written by
/// the app and by background checks (files are shared across isolates, prefs caches aren't).
class VaultSummaryInfo {
  const VaultSummaryInfo({this.balanceZat, this.actionable = 0});
  final BigInt? balanceZat;
  final int actionable;
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
      );
    } catch (_) {
      return const VaultSummaryInfo();
    }
  }

  /// Updates the given fields, keeping the others.
  static Future<void> write(
    String vaultId, {
    BigInt? balanceZat,
    int? actionable,
  }) async {
    final old = await read(vaultId);
    final f = await _file(vaultId);
    await f.parent.create(recursive: true);
    final tmp = File('${f.path}.tmp');
    await tmp.writeAsString(
      jsonEncode({
        'balance': (balanceZat ?? old.balanceZat)?.toString(),
        'actionable': actionable ?? old.actionable,
      }),
    );
    await tmp.rename(f.path);
  }
}
