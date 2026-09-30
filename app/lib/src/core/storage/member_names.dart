import 'dart:convert';
import 'dart:io';

import 'zafe_paths.dart';

/// Names this member gave the vault's other signers (signing key hex → name). Local to
/// this device: never sent to the relay or other members, and not in backups.
class MemberNames {
  static Future<File> _file(String vaultId) async =>
      File('${(await ZafePaths.get()).vaultDir(vaultId)}/names.json');

  /// Longest name kept (it has to fit a signer row).
  static const maxLength = 32;

  static Future<Map<String, String>> read(String vaultId) async {
    try {
      final j = jsonDecode(await (await _file(vaultId)).readAsString()) as Map;
      return {
        for (final e in j.entries)
          if (e.key is String && e.value is String)
            e.key as String: e.value as String,
      };
    } catch (_) {
      return const {};
    }
  }

  static Future<void> write(String vaultId, Map<String, String> names) async {
    final f = await _file(vaultId);
    await f.parent.create(recursive: true);
    final tmp = File('${f.path}.${DateTime.now().microsecondsSinceEpoch}.tmp');
    await tmp.writeAsString(jsonEncode(names), flush: true);
    await tmp.rename(f.path);
  }

  /// `name` trimmed and capped; empty removes the name.
  static String clean(String name) {
    final t = name.trim().replaceAll(RegExp(r'\s+'), ' ');
    return t.length > maxLength ? t.substring(0, maxLength) : t;
  }
}
