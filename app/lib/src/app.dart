import 'package:flutter/cupertino.dart' show CupertinoPage;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'core/theme/app_theme_host.dart';
import 'core/theme/legacy_material_theme.dart';
import 'features/home/home_screen.dart';
import 'features/onboarding/create_vault_screen.dart';
import 'features/onboarding/join_vault_screen.dart';
import 'features/onboarding/setup_screen.dart';
import 'features/onboarding/welcome_screen.dart';
import 'features/receive/receive_screen.dart';
import 'providers/vault_provider.dart';

final _routerProvider = Provider<GoRouter>((ref) {
  final boot = ref.read(vaultBootstrapProvider);
  final initial = boot.material != null
      ? '/home'
      : boot.invite != null
      ? '/setup'
      : '/welcome';

  Page<void> page(Widget child) => CupertinoPage(child: child);

  return GoRouter(
    initialLocation: initial,
    redirect: (context, state) {
      final vault = ref.read(vaultProvider);
      final loc = state.matchedLocation;
      if (vault.hasVault && !(loc == '/home' || loc == '/receive')) return '/home';
      if (!vault.hasVault && (loc == '/home' || loc == '/receive')) {
        return vault.isSettingUp ? '/setup' : '/welcome';
      }
      return null;
    },
    routes: [
      GoRoute(path: '/welcome', pageBuilder: (_, _) => page(const WelcomeScreen())),
      GoRoute(path: '/create', pageBuilder: (_, _) => page(const CreateVaultScreen())),
      GoRoute(path: '/join', pageBuilder: (_, _) => page(const JoinVaultScreen())),
      GoRoute(path: '/setup', pageBuilder: (_, _) => page(const SetupScreen())),
      GoRoute(path: '/home', pageBuilder: (_, _) => page(const HomeScreen())),
      GoRoute(path: '/receive', pageBuilder: (_, _) => page(const ReceiveScreen())),
    ],
  );
});

class ZafeApp extends ConsumerWidget {
  const ZafeApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return AppThemeHost(
      themeMode: ThemeMode.system,
      child: MaterialApp.router(
        title: 'Zafe',
        debugShowCheckedModeBanner: false,
        theme: buildLegacyLightTheme(),
        darkTheme: buildLegacyDarkTheme(),
        routerConfig: ref.watch(_routerProvider),
      ),
    );
  }
}
