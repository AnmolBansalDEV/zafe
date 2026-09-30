import 'package:flutter_test/flutter_test.dart';
import 'package:zafe/src/core/config/network_config.dart';

void main() {
  test('regtest is the default build', () {
    expect(kZafeNetwork, 'regtest');
    expect(kZafePreset, same(kRegtestPreset));
    expect(kZafeRelayUrl, kRegtestRelayUrl);
    expect(kZafeLightwalletdUrl, kRegtestLightwalletdUrl);
    expect(kZafeRelayIsPlaceholder, isFalse);
  });

  test('testnet preset uses TLS endpoints', () {
    expect(kTestnetPreset.network, 'test');
    expect(Uri.parse(kTestnetPreset.lightwalletdUrl).scheme, 'https');
    expect(Uri.parse(kTestnetPreset.relayUrl).scheme, 'https');
    // Until a relay is deployed, the testnet relay is a placeholder that never resolves.
    expect(kTestnetPreset.relayUrl, kPlaceholderRelayUrl);
    expect(Uri.parse(kPlaceholderRelayUrl).host, endsWith('.invalid'));
  });

  test('mainnet preset uses TLS endpoints', () {
    expect(kMainnetPreset.network, 'main');
    expect(Uri.parse(kMainnetPreset.lightwalletdUrl).scheme, 'https');
  });
}
