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
import 'features/onboarding/scan_invite_screen.dart';
import 'features/onboarding/setup_screen.dart';
import 'features/onboarding/welcome_screen.dart';
import 'features/proposals/activity_screen.dart';
import 'features/proposals/proposal_screen.dart';
import 'features/proposals/sending_screen.dart';
import 'features/receive/receive_screen.dart';
import 'features/send/send_screen.dart';
import 'features/settings/settings_screen.dart';
import 'providers/theme_mode_provider.dart';
import 'notifications/vault_watch.dart';
import 'providers/vault_provider.dart';
import 'services/invite_links.dart';

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

  // Invite links open the Join screen with the invite filled in; joining still takes a
  // tap there and the safety number check after it. From inside a vault it's "Add vault".
  // A link that arrives during key generation or the backup prompt right after it waits
  // for the next navigation away from them.
  void openInviteLink() {
    final invite = inviteLinks.value;
    if (invite == null) return;
    final vaults = ref.read(vaultProvider);
    if (vaults.isSettingUp && (vaults.membership?.sealed ?? false)) return;
    final here = router.routerDelegate.currentConfiguration.uri.path;
    if (here == '/backup-prompt') return;
    inviteLinks.value = null;
    if (vaults.activeId != null) {
      ref.read(vaultProvider.notifier).beginAddVault();
    }
    router.go('/welcome'); // so back from Join lands on onboarding
    router.push(
      Uri(path: '/join', queryParameters: {'invite': invite}).toString(),
    );
  }

  inviteLinks.addListener(openInviteLink);
  ref.onDispose(() => inviteLinks.removeListener(openInviteLink));
  WidgetsBinding.instance.addPostFrameCallback((_) {
    void retry() => scheduleMicrotask(openInviteLink);
    router.routerDelegate.addListener(retry);
    ref.onDispose(() => router.routerDelegate.removeListener(retry));
    openInviteLink();
  });

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
        '/activity',
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
        pageBuilder: (_, state) => page(
          JoinVaultScreen(initialInvite: state.uri.queryParameters['invite']),
        ),
      ),
      GoRoute(
        path: '/scan-invite',
        pageBuilder: (_, _) => page(const ScanInviteScreen()),
      ),
      GoRoute(path: '/setup', pageBuilder: (_, _) => page(const SetupScreen())),
      GoRoute(path: '/home', pageBuilder: (_, _) => page(const HomeScreen())),
      GoRoute(
        path: '/receive',
        pageBuilder: (_, _) => page(const ReceiveScreen()),
      ),
      GoRoute(
        path: '/activity',
        pageBuilder: (_, _) => page(const ActivityScreen()),
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
        routes: [
          GoRoute(
            path: 'send',
            pageBuilder: (_, state) =>
                page(SendingScreen(id: state.pathParameters['id']!)),
          ),
        ],
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
