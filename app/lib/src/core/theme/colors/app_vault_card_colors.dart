import 'package:flutter/painting.dart';

import '../primitives.dart';

/// The vault card (Home's balance card) and the payment card that mirrors it
/// (docs/brand.md §3, "Vault card in light mode").
///
/// Dark: a deep verdigris ink card with the dial, shield and signer dots in the
/// brand accent. Light: the card follows the theme (user's decision, 2026-10-01): a
/// pale verdigris card with a 1 px line border, the dial drawn in the deep accent at
/// [dialOpacity].
///
/// * [background] / [border] — The card itself.
/// * [text] / [textSecondary] — Amount, and the label, notes and threshold text.
/// * [accent] — Shield icon, filled signer dots, dial rings, ticks and glow.
/// * [dialOpacity] — Multiplies the dial drawing's own alphas.
/// * [emptyDot] — Outline of a signer dot beyond the threshold.
/// * [ticker] — The currency ticker next to amounts: Zcash gold, the value colour.
/// * [chip] — Small buttons on the card (show/hide balance, full address).
/// * [inset] / [insetBorder] — The recipient box on the payment card.
class AppVaultCardColors {
  const AppVaultCardColors({
    required this.background,
    required this.border,
    required this.text,
    required this.textSecondary,
    required this.accent,
    required this.dialOpacity,
    required this.emptyDot,
    required this.ticker,
    required this.chip,
    required this.inset,
    required this.insetBorder,
  });

  final Color background;
  final Color border;
  final Color text;
  final Color textSecondary;
  final Color accent;
  final double dialOpacity;
  final Color emptyDot;
  final Color ticker;
  final Color chip;
  final Color inset;
  final Color insetBorder;

  static const dark = AppVaultCardColors(
    background: Color(0xFF091312),
    border: Color(0x1A51DDD2),
    text: Primitives.p800Dark,
    textSecondary: Color(0xB3F1F5F5),
    accent: BrandPrimitives.p400Dark,
    dialOpacity: 1,
    emptyDot: Color(0x66F1F5F5),
    ticker: GoldPrimitives.p500Dark,
    chip: Color(0x14FFFFFF),
    inset: Color(0x0DFFFFFF),
    insetBorder: Color(0x14FFFFFF),
  );

  static const light = AppVaultCardColors(
    background: Color(0xFFE0F3F1),
    border: Primitives.p150Light,
    text: Primitives.p900Light,
    textSecondary: Primitives.p600Light,
    accent: BrandPrimitives.p300Light,
    dialOpacity: 0.7,
    emptyDot: Primitives.p500Light,
    ticker: GoldPrimitives.p400Light,
    chip: Color(0x0F090E0E),
    inset: Color(0x99FFFFFF),
    insetBorder: Primitives.p150Light,
  );
}
