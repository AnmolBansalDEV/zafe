import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import 'app_radii.dart';
import 'colors/app_colors.dart';

/// Material theme for the Material widgets Zafe still uses (dialogs, pickers,
/// switches, text selection, scaffolds). Its [ColorScheme] is built from the
/// app's own tokens ([AppColors], "Verdigris", docs/brand.md), so Material
/// widgets match the rest of the app. Screens style themselves through
/// `context.colors`; don't read `Theme.of(context)` colours in new code.
ColorScheme _scheme(AppColors c, Brightness brightness) => ColorScheme(
  brightness: brightness,
  // Surfaces
  surface: c.background.ground,
  onSurface: c.text.accent,
  surfaceContainerLowest: c.background.window,
  surfaceContainerLow: c.background.base,
  surfaceContainer: c.background.raised,
  surfaceContainerHigh: c.background.raised,
  surfaceContainerHighest: c.background.overlay,
  onSurfaceVariant: c.text.secondary,
  // Brand: primary buttons, focus, selection
  primary: c.button.primary.bg,
  onPrimary: c.button.primary.label,
  primaryContainer: c.background.brandSubtle,
  onPrimaryContainer: c.text.brand,
  // Value (Zcash gold)
  secondary: c.text.value,
  onSecondary: c.text.inverse,
  secondaryContainer: c.background.valueAlpha,
  onSecondaryContainer: c.text.value,
  tertiary: c.text.warning,
  onTertiary: c.text.inverse,
  // Error (rose)
  error: c.text.destructive,
  onError: c.button.destructive.label,
  errorContainer: c.background.destructiveSubtle,
  onErrorContainer: c.text.destructive,
  // Lines
  outline: c.border.regular,
  outlineVariant: c.border.subtle,
  inverseSurface: c.background.inverse,
  onInverseSurface: c.text.inverse,
  inversePrimary: c.text.brand,
  shadow: const Color(0xFF000000),
  scrim: const Color(0xFF000000),
);

// ---------------------------------------------------------------------------
// Text themes
// ---------------------------------------------------------------------------

const _bodyFamily = 'DM Sans';

TextTheme _buildTextTheme(Color textColor) {
  return TextTheme(
    // Hero balance (56px, w800)
    displayLarge: TextStyle(
      fontWeight: FontWeight.w800,
      fontSize: 56,
      height: 1.0,
      letterSpacing: -2,
      color: textColor,
    ),
    // Large heading (28px, w600) — e.g. "ZEC" unit
    displayMedium: TextStyle(
      fontWeight: FontWeight.w600,
      fontSize: 28,
      height: 1.2,
      letterSpacing: -0.5,
      color: textColor,
    ),
    // Section heading (20px, w700)
    titleLarge: TextStyle(
      fontWeight: FontWeight.w700,
      fontSize: 20,
      height: 1.2,
      letterSpacing: -0.3,
      color: textColor,
    ),
    // List item title (15px, w700)
    titleMedium: TextStyle(
      fontWeight: FontWeight.w700,
      fontSize: 15,
      height: 1.4,
      color: textColor,
    ),
    // Body (14px Inter 400)
    bodyLarge: TextStyle(
      fontFamily: _bodyFamily,
      fontWeight: FontWeight.w400,
      fontSize: 14,
      height: 1.5,
      color: textColor,
    ),
    // Body secondary (14px Inter 500)
    bodyMedium: TextStyle(
      fontFamily: _bodyFamily,
      fontWeight: FontWeight.w500,
      fontSize: 14,
      height: 1.5,
      color: textColor,
    ),
    // Small body (12px Inter 500)
    bodySmall: TextStyle(
      fontFamily: _bodyFamily,
      fontWeight: FontWeight.w500,
      fontSize: 12,
      height: 1.5,
      color: textColor,
    ),
    // Uppercase label (11px Inter 600)
    labelLarge: TextStyle(
      fontFamily: _bodyFamily,
      fontWeight: FontWeight.w600,
      fontSize: 11,
      letterSpacing: 1.5,
      color: textColor,
    ),
    // Button label (11px, w700)
    labelMedium: TextStyle(
      fontWeight: FontWeight.w700,
      fontSize: 11,
      letterSpacing: 2,
      color: textColor,
    ),
    // Caption (10px Inter 700)
    labelSmall: TextStyle(
      fontFamily: _bodyFamily,
      fontWeight: FontWeight.w700,
      fontSize: 10,
      letterSpacing: 1.5,
      color: textColor,
    ),
  );
}

// ---------------------------------------------------------------------------
// ThemeData builder — single source of truth for both light and dark
// ---------------------------------------------------------------------------

// Desktop platforms get instant page transitions (no slide/fade/zoom).
// Individual routes can still override via CustomTransitionPage in GoRouter.
class _NoTransitionsBuilder extends PageTransitionsBuilder {
  const _NoTransitionsBuilder();

  @override
  Duration get transitionDuration => Duration.zero;

  @override
  Duration get reverseTransitionDuration => Duration.zero;

  @override
  Widget buildTransitions<T>(
    PageRoute<T> route,
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) => child;
}

bool get _isDesktop =>
    Platform.isMacOS || Platform.isWindows || Platform.isLinux;

ThemeData _buildTheme(AppColors colors, Brightness brightness) {
  final colorScheme = _scheme(colors, brightness);
  return ThemeData(
    useMaterial3: true,
    dialogTheme: defaultTargetPlatform == TargetPlatform.iOS
        ? const DialogThemeData(
            shape: RoundedSuperellipseBorder(
              borderRadius: BorderRadius.all(Radius.circular(AppRadii.xLarge)),
            ),
          )
        : null,
    brightness: colorScheme.brightness,
    colorScheme: colorScheme,
    textTheme: _buildTextTheme(colorScheme.onSurface),
    fontFamily: _bodyFamily,
    scaffoldBackgroundColor: colors.background.window,
    textSelectionTheme: TextSelectionThemeData(
      cursorColor: colors.text.brand,
      selectionColor: colors.background.brandAlpha,
      selectionHandleColor: colors.text.brand,
    ),
    pageTransitionsTheme: _isDesktop
        ? const PageTransitionsTheme(
            builders: {
              TargetPlatform.macOS: _NoTransitionsBuilder(),
              TargetPlatform.windows: _NoTransitionsBuilder(),
              TargetPlatform.linux: _NoTransitionsBuilder(),
            },
          )
        : null,
    appBarTheme: AppBarTheme(
      backgroundColor: colorScheme.surface,
      foregroundColor: colorScheme.onSurface,
      elevation: 0,
      scrolledUnderElevation: 0,
      titleTextStyle: TextStyle(
        fontWeight: FontWeight.w700,
        fontSize: 20,
        letterSpacing: -0.3,
        color: colorScheme.onSurface,
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: colorScheme.primary,
        foregroundColor: colorScheme.onPrimary,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        textStyle: const TextStyle(fontWeight: FontWeight.w700, fontSize: 14),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: colorScheme.onSurface,
        side: BorderSide(color: colorScheme.outline),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        textStyle: const TextStyle(fontWeight: FontWeight.w700, fontSize: 14),
      ),
    ),
  );
}

ThemeData buildMaterialLightTheme() =>
    _buildTheme(AppColors.light, Brightness.light);
ThemeData buildMaterialDarkTheme() =>
    _buildTheme(AppColors.dark, Brightness.dark);
