import 'package:flutter/material.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../../core/theme/app_theme.dart';

/// Original Zafe onboarding illustrations in `assets/illustrations`, one SVG per
/// theme (`<name>_dark.svg` / `<name>_light.svg`).
String _asset(BuildContext context, String name) {
  final dark = identical(context.colors, AppColors.dark);
  return 'assets/illustrations/${name}_${dark ? 'dark' : 'light'}.svg';
}

/// Full-width art pinned to the top of the screen that fades into the window
/// colour, so text and buttons below sit on a clean background (Vizor's hero
/// placement, flipped because Zafe's content sits at the bottom).
class OnboardingHero extends StatelessWidget {
  const OnboardingHero(this.name, {super.key});

  final String name;

  @override
  Widget build(BuildContext context) {
    final window = context.colors.background.window;
    return Positioned(
      top: 0,
      left: 0,
      right: 0,
      child: ExcludeSemantics(
        child: Stack(
          children: [
            AspectRatio(
              aspectRatio: 1080 / 1240,
              child: SvgPicture.asset(_asset(context, name), fit: BoxFit.cover),
            ),
            Positioned.fill(
              child: DecoratedBox(
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.topCenter,
                    end: Alignment.bottomCenter,
                    // Soft scrim under the status bar, clear art, then a
                    // fade into the window colour before the text starts.
                    stops: const [0, 0.1, 0.4, 0.74],
                    colors: [
                      window.withValues(alpha: 0.85),
                      window.withValues(alpha: 0),
                      window.withValues(alpha: 0),
                      window,
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Rounded banner at the top of a form screen.
class OnboardingBanner extends StatelessWidget {
  const OnboardingBanner(this.name, {super.key});

  final String name;

  @override
  Widget build(BuildContext context) {
    return ExcludeSemantics(
      child: ClipRRect(
        borderRadius: BorderRadius.circular(AppRadii.large),
        child: AspectRatio(
          aspectRatio: 1080 / 560,
          child: SvgPicture.asset(_asset(context, name), fit: BoxFit.cover),
        ),
      ),
    );
  }
}
