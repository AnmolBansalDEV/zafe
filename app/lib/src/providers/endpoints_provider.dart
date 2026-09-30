import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/config/endpoints.dart';
import 'vault_provider.dart';

/// The relay and lightwalletd URLs in use (build defaults or the user's overrides from
/// Settings). Read in the bootstrap; background checks read prefs themselves
/// (`ZafeEndpoints.load(reload: true)`).
class EndpointsNotifier extends Notifier<ZafeEndpoints> {
  @override
  ZafeEndpoints build() => ref.watch(vaultBootstrapProvider).endpoints;

  /// Saves an already checked URL (see `checkEndpointUrl` and the connection test).
  Future<void> set(EndpointKind kind, String url) async {
    await ZafeEndpoints.save(kind, url);
    state = state.withUrl(kind, url);
  }

  Future<void> reset(EndpointKind kind) =>
      set(kind, ZafeEndpoints.defaults.url(kind));
}

final endpointsProvider = NotifierProvider<EndpointsNotifier, ZafeEndpoints>(
  EndpointsNotifier.new,
);
