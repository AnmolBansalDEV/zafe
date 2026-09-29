import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/app.dart';
import 'src/providers/vault_provider.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final bootstrap = await VaultBootstrap.load();
  runApp(
    ProviderScope(
      overrides: [vaultBootstrapProvider.overrideWithValue(bootstrap)],
      child: const ZafeApp(),
    ),
  );
}
