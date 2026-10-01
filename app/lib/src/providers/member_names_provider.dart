import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/storage/member_names.dart';
import 'proposals_provider.dart';
import 'vault_provider.dart';

/// Local names for the active vault's signers (key hex → name).
class MemberNamesNotifier extends Notifier<Map<String, String>> {
  @override
  Map<String, String> build() {
    final id = ref.watch(vaultProvider.select((v) => v.activeId));
    if (id != null) {
      MemberNames.read(id).then((names) {
        if (ref.read(vaultProvider).activeId == id) state = names;
      });
    }
    return const {};
  }

  /// Names a signer (an empty name removes it).
  Future<void> rename(String keyHex, String name) async {
    final id = ref.read(vaultProvider).activeId;
    if (id == null) return;
    final clean = MemberNames.clean(name);
    final next = {...state};
    if (clean.isEmpty) {
      next.remove(keyHex);
    } else {
      next[keyHex] = clean;
    }
    await MemberNames.write(id, next);
    state = next;
  }
}

final memberNamesProvider =
    NotifierProvider<MemberNamesNotifier, Map<String, String>>(
      MemberNamesNotifier.new,
    );

/// The names to show for the active vault's signers: this device's labels first, then
/// the names members shared in the vault log.
final signerNamesProvider = Provider<Map<String, String>>(
  (ref) => MemberNames.merge(
    ref.watch(proposalsProvider.select((s) => s.sharedNames)),
    ref.watch(memberNamesProvider),
  ),
);
