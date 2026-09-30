import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../../core/layout/mobile/app_mobile_shell.dart';
import '../../core/layout/mobile/app_mobile_tab_bar.dart';
import '../../core/widgets/app_icon.dart';

/// The vault's tabs (Home, Activity, Signers, Settings) under the floating tab bar.
/// Pushed pages (a payment, Send, Receive...) cover the bar.
class VaultShell extends StatelessWidget {
  const VaultShell({required this.shell, super.key});
  final StatefulNavigationShell shell;

  @override
  Widget build(BuildContext context) {
    return AppMobileShell(
      body: shell,
      tabBar: AppMobileTabBar(
        items: const [
          AppMobileTabItem(iconName: AppIcons.home, label: 'Home'),
          AppMobileTabItem(iconName: AppIcons.history, label: 'Activity'),
          AppMobileTabItem(iconName: AppIcons.users, label: 'Signers'),
          AppMobileTabItem(iconName: AppIcons.cog, label: 'Settings'),
        ],
        currentIndex: shell.currentIndex,
        // Selecting the current tab again goes back to its root.
        onSelect: (i) =>
            shell.goBranch(i, initialLocation: i == shell.currentIndex),
      ),
    );
  }
}
