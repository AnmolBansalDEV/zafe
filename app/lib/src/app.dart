import 'dart:async';

import 'package:flutter/cupertino.dart' show CupertinoPage;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'core/theme/app_theme_host.dart';
import 'core/theme/legacy_material_theme.dart';
import 'features/backup/backup_prompt_screen.dart';
import 'features/backup/export_screen.dart';
import 'features/backup/restore_screen.dart';
import 'features/home/home_screen.dart';
import 'features/onboarding/create_vault_screen.dart';
import 'features/onboarding/join_vault_screen.dart';
import 'features/onboarding/setup_screen.dart';
import 'features/onboarding/welcome_screen.dart';
import 'features/proposals/proposal_screen.dart';
import 'features/receive/receive_screen.dart';
import 'features/send/send_screen.dart';
import 'features/settings/settings_screen.dart';
import 'providers/theme_mode_provider.dart';
import 'notifications/vault_watch.dart';
import 'providers/vault_provider.dart';

final _routerProvider = Provider<GoRouter>((ref) {
  final vault = ref.read(vaultProvider);
  final initial = vault.hasVault
      ? '/home'
      : vault.isSettingUp
      ? '/setup'
      : '/welcome';

  Page<void> page(Widget child) => CupertinoPage(child: child);

  late final GoRouter router;
  // Tapped notifications open their payment (also the one that launched the app),
  // switching to that payment's vault first.
  Future<void> openTapped() async {
    final payload = notificationTaps.value;
    final ids = payload == null ? null : parsePayload(payload);
    if (ids == null) return;
    notificationTaps.value = null;
    final (vaultId, proposalId) = ids;
    final vaults = ref.read(vaultProvider);
    if (!vaults.vaults.any((v) => v.id == vaultId && v.ready)) return;
    if (vaults.activeId != vaultId || vaults.isAdding) {
      await ref.read(vaultProvider.notifier).switchTo(vaultId);
      router.go('/home');
    }
    router.push('/proposal/$proposalId');
  }

  void onTap() => unawaited(openTapped());
  notificationTaps.addListener(onTap);
  ref.onDispose(() => notificationTaps.removeListener(onTap));
  WidgetsBinding.instance.addPostFrameCallback((_) => unawaited(openTapped()));

  return router = GoRouter(
    initialLocation: initial,
    redirect: (context, state) {
      final vault = ref.read(vaultProvider);
      final loc = state.matchedLocation;
      final inVault = const [
        '/home',
        '/receive',
        '/send',
        '/proposal',
        '/settings',
        '/export',
        '/backup-prompt',
      ].any(loc.startsWith);
      if (vault.hasVault && !inVault) return '/home';
      if (!vault.hasVault && inVault) {
        return vault.isSettingUp ? '/setup' : '/welcome';
      }
      return null;
    },
    routes: [
      GoRoute(
        path: '/welcome',
        pageBuilder: (_, _) => page(const WelcomeScreen()),
      ),
      GoRoute(
        path: '/create',
        pageBuilder: (_, _) => page(const CreateVaultScreen()),
      ),
      GoRoute(
        path: '/join',
        pageBuilder: (_, _) => page(const JoinVaultScreen()),
      ),
      GoRoute(path: '/setup', pageBuilder: (_, _) => page(const SetupScreen())),
      GoRoute(path: '/home', pageBuilder: (_, _) => page(const HomeScreen())),
      GoRoute(
        path: '/receive',
        pageBuilder: (_, _) => page(const ReceiveScreen()),
      ),
      GoRoute(
        path: '/export',
        pageBuilder: (_, _) => page(const ExportScreen()),
      ),
      GoRoute(
        path: '/backup-prompt',
        pageBuilder: (_, _) => page(const BackupPromptScreen()),
      ),
      GoRoute(
        path: '/restore',
        pageBuilder: (_, _) => page(const RestoreScreen()),
      ),
      GoRoute(
        path: '/settings',
        pageBuilder: (_, _) => page(const SettingsScreen()),
      ),
      GoRoute(path: '/send', pageBuilder: (_, _) => page(const SendScreen())),
      GoRoute(
        path: '/proposal/:id',
        pageBuilder: (_, state) =>
            page(ProposalScreen(id: state.pathParameters['id']!)),
      ),
    ],
  );
});

class ZafeApp extends ConsumerWidget {
  const ZafeApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    return AppThemeHost(
      themeMode: themeMode,
      child: MaterialApp.router(
        title: 'Zafe',
        debugShowCheckedModeBanner: false,
        theme: buildLegacyLightTheme(),
        darkTheme: buildLegacyDarkTheme(),
        themeMode: themeMode,
        routerConfig: ref.watch(_routerProvider),
      ),
    );
  }
}
