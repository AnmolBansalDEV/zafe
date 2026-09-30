import 'package:flutter/widgets.dart';

import '../../theme/app_theme.dart';
import '../app_icon.dart';

// Detail-card vocabulary shared by the review and proposal screens (send review and
// status detail cards).

/// 40px ZEC coin (fixed Zcash yellow).
class ZecCoin extends StatelessWidget {
  const ZecCoin({super.key});

  @override
  Widget build(BuildContext context) => Container(
    width: 40,
    height: 40,
    decoration: const BoxDecoration(
      color: Color(0xFFF4B728),
      shape: BoxShape.circle,
    ),
    alignment: Alignment.center,
    child: const AppIcon(
      AppIcons.zcashCurrency,
      size: 22,
      color: Color(0xFFFFFFFF),
    ),
  );
}

class DetailRow extends StatelessWidget {
  const DetailRow({
    super.key,
    required this.label,
    required this.value,
    this.trailing,
  });
  final String label;
  final String value;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    return ConstrainedBox(
      constraints: const BoxConstraints(minHeight: 32),
      child: Row(
        children: [
          Text(
            label,
            style: AppTypography.labelLarge.copyWith(
              color: colors.text.secondary,
            ),
          ),
          const SizedBox(width: AppSpacing.sm),
          Expanded(
            child: Text(
              value,
              textAlign: TextAlign.end,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: AppTypography.labelLarge.copyWith(
                color: colors.text.accent,
              ),
            ),
          ),
          if (trailing != null) ...[
            const SizedBox(width: AppSpacing.xs),
            trailing!,
          ],
        ],
      ),
    );
  }
}

class DetailDivider extends StatelessWidget {
  const DetailDivider({super.key});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: AppSpacing.sm),
    child: Container(height: 1, color: context.colors.border.regular),
  );
}
