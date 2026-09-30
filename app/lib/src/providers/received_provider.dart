import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/errors/zafe_error_copy.dart';
import '../core/storage/zafe_paths.dart';
import '../core/storage/zafe_secure_store.dart';
import '../notifications/vault_watch.dart' show recordSeen;
import '../rust/api/received.dart' as rust;
import 'vault_provider.dart';

class ReceivedState {
  const ReceivedState({this.items = const [], this.loaded = false});

  /// Newest first; unmined ones first of all.
  final List<rust.ReceivedInfo> items;
  final bool loaded;

  rust.ReceivedInfo? byTxid(String txid) {
    for (final r in items) {
      if (r.txid == txid) return r;
    }
    return null;
  }
}

/// Money the active vault received, read from its local wallet database. Reloads after
/// every wallet sync (a new balance or height) and when another vault becomes active.
class ReceivedNotifier extends Notifier<ReceivedState> {
  bool _loading = false;

  @override
  ReceivedState build() {
    ref.watch(vaultProvider.select((v) => v.activeId));
    ref.listen(
      vaultProvider.select((v) => v.balance),
      (_, _) => refresh(),
      fireImmediately: true,
    );
    return const ReceivedState();
  }

  Future<void> refresh() async {
    final vault = ref.read(vaultProvider);
    final material = vault.material;
    final vaultId = vault.activeId;
    if (material == null || vaultId == null || _loading) return;
    _loading = true;
    try {
      final paths = await ZafePaths.get();
      final items = await rust.listReceived(
        dbDir: paths.dbDir,
        dbKey: await ZafeSecureStore.instance.walletKey(vaultId),
        material: material,
      );
      // The vault may have changed while reading.
      if (ref.read(vaultProvider).activeId == vaultId) {
        state = ReceivedState(items: items, loaded: true);
        // Seen on screen: never announced from the background (foreground only, so a
        // refresh running in the background doesn't swallow news).
        if (WidgetsBinding.instance.lifecycleState ==
            AppLifecycleState.resumed) {
          await recordSeen(vaultId, null, received: items);
        }
      }
    } catch (e) {
      debugPrint('received payments failed: ${describeError(e)}');
    } finally {
      _loading = false;
    }
  }
}

final receivedProvider = NotifierProvider<ReceivedNotifier, ReceivedState>(
  ReceivedNotifier.new,
);
