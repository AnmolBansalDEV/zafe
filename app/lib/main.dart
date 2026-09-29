import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/app.dart';
import 'src/notifications/vault_watch.dart';
import 'src/providers/vault_provider.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // Shown on the licenses page (Settings > Open-source licenses): bundled fonts and the
  // Vizor-derived code.
  LicenseRegistry.addLicense(() async* {
    for (final (packages, asset) in const [
      (['Geist', 'Geist Mono'], 'assets/fonts/licenses/Geist-OFL.txt'),
      (['Young Serif'], 'assets/fonts/licenses/YoungSerif-OFL.txt'),
      (['Vizor (chainapsis/vizor-wallet)'], 'NOTICE'),
    ]) {
      yield LicenseEntryWithLineBreaks(
        packages,
        await rootBundle.loadString(asset),
      );
    }
  });
  await RustLib.init();
  await initVaultNotifications();
  final bootstrap = await VaultBootstrap.load();
  runApp(
    ProviderScope(
      overrides: [vaultBootstrapProvider.overrideWithValue(bootstrap)],
      child: const ZafeApp(),
    ),
  );
}
