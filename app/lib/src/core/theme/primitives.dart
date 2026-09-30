import 'package:flutter/painting.dart';

/// Raw color primitives from the Zcash design system Figma spec.
///
/// 12-step neutral ladder. Each step has a dark-mode face (`*Dark`) and a
/// light-mode face (`*Light`). Semantic tokens under `colors/` pick the
/// appropriate face per mode — they do **not** always share the same
/// primitive step across modes (e.g. `border.subtle` uses `p150Dark` and
/// `p150Light`, while text tokens reach opposite ends of the ladder).
///
/// Values come from the Figma Dark/Light token JSON exports after the file
/// was converted from Display P3 to sRGB with "Keep appearance", so the
/// hex values are the sRGB approximations that preserve the visual intent
/// of the original P3 design.
///
/// Widgets must never reference these directly. Route through the semantic
/// categories in [AppColors] so roles stay decoupled from the palette.
abstract final class Primitives {
  // Primitive/0 — darkest anchor / inverse of lightest.
  static const p0Dark = Color(0xFF0B0E14);
  static const p0Light = Color(0xFFFFFFFF);

  // Primitive/50 — base surface.
  static const p50Dark = Color(0xFF11151D);
  static const p50Light = Color(0xFFF3F5F9);

  // Primitive/100 — raised surface.
  static const p100Dark = Color(0xFF181D27);
  static const p100Light = Color(0xFFE8ECF2);

  // Primitive/150 — overlay / accent surface.
  static const p150Dark = Color(0xFF202633);
  static const p150Light = Color(0xFFDDE2EA);

  // Primitive/200 — subtle border.
  static const p200Dark = Color(0xFF2A3140);
  static const p200Light = Color(0xFFCDD3DD);

  // Primitive/300 — default border / disabled icon.
  static const p300Dark = Color(0xFF3A4254);
  static const p300Light = Color(0xFFAEB5C2);

  // Primitive/400 — strong border / disabled text.
  static const p400Dark = Color(0xFF545D70);
  static const p400Light = Color(0xFF8C94A3);

  // Primitive/500 — mid-gray. Identical in both modes by design.
  static const p500Dark = Color(0xFF7C8496);
  static const p500Light = Color(0xFF7C8496);

  // Primitive/600 — secondary text.
  static const p600Dark = Color(0xFFA2A9B8);
  static const p600Light = Color(0xFF5A6273);

  // Primitive/700 — primary text.
  static const p700Dark = Color(0xFFC9CEDA);
  static const p700Light = Color(0xFF3F4757);

  // Primitive/800 — accent / primary button fill.
  static const p800Dark = Color(0xFFF2F4F8);
  static const p800Light = Color(0xFF161C28);

  // Primitive/900 — lightest / inverse of ground.
  static const p900Dark = Color(0xFFFFFFFF);
  static const p900Light = Color(0xFF0B0E14);

  // Primitive/Gray/Alpha tokens. These are explicit Figma exports, not
  // derived at runtime, because a few semantic alpha tokens intentionally
  // point at different ladder steps per mode.
  static const p0Alpha0Dark = Color(0x000B0E14);
  static const p0Alpha0Light = Color(0x00FFFFFF);

  static const p0Alpha5Dark = Color(0x0D0B0E14);
  static const p0Alpha5Light = Color(0x0DFFFFFF);

  static const p0Alpha10Dark = Color(0x1A0B0E14);
  static const p0Alpha10Light = Color(0x1AFFFFFF);

  static const p0Alpha15Dark = Color(0x260B0E14);
  static const p0Alpha15Light = Color(0x26FFFFFF);

  static const p0Alpha30Dark = Color(0x4D0B0E14);
  static const p0Alpha30Light = Color(0x4DFFFFFF);

  static const p0Alpha50Dark = Color(0x800B0E14);
  static const p0Alpha50Light = Color(0x80FFFFFF);

  static const p150Alpha15Dark = Color(0x26202633);
  static const p150Alpha15Light = Color(0x26DDE2EA);

  static const p300Alpha10Dark = Color(0x1A3A4254);
  static const p300Alpha10Light = Color(0x1AAEB5C2);

  static const p300Alpha15Dark = Color(0x263A4254);
  static const p300Alpha15Light = Color(0x26AEB5C2);

  static const p300Alpha20Dark = Color(0x333A4254);
  static const p300Alpha20Light = Color(0x33AEB5C2);

  static const p300Alpha35Dark = Color(0x593A4254);
  static const p300Alpha35Light = Color(0x59AEB5C2);

  static const p300Alpha50Dark = Color(0x803A4254);
  static const p300Alpha50Light = Color(0x80AEB5C2);

  static const p400Alpha20Dark = Color(0x33545D70);
  static const p400Alpha20Light = Color(0x338C94A3);

  static const p400Alpha35Dark = Color(0x59545D70);
  static const p400Alpha35Light = Color(0x598C94A3);

  static const p500Alpha50Dark = Color(0x807C8496);
  static const p500Alpha50Light = Color(0x807C8496);

  static const p700Alpha50Dark = Color(0x80C9CEDA);
  static const p700Alpha50Light = Color(0x803F4757);

  static const p900Alpha5Dark = Color(0x0DFFFFFF);
  static const p900Alpha5Light = Color(0x0D0B0E14);

  static const p900Alpha10Dark = Color(0x1AFFFFFF);
  static const p900Alpha10Light = Color(0x1A0B0E14);

  static const p900Alpha20Dark = Color(0x33FFFFFF);
  static const p900Alpha20Light = Color(0x330B0E14);

  static const p900Alpha50Dark = Color(0x80FFFFFF);
  static const p900Alpha50Light = Color(0x800B0E14);
}

/// Zafe's brand ladder: jade. Used by primary buttons, brand text/icons,
/// shielded-address feedback and focus rings.
abstract final class BrandPrimitives {
  static const p0Dark = Color(0xFF03110F);
  static const p0Light = Color(0xFFECFBF7);

  static const p50Dark = Color(0xFF06201C);
  static const p50Light = Color(0xFFC6F2E8);

  static const p100Dark = Color(0xFF0B332C);
  static const p100Light = Color(0xFF9BE7D6);

  static const p150Dark = Color(0xFF0F4A40);
  static const p150Light = Color(0xFF72DCC5);

  static const p200Dark = Color(0xFF136356);
  static const p200Light = Color(0xFF3FB89E);

  static const p300Dark = Color(0xFF178270);
  static const p300Light = Color(0xFF0F8C76);

  static const p400Dark = Color(0xFF2EC4A6);
  static const p400Light = Color(0xFF0B6E5E);

  static const p500Dark = Color(0xFF4FD2B6);
  static const p500Light = Color(0xFF095749);

  static const p600Dark = Color(0xFF72DCC5);
  static const p600Light = Color(0xFF074238);

  static const p700Dark = Color(0xFF9BE7D6);
  static const p700Light = Color(0xFF052E27);

  static const p800Dark = Color(0xFFC6F2E8);
  static const p800Light = Color(0xFF031C18);

  static const p900Dark = Color(0xFFECFBF7);
  static const p900Light = Color(0xFF020F0D);

  static const p300Alpha10Dark = Color(0x1A178270);
  static const p300Alpha10Light = Color(0x1A0F8C76);

  static const p300Alpha15Dark = Color(0x26178270);
  static const p300Alpha15Light = Color(0x260F8C76);

  static const p300Alpha25Dark = Color(0x40178270);
  static const p300Alpha25Light = Color(0x400F8C76);

  static const p300Alpha35Dark = Color(0x59178270);
  static const p300Alpha35Light = Color(0x590F8C76);

  static const p400Alpha15Dark = Color(0x262EC4A6);
  static const p400Alpha15Light = Color(0x260B6E5E);

  static const p400Alpha25Dark = Color(0x402EC4A6);
  static const p400Alpha25Light = Color(0x400B6E5E);

  static const p400Alpha35Dark = Color(0x592EC4A6);
  static const p400Alpha35Light = Color(0x590B6E5E);
}

/// Utility plum primitive ladder.
///
/// Used for destructive actions and validation errors.
abstract final class PlumPrimitives {
  static const p0Dark = Color(0xFF0B060D);
  static const p0Light = Color(0xFFF6ECF9);

  static const p50Dark = Color(0xFF2F133A);
  static const p50Light = Color(0xFFE6C5EC);

  static const p100Dark = Color(0xFF4E205F);
  static const p100Light = Color(0xFFCD8CD9);

  static const p150Dark = Color(0xFF5E2673);
  static const p150Light = Color(0xFFC06ECE);

  static const p200Dark = Color(0xFF772E89);
  static const p200Light = Color(0xFFB85BC8);

  static const p300Dark = Color(0xFF9338A7);
  static const p300Light = Color(0xFFAB40BF);

  static const p400Dark = Color(0xFFAB40BF);
  static const p400Light = Color(0xFF9338A7);

  static const p500Dark = Color(0xFFB85BC8);
  static const p500Light = Color(0xFF772E89);

  static const p600Dark = Color(0xFFC06ECE);
  static const p600Light = Color(0xFF5E2673);

  static const p700Dark = Color(0xFFCD8CD9);
  static const p700Light = Color(0xFF4E205F);

  static const p800Dark = Color(0xFFE6C5EC);
  static const p800Light = Color(0xFF2F133A);

  static const p900Dark = Color(0xFFF6ECF9);
  static const p900Light = Color(0xFF0C050E);

  static const p400Alpha4Dark = Color(0x0AAB40BF);
  static const p400Alpha4Light = Color(0x0A9338A7);

  static const p400Alpha8Dark = Color(0x14AB40BF);
  static const p400Alpha8Light = Color(0x149338A7);

  static const p400Alpha15Dark = Color(0x26AB40BF);
  static const p400Alpha15Light = Color(0x269338A7);

  static const p400Alpha25Dark = Color(0x40AB40BF);
  static const p400Alpha25Light = Color(0x409338A7);
}

/// Utility gold primitive ladder.
///
/// Used by the current Figma success/warning utility tokens.
abstract final class GoldPrimitives {
  static const p0Dark = Color(0xFF0E0905);
  static const p0Light = Color(0xFFFCF8F2);

  static const p50Dark = Color(0xFF180F08);
  static const p50Light = Color(0xFFF8EDDA);

  static const p100Dark = Color(0xFF241810);
  static const p100Light = Color(0xFFF2DCB8);

  static const p150Dark = Color(0xFF36251A);
  static const p150Light = Color(0xFFEAC890);

  static const p200Dark = Color(0xFF4D3624);
  static const p200Light = Color(0xFFDDB37A);

  static const p300Dark = Color(0xFF6E4E33);
  static const p300Light = Color(0xFFCD9F64);

  static const p400Dark = Color(0xFF956B45);
  static const p400Light = Color(0xFFB0844F);

  static const p500Dark = Color(0xFFCD9F64);
  static const p500Light = Color(0xFF956B45);

  static const p600Dark = Color(0xFFDDB37A);
  static const p600Light = Color(0xFF6E4E33);

  static const p700Dark = Color(0xFFEAC890);
  static const p700Light = Color(0xFF4D3624);

  static const p800Dark = Color(0xFFF5E0B8);
  static const p800Light = Color(0xFF241810);

  static const p900Dark = Color(0xFFFCF8F2);
  static const p900Light = Color(0xFF0E0905);

  static const p300Alpha15Dark = Color(0x266E4E33);
  static const p300Alpha15Light = Color(0x26CD9F64);

  static const p300Alpha25Dark = Color(0x406E4E33);
  static const p300Alpha25Light = Color(0x40CD9F64);

  static const p300Alpha35Dark = Color(0x596E4E33);
  static const p300Alpha35Light = Color(0x59CD9F64);

  static const p400Alpha15Dark = Color(0x26956B45);
  static const p400Alpha15Light = Color(0x26B0844F);

  static const p400Alpha25Dark = Color(0x40956B45);
  static const p400Alpha25Light = Color(0x40B0844F);

  static const p400Alpha35Dark = Color(0x59956B45);
  static const p400Alpha35Light = Color(0x59B0844F);
}

/// Utility green primitive ladder.
///
/// Reserved for positive states that need an explicitly green affordance.
abstract final class GreenPrimitives {
  static const p0Dark = Color(0xFF001E0A);
  static const p0Light = Color(0xFFD3FFE4);

  static const p50Dark = Color(0xFF023A21);
  static const p50Light = Color(0xFFC2F5D5);

  static const p100Dark = Color(0xFF005B35);
  static const p100Light = Color(0xFFA9ECC4);

  static const p150Dark = Color(0xFF007F49);
  static const p150Light = Color(0xFF89E5B0);

  static const p200Dark = Color(0xFF00A460);
  static const p200Light = Color(0xFF64DD9C);

  static const p300Dark = Color(0xFF0DC87D);
  static const p300Light = Color(0xFF3BD38B);

  static const p400Dark = Color(0xFF3BD38B);
  static const p400Light = Color(0xFF0DC87D);

  static const p500Dark = Color(0xFF64DD9C);
  static const p500Light = Color(0xFF00A460);

  static const p600Dark = Color(0xFF89E5B0);
  static const p600Light = Color(0xFF007F49);

  static const p700Dark = Color(0xFFA9ECC4);
  static const p700Light = Color(0xFF005B35);

  static const p800Dark = Color(0xFFC2F5D5);
  static const p800Light = Color(0xFF023A21);

  static const p900Dark = Color(0xFFD3FFE4);
  static const p900Light = Color(0xFF001E0A);

  static const p300Alpha15Dark = Color(0x260DC87D);
  static const p300Alpha15Light = Color(0x263BD38B);

  static const p300Alpha25Dark = Color(0x400DC87D);
  static const p300Alpha25Light = Color(0x403BD38B);

  static const p400Alpha15Dark = Color(0x263BD38B);
  static const p400Alpha15Light = Color(0x260DC87D);

  static const p400Alpha25Dark = Color(0x403BD38B);
  static const p400Alpha25Light = Color(0x400DC87D);

  static const p900Alpha50Dark = Color(0x80D3FFE4);
  static const p900Alpha50Light = Color(0x80001E0A);

  static const p900Alpha65Dark = Color(0xA6D3FFE4);
  static const p900Alpha65Light = Color(0xA6001E0A);
}
