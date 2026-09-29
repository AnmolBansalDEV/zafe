/// Build-time network and endpoint configuration.
///
/// `--dart-define=ZAFE_NETWORK=main|test|regtest` (default `regtest` during development)
/// `--dart-define=ZAFE_RELAY_URL=...` and `--dart-define=ZAFE_LIGHTWALLETD_URL=...`
library;

const String kZafeNetwork = String.fromEnvironment('ZAFE_NETWORK', defaultValue: 'regtest');

const String kZafeRelayUrl =
    String.fromEnvironment('ZAFE_RELAY_URL', defaultValue: 'http://127.0.0.1:8787');

const String kZafeLightwalletdUrl =
    String.fromEnvironment('ZAFE_LIGHTWALLETD_URL', defaultValue: 'http://127.0.0.1:9067');

/// Ticker shown next to amounts: ZEC on mainnet, TAZ on test networks (as in Vizor).
const String kZcashDefaultCurrencyTicker = kZafeNetwork == 'main' ? 'ZEC' : 'TAZ';
