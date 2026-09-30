import 'dart:math' as math;

import 'package:flutter/widgets.dart';

import '../../config/network_config.dart';
import '../../formatting/member_label.dart';
import '../../privacy/privacy_mask.dart';
import '../../theme/app_theme.dart';
import '../app_icon.dart';
import '../app_tappable.dart';

// Detail vocabulary shared by the payment review and proposal screens: the payment
// card, detail rows, signer tiles/rows and the approval dots.

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

// Payment card colours: the same theme-invariant ink card as the home vault card.
const _cardInk = Color(0xFF0E131B);
const _cardJade = Color(0xFF2EC4A6);
const _cardText = Color(0xFFF2F4F8);

/// The payment at the top of the review and proposal screens: amount in large type on
/// the dark vault card, and the recipient (shielded address, compact) with a
/// "Full address" action.
class PaymentCard extends StatelessWidget {
  const PaymentCard({
    super.key,
    required this.amountText,
    this.address,
    this.onFullAddress,
    this.label = 'PAYMENT',
    this.recipients = 1,
    this.hidden = false,
    this.strikethrough = false,
  });

  /// Amount without the ticker ("1.5").
  final String amountText;

  /// First recipient's address; null for money the vault received (no recipient box).
  final String? address;
  final VoidCallback? onFullAddress;
  final String label;
  final int recipients;
  final bool hidden;

  /// Rejected or cancelled payments.
  final bool strikethrough;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final decoration = strikethrough
        ? TextDecoration.lineThrough
        : TextDecoration.none;
    return Container(
      decoration: BoxDecoration(
        color: _cardInk,
        borderRadius: BorderRadius.circular(AppRadii.large),
        border: Border.all(color: const Color(0x1A2EC4A6), width: 1),
        boxShadow: appSurfaceShadow(colors),
      ),
      clipBehavior: Clip.antiAlias,
      child: Stack(
        children: [
          const Positioned.fill(
            child: CustomPaint(painter: _PaymentDialPainter()),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 18, 16, 16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      label,
                      style: AppTypography.labelSmall.copyWith(
                        color: _cardText.withValues(alpha: 0.7),
                        letterSpacing: 1.6,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(width: AppSpacing.xs),
                    const AppIcon(
                      AppIcons.shieldKeyhole,
                      size: 14,
                      color: _cardJade,
                    ),
                  ],
                ),
                const SizedBox(height: AppSpacing.s),
                Text.rich(
                  TextSpan(
                    children: [
                      TextSpan(
                        text: hidden ? fixedPrivacyMask() : amountText,
                        style: TextStyle(
                          fontFamily: 'Space Grotesk',
                          fontWeight: FontWeight.w500,
                          fontSize: 42,
                          height: 1.05,
                          letterSpacing: -1.6,
                          color: _cardText.withValues(
                            alpha: strikethrough ? 0.55 : 1,
                          ),
                          decoration: decoration,
                          decorationColor: _cardText.withValues(alpha: 0.55),
                        ),
                      ),
                      TextSpan(
                        text: ' $kZcashDefaultCurrencyTicker',
                        style: TextStyle(
                          fontFamily: 'Space Grotesk',
                          fontWeight: FontWeight.w500,
                          fontSize: 20,
                          color: _cardJade.withValues(
                            alpha: strikethrough ? 0.55 : 1,
                          ),
                        ),
                      ),
                    ],
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                if (address != null) ...[
                  const SizedBox(height: AppSpacing.md),
                  _Recipient(
                    address: address!,
                    recipients: recipients,
                    onFullAddress: onFullAddress ?? () {},
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _Recipient extends StatelessWidget {
  const _Recipient({
    required this.address,
    required this.recipients,
    required this.onFullAddress,
  });
  final String address;
  final int recipients;
  final VoidCallback onFullAddress;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.fromLTRB(12, 10, 8, 10),
      decoration: BoxDecoration(
        color: const Color(0x0DFFFFFF),
        borderRadius: BorderRadius.circular(AppRadii.small),
        border: Border.all(color: const Color(0x14FFFFFF)),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const AppIcon(
                      AppIcons.shieldKeyhole,
                      size: 12,
                      color: _cardJade,
                    ),
                    const SizedBox(width: 4),
                    Flexible(
                      child: Text(
                        recipients == 1
                            ? 'To shielded address'
                            : 'To $recipients recipients',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: AppTypography.labelSmall.copyWith(
                          color: _cardText.withValues(alpha: 0.6),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  compactAddress(address),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    fontFamily: 'JetBrains Mono',
                    fontSize: 13,
                    height: 1.3,
                    color: _cardText,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: AppSpacing.xs),
          AppTappable(
            onTap: onFullAddress,
            semanticsLabel: 'Full address',
            child: Container(
              height: 32,
              padding: const EdgeInsets.symmetric(horizontal: 10),
              decoration: BoxDecoration(
                color: const Color(0x14FFFFFF),
                borderRadius: BorderRadius.circular(AppRadii.xSmall),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const AppIcon(AppIcons.eye, size: 14, color: _cardText),
                  const SizedBox(width: 6),
                  Text(
                    'Full address',
                    style: AppTypography.labelSmall.copyWith(
                      color: _cardText,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// A quarter of the vault dial (rings and ticks) behind the payment card's top-right.
class _PaymentDialPainter extends CustomPainter {
  const _PaymentDialPainter();

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width + 8, -12);
    final glow = size.width * 0.55;
    canvas.drawCircle(
      center,
      glow,
      Paint()
        ..shader = const RadialGradient(
          colors: [Color(0x332EC4A6), Color(0x002EC4A6)],
        ).createShader(Rect.fromCircle(center: center, radius: glow)),
    );
    final ring = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1;
    for (var i = 0; i < 4; i++) {
      ring.color = Color.fromRGBO(46, 196, 166, 0.20 - i * 0.04);
      canvas.drawCircle(center, 56.0 + i * 30, ring);
    }
    final tick = Paint()
      ..color = const Color(0x552EC4A6)
      ..strokeWidth = 1.5
      ..strokeCap = StrokeCap.round;
    const outer = 56.0 + 30;
    for (var i = 0; i < 72; i++) {
      final dir = Offset.fromDirection(i * 5 * math.pi / 180);
      final len = i % 6 == 0 ? 8.0 : 4.0;
      canvas.drawLine(center + dir * (outer - len), center + dir * outer, tick);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
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

/// A member's tile: a rounded square with a mirrored 5 x 5 pattern and a hue both
/// derived from the member key, so the same signer looks the same on every device.
/// [me] adds a jade outline.
class SignerTile extends StatelessWidget {
  const SignerTile({
    super.key,
    required this.keyHex,
    this.me = false,
    this.size = 32,
  });
  final String keyHex;
  final bool me;
  final double size;

  @override
  Widget build(BuildContext context) {
    final dark = context.appTheme == AppThemeData.dark;
    final seed = _SignerSeed.of(keyHex);
    final background = HSLColor.fromAHSL(
      1,
      seed.hue,
      dark ? 0.16 : 0.22,
      dark ? 0.17 : 0.91,
    ).toColor();
    final foreground = HSLColor.fromAHSL(
      1,
      seed.hue,
      dark ? 0.28 : 0.26,
      dark ? 0.52 : 0.48,
    ).toColor();
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(size * 0.28),
        border: me
            ? Border.all(color: context.colors.border.brandStrong, width: 1.5)
            : null,
      ),
      child: CustomPaint(
        painter: _SignerPatternPainter(seed.cells, foreground),
      ),
    );
  }
}

class _SignerSeed {
  const _SignerSeed(this.hue, this.cells);
  final double hue;

  /// 15 bits: 5 rows x 3 columns (mirrored to 5 columns).
  final int cells;

  static _SignerSeed of(String keyHex) {
    final bytes = <int>[];
    for (var i = 0; i + 1 < keyHex.length && bytes.length < 4; i += 2) {
      final b = int.tryParse(keyHex.substring(i, i + 2), radix: 16);
      if (b == null) break;
      bytes.add(b);
    }
    if (bytes.length < 4) {
      // Not a hex key: fall back to a stable string hash.
      var h = 0x811C9DC5;
      for (final c in keyHex.codeUnits) {
        h = ((h ^ c) * 0x01000193) & 0xFFFFFFFF;
      }
      bytes
        ..clear()
        ..addAll([h >> 24, (h >> 16) & 0xFF, (h >> 8) & 0xFF, h & 0xFF]);
    }
    final hue = (((bytes[0] << 8) | bytes[1]) % 360).toDouble();
    var cells = ((bytes[2] << 8) | bytes[3]) & 0x7FFF;
    // Keep the tile from looking empty or solid.
    final filled = _popCount(cells);
    if (filled < 5) cells |= 0x4A52;
    if (filled > 11) cells &= 0x5AD5;
    return _SignerSeed(hue, cells);
  }

  static int _popCount(int v) {
    var n = 0;
    for (; v != 0; v >>= 1) {
      n += v & 1;
    }
    return n;
  }
}

class _SignerPatternPainter extends CustomPainter {
  const _SignerPatternPainter(this.cells, this.color);
  final int cells;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final inset = size.width * 0.2;
    final cell = (size.width - 2 * inset) / 5;
    final paint = Paint()..color = color;
    final r = Radius.circular(cell * 0.3);
    for (var row = 0; row < 5; row++) {
      for (var col = 0; col < 3; col++) {
        if (cells & (1 << (row * 3 + col)) == 0) continue;
        for (final c in {col, 4 - col}) {
          canvas.drawRRect(
            RRect.fromRectAndRadius(
              Rect.fromLTWH(
                inset + c * cell + cell * 0.08,
                inset + row * cell + cell * 0.08,
                cell * 0.84,
                cell * 0.84,
              ),
              r,
            ),
            paint,
          );
        }
      }
    }
  }

  @override
  bool shouldRepaint(_SignerPatternPainter old) =>
      old.cells != cells || old.color != color;
}

/// A signer: tile, name ("You" or the short key; yours also shows the key below)
/// and an optional trailing tag.
class SignerRow extends StatelessWidget {
  const SignerRow({
    super.key,
    required this.keyHex,
    required this.me,
    this.trailing,
  });
  final String keyHex;

  /// This device's key (for "You"), if known.
  final String? me;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final isMe = keyHex == me;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          SignerTile(keyHex: keyHex, me: isMe),
          const SizedBox(width: AppSpacing.s),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  memberLabel(keyHex, me: me),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: isMe
                      ? AppTypography.labelLarge.copyWith(
                          color: colors.text.primary,
                          fontWeight: FontWeight.w600,
                        )
                      : AppTypography.labelLarge.copyWith(
                          fontFamily: 'JetBrains Mono',
                          color: colors.text.primary,
                        ),
                ),
                if (isMe)
                  Text(
                    memberLabel(keyHex),
                    style: AppTypography.labelSmall.copyWith(
                      fontFamily: 'JetBrains Mono',
                      color: colors.text.muted,
                    ),
                  ),
              ],
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

enum SignerMark { approved, rejected, waiting }

/// One dot per signer, approvals first (jade), then open seats (outline), then
/// rejections (muted, crossed). A short bar after the [threshold]-th dot marks how
/// many approvals the payment needs.
class ApprovalDots extends StatelessWidget {
  const ApprovalDots({
    super.key,
    required this.approved,
    required this.rejected,
    required this.members,
    required this.threshold,
  });
  final int approved;
  final int rejected;
  final int members;
  final int threshold;

  List<SignerMark> get marks {
    final waiting = math.max(0, members - approved - rejected);
    return [
      for (var i = 0; i < approved; i++) SignerMark.approved,
      for (var i = 0; i < waiting; i++) SignerMark.waiting,
      for (var i = 0; i < rejected; i++) SignerMark.rejected,
    ];
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    final all = marks;
    return Semantics(
      label: '$approved of $threshold approvals',
      child: Wrap(
        spacing: 6,
        runSpacing: 6,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          for (var i = 0; i < all.length; i++) ...[
            _Dot(mark: all[i]),
            if (i == threshold - 1 && i != all.length - 1)
              Container(
                width: 1.5,
                height: 12,
                margin: const EdgeInsets.symmetric(horizontal: 2),
                decoration: BoxDecoration(
                  color: colors.border.medium,
                  borderRadius: BorderRadius.circular(1),
                ),
              ),
          ],
        ],
      ),
    );
  }
}

class _Dot extends StatelessWidget {
  const _Dot({required this.mark});
  final SignerMark mark;

  @override
  Widget build(BuildContext context) {
    final colors = context.colors;
    const size = 10.0;
    return switch (mark) {
      SignerMark.approved => Container(
        width: size,
        height: size,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: colors.icon.brand,
        ),
      ),
      SignerMark.waiting => Container(
        width: size,
        height: size,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          border: Border.all(color: colors.border.strong, width: 1.5),
        ),
      ),
      SignerMark.rejected => Container(
        width: size,
        height: size,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: colors.background.utilityDestructiveAlpha,
          border: Border.all(color: colors.border.utilityDestructive),
        ),
        alignment: Alignment.center,
        child: AppIcon(AppIcons.cross, size: 7, color: colors.icon.destructive),
      ),
    };
  }
}

/// Quiet vote status at the end of signer rows (icon + text).
class SignerTag extends StatelessWidget {
  const SignerTag({
    super.key,
    required this.label,
    required this.color,
    this.background,
    this.icon,
    this.iconColor,
  });
  final String label;
  final Color color;

  /// Only for states that need attention; otherwise the tag is plain text.
  final Color? background;
  final String? icon;
  final Color? iconColor;

  @override
  Widget build(BuildContext context) {
    final row = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (icon != null) ...[
          AppIcon(icon!, size: 13, color: iconColor ?? color),
          const SizedBox(width: 4),
        ],
        Text(label, style: AppTypography.labelMedium.copyWith(color: color)),
      ],
    );
    if (background == null) return row;
    return Container(
      height: 24,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(AppRadii.xSmall),
      ),
      child: row,
    );
  }
}
