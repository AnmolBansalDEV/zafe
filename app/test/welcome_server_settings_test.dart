import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zafe/src/core/config/endpoints.dart';
import 'package:zafe/src/core/config/network_config.dart';
import 'package:zafe/src/core/theme/app_theme.dart';
import 'package:zafe/src/features/onboarding/welcome_screen.dart';
import 'package:zafe/src/providers/endpoints_provider.dart';
import 'package:zafe/src/providers/vault_provider.dart';

class _Endpoints extends EndpointsNotifier {
  _Endpoints(this.endpoints);
  final ZafeEndpoints endpoints;

  @override
  ZafeEndpoints build() => endpoints;
}

class _Vault extends VaultNotifier {
  @override
  VaultState build() => const VaultState();
}

Future<void> _pump(WidgetTester tester, String relayUrl) async {
  // The test font (Ahem) is wider than DM Sans; at 390 pt a button label overflows.
  tester.view.devicePixelRatio = 3;
  tester.view.physicalSize = const Size(600 * 3, 900 * 3);
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        endpointsProvider.overrideWith(
          () => _Endpoints(
            ZafeEndpoints(
              relayUrl: relayUrl,
              lightwalletdUrl: 'https://testnet.zec.rocks:443',
            ),
          ),
        ),
        vaultProvider.overrideWith(_Vault.new),
      ],
      child: MaterialApp(
        builder: (context, child) =>
            AppTheme(data: AppThemeData.light, child: child!),
        home: const WelcomeScreen(),
      ),
    ),
  );
  await tester.pump();
}

void main() {
  testWidgets('server settings open from the welcome screen', (tester) async {
    await _pump(tester, 'https://relay.example.org');
    expect(find.textContaining('No relay set'), findsNothing);

    await tester.tap(find.bySemanticsLabel('Server settings'));
    await tester.pumpAndSettle();
    expect(find.text('Server settings'), findsOneWidget);
    expect(find.text('relay.example.org'), findsOneWidget);
    // Uri drops the default https port.
    expect(find.text('testnet.zec.rocks'), findsOneWidget);
  });

  testWidgets('a build without a relay says so', (tester) async {
    await _pump(tester, kPlaceholderRelayUrl);
    expect(find.textContaining('No relay set'), findsOneWidget);

    await tester.tap(find.textContaining('No relay set'));
    await tester.pumpAndSettle();
    expect(find.text('Not configured'), findsOneWidget);
  });
}
