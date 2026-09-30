import 'dart:convert';
import 'dart:io';

import '../../rust/api/names.dart';
import 'zafe_paths.dart';

/// Names this member gave the vault's other signers (signing key hex → name). Local to
/// this device: never sent to the relay or other members. Encrypted backups carry them.
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

  /// The names as the bridge's list (backups, CSV export).
  static List<SignerName> toSigners(Map<String, String> names) => [
    for (final e in names.entries) SignerName(keyHex: e.key, name: e.value),
  ];

  /// Names from the bridge's list (a restored backup), cleaned like typed names; entries
  /// that clean to nothing or whose key isn't hex are dropped.
  static Map<String, String> fromSigners(Iterable<SignerName> signers) {
    final hex = RegExp(r'^[0-9a-f]{64}$');
    return {
      for (final s in signers)
        if (hex.hasMatch(s.keyHex) && clean(s.name).isNotEmpty)
          s.keyHex: clean(s.name),
    };
  }

  /// `name` trimmed and capped; empty removes the name.
  static String clean(String name) {
    final t = name.trim().replaceAll(RegExp(r'\s+'), ' ');
    return t.length > maxLength ? t.substring(0, maxLength) : t;
  }
}
