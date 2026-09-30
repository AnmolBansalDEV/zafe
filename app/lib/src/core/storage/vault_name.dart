import 'dart:io';

import 'member_names.dart';
import 'zafe_paths.dart';

/// The name this member gave a vault on this device, shown instead of the name the
/// creator chose. Local like [MemberNames]: never sent to the relay or other members.
class VaultName {
  static Future<File> _file(String vaultId) async =>
      File('${(await ZafePaths.get()).vaultDir(vaultId)}/vault_name.txt');

  /// Longest name kept (it has to fit the top bar).
  static const maxLength = 32;

  /// The local name, or null when there's none.
  static Future<String?> read(String vaultId) async {
    try {
      final name = clean(await (await _file(vaultId)).readAsString());
      return name.isEmpty ? null : name;
    } catch (_) {
      return null;
    }
  }

  /// Stores `name`; an empty name goes back to the creator's.
  static Future<void> write(String vaultId, String name) async {
    final f = await _file(vaultId);
    final cleaned = clean(name);
    if (cleaned.isEmpty) {
      if (await f.exists()) await f.delete();
      return;
    }
    await f.parent.create(recursive: true);
    final tmp = File('${f.path}.${DateTime.now().microsecondsSinceEpoch}.tmp');
    await tmp.writeAsString(cleaned, flush: true);
    await tmp.rename(f.path);
  }

  /// Trimmed, whitespace collapsed, capped at [maxLength].
  static String clean(String name) => MemberNames.clean(name);

  /// The name to show: the local one, else the creator's.
  static String display(String creatorName, String? local) =>
      local ?? creatorName;
}
