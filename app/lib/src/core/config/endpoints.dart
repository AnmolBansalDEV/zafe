/// The relay and lightwalletd URLs this app talks to.
///
/// Defaults come from the build (`network_config.dart`: the network's preset, or the
/// `ZAFE_RELAY_URL` / `ZAFE_LIGHTWALLETD_URL` dart-defines). The user can override either
/// one in Settings; overrides live in shared preferences under a key per network, are read
/// in the bootstrap for the UI and re-read (after `prefs.reload()`) by background checks,
/// which run in another isolate. The network itself stays compile-time.
library;

import 'package:shared_preferences/shared_preferences.dart';

import 'network_config.dart';

enum EndpointKind { relay, lightwalletd }

const kRelayUrlKey = 'zafe_relay_url_$kZafeNetwork';
const kLightwalletdUrlKey = 'zafe_lightwalletd_url_$kZafeNetwork';

class ZafeEndpoints {
  const ZafeEndpoints({
    this.relayUrl = kZafeRelayUrl,
    this.lightwalletdUrl = kZafeLightwalletdUrl,
  });

  final String relayUrl;
  final String lightwalletdUrl;

  /// The build's defaults (no overrides).
  static const defaults = ZafeEndpoints();

  String url(EndpointKind kind) => switch (kind) {
    EndpointKind.relay => relayUrl,
    EndpointKind.lightwalletd => lightwalletdUrl,
  };

  bool isDefault(EndpointKind kind) => url(kind) == defaults.url(kind);

  /// Still the placeholder relay (a testnet build before a relay was configured).
  bool get relayIsPlaceholder => relayUrl == kPlaceholderRelayUrl;

  ZafeEndpoints withUrl(EndpointKind kind, String url) => switch (kind) {
    EndpointKind.relay => ZafeEndpoints(
      relayUrl: url,
      lightwalletdUrl: lightwalletdUrl,
    ),
    EndpointKind.lightwalletd => ZafeEndpoints(
      relayUrl: relayUrl,
      lightwalletdUrl: url,
    ),
  };

  /// Reads the saved overrides. `reload` re-reads the platform store first: background
  /// isolates must, since each isolate caches prefs.
  static Future<ZafeEndpoints> load({bool reload = false}) async {
    try {
      final prefs = await SharedPreferences.getInstance();
      if (reload) await prefs.reload();
      return fromPrefs(prefs);
    } catch (_) {
      return defaults;
    }
  }

  static ZafeEndpoints fromPrefs(SharedPreferences prefs) {
    // An override that no longer validates (e.g. a rule changed) falls back to default.
    String pick(String key, EndpointKind kind) {
      final saved = prefs.getString(key);
      if (saved == null) return defaults.url(kind);
      final check = checkEndpointUrl(saved);
      return check.url ?? defaults.url(kind);
    }

    return ZafeEndpoints(
      relayUrl: pick(kRelayUrlKey, EndpointKind.relay),
      lightwalletdUrl: pick(kLightwalletdUrlKey, EndpointKind.lightwalletd),
    );
  }

  /// Saves `url` for `kind`, or removes the override when it equals the default.
  static Future<void> save(EndpointKind kind, String url) async {
    final prefs = await SharedPreferences.getInstance();
    final key = kind == EndpointKind.relay ? kRelayUrlKey : kLightwalletdUrlKey;
    if (url == defaults.url(kind)) {
      await prefs.remove(key);
    } else {
      await prefs.setString(key, url);
    }
  }
}

/// A checked URL (`url`, normalized) or why it can't be used (`error`, user copy).
class EndpointUrlCheck {
  const EndpointUrlCheck.ok(String this.url) : error = null;
  const EndpointUrlCheck.invalid(String this.error) : url = null;
  final String? url;
  final String? error;
}

/// Hosts that may use plain `http` on regtest: this machine, and the host machine as the
/// Android emulator sees it.
const kLocalEndpointHosts = {'localhost', '127.0.0.1', '10.0.2.2'};

/// Validates an endpoint URL typed by the user. `https` is required, except for a local
/// host on regtest. Only scheme, host and port are allowed; a trailing `/` is dropped.
EndpointUrlCheck checkEndpointUrl(
  String input, {
  String network = kZafeNetwork,
}) {
  final text = input.trim();
  if (text.isEmpty) return const EndpointUrlCheck.invalid('Enter a URL.');
  final uri = Uri.tryParse(text);
  if (uri == null || !uri.hasScheme || uri.host.isEmpty) {
    return const EndpointUrlCheck.invalid(
      'Enter a full URL, like https://example.com:443.',
    );
  }
  final scheme = uri.scheme.toLowerCase();
  if (scheme != 'https' && scheme != 'http') {
    return const EndpointUrlCheck.invalid('Use an https:// URL.');
  }
  if (uri.userInfo.isNotEmpty ||
      (uri.path.isNotEmpty && uri.path != '/') ||
      uri.hasQuery ||
      uri.hasFragment) {
    return const EndpointUrlCheck.invalid(
      'Use only the server address and port, without a path.',
    );
  }
  final host = uri.host.toLowerCase();
  if (scheme == 'http' &&
      !(network == 'regtest' && kLocalEndpointHosts.contains(host))) {
    return const EndpointUrlCheck.invalid(
      'Use an https:// URL. Plain http is only allowed for a local regtest server.',
    );
  }
  // `Uri` drops a port equal to the scheme's default; keep one the user typed (the
  // presets spell out `:443`, so the default URL stays recognisable).
  final explicitPort = RegExp(
    r':(\d+)$',
  ).firstMatch(RegExp(r'^[^:]+://([^/?#]*)').firstMatch(text)?.group(1) ?? '');
  final port = uri.hasPort
      ? ':${uri.port}'
      : (explicitPort != null ? ':${explicitPort.group(1)}' : '');
  // `Uri.host` drops the brackets of an IPv6 address.
  final hostPart = host.contains(':') ? '[$host]' : host;
  return EndpointUrlCheck.ok('$scheme://$hostPart$port');
}
