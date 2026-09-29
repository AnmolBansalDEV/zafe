# Vizor wallet reference (for building Zafe)

Upstream: **`github.com/chainapsis/vizor-wallet`** (the Keplr team; Apache-2.0). This document was written from an older snapshot (`ff02152`, via a stale `valargroup` mirror); upstream has since moved on (checked at `4bff2e7`: Ironwood, Tor/network privacy, Ledger, payment links). Tokens and components changed only additively (new utility colors, extra `AppButton` options). Re-check upstream before copying anything new. All paths below are relative to that repo root unless stated otherwise.

Vizor's `AGENTS.md` covers architecture and sync. This document covers what it leaves out: exact design tokens, components, mobile screen layouts, the Keystone/PCZT UX, Dart state patterns, flutter_rust_bridge wiring, copy rules and licensing. Everything was read from source. Where the design screenshots disagree with the code, the code is treated as authoritative.

Contents:
1. Design tokens
2. Fonts and assets
3. Component library
4. Mobile shell, navigation and screens
5. Keystone / external-signer UX
6. State and data flow in Dart
7. flutter_rust_bridge setup
8. Copy conventions
9. What to copy vs. adapt for Zafe
10. Licensing

---

## 1. Design tokens

All tokens live in `lib/src/core/theme/`. The barrel file `app_theme.dart` exports everything. Widgets read colors with `context.colors.<group>.<token>` and read typography, spacing and radii as static consts (`AppTypography.bodyMedium`, `AppSpacing.sm`, `AppRadii.large`).

### 1.1 Architecture of the token system

| Layer | File | Notes |
|---|---|---|
| Primitives | `theme/primitives.dart` | Raw ladders: `Primitives` (neutral 0–900), `CrimsonPrimitives`, `PlumPrimitives`, `GoldPrimitives`, `GreenPrimitives`. Each step has `pNNNDark` and `pNNNLight` faces, plus explicit alpha variants (`p300Alpha20Dark`, and so on). The comment says "Widgets must never reference these directly." |
| Semantic groups | `theme/colors/app_*_colors.dart` | 12 classes, each with `static const dark` and `static const light`. |
| Aggregate | `colors/app_colors.dart` | `AppColors{background, surface, border, text, icon, button, state, fade, navPanel, shadows, sync, macosUtility}` with `AppColors.dark` / `AppColors.light`. |
| Theme data | `app_theme_data.dart` | `AppThemeData{colors}` with `.dark` / `.light` consts. |
| Inherited widget | `app_theme.dart` | `AppTheme extends InheritedWidget`. `AppTheme.of(context)`. Extension `AppThemeX on BuildContext { appTheme, colors }`. |
| Host | `app_theme_host.dart` | `AppThemeHost(themeMode, child)` resolves `ThemeMode` against platform brightness into `AppTheme(data: AppThemeData.dark|light)`. It also syncs native chrome (see below). |
| Legacy | `legacy_material_theme.dart` | A Material 3 `ThemeData` (Inter body font) passed to `MaterialApp.theme/darkTheme` for stray Material widgets only. New UI never reads it. |

Colors are the same on both form factors. Typography and component sizing differ between Desktop and Mobile (1.4).

`AppThemeHost` side effects:
- **Android:** `SystemChrome.setSystemUIOverlayStyle`. Status and navigation bars are painted `background.window`, with icon brightness inverted from the theme and `systemStatusBarContrastEnforced: false` / `systemNavigationBarContrastEnforced: false`. On Android 15+ edge-to-edge, only the icon brightness takes effect.
- **iOS and macOS:** `MethodChannel('com.zcash.wallet/window_appearance').invokeMethod('setBrightness', {'brightness': 'system'|'dark'|'light'})`.

### 1.2 Primitive ladders (hex)

In the neutral ladder, the light-mode faces run in the reverse direction to the dark ones.

| Step | Dark | Light | Role hint |
|---|---|---|---|
| p0 | #141818 | #FFFFFF | darkest anchor / white |
| p50 | #1B1F1F | #F7F7F7 | base surface |
| p100 | #232828 | #EBEBEB | raised |
| p150 | #2D3232 | #E1E1E1 | overlay |
| p200 | #393E3E | #D4D4D4 | subtle border |
| p300 | #4D5252 | #B8B8B8 | border / disabled icon |
| p400 | #626767 | #9A9A9A | strong border / disabled text |
| p500 | #858686 | #858686 | mid grey (same in both modes) |
| p600 | #A3A4A4 | #626767 | secondary text |
| p700 | #C2C3C3 | #4D5252 | primary text |
| p800 | #F7F7F7 | #2E3232 | accent / primary button fill |
| p900 | #FFFFFF | #141818 | max contrast |

Brand and utility ladders. Each shows the Dark face; the Light face is the same list mirrored.

| Ladder | p0 | p50 | p100 | p150 | p200 | p300 | p400 | p500 | p600 | p700 | p800 | p900 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Crimson (brand) | #080305 | #19080F | #32111D | #4C192C | #6D243F | #862D4E | #A83861 | #B35074 | #BE6A88 | #CF92A8 | #E1B9C8 | #F5EBEE |
| Plum (destructive) | #0B060D | #2F133A | #4E205F | #5E2673 | #772E89 | #9338A7 | #AB40BF | #B85BC8 | #C06ECE | #CD8CD9 | #E6C5EC | #F6ECF9 |
| Gold (warning/"success") | #0E0905 | #180F08 | #241810 | #36251A | #4D3624 | #6E4E33 | #956B45 | #CD9F64 | #DDB37A | #EAC890 | #F5E0B8 | #FCF8F2 |
| Green (positive) | #001E0A | #023A21 | #005B35 | #007F49 | #00A460 | #0DC87D | #3BD38B | #64DD9C | #89E5B0 | #A9ECC4 | #C2F5D5 | #D3FFE4 |

The key brand color is **crimson #A83861**: `CrimsonPrimitives.p400Dark` is the same value as `p300Light`. Destructive is **plum**, not red. The "success" text token is **gold**. Positive amounts use **green** (`text.positiveStrong`).

### 1.3 Semantic color tokens (resolved to hex)

The table below was generated by resolving every token in `colors/*.dart` against the primitives. `@N%` means alpha.


#### AppBackgroundColors

| Token | Dark | Light |
|---|---|---|
| `window` | #0F0F0F | #F7F7F7 |
| `ground` | #1B1F1F (p50Dark) | #FFFFFF (p0Light) |
| `base` | #232828 (p100Dark) | #F7F7F7 (p50Light) |
| `raised` | #2D3232 (p150Dark) | #EBEBEB (p100Light) |
| `overlay` | #393E3E (p200Dark) | #E1E1E1 (p150Light) |
| `inverse` | #F7F7F7 (p800Dark) | #2E3232 (p800Light) |
| `neutralScrim` | #141818 @50% (p0Alpha50Dark) | #141818 @50% (p900Alpha50Light) |
| `neutralSubtleOpacity` | #626767 @20% (p400Alpha20Dark) | #B8B8B8 @20% (p300Alpha20Light) |
| `neutralStrongOpacity` | #4D5252 @50% (p300Alpha50Dark) | #B8B8B8 @35% (p300Alpha35Light) |
| `brandCrimsonSubtle` | #32111D (crimson.p100Dark) | #F6EBEF (crimson.p0Light) |
| `brandCrimsonStrong` | #A83861 (crimson.p400Dark) | #A83861 (crimson.p300Light) |
| `brandCrimsonAlpha` | #862D4E @35% (crimson.p300Alpha35Dark) | #A83861 @15% (crimson.p300Alpha15Light) |
| `utilityDestructiveSubtle` | #2F133A (plum.p50Dark) | #F6ECF9 (plum.p0Light) |
| `utilityDestructiveAlphaSubtle` | #AB40BF @8% (plum.p400Alpha8Dark) | #9338A7 @8% (plum.p400Alpha8Light) |
| `utilityDestructiveAlpha` | #AB40BF @25% (plum.p400Alpha25Dark) | #9338A7 @15% (plum.p400Alpha15Light) |
| `utilitySuccessSubtle` | #36251A (gold.p150Dark) | #F8EDDA (gold.p50Light) |
| `utilitySuccessStrong` | #CD9F64 (gold.p500Dark) | #CD9F64 (gold.p300Light) |
| `utilitySuccessAlpha` | #0DC87D @15% (green.p300Alpha15Dark) | #3BD38B @15% (green.p300Alpha15Light) |
| `homeCard` | #1B1F1F (p50Dark) | #2E3232 (p800Light) |

#### AppBorderColors

| Token | Dark | Light |
|---|---|---|
| `subtle` | #2D3232 (p150Dark) | #E1E1E1 (p150Light) |
| `subtleOpacity` | #FFFFFF @10% (p900Alpha10Dark) | #141818 @5% (p900Alpha5Light) |
| `inverseOpacity` | #2D3232 @15% (p150Alpha15Dark) | #FFFFFF @10% (p0Alpha10Light) |
| `regular` | #4D5252 (p300Dark) | #D4D4D4 (p200Light) |
| `medium` | #626767 (p400Dark) | #B8B8B8 (p300Light) |
| `strong` | #F7F7F7 (p800Dark) | #141818 (p900Light) |
| `utilityDestructive` | #AB40BF (plum.p400Dark) | #AB40BF (plum.p300Light) |
| `utilityDestructiveSubtle` | #4E205F (plum.p100Dark) | #CD8CD9 (plum.p100Light) |
| `utilitySuccess` | #CD9F64 (gold.p500Dark) | #B0844F (gold.p400Light) |
| `brandCrimsonStrong` | #A83861 (crimson.p400Dark) | #A83861 (crimson.p300Light) |

#### AppPrimaryButtonColors

| Token | Dark | Light |
|---|---|---|
| `bg` | #F7F7F7 (p800Dark) | #2E3232 (p800Light) |
| `bgHover` | #862D4E (crimson.p300Dark) | #862D4E (crimson.p400Light) |
| `bgPressed` | #862D4E (crimson.p300Dark) | #862D4E (crimson.p400Light) |
| `border` | #2D3232 @15% (p150Alpha15Dark) | #FFFFFF @10% (p0Alpha10Light) |
| `borderHover` | #FFFFFF @10% (p900Alpha10Dark) | #141818 @5% (p900Alpha5Light) |
| `borderPressed` | #FFFFFF @10% (p900Alpha10Dark) | #141818 @5% (p900Alpha5Light) |
| `label` | #1B1F1F (p50Dark) | #EBEBEB (p100Light) |
| `labelHover` | #F7F7F7 (p800Dark) | #EBEBEB (p100Light) |

#### AppSecondaryButtonColors

| Token | Dark | Light |
|---|---|---|
| `bg` | #2D3232 (p150Dark) | #FFFFFF (p0Light) |
| `bgHover` | #393E3E (p200Dark) | #EBEBEB (p100Light) |
| `bgPressed` | #393E3E (p200Dark) | #EBEBEB (p100Light) |
| `label` | #F7F7F7 (p800Dark) | #141818 (p900Light) |

#### AppGhostButtonColors

| Token | Dark | Light |
|---|---|---|
| `bg` | #141818 (p0Dark) | #FFFFFF (p0Light) |
| `bgHover` | #2D3232 (p150Dark) | #EBEBEB (p100Light) |
| `border` | #4D5252 (p300Dark) | #B8B8B8 (p300Light) |
| `label` | #C2C3C3 (p700Dark) | #2E3232 (p800Light) |

#### AppDisabledButtonColors

| Token | Dark | Light |
|---|---|---|
| `bg` | #4D5252 @20% (p300Alpha20Dark) | #B8B8B8 @20% (p300Alpha20Light) |
| `label` | #858686 @50% (p500Alpha50Dark) | #858686 @50% (p500Alpha50Light) |

#### AppDestructiveButtonColors

| Token | Dark | Light |
|---|---|---|
| `bg` | #772E89 (plum.p200Dark) | #772E89 (plum.p500Light) |
| `bgHover` | #5E2673 (plum.p150Dark) | #5E2673 (plum.p600Light) |
| `bgPressed` | #5E2673 (plum.p150Dark) | #5E2673 (plum.p600Light) |
| `border` | #FFFFFF @10% (p900Alpha10Dark) | #141818 @5% (p900Alpha5Light) |
| `borderHover` | #FFFFFF @10% (p900Alpha10Dark) | #141818 @5% (p900Alpha5Light) |
| `borderPressed` | #FFFFFF @10% (p900Alpha10Dark) | #141818 @5% (p900Alpha5Light) |
| `label` | #E6C5EC (plum.p800Dark) | #E6C5EC (plum.p50Light) |

#### AppFadeColors

| Token | Dark | Light |
|---|---|---|
| `illustration` | #141818 @50% (p0Alpha50Dark) | #141818 @0% (p0Alpha0Dark) |

#### AppIconColors

| Token | Dark | Light |
|---|---|---|
| `accent` | #F7F7F7 (p800Dark) | #141818 (p900Light) |
| `regular` | #C2C3C3 (p700Dark) | #4D5252 (p700Light) |
| `muted` | #858686 (p500Dark) | #858686 (p500Light) |
| `disabled` | #4D5252 (p300Dark) | #B8B8B8 (p300Light) |
| `inverse` | #141818 (p0Dark) | #FFFFFF (p0Light) |
| `onPrimary` | #141818 (p0Dark) | #FFFFFF (p0Light) |
| `warning` | #CD9F64 (gold.p500Dark) | #CD9F64 (gold.p300Light) |
| `destructive` | #AB40BF (plum.p400Dark) | #AB40BF (plum.p300Light) |
| `destructiveLight` | #9338A7 (plum.p300Dark) | #B85BC8 (plum.p200Light) |
| `success` | #0DC87D (green.p300Dark) | #00A460 (green.p500Light) |
| `brandCrimson` | #A83861 (crimson.p400Dark) | #A83861 (crimson.p300Light) |

#### AppMacosUtilityColors

| Token | Dark | Light |
|---|---|---|
| `window` | #0F0F0F | #F7F7F7 |
| `windowTransparent` | #0F0F0F @0% | #F5F5F5 @0% |
| `navPanel` | #1A1A1A @30% | #FFFFFF @30% |
| `font` | #FFFFFF @80% | #1A1A1A @85% |
| `thinBorder` | #1A1A1A @23% | #FFFFFF @55% |
| `innerBorder` | #FFFFFF @15% | #FFFFFF @15% |

#### AppNavPanelColors

| Token | Dark | Light |
|---|---|---|
| `badgeBg` | #862D4E (crimson.p300Dark) | #A83861 (crimson.p300Light) |
| `activeBg` | #A83861 @15% (crimson.p400Alpha15Dark) | #A83861 @10% (crimson.p300Alpha10Light) |
| `activeIcon` | #A83861 (crimson.p400Dark) | #A83861 (crimson.p300Light) |
| `activeLabel` | #F5EBEE (crimson.p900Dark) | #19080F (crimson.p800Light) |

#### AppShadowColors

| Token | Dark | Light |
|---|---|---|
| `shadow1` | #141818 @0% (p0Alpha0Dark) | #E1E1E1 (p150Light) |
| `shadow2` | #141818 @0% (p0Alpha0Dark) | #B8B8B8 (p300Light) |
| `shadow3` | #141818 @0% (p0Alpha0Dark) | #141818 @20% (p900Alpha20Light) |
| `subtle` | #141818 @0% (p0Alpha0Dark) | #141818 @5% (p900Alpha5Light) |
| `regular` | #141818 @0% (p0Alpha0Dark) | #141818 @10% (p900Alpha10Light) |

#### AppStateColors

| Token | Dark | Light |
|---|---|---|
| `hover` | #232828 (p100Dark) | #F7F7F7 (p50Light) |
| `hoverOpacity` | #141818 @15% (p0Alpha15Dark) | #141818 @5% (p900Alpha5Light) |
| `pressed` | #2D3232 (p150Dark) | #E1E1E1 (p150Light) |
| `focus` | #393E3E (p200Dark) | #D4D4D4 (p200Light) |
| `selected` | #2D3232 (p150Dark) | #E1E1E1 (p150Light) |
| `selectedOpacity` | #141818 @30% (p0Alpha30Dark) | #141818 @5% (p900Alpha5Light) |
| `focusRing` | #F7F7F7 (p800Dark) | #141818 (p900Light) |
| `focusGap` | #141818 (p0Dark) | #FFFFFF (p0Light) |
| `focusRingBrand` | #A83861 (crimson.p400Dark) | #A83861 (crimson.p300Light) |
| `focusRingDestructive` | #772E89 (plum.p200Dark) | #9338A7 (plum.p400Light) |

#### AppSurfaceColors

| Token | Dark | Light |
|---|---|---|
| `card` | #232828 (p100Dark) | #F7F7F7 (p50Light) |
| `input` | #1B1F1F (p50Dark) | #FFFFFF (p0Light) |
| `inputFocus` | #232828 (p100Dark) | #F7F7F7 (p50Light) |
| `nav` | #1B1F1F (p50Dark) | #FFFFFF (p0Light) |
| `navActive` | #2D3232 (p150Dark) | #EBEBEB (p100Light) |
| `tooltip` | #393E3E (p200Dark) | #2E3232 (p800Light) |
| `qrCode` | #FFFFFF (p0Light) | #FFFFFF (p0Light) |
| `scrollbarThumb` | #393E3E (p200Dark) | #E1E1E1 (p150Light) |

#### AppSyncColors

| Token | Dark | Light |
|---|---|---|
| `text` | #D3FFE4 (green.p900Dark) | #005B35 (green.p700Light) |
| `textSyncing` | #D3FFE4 @65% (green.p900Alpha65Dark) | #001E0A @65% (green.p900Alpha65Light) |
| `textError` | #FFFFFF @50% (p900Alpha50Dark) | #141818 @50% (p900Alpha50Light) |
| `glow` | #858686 (p500Dark) | #64DD9C (green.p200Light) |
| `lightSuccess` | #0DC87D (green.p300Dark) | #0DC87D (green.p400Light) |
| `lightError` | #A3A4A4 (p600Dark) | #858686 (p500Light) |

#### AppTextColors

| Token | Dark | Light |
|---|---|---|
| `accent` | #FFFFFF (p900Dark) | #141818 (p900Light) |
| `primary` | #C2C3C3 (p700Dark) | #4D5252 (p700Light) |
| `secondary` | #A3A4A4 (p600Dark) | #626767 (p600Light) |
| `muted` | #858686 (p500Dark) | #858686 (p500Light) |
| `disabled` | #626767 (p400Dark) | #9A9A9A (p400Light) |
| `inverse` | #141818 (p0Dark) | #FFFFFF (p0Light) |
| `warning` | #CD9F64 (gold.p500Dark) | #B0844F (gold.p400Light) |
| `positiveStrong` | #3BD38B (green.p400Dark) | #00A460 (green.p500Light) |
| `destructive` | #B85BC8 (plum.p500Dark) | #AB40BF (plum.p300Light) |
| `destructiveLight` | #AB40BF (plum.p400Dark) | #C06ECE (plum.p150Light) |
| `success` | #CD9F64 (gold.p500Dark) | #B0844F (gold.p400Light) |
| `brandCrimson` | #A83861 (crimson.p400Dark) | #A83861 (crimson.p300Light) |
| `homeCard` | #F7F7F7 (p800Dark) | #FFFFFF (p0Light) |

**Semantic roles worth internalizing:**
- **Surfaces.** Screen background is `background.window` (#0F0F0F dark / #F7F7F7 light). Cards are `background.ground` (#1B1F1F / white). Sheets are `background.base`. Segmented-control tracks and unselected option fills are `background.raised`. Progress tracks are `background.overlay`.
- **Primary button.** The primary button is an inverse neutral pill (near-white in dark mode, #2E3232 in light) whose hover/pressed fill turns **crimson #862D4E**. Crimson is used sparingly: shield icons, focus rings, active tab tint, passcode dots, address-chunk highlights.
- **Home card.** `background.homeCard` is a dark card in *both* themes (#1B1F1F dark / #2E3232 light) with `text.homeCard` light text. It is used for the balance card, seed card, onboarding info cards and the "Create wallet" method card.
- **Shadows.** All `shadows.*` are alpha 0 in dark mode, so raised surfaces lose their shadow automatically.
- **Scrim.** `background.neutralScrim` = #141818 @ 50% in both modes. It is the barrier color for all sheets.
- **Icon disc.** `background.neutralSubtleOpacity` is the standard fill for 40px icon discs, passcode keys and empty radios.

### 1.4 Typography

The Figma `Fonts` collection has Desktop and Mobile modes. Both are materialized as full const sets, `AppTypographyDesktop` and `AppTypographyMobile`, in `theme/app_typography.dart`. `AppTypography.*` picks one at compile time. Line heights are stored as `height = figmaLineHeightPx / fontSize`. Colors are never baked in: callers use `.copyWith(color: ...)`.

| Token (`AppTypography.`) | Figma name | Family / weight | Mobile size / line / tracking | Desktop size / line / tracking |
|---|---|---|---|---|
| `displayLarge` (= `displayMedium`) | Headline XL | Young Serif 400 | 40 / 40 / −1.35 | 45 / 48 / −1.35 |
| `displaySmall` = `headlineLarge` | Headline L | Young Serif 400 | 32 / 33 / 0 | 32 / 33 / 0 |
| `headlineMedium` | Headline M | Young Serif 400 | 28 / 30 / −0.28 | 28 / 30 / −0.28 |
| `headlineSmall` | Headline S | Geist 500 | 18 / 22 / 0 | 16 / 20 / 0 |
| `bodyLarge` | Body L | Geist 500 | 18 / 26 / −0.24 | 16 / 24 / −0.24 |
| `bodyMedium` | Body M | Geist 400 | 16 / 25 / −0.21 | 14 / 21 / −0.21 |
| `bodyMediumStrong` | Body M Medium | Geist 500 | 16 / 25 / −0.21 | 14 / 21 / −0.21 |
| `bodySmall` | Body S | Geist 400 | 14 / 20 / −0.12 | 12 / 18 / −0.12 |
| `bodyExtraSmall` | Body XS | Geist 400 | 13 / 18 / −0.055 | 11 / 16 / −0.055 |
| `labelLarge` | Label M | Geist 500 | 16 / 17 / −0.06 | 14 / 16 / −0.06 |
| `labelMedium` = `labelSmall` | Label S | Geist 500 | 14 / 15 / 0 | 13 / 14 / 0 |
| `codeMedium` | Code M | Geist Mono 500 | 16 / 21 / 0 | 14 / 21 / 0 |
| `codeSmall` | Code S | Geist Mono 500 | 13 / 17 / 0 | 13 / 17 / 0 |

Young Serif font features:
- Mobile serif styles set `fontFeatures: [FontFeature.liningFigures()]`.
- Desktop serif styles set `FontFeature.enable('case')`.
- Both exist because Young Serif defaults to old-style figures, which look wrong in balances.
- `appSerifDisplayStyle({required Color color})` in `app_serif_display.dart` returns `displaySmall.copyWith(color:)`.

Typography used directly on screens without a token:
- Home balance: Young Serif 45/48, −1.35, with a Young Serif 32 "ZEC" suffix.
- Send amount: `displayLarge` at fontSize 48, w500, line height 40px.
- Welcome hero: Young Serif 48, height 1.1, −1.35.
- Sheet titles: `bodyLarge` with `fontWeight: w600`.
- Account name in the top nav: `labelLarge` w600.

### 1.5 Spacing, radii, icon sizes (mode-invariant)

`AppSpacing` (`app_spacing.dart`). Note that `base` is 32; it is not the middle of the scale.

| xxs | xs | s | sm | md | base | lg | xl | xl2 | xl3 |
|---|---|---|---|---|---|---|---|---|---|
| 4 | 8 | 12 | 16 | 24 | 32 | 48 | 64 | 96 | 128 |

`AppRadii` (`app_radii.dart`): `xSmall 8`, `small 12`, `medium 16`, `large 24`, `xLarge 32`, `full 999` (prefer `StadiumBorder` for pills).
- Figma radius names are shifted one step against Dart: Figma SM/M/L = Dart medium/large/xLarge. Map by value, not by name.
- Common literal radii: `MobileSurfaceCard.defaultRadius = 20` (most screens override it to 24), 28 on some Keystone permission cards, and `AppWindowSizing.paneRadius = 20` (desktop).

`AppIconSize`: `medium 16`, `large 24`. Other icon sizes used in practice: 20 inside buttons and inputs on mobile, 28 in the tab bar, 18 inside 40px discs.

### 1.6 Component sizing (form-factor dependent, `app_sizing.dart`)

| Class | Field | Desktop | Mobile |
|---|---|---|---|
| `AppAssetSize` | size / icon / padding | 32 / 16 / 4 | 40 / 18 / 0 |
| `AppButtonSizing` | largeHeight | 44 | **50** |
| | mediumSmallIconSize | 16 | 20 |
| `AppInputSizing` | height | 46 | **60** |
| | iconWrapWidth / iconSize | 32 / 20 | 36 / 24 |
| | radius | 12 (`small`) | 16 (`medium`) |
| `AppWindowSizing` (desktop only) | min 1080×720, max 1296×864, content max width 420, pane radius 20 | | |

`AppProfilePictureSize`: medium 24, large 32, navLarge 40, xLarge 56, xxLarge 72.

Fixed chrome constants:

| Constant | Value |
|---|---|
| `kMobileTopNavHeight` | 72 |
| `kMobileTabBarHeight` | 64 |
| `MobileListRow.minHeight` | 52 (settings uses 44) |
| `kIosHomeIndicatorClearance` | 16 |

### 1.7 Shadows

`appSurfaceShadow(AppColors c)` (`app_shadows.dart`) has four layers, all using `c.shadows.subtle` (#141818 @ 5% light, transparent dark):

```dart
[BoxShadow(color: s, blurRadius: 1),
 BoxShadow(color: s, offset: Offset(0,1), blurRadius: 2),
 BoxShadow(color: s, offset: Offset(0,2), blurRadius: 4),
 BoxShadow(color: s, blurRadius: 1)]
```

This is used by text fields, `ReviewWrapCard` and the home balance card.

The modal card shadow (`_modalShadow` in `app_mobile_sheet.dart`, and the same in `AppModalCard`) is hard-coded black and theme-independent:

```dart
[BoxShadow(color: Color(0x14000000), offset: Offset(0,14), blurRadius: 28),
 BoxShadow(color: Color(0x08000000), offset: Offset(0,-6), blurRadius: 12),
 BoxShadow(color: Color(0x0F000000), offset: Offset(0,2),  blurRadius: 8)]
```

It is combined with an inner rim, painted by `_ModalInnerHighlightPainter`: a 1px stroke of `0x26FFFFFF` with `MaskFilter.blur(BlurStyle.inner, 2)`.

Tab bar shadow: a 1px spread ring in `macosUtility.thinBorder`, plus `BoxShadow(black @ 5%, offset (0,25), blur 25)`.

### 1.8 Motion

| Where | Duration / curve |
|---|---|
| Onboarding route transitions (`core/motion/onboarding_motion.dart`) | forward 320ms `easeOutCubic`; reverse 260ms `easeInCubic` (`kOnboardingForwardDuration`, `kOnboardingReverseDuration`, `kOnboardingForwardCurve`, `kOnboardingReverseCurve`) |
| `AppButton` fill/border change | 120ms `easeOut` (ghost: 0ms); focus ring fades in over 120ms |
| Tab bar selection pill + icon tint | 240ms, `Cubic(0.34, 1.40, 0.64, 1.0)` (easeOutBack with half the overshoot) |
| Tab icon pop on activate | 250ms, scale 1.0 → 1.12 (easeOut, 45%) → 1.0 (easeIn, 55%) |
| Tab icon press | scale 0.9; 90ms press (`easeOut`) / 180ms release (`easeOutBack`) |
| Sync status shimmer + edge-bar glow breathing | period 1400ms; glow blur 8→13, alpha 0.2→0.45, cosine breath |
| Receive segmented control indicator | 160ms `easeOut`; PageView `animateToPage` 220ms |
| Home transparent-balance strip | 280ms in / 240ms out, `easeOutCubic`, size + fade + slide |
| Skeleton shimmers | price bar 1200ms repeat; Keystone QR placeholder 1400ms repeat |
| Animated UR QR | 100ms per frame (10 fps) |
| Toast | visible for 2s (`AppToast.defaultDuration`); appears and disappears with no animation |
| Push routes (mobile) | `CupertinoPage`, the native iOS slide with edge-swipe back |
| Tab roots | `NoTransitionPage` |

Every animated widget checks `MediaQuery.maybeDisableAnimationsOf(context)` and snaps instantly under reduced motion.

### 1.9 Form factor selection (`core/layout/app_form_factor.dart`)

The form factor is a **build-time** constant. There is one binary per form factor, and unused tokens are tree-shaken.

```dart
enum AppFormFactor { desktop, mobile }

const String _formFactorDefine =
    String.fromEnvironment('VIZOR_FORM_FACTOR', defaultValue: 'desktop');

const AppFormFactor kAppFormFactor =
    _formFactorDefine == 'mobile' ? AppFormFactor.mobile : AppFormFactor.desktop;

bool get isDesktopLayoutPlatform =>
    !kIsWeb && (Platform.isMacOS || Platform.isWindows || Platform.isLinux);

// main(): assert(debugCheckFormFactorMatchesPlatform());
// throws StateError('... Rebuild with --dart-define=VIZOR_FORM_FACTOR=mobile.')

// Token selector pattern (app_typography.dart / app_sizing.dart):
const _mobile = kAppFormFactor == AppFormFactor.mobile;
abstract final class AppTypography {
  static const bodyMedium =
      _mobile ? AppTypographyMobile.bodyMedium : AppTypographyDesktop.bodyMedium;
  ...
}
```

Mobile builds need `--dart-define=VIZOR_FORM_FACTOR=mobile`. The router also branches on `kAppFormFactor` (section 6.2).

**For Zafe (mobile-only):** drop the Desktop sets and keep a single `AppTypography` / `AppSizing` with the mobile values. Keeping the `*Mobile` names costs nothing if a tablet or desktop build might come later.

---

## 2. Fonts and assets

### 2.1 Fonts (`assets/fonts/`, declared in `pubspec.yaml` `flutter.fonts`)

| Family | Files (weights) | Used for | License (from the font's embedded name table) |
|---|---|---|---|
| **Geist** | Regular 400, Medium 500, SemiBold 600, Bold 700 (v1.800) | All sans UI text: body, labels, buttons, `headlineSmall` | SIL OFL 1.1. "Copyright 2024 The Geist Project Authors (github.com/vercel/geist-font)" |
| **Geist Mono** | Regular 400, Medium 500 (v1.700) | `codeMedium` / `codeSmall`: mnemonic indices, txids, some address chips | SIL OFL 1.1, same authors |
| **Young Serif** | Regular 400 only (v6.002) | All display/headline serif: balances, amounts, screen titles in the top nav, onboarding titles, passcode digits | SIL OFL 1.1. "Copyright 2023 The Young Serif Project Authors (github.com/noirblancrouge/YoungSerif)" |
| **Inter** | 400/500/600/700 (v4.001) | Only `legacy_material_theme.dart` (the Material fallback theme). Effectively unused by the design system. | Embedded strings reference openfontlicense.org. "Copyright 2016 The Inter Project Authors (github.com/rsms/inter)". Inter is OFL 1.1. |

**The repo ships no `OFL.txt` license files for any of these fonts.** The OFL permits bundling but requires the copyright notice and license to accompany redistributed copies. When Zafe bundles these fonts, it should add `assets/fonts/OFL-Geist.txt`, `OFL-YoungSerif.txt` (and `OFL-Inter.txt` if used), and register them with `LicenseRegistry.addLicense` so they appear in `showLicensePage`. Zafe can also skip Inter entirely.

### 2.2 Icons (`assets/icons/`, 86 files)

Icons are monochrome single-fill SVGs on a 24×24 viewBox that keeps the Figma frame padding. `lib/src/core/widgets/app_icon.dart` defines them.

- `abstract final class AppIcons` holds string constants that map to `assets/icons/<name>.svg`. The set: `addNew, arrowBack, arrowBottomLeft, arrowDown, arrowDownCircle, arrowDownward, arrowForwardIos, arrowTopRight, arrowUpward, backspace, block, book, calendar, camera, cameraDenied, cancel, check, checkCircle, chevronBackward, chevronForward, collapsed, cog, coins, copy, cross, crystalBall, day, doubleArrowVertical, dragon, edit, editFilled, endpoint, eye, eyeClosed, expand, faceId, filter, github, globe, help, history, home, importWallet, key, keystone, link, loader, lock, logOut, monitor, night, options, plane, plus, qr, qrCodeFill, renew, scroll, search, share, shieldAsset, shieldKeyhole, shieldKeyholeOutline, skip, skull, swapArrows, sync, theme, time, transparentBalance, unlock, trash, user, users, uturnUp, vizor, wallet, warning, zcash, zcashCurrency`.
- `AppIcon(String name, {double size = 16, Color? color, bool animated = true, String? semanticLabel})` renders `SvgPicture.asset('assets/icons/$name.svg', colorFilter: ColorFilter.mode(color, BlendMode.srcIn))`.
- **Tint resolution order:** explicit `color`, then `IconTheme.of(context).color` (so `AppButton` tints its icons automatically), then `colors.icon.regular`.
- `AppIcons.loader` is code-drawn by `AppLoadingIcon` (`app_loading_icon.dart`): eight rounded spokes, with an opacity pulse stepping around them.
- Raster exceptions: `receive_qr_shield_crimson.png` (96×96, the crimson shield QR centre badge), `receive_qr_transparent_{light,dark}.png`, `network_zec.png`.
- The icon art is Vizor/Keplr's own work. It is Apache-2.0 as part of the repo, but distinctive glyphs like `shield_keyhole` and `vizor` are brand-adjacent.

Semantic icon choices to mirror:

| Meaning | Icon |
|---|---|
| Send | `plane` |
| Receive | `arrowDownCircle` |
| Shielded | `shieldKeyhole` (tinted crimson) |
| Transparent | `transparentBalance` (tinted `icon.muted`) |
| Tab: home | `home` |
| Tab: activity | `history` |
| Tab: settings | `cog` |
| Close | `cross` |
| Back | `chevronBackward` |
| Row chevron | `chevronForward` |
| Explorer link | `arrowTopRight` |
| Help | `help` |
| Fee/memo toggle | `doubleArrowVertical` |
| New shielded address | `renew` |

### 2.3 Illustrations and other raster assets

- **`assets/illustrations/`** (about 60 PNGs, most with `_light` / `_dark` variants): a hand-drawn **medieval knight** universe (engraving/comic style, crimson accents). Examples:
  - `mobile_welcome_hero.png` (786×1704): a knight lounging on a throne among treasure chests; bottom-anchored, black above.
  - `send_status_illustration_dark.png`: a knight flipping a ZEC coin.
  - Also: `home_rest_character.png`, `biometrics_faceid_knight.png`, `settings_backdrop_castle.png`, `method_*` cards, and the onboarding sidebars.
  - `home_balance_card_pattern_*.png` and `onboarding_card_pattern.png` are subtle texture overlays for the dark cards.
- **`assets/profile_pictures/`**: 15 avatar PNGs, named in settings as "Knight", "Viking", "Samurai", "Wizard" and so on.
- **`assets/swap/tokens/`** (109 files) and **`assets/swap/chains/`**: third-party token and chain logos.

**These illustrations are Vizor's brand identity.** Zafe should not reuse them (section 10), and they are the one part of "their UI" Zafe must replace. The layouts reserve explicit art slots:

| Slot | Size |
|---|---|
| Welcome hero | full-bleed, bottom-anchored |
| Home empty state | 340×220 canvas |
| Biometrics hero | 321 tall |
| Method-selection cards | 180×120 art on the right |
| Home importing background | 1080×720 |

Zafe can fill these with its own art or drop them.

---

## 3. Component library

Shared widgets live in `lib/src/core/widgets/` (mobile-specific ones in `widgets/mobile/`) and `lib/src/core/layout/mobile/`. Most are browsable in Widgetbook (`lib/widgetbook/*_use_cases.dart`; run with `fvm flutter run -t lib/widgetbook.dart --dart-define=VIZOR_FORM_FACTOR=mobile`).

### 3.1 Buttons: `AppButton` (`widgets/app_button.dart`)

```dart
AppButton({required VoidCallback? onPressed, required Widget child,
  AppButtonVariant variant = primary,   // primary | secondary | ghost | destructive
  AppButtonSize size = large,           // large | mediumLarge | medium | small
  double? height, EdgeInsets? contentPadding, Widget? leading, Widget? trailing,
  double? minWidth, double? iconGap, Color? focusRingColor,
  Color? disabledBackgroundColor, Color? enabledBorderColor,
  FocusNode? focusNode, bool autofocus = false,
  bool expand = false,            // true = fill parent width (mobile CTAs)
  bool constrainContent = false}) // allow label to ellipsize
```

- **Shape:** `StadiumBorder` pill. `onPressed == null` means disabled.
- **Fill priority:** disabled > pressed > hover > default.
- **Keyboard:** Space/Enter activate the button.

Sizes:

| Size | Height | Padding (h / v) | Icon | Gap | Label style |
|---|---|---|---|---|---|
| `large` | `AppButtonSizing.largeHeight` (50 mobile) | 16 / 8 | 20 | 4 | `labelLarge` |
| `mediumLarge` | 36 | 12 / 4 | 16 | 4 | `labelLarge` |
| `medium` | 32 | 8 / 4 | 20 mobile / 16 desktop | 4 | mobile `labelLarge`, desktop `labelMedium` |
| `small` | 24 | 4 / 0 | 20 mobile | 4 | `labelSmall` |

The label also gets an extra 4px of horizontal padding on every size except small.

Variants:

| Variant | bg / hover+pressed | Border | Label |
|---|---|---|---|
| primary | `button.primary.bg` (#F7F7F7 dark / #2E3232 light) → `bgHover` crimson #862D4E | 1.5px `button.primary.border` | `button.primary.label` |
| secondary | `button.secondary.bg` (#2D3232 / white) → `bgHover` | none | `button.secondary.label` |
| ghost | transparent → `button.ghost.bgHover` | none | `button.ghost.label` |
| destructive | plum #772E89 → #5E2673 | 1.5px | #E6C5EC |
| disabled (any) | `button.disabled.bg` (p300 @ 20%) | none | `button.disabled.label` (p500 @ 50%) |

- **Focus ring:** drawn outside the pill as a `StadiumBorder` with `strokeAlignOutside`. Width is 2 (1.5 on small). Outset is 3.5 for large primary and for destructive, 3 for other primary sizes, 2 for secondary/ghost.
- **Mobile usage:** primary CTAs are full width (`expand: true`) and pinned to the bottom with padding `(16, 0, 16, 12)`. A secondary/ghost "Cancel" sits 12 below.

Related small buttons:
- `ReviewButtonsStack` (`review_buttons_stack.dart`): primary + ghost, `minWidth` 196, 12 gap. This is the desktop review variant.
- `AppIconHoverButton` (`app_icon_hover_button.dart`): 32 circle icon button with a hover fill.
- `AppTappable` (`app_tappable.dart`): Semantics(button) + click cursor + opaque GestureDetector. Use it for any custom tap target.
- `_ModalCloseButton` (inside `app_mobile_sheet.dart`): a 32×32 circle filled `button.secondary.bg`, with `AppIcons.cross` 20 in `icon.accent`. Semantics "Close".
- Inline field pills ("Paste" / "Clear" / "Max"): 36h, `button.secondary.bg`, `AppRadii.full`, horizontal padding 12–16, `labelLarge` in `button.secondary.label`.

### 3.2 Inputs

**`MobileTextField`** (`widgets/mobile_text_field.dart`) is the canonical mobile input.

```dart
MobileTextField({required TextEditingController controller, required FocusNode focusNode,
  Key? fieldKey, String? hintText, onChanged, onSubmitted, textInputAction, keyboardType,
  inputFormatters, Widget? leading, Widget? trailing, Color? backgroundColor,
  Color? restingBorderColor, Color? focusedBorderColor, List<BoxShadow>? focusedBoxShadow,
  TextStyle? textStyle, TextStyle? hintStyle, double? height, double? radius})
```

Visual spec:
- Height 60 and radius 16 (from `AppInputSizing`), fill `surface.input` (#1B1F1F dark / white light).
- **No border at rest.** A 1.5px `background.inverse` border (`strokeAlignInside`) appears when focused.
- Surface shadow always. The inner `TextField` uses `InputDecoration.collapsed` with 12px horizontal padding.
- Text is `labelMedium` w500 in `text.accent`; hint is `labelMedium` in `text.muted`; cursor is `text.accent`.
- `leading` and `trailing` are laid out inside the row. A leading icon slot is `AppInputSizing.iconWrapWidth` (36) wide.
- Error state: pass `restingBorderColor` / `focusedBorderColor: colors.border.utilityDestructive`.
- **"Focus spotlight" trick** (send recipient): `focusedBoxShadow: [BoxShadow(color: neutralScrim, offset: (0,4), blurRadius: 4, spreadRadius: 1000)]` dims the whole screen around the focused field.

**`AppTextField`** (`widgets/app_text_field.dart`, 1044 lines) is the full-featured field. It is desktop-first but works on mobile.
- Label row (16h) with optional `rightLabel` / `rightSlot`, shell, and message row below.
- Tones: `AppTextFieldTone.neutral | destructive | success | brandCrimson`. Each changes the border, message color and default message icon (`warning` / `checkCircle` / `shieldKeyhole`).
- Surfaces: `AppTextFieldSurface.primary` (white `surface.input` + shadow) or `secondary` (filled `button.secondary.bg`, flat).
- Hover border is `border.subtleOpacity`; focus border is `background.inverse`.
- Multiline mode has a fixed 148 shell height and a custom 6px scrollbar.
- Also: clear button, `obscureText`.

Other inputs:
- `PasswordTextField` (`password_text_field.dart`).
- `CommaToDotInputFormatter` (`comma_to_dot_input_formatter.dart`).
- `ZecAmountInputFormatter` in `core/formatting/zec_amount.dart`: one dot, at most 8 decimals.

### 3.3 Cards and surfaces

| Widget | File | Spec |
|---|---|---|
| `MobileSurfaceCard({child, padding = all(16), cornerRadius = 20})` | `widgets/mobile/mobile_surface_card.dart` | `background.ground` fill, no shadow. Screens usually pass `cornerRadius: AppRadii.large` and `padding: EdgeInsets.fromLTRB(16, 32, 16, 32)`. |
| `ReviewWrapCard({children, surfaceColor})` + `ReviewWrapDivider` | `widgets/review_wrap_card.dart` | Full width, radius 24, `ground`, `appSurfaceShadow`, padding v24 / h16, children 16 apart. Divider is 1px `border.regular`. |
| `AppModalCard` / `AppModalActions` | `widgets/app_modal_card.dart` | Desktop modal card, radius 24, modal shadow. |
| Home balance card | private `_BalanceCard` in `mobile_home_screen.dart` | See 4.4. |
| Option / radio card | private in theme sheet, Keystone select-account, endpoint | Height 64, `ground`, radius 16. Border 1.5 `border.strong` when selected, else 1 `border.subtle` (or none). Leading icon 20 (50% opacity when unselected). Label `bodyMediumStrong`. 24px radio on the right: selected = `background.inverse` circle with `check` 14; unselected = `raised` / `neutralSubtleOpacity` circle. |

### 3.4 List rows

- **`MobileListRow`** (`widgets/mobile/mobile_list_row.dart`)
  - Constructor: `MobileListRow({required String label, Widget? leading, String? value, Widget? trailing, bool showChevron = false, VoidCallback? onTap, bool enabled = true, Color? labelColor, valueColor, chevronColor, double minRowHeight = 52, TextStyle? textStyle, valueTextStyle})`.
  - Layout: leading, 12 gap, label (`bodyMedium`, `text.accent`), then either an expanded right-aligned value (`text.secondary`, ellipsis) or an expanded label. Then 8 gap and either `trailing` or `chevronForward` 16 (`icon.muted`).
  - Disabled rows use `text.disabled` / `icon.disabled` and ignore taps.
  - Settings rows override `minRowHeight: 44`, use `labelLarge` w400, value in `text.accent`, and chevron in `icon.accent`.
- **Activity row** `ActivityFeedRow` (`features/activity/widgets/activity_feed.dart`): see 4.7.
- **`ReviewListRow`** (`widgets/review_list_row.dart`): the 32px "List Item" row used in review/status detail cards. Left label; right value cluster with optional 16px leading/trailing icons (status icon, expand, explorer link, fee help).
- **`ReviewInfoRow`** (desktop) / **`MobileReviewInfoRow`** (`widgets/mobile/mobile_review_row.dart`)
  - The 90px min-height "serif review row": 40px leading slot, 16 gap, then a column:
    - label: 24h, `labelMedium`, `text.secondary`
    - 4 gap
    - value: `headlineLarge` Young Serif 32, `text.accent`, one line, optional strikethrough
    - 4 gap
    - 24h bottom strip (pool badge, address, "Show full address" action)
  - Companions:
    - `MobileReviewFlowArrow`: 40-wide column with `arrowDown` 24 in `icon.accent`.
    - `MobileReviewZecBadge`: 32px round ZEC coin.
    - `MobileReviewIconBadge`: 40px circle in `neutralSubtleOpacity`.

### 3.5 Sheets and modals (`core/layout/mobile/app_mobile_sheet.dart`)

- `Future<T?> showAppMobileSheet<T>({required context, required WidgetBuilder builder, bool isDismissible = true, bool transparentBackground = false})`
  - Wraps `showModalBottomSheet(isScrollControlled: true, useSafeArea: true, useRootNavigator: true, backgroundColor: transparent, elevation: 0, barrierColor: colors.background.neutralScrim)`.
  - `useRootNavigator: true` makes the sheet cover the floating tab bar.
  - It re-provides `AppTheme` and the Riverpod `ProviderScope` container inside the sheet via `UncontrolledProviderScope`.
- **`MobileModalCard`** is the floating-card frame, not an edge-attached sheet:
  - 16px side margins.
  - Bottom gap: 32 on iOS (the home indicator floats inside it); 32 + `viewPadding.bottom` on Android; `keyboardInset + 16` when the keyboard is open.
  - All four corners radius 32 (`AppRadii.xLarge`), `background.base` fill, modal shadow, inner rim highlight.
  - `transparentBackground: true` applies only the margins.
- **`MobileModalScaffold`** is the sheet content layout:
  - Constructor: `MobileModalScaffold({required String title, required VoidCallback onClose, required Widget child, Widget? leading, TextStyle? titleStyle, int titleMaxLines = 1, bool showTitle = true, bool showClose = true, double bodyGap = 16, double bottomPadding = 24})`.
  - Padding: top 32, sides 16, bottom 24.
  - Title row: min height 26, 40px right inset for the close button. Title is `bodyLarge` w600 in `text.accent`, with an optional leading widget and 12 gap.
  - Then `bodyGap`, then child.
  - Close button is absolutely positioned at `top: 15.5, right: 16`.
- `MobileSheetCancel`: a centred "Cancel" text action at the bottom of confirm sheets.
- Examples:
  - `mobile_tx_fee_info_sheet.dart`: padding `(16,32,16,32)`, title "Tx fee" in `headlineSmall`, body, secondary "Close" button.
  - `mobile_address_verify_sheet.dart`: see 4.6.
  - `unsupported_sheet.dart`.
- Full-screen transparent overlays (Keystone signing) use a `CustomTransitionPage(opaque: false, barrierDismissible: false, barrierColor: neutralScrim, transitionDuration: Duration.zero)` route, with a `MobileModalCard` bottom-aligned inside it.
- Confirm-sheet pattern (destructive): title with a leading icon, body in `bodyMedium` `text.secondary`, destructive `AppButton` (optionally with a countdown label such as "Reset after 5s..."), then `MobileSheetCancel`.

### 3.6 Tab bar: `AppMobileTabBar` (`core/layout/mobile/app_mobile_tab_bar.dart`)

- API: `AppMobileTabBar({required List<AppMobileTabItem> items, required int currentIndex, required ValueChanged<int> onSelect})`, with `AppMobileTabItem({iconName, label})`. The label is for accessibility only; the bar is **icon-only**.
- **Pill:** height 64, `AppRadii.full`, floating 16px from the screen sides.
- **Glass:** `BackdropFilter(ImageFilter.blur(17.5, 17.5))` over `colors.macosUtility.navPanel` (#1A1A1A @ 30% dark / white @ 30% light). A hairline ring (`macosUtility.thinBorder`, spread 1) and a soft drop shadow.
- **Items:** padding 4, equal-width items 56 tall, icons 28. Inactive icons use `icon.muted`; the active icon uses `navPanel.activeIcon` (crimson #A83861).
- **Active pill:** one shared pill slides between items with `AnimatedAlign`. Fill `navPanel.activeBg` (crimson @ 15% dark / 10% light), 1px `border.subtleOpacity`.
- **Haptics:** `AppHaptics.auxiliaryKey()` on tap.
- Selecting the active tab again resets that branch to its root.

### 3.7 Top nav: `MobileTopNav` (`core/layout/mobile/mobile_top_nav.dart`), height 72

- **`.account(accountName, balanceLabel?, syncLabel?, syncLabelColor?, syncIndicatorColor?, syncAnimated, syncHighlightColor?, avatar?, onAccountTap?)`** (tab roots)
  - Left inset 16, 40px avatar, 12 gap, name in `labelLarge` w600 `text.accent`, optional balance line in `labelLarge` w400 secondary.
  - Right: sync label (`labelMedium`, `sync.text` green) plus a 6×50 glowing edge bar hugging the right screen edge. It shimmers and breathes while syncing.
  - Labels (`core/formatting/sync_status_label.dart`): "Vizor is synced", "NN% Syncing...", "Syncing failed. <reason>...".
  - Wrapped by `MobileTopNavAccount` (`mobile_top_nav_account.dart`), which binds it to providers.
- **`.back(title, onBack?, trailing?, backIcon = chevronBackward, titleStyle?, height = 72)`** (pushed screens)
  - A 44×44 back button at left 12 (icon 24 in `icon.accent`).
  - Centred title in `headlineLarge` (Young Serif 32), `text.accent`, inset by 56 on both sides.
  - Optional trailing widget at right 12. `onBack: null` hides the back button.
- **`.steps(progress, onBack?, showBackButton = true)`** (onboarding/wizards)
  - Centred 196×6 track in `background.overlay`, filled with `background.inverse`, full radius.
  - Back button at left 12.

`MobileTopScrollFade(child, height = 32)`: a window-colored gradient overlay at the top of a scroll view. It is transparent at rest and fades in over the first 24px of scroll, using smoothstep-sampled stops.

### 3.8 Feedback: toasts, haptics, copy

- **Toast** (`widgets/app_toast.dart`)
  - `showAppToast(context, String message, {Duration duration = 2s, String iconName = AppIcons.checkCircle})`.
  - Visual: a pill on `background.inverse`, radius 12, padding h12 / v8. Leading icon 16 in `icon.inverse`, 4 gap, message in `labelLarge` `text.inverse`, at most 2 lines, centred.
  - Position: top-centred at `max(32, safeTop + 8)`, horizontal padding 16.
  - Needs an `AppToastHost(child)` ancestor. `AppMobileShell` and each full-screen route scaffold add one.
  - Fallback: when called from inside a modal, it renders into the root overlay and re-provides `AppTheme`.
  - Warnings pass `iconName: AppIcons.warning`.
- **`copyTextWithToast(context, text:, toastMessage:)`** (`app_copy_feedback.dart`).
  - For secrets, use `copySensitiveText` in `core/clipboard/sensitive_clipboard.dart`. It expires after 1 minute on iOS via a native pasteboard `expirationDate` + `localOnly`; other platforms get a plain copy.
- **`NetworkFallbackToastHost`** (`network_fallback_toast.dart`): the toast shown on endpoint failover.
- **`AppHaptics`** (`core/feedback/app_haptics.dart`):

  | Method | Feedback | Used for |
  |---|---|---|
  | `digit()` | lightImpact | passcode digit |
  | `auxiliaryKey()` | selectionClick | delete key, tab switch |
  | `privacyToggle()` | mediumImpact | hide-balance eye |
  | `copy()` | lightImpact | copying secrets |
  | `error()` | native error channel `com.zcash.wallet/haptics`, else 2× heavyImpact 90ms apart | wrong passcode |

  The send and Keystone flows use no haptics.

### 3.9 QR display

- **`DotQrShape`** (`widgets/dot_qr_shape.dart`): a `PrettyQrShape` for `pretty_qr_code ^3.6.0`.
  - Data modules are circular dots (`PrettyQrSmoothSymbol(roundFactor: 1)`).
  - Finder patterns are custom: a ring 1 module thick (7-module outer diameter) plus a 3-module centre dot.
  - `finderReferenceDimension` keeps dense codes proportional.
- **Receive QR:**
  - `QrImage(QrCode.fromData(data, errorCorrectLevel: M))`, rendered once to a 1536px bitmap and displayed with `RawImage`.
  - `PrettyQrDecoration(quietZone: zero, shape: DotQrShape(...), image: PrettyQrDecorationImage(position: embedded, scale: 54.26/size))`, with a centre logo badge.
  - Size 260 inside a 292×308 container, radius 32, 1px border.
  - Shielded QR: light dots on a dark surface in both themes. Transparent QR: dark dots on white in light mode.
- **Keystone QR (scan-optimized):**
  - `PrettyQrView` with `PrettyQrSquaresSymbol(color: black)` (plain squares).
  - `errorCorrectLevel M`, `quietZone: modules(3)`, on `ColoredBox(colors.surface.qrCode)` (always white).
  - AGENTS.md says never to apply decorative QR treatments to codes a Keystone device must scan.
- **Scanner:** `mobile_scanner` (Chainapsis fork) behind `lib/src/services/qr_scanner.dart`.
  - Scan window `min(shortestSide × 0.9, 280)`.
  - Viewfinder: `MobileScanViewfinderCorners(cornerLength: 56, cornerRadius: 32, strokeWidth: 4)` over a `0xB3000000` scrim with a 256px clear window (radius 32).
  - Address scanning: `features/address_scan/widgets/mobile_address_scan_card.dart` (`MobileQrScanCard`).

### 3.10 Amount and address display

- **Amounts** (`core/formatting/zec_amount.dart`)
  - `formatZecAmount(BigInt zat, {minFractionDigits})`, `parseZecAmount(String)` / `ZecAmount.tryParse`, `ZecAmount.fromZatoshi(z).<preset>`. Presets return `ZecAmountPretty{amountText, denomText}`:

  | Preset | Output |
  |---|---|
  | `.balance` | `"0.00"`, `"1.25"` (min 2, max 8 decimals, trailing zeros trimmed, no ticker) |
  | `.fee` | `"0.0001 ZEC"` |
  | `.activity` | `"1.2345 ZEC"` (max 4 decimals; full 8 when 0 < v < 0.01) |
  | `.signedActivity` | `"+1.25 ZEC"` / `"-0.5 ZEC"` |
  | `.activityDetail` | `"1.00 ZEC"` |
  | `.receipt` | `"1.50 zec"` |

  - The ticker is `kZcashDefaultCurrencyTicker`: "ZEC" on mainnet, "TAZ" on testnet/regtest.
  - Mobile activity compaction (`features/activity/activity_amount_text.dart`): at most 14 characters, `"12.34K ZEC"`, `"1.234M ZEC"`.
  - Fiat: `$12.34`, `$1.5K`, `$1.234M`. The 24h change is `"+ 1.23% (24h)"` in `text.positiveStrong`, or `"- 0.45% (24h)"` in `text.destructive`.
- **Privacy masking** (`core/privacy/privacy_mask.dart`)
  - `fixedPrivacyMask()` returns `"******"`.
  - `hideAmountIfPrivacyMode(text, enabled, denomination)` returns `"****** ZEC"`. Activity rows use a 3-star mask.
- **Addresses** (`core/formatting/address_display.dart`)
  - `truncatedAddress(a)`: first 7 + `" ... "` + last 6 (unchanged if 18 characters or fewer).
  - `truncatedTxid(t)`: 8 + `"..."` + 8.
  - `zcashAddressDisplayKind(a)`: `t…` means transparent.
  - `addressVerifyGrid(a)`: 5-character groups, 5 per row. Groups 0, 2, N−3 and N−1 are flagged for crimson highlight.
  - Local variants: receive screen 13 + `" ... "` + 11; contacts 6 + 5; send review 7 + `" .... "` + 7.
  - Monospace is **not** used for addresses on mobile. Addresses use Geist `labelLarge` / 14/16 w500. Geist Mono is used for mnemonic indices and code chips.

### 3.11 Loading, skeletons, empty states

- **Spinner:** `AppIcon(AppIcons.loader)` (the animated spoke loader) at 16–20px. It is also used as the leading icon of pending activity rows and the "In progress" status chip.
- **Shimmer bars:** a pill of `background.overlay` @ 0.15 with a sweeping `background.raised` highlight (1200ms). Example: the 48×12 fiat-price placeholder.
- **QR placeholder:** a radius-24 square with a vertical band sweep (1400ms): `raised` base, `overlay` @ 0.72 highlight.
- **Empty states:** illustration, then title (`headlineSmall` or `headlineLarge`, `text.accent`), then 4 gap, then body (`bodyMedium` `text.secondary`, centred, max width about 236–250), then optional CTA 32 below. Examples:
  - Home: "No activity, yet..." / "How about running your\nfirst ZEC tx?"
  - Contacts: "No contacts yet" / "Add your first contact to get started."
- **Message cards** (activity list): a 160-tall section card with centred `labelLarge` `text.secondary`: "No activity yet", "Loading activity...", or "Couldn't load activity. Try again in a moment." (the error in `text.destructive`).
- **Sensitive overlay:** `SensitivePrivacyOverlay(sensitiveContentVisible:, child:)` (`core/privacy/sensitive_privacy_overlay.dart`) blurs the child (sigma 30, #141818 @ 20% scrim) and shows a 98×98 lock badge when the app is backgrounded or being captured. It wraps the seed phrase screens. On Android, `FLAG_SECURE` is toggled via `com.zcash.wallet/privacy_shield`.

### 3.12 Misc

- `AppChip`: a mnemonic word chip, min height 26, padding 4, optional `codeSmall` index and `labelLarge` label, default fixed width 80.
- `AppContextMenu`: desktop.
- `AppTooltip`.
- `AppDecorativeDivider`: a 256×16 ornament divider for onboarding.
- `AppProfilePicture` / `AppProfilePicturePickerModal`.
- **`MobileAccountAvatar`:** a profile picture plus, for hardware accounts, a **Keystone badge**. The badge is a 20×20 rounded square (radius 6) in `background.inverse` with a 2px ring in the color of the surface underneath, `AppIcons.keystone` 12 in `icon.inverse`, positioned at right −4 / bottom −2. *Zafe could use the same badge to mark a FROST vault.*

---

## 4. Mobile shell, navigation and screens

### 4.1 Shell: `AppMobileShell` (`core/layout/mobile/app_mobile_shell.dart`)

```dart
Scaffold(
  backgroundColor: colors.background.window,
  extendBody: true,                       // content scrolls under the floating bar
  body: AppToastHost(child: body),
  bottomNavigationBar: MobileBottomSafeArea(
    bottomPadding: bottomGap,             // iOS 16, Android 12
    child: Padding(padding: EdgeInsets.fromLTRB(16, 0, 16, bottomGap), child: tabBar)));
```

`MobileBottomSafeArea(bottomPadding, child)` handles the iOS home indicator:
- On iOS, it skips the bottom inset when `bottomPadding >= 16`, because the home indicator is an overlay.
- On Android, it always applies the inset.
- The same rule applies to sheets and pinned CTAs.

Tab root content needs its own bottom scroll padding: `64 (bar) + 48`, so the last row clears the floating bar.

**Pushed full-screen route pattern** (send, receive, settings detail):

```
Scaffold(backgroundColor: background.window)
 → AppToastHost
 → SafeArea
 → Column[ MobileTopNav.back(title, onBack), Expanded(body), pinned CTA area ]
```

### 4.2 Router (`lib/src/core/navigation/mobile_routes.dart`, `mobile_onboarding_routes.dart`; wired in `lib/app.dart`)

`go_router ^17.1.0`. `buildMobileRoutes(entryRoutes: [...appAuthRoutes(ref, bootstrap, unlockScreen: MobileUnlockScreen()), ...mobileOnboardingRoutes()])`:

- **Tab shell.** `StatefulShellRoute.indexedStack` (page = `CupertinoPage(_MobileTabShell)`).
  - Branches: `/home` (plus `/accounts` in the same branch, keeping the tab bar), `/swap`, `/activity`, `/settings`.
  - Tab pages use `NoTransitionPage`.
  - `_allMobileTabs` is a single const list driving both the branch order and the tab-bar order.
  - Hidden tabs (swap when disabled) are filtered out while indices are remapped.
  - `mobilePreviousTabPathProvider` records the outgoing tab so a tab root can offer a "back to previous tab" action.
- **Full-screen pushes over the shell** (`CupertinoPage`, which hides the tab bar):
  - Settings detail: `/settings/seed-phrase`, `/settings/address-book`, `/settings/endpoint`, `/settings/change-password`.
  - Send: `/send` (extra: prefilled recipient `String`), `/send/amount` (extra `MobileSendAmountArgs`), `/send/review` (extra `MobileSendReviewDraftArgs`), `/send/status` (extra `SendReviewArgs` | `KeystoneBroadcastArgs`).
  - Swap: `/swap/review`.
  - Activity: `/activity/tx/:txid?kind=`, `/activity/swap/:swapId`.
  - Home: `/home/keystone-shield`.
  - Other: `/receive`, `/about`.
  - A route that receives the wrong `extra` type falls back to the first step, e.g. `MobileSendScreen`.
- **Transparent modal routes** (`_mobileModalPage`: `opaque:false`, scrim barrier, zero-duration transition): `/send/keystone-sign`, `/swap/keystone-sign`.
- **Onboarding** (all `CupertinoPage`):
  - `/welcome`, `/add-account`, `/onboarding/method`, `/onboarding/intro`, `/onboarding/address-types`, `/onboarding/things-to-know`, `/onboarding/secret-passphrase`.
  - `/onboarding/set-passcode` redirects to `/welcome` unless `extra is SetPasswordScreenArgs`.
  - `/onboarding/biometrics`.
  - `/import`, `/import/manual`, `/import/birthday`.
  - `/onboarding/keystone`, `/onboarding/keystone/scan`, `/onboarding/keystone/select-account`, `/onboarding/keystone/birthday`.
- **Auth routes:** `/unlock`, `/lost-password`, `/storage-unavailable`, `/terms`, `/privacy`.
- **Redirect guard** `appRedirect`, evaluated in order:
  1. Bootstrap blocking failure goes to `/storage-unavailable`.
  2. No wallet and not in onboarding goes to `/welcome`.
  3. Locked goes to `/unlock`.
  4. Unlocked while on unlock goes to `/home`.
  5. Wallet exists while on `/welcome` goes to `/unlock` or `/home`.
  6. Feature-flagged routes go to `/home`.
- **Refresh:** `refreshListenable` is a `RouterRefreshController` (`ChangeNotifier`), poked by `ref.listen` on the wallet, security and feature providers. `pauseWhile(action)` defers redirects during multi-step mutations, so onboarding does not bounce mid-flow.
- `onException` falls back to `/home`. Paths are identical to desktop so deep links and the guard are shared.

### 4.3 Screen inventory (mobile)

| Area | Screen class | File |
|---|---|---|
| Entry | `MobileWelcomeScreen` | `features/onboarding/mobile/mobile_welcome_screen.dart` |
| | `MobileMethodSelectionScreen` | `.../mobile_method_selection_screen.dart` |
| | `MobileOnboardingIntroScreen`, `MobileAddressTypesScreen`, `MobileThingsToKnowScreen` | `.../mobile_create_steps.dart` |
| | `MobileSecretPassphraseScreen` (+ `SeedCard`) | `.../mobile_secret_passphrase_screen.dart`, `seed_card.dart` |
| | `MobilePasscodeScreen` (+ `PasscodePromptField`, `PasscodeNumpad`, `PasscodeBiometricButton`) | `.../mobile_passcode_screen.dart`, `passcode_widgets.dart` |
| | `MobileBiometricsScreen` | `.../mobile_biometrics_screen.dart` |
| | `MobileUnlockScreen`, forgot-passcode sheets | `.../mobile_unlock_screen.dart`, `forgot_passcode_sheet.dart` |
| | `MobileImportScreen`, `MobileImportManualScreen`, `MobileImportBirthdayScreen` | `.../mobile_import_*.dart` |
| | Keystone intro/scan/select/birthday | `.../mobile_keystone_screens.dart`, `mobile_keystone_scan_card.dart` |
| | Shared scaffold | `.../mobile_onboarding_scaffold.dart` (`MobileOnboardingStepScaffold`), `mobile_onboarding_progress.dart` |
| Tabs | `MobileHomeScreen` | `features/home/screens/mobile/mobile_home_screen.dart` |
| | `MobileActivityScreen` | `features/activity/screens/mobile/mobile_activity_screen.dart` |
| | `MobileSettingsScreen` | `features/settings/screens/mobile/mobile_settings_screen.dart` |
| Flows | `MobileSendScreen` / `MobileSendAmountScreen` / `MobileSendReviewScreen` | `features/send/screens/mobile/mobile_send_screen.dart` (3824 lines) |
| | `MobileSendStatusScreen` | `.../mobile_send_status_screen.dart` |
| | `MobileKeystoneSignScreen` | `.../mobile_keystone_sign_screen.dart` |
| | `MobileKeystoneShieldScreen` | `features/home/screens/mobile/mobile_keystone_shield_screen.dart` |
| | `MobileReceiveScreen` | `features/receive/screens/mobile/mobile_receive_screen.dart` |
| | `MobileTransactionStatusScreen` | `features/activity/screens/mobile/mobile_transaction_status_screen.dart` |
| | `MobileAccountsScreen`, `showMobileAccountsSheet` | `features/accounts/screens/mobile/`, `widgets/mobile/` |
| | `MobileAddressBookScreen` | `features/address_book/screens/mobile/` |
| | Seed phrase / change passcode / endpoint | `features/settings/screens/mobile/` |

### 4.4 Home (`MobileHomeScreen`)

Top to bottom, inside `SafeArea(bottom: false)` → `Column`:

1. **`MobileTopNavAccount`**
   - Avatar 40, account name, sync label and edge glow on the right.
   - Tapping the account opens `showMobileAccountsSheet`.
2. **`MobileTopScrollFade` → `ListView`**, padding `LTRB(16, 12, 16, 64+48)`.
3. **Balance card** (`_BalanceCard`):
   - **Outer container:** `background.ground`, radius 24, `appSurfaceShadow`, clipped.
   - **Inner dark card:** height 200, padding 16, `background.homeCard`, radius 24, 1.5px border white @ 7%.
   - **Top row:**
     - `shieldKeyhole` 20 in `text.homeCard`, 8 gap.
     - "Shielded balance" (Geist 14/16 w400).
     - Spacer, then the **privacy eye**: 32×32 circle in white @ 5%, `eye`/`eyeClosed` 16, Semantics "Hide balance" / "Show balance". It fires `AppHaptics.privacyToggle()` and toggles `privacyModeProvider`.
   - `Spacer()`.
   - **Fiat row** (only when a price is known): fiat in Geist 14 @ 80%, 8 gap, 24h change `"+ 1.23% (24h)"` (green / plum / neutral).
   - 8 gap.
   - **Balance** `Text.rich`: amount in Young Serif 45/48, −1.35, lining figures, then `" ZEC"` in Young Serif 32/33. One line with ellipsis.
     - Shielded balance = sapling + orchard + pending of each.
     - Privacy mode shows `"******"`.
   - **Transparent strip** (`_MobileTransparentBalanceStrip`), only when transparent > 0:
     - 57 tall on the white outer container under the dark card, padding h16 / v8, animated in and out.
     - `transparentBalance` 20, then `"Transparent: 0.50 ZEC"` in Geist 14 w500 `text.primary`.
     - Right side: text button "Shield ›" (or "Shielding..." with a loader).
     - Software accounts shield inline, then toast "Shielding complete". Keystone accounts push `/home/keystone-shield`.
4. 12 gap, then the **action buttons**:
   - With a balance: `Row[Expanded(AppButton primary "Send", leading plane), 8, Expanded(AppButton secondary "Receive", leading arrowDownCircle)]`, height 50.
   - With zero balance: a single primary "Receive your first ZEC" (leading `addNew`).
   - There is no scan button.
5. 24 gap, then **Recent activity** (padding h8 / v12):
   - Header: "Recent activity" (`labelLarge` w600) and a "See all ›" link (24h, `button.ghost.label`) → `context.go('/activity')`.
   - 24 gap, then up to 10 `ActivityFeedRowGroup` rows 12 apart, with date-only timestamps and no section cards.
   - Empty state: "No activity, yet..." (`headlineSmall`), "How about running your\nfirst ZEC tx?" (`bodyMedium` secondary), and a 340×220 character illustration.
6. **Importing state** (first sync after import):
   - A background illustration.
   - Bottom-aligned: big `"NN%"` in `displayLarge`, "We're importing your wallet..." (Young Serif 24/28), and "Hang tight ... It might take some time. Keep Vizor open & running."
   - The sync label is hidden during this state.

There is no pull-to-refresh anywhere. Sync is automatic: a 10s poll plus a lifecycle resume.

### 4.5 Receive (`MobileReceiveScreen`)

1. `MobileTopNav.back(title: 'Receive ZEC')`.
2. `ListView`, padding `LTRB(16, 12, 16, 24)`, then a 16.5 gap.
3. **`ReceiveTabs`** segmented control:
   - 320×44 stadium, `background.raised`.
   - The sliding indicator is `background.inverse` (always dark) with a 2px inset and 160ms animation.
   - Tabs: "Shielded" (`shieldKeyhole` 20) and "Transparent" (`transparentBalance` 20), in `labelLarge`. The selected label is `text.inverse`.
4. 32 gap, then a **PageView** (292×340) synced with the tabs. Each page:
   - `ReceiveQrSurface`: QR 260 in a 292×308 container, radius 32, 1px border, padding x16 / y24. Dot-style QR with a centre badge.
   - Shielded page only: **`ReceiveRenewButton`**, a 48px circle overlapping the QR's bottom edge at top 292.
     - `homeCard` fill, 2px `border.inverseOpacity` ring, 4px `macosUtility.window` halo, `renew` 24.
     - It generates a new diversified address. Semantics "Generate new shielded address".
5. 24 gap, then the address summary (288×70):
   - Account name (`bodyLarge` w600).
   - 4 gap, then the compact address (13 + " ... " + 11) in `labelLarge` `text.secondary`, with a 140ms fade on change.
   - A `help` 20 icon button (40 box) opens `showReceiveAddressInfoSheet`.
6. 32 gap, then the buttons (300 wide):
   - Primary (shielded) or secondary (transparent) `AppButton` "Share shielded address" / "Share transparent address", leading `share`. Uses `share_plus`.
   - 12 gap, then a text button (50h) with `copy` 20 and "Copy shielded address". Toast "Address copied".

The info sheet has a title with a one-line subtitle, then bullet rows (icon 20, 12 gap, `bodyMedium`, 8 apart), then a secondary "Close" button. Shielded bullets:
- "Tx details — sender, receiver, and amount — are encrypted on-chain & hidden."
- "A new Zcash shielded address is generated only when you tap the renew button."
- "Each new address is a diversified address derived from the same key. They all receive to the same wallet."

### 4.6 Send (mobile wizard)

All three steps are one widget (`MobileSendScreen`), selected by route. `_SendStep {recipient, amount, review}`. A `sendFlowId` (random 16-byte hex) threads through all Rust calls.

Chrome for every step:
- `Scaffold(bg: window, resizeToAvoidBottomInset: true)` → `AppToastHost` → `SafeArea(Column[MobileTopNav.back(title), Expanded(body)])`.
- Titles: "Select Recipient" → "Enter Amount" → "Review Send" (or "Send failed").

**Step 1: Recipient**
- **Floating address field** at `top: safeTop + 72 + 12`, sides 16:
  - `MobileTextField(hintText: 'Zcash Address')`.
  - Leading `plane` 20 (`icon.regular` when empty, `icon.accent` when filled).
  - Trailing pill "Paste" / "Clear" (36h).
  - When focused, the spread-1000 scrim shadow dims everything; tapping outside unfocuses.
- **Validation** runs on each change via `rust_sync.validateAddress(address:)`, returning `addressType` 'unified' | 'sapling' | 'transparent' | 'tex' or invalid. A sequence number drops stale results.
  - Error row: `labelLarge` in `text.destructive`, and the field border turns `border.utilityDestructive`. Copy: "Invalid address", "Address validation failed", "Keystone does not support TEX sends yet."
- **Below the field:**
  - A scan row (44h): 40 disc with `qr` icon, "Scan a QR Code" / "Scan an address using camera".
  - Contacts: header `users` 20 + "N contacts". Rows are 44h with a 32 avatar and the label over the truncated address.
- **CTA:** full-width primary at the bottom, padding `(16,0,16,12)`. Label is "Enter address to continue" when empty, otherwise "Continue".

**Step 2: Amount** (system decimal keyboard, autofocus; no custom numpad in code, although the design mock `22_send_amount_ready.png` shows one)
- Fixed 285h top block, padding `(16,16,16,0)`:
  1. **Balance row** (44h):
     - 40px ZEC coin (fixed `#F4B728`, white `zcashCurrency` 22).
     - Spendable amount (`labelLarge` w600).
     - "Max" pill on the right, Semantics "Use maximum spendable balance".
  2. 24 gap, then the **amount row** (64h, centred, baseline aligned):
     - Input in Young Serif at fontSize 48, `text.accent` (or `text.destructive` on error).
     - ZEC mode: the unit "ZEC" follows at size 38 with 50% alpha. USD mode: a "$" prefix.
     - The input width is measured with `TextPainter`. A custom iOS-style blinking empty cursor is drawn.
     - Hint "0" in `text.disabled`.
  3. 12 gap, then the **conversion toggle**: `doubleArrowVertical` 20 + "$ 12.34" or "0.1234 ZEC" in `labelLarge` secondary. Tapping swaps the input currency. A shimmer bar shows while the price loads.
  4. 24 gap, then **"Sending to"**: label, then a 68h row with a 40 avatar and the contact name over the truncated address.
- **Formatters:** comma→dot; ZEC 8 decimals / 17 characters; USD 2 decimals / 12 characters.
- **Validation:** fee via `rust_sync.estimateFee`. Error "Not enough ZEC". Max uses `rust_sync.estimateSendMax`, with errors "Max amount unavailable" and "Message is too long".
- **CTA label carries the state:** "Enter amount to continue" → "Calculating max amount" → "Not enough ZEC" (disabled) → "Finish & review".

**Step 3: Review**
- Padding `(16,12,16,12)`. Info block (268h, v24):
  - `_ReviewInfoRow` "Amount": ZEC coin, headline "123.12 ZEC" in Young Serif 32, fiat on the bottom line.
  - 8, then `arrowDownward` 24, then 8.
  - "To" row:
    - Leading: avatar 40, or a wallet disc.
    - Headline: contact name, or "Shielded address" / "Transparent address" / "TEX address".
    - Bottom line: `shieldKeyhole` 16 (crimson) or `transparentBalance` (muted) with the compact address (7 + " .... " + 7).
    - Ghost small button: `eye` + "Full address".
  - The full-address sheet is `showMobileAddressVerifySheet`:
    - The address as 5-character chunks, 5 per line (Geist 14/16 w500).
    - Chunks 0, 2, N−3 and N−1 in crimson w600.
    - 1px `border.regular` dividers, 12 gaps, then a "Cancel" text button.
    - This is the address-verification affordance.
- 32 gap, then the **detail card** (`MobileSurfaceCard`, radius 24, padding h16 / v32):
  - "Message" row (shielded recipients only):
    - Empty: `edit` + "Add short encrypted message".
    - Set: memo text with `doubleArrowVertical`.
  - A 16/1px/16 divider.
  - "Tx fee" row: `"0.0001 ZEC"` (or "—" before the estimate) plus `help` 20, which opens the fee sheet. Fee sheet body: "The network fee is set by the Zcash protocol (ZIP 317) based on the transaction size. Vizor adds no extra fee."
  - **Memo sheet** ("Add Memo"): header "Message" with a byte counter "51/512".
    - 148h text area (`ground`, radius 12, 1.5 inverse border when focused), hint "Only the recipient can read this".
    - Buttons: primary "Add Memo" (or "Clear memo" with `trash`), ghost "Cancel". Error "Message is too long".
- **Buttons:**
  - Primary "Confirm & Send" (leading `plane`), which becomes "Preparing..." while proposing.
  - Ghost "Cancel", which does `go('/home')`.
- **Confirm:**
  1. `proposeSendTransfer` → `rust_sync.proposeSend(dbPath, network, accountUuid, sendFlowId, toAddress, amountZatoshi, memo)`, returning `SendReviewArgs{proposalId, sendFlowId, proposalAccountUuid, address, addressType, amountZatoshi, feeZatoshi, needsSaplingParams, memo}`.
  2. Software accounts: `go('/home')` then `push('/send/status', extra: args)`.
  3. Hardware accounts: `push<KeystoneBroadcastArgs>('/send/keystone-sign', extra: args)` (section 5).
- **In-screen failed phase:**
  - Warning icon 48 (destructive), then "Send failed" (`displayLarge`), then "`<amount>` to `<label>`".
  - Friendly error, one of: "Insufficient shielded balance to cover amount and fee." / "Network error. Check your connection and try again." / "The network rejected this transaction. Try again." / "Transaction expired before it could be sent. Try again." / "Send failed. Try again."
  - Buttons: "Try again" and the text link "Back to wallet".

**Status** (`MobileSendStatusScreen`)
- Phases `{sending, pendingBroadcast, succeeded, failed}`. Titles: "Sending...", "Sent successfully", "Send failed". Back is blocked while sending.
- Layout: `MobileReviewInfoRow` Amount (ZEC badge 32), `MobileReviewFlowArrow`, `MobileReviewInfoRow` To (struck through on failure).
- 24 gap, then the detail card (`ground`, radius 24, padding h16 / v32) with 32h rows:
  - "Status" chip:
    - `loader` "In progress" (`text.secondary`)
    - `checkCircle` "Completed" (`text.positiveStrong`)
    - `cross` "Failed, funds returned" (destructive)
  - "Message" (18 characters + "..."), "Timestamp", "Tx ID" (truncated, `arrowTopRight` opens the explorer; on failure it copies with toast "Transaction Hash Copied"), divider, "Tx fee".
- Pending note (`bodySmall` secondary): "Transaction was created locally but could not be broadcast. It will retry automatically when the network is available. Do not send again unless this transaction expires."
- **No hero illustration or celebration animation on mobile.** State is carried by the title, the chip and the strikethrough.
- **Sapling params gate** (non-dismissible sheet):
  - "Download Required" / "To create this private transaction, your wallet needs to download about 50MB of cryptographic parameters." / "This happens once, then it's done.\nNetwork data charges may apply."
  - Buttons: "Download" / "Cancel".
  - Params come from `https://download.z.cash/downloads/`, are SHA-1 verified and written to `<appSupport>/sapling_params/`.

### 4.7 Activity

**`MobileActivityScreen`**
- `MobileTopNav.back(title: 'Activity')`.
- `ListView`, padding `LTRB(4, 12, 4, 112)`, containing `ActivityFeed(showHeader: false)`.
- **Sections** (`activity_feed_sections.dart`): "This week" (Monday-start), then "May 2026" style month-year, then "Earlier" for rows with no timestamp.
- One card per section: `ground`, radius 24, no shadow on mobile, padding `(16,24,16,24)`.
  - Title: 24h `labelLarge` w600 `text.secondary`.
  - Rows 12 apart. Cards 24 apart.
- **Row anatomy** (`ActivityFeedRow`, 44h, padding h4, radius 12):
  - Left: 40 disc (`neutralSubtleOpacity`) with an 18 icon (`icon.regular`), 8 gap.
  - Middle column:
    - Title (`labelLarge` `text.accent`): "Received" / "Sent" / "Shielded" / "Sending..." / "Receiving..." / "Send failed".
    - 4 gap, then a subtitle row: pool icon 16 (crimson `shieldKeyholeOutline` or muted transparent) and "Shielded" / "Transparent" / "Mixed" (`labelLarge` w400 secondary).
  - Right column (max width 128, end-aligned):
    - Amount (`labelLarge` w600): `"+1.25 ZEC"` in green for inbound, `text.accent` for outbound.
    - 4 gap, then `"May 29, 13:40"` in `text.muted`, or "Failed" with a `skull` 12 in destructive.
- Row icons: received → `arrowDownCircle`, sent → `plane`, shielded → `shieldKeyholeOutline`, pending → `loader`.
- Tapping a row pushes `/activity/tx/<txid>?kind=`.

**`MobileTransactionStatusScreen`** uses the same review-row and detail-card vocabulary as the send status screen:
- Received: **From** → arrow → Amount.
- Self-shield: Amount ("From transparent balance") → To "Shielded balance".
- Includes the expandable memo row and the fee row.

### 4.8 Settings (`MobileSettingsScreen`)

- `MobileTopNav.back(title: 'Settings')` with no back button.
- `ListView`, padding `LTRB(16, 12, 16, 112)`, with two group cards 24 apart.
- Group card: `MobileSurfaceCard(cornerRadius 24, padding (16,32,16,32))`. Group title in `labelLarge` w400 `text.secondary` (padding left 4 / bottom 8). `MobileListRow` 44h rows with a leading icon 20 in `icon.muted`.

| Group | Row (icon) → action |
|---|---|
| Account | "Secret Passphrase" (key) → `/settings/seed-phrase` (disabled for Keystone accounts) |
| | "Password" (lock) → `/settings/change-password` → toast "Passcode updated" |
| | "Profile Picture" (user) → picker sheet; value shows a 24 avatar + name |
| | "Account Name" (scroll) → edit sheet |
| | "Contacts" (users) → `/settings/address-book` |
| System | "Endpoint" (endpoint), value host:port → `/settings/endpoint` |
| | "Theme" (theme), value "System" / "Light" / "Dark" → theme sheet (option cards "System (Auto)", "Light", "Dark"; "Update" / "Cancel") |
| | "Face ID" / "Fingerprint" (lock), value "On" / "Off" |

**Seed phrase screen**
- Passcode gate first: "Enter Passcode" / "Confirm your access".
- Then, inside `SensitivePrivacyOverlay`:
  - Card (radius 32) with a `Wrap` of 90×32 word cells (`codeSmall` index "01" + `labelMedium` word).
  - "Copy" chip (inverse stadium) → toast "Secret passphrase copied".
  - Birthday card with copy icons.
- An iOS screenshot triggers a warning sheet: "Don’t take screenshots of your Secret Passphrase" / "I understand".

### 4.9 Onboarding (create flow)

**Flow:** Welcome → Method (step 2) → Intro (3) → Address types (4) → Things to know (5) → Secret passphrase (6) → Create passcode (7) → Biometrics (1.0) → `/home`. Progress is `step / 8`.

**`MobileOnboardingStepScaffold(progress, title, subtitle, child, bottomArea, aboveTitle?)`**
- `MobileTopNav.steps(progress)`.
- Scroll area, padding `(8,24,8,24)`:
  - Optional `aboveTitle` + 24.
  - Title in `displayLarge` (Young Serif 40), centred, `text.accent`.
  - 16 gap, subtitle in `bodyMediumStrong` `text.primary`, max width 320, centred.
  - 24 gap, child.
- Pinned bottom area, padding `(16,12,16,12)`.

**Welcome**
- Full-bleed hero illustration, fading to transparent at the top.
- Wordmark 106×40 at top 35.
- 38 gap, "Private Money.\nBy default" in Young Serif 48.
- 29 gap, primary "Get started ›" (min width 200).

**Method selection**
- "Welcome to Vizor" / "Select the method you want."
- Three 120h cards (radius 24, 1.5 `border.subtle`), 16 apart, each with art 180×120 on the right.
  - Content padding 16: icon 20 at the top, label in `headlineMedium` (Young Serif 28) at the bottom.
  - "Create wallet" is emphasised with the dark `homeCard` fill; the others use `raised`.
  - Labels: "Create wallet", "Import wallet", "Connect Keystone".

**Info steps**
- Dark `homeCard` info cards (radius 24, pattern overlay) or `ground` cards (radius 24, padding h20 / v44).
- Section headers: icon 24, 8 gap, `bodyLarge`.
- Copy examples:
  - "The Shielded World" / "Zcash (ZEC) built around financial\nprivacy & self-custody."
  - Primary "Tell me how Zcash works ›", plus a text link "I know how to use Zcash" (skip).

**Secret passphrase**
- Before reveal: a warning card (`homeCard`, radius 32, min height 440) with a 32 crimson key tile, "You are about to see your\nSecret Passphrase.", and body at 70% opacity.
- After reveal: `SeedCard`, a 3-column grid of "01 word" (`codeSmall` @ 45% + `bodyMedium`), 19 row gap.
  - "Copy" / "Copied" uses the sensitive clipboard and a haptic.
- Buttons: "Reveal phrase" → "Continue" → "Creating wallet...".

**Passcode**
- "Create Passcode" / "Confirm Passcode" (`displayLarge`), subtitle "6 digits length" / "Re-enter your passcode.".
- **Dots:** 6 diamonds, each a 14×14 square rotated 45° with radius 3, 8px horizontal padding. Filled = `icon.brandCrimson`; empty = `neutralSubtleOpacity`. The error text is positioned 36px below the dots' centre, outside the layout, so nothing shifts.
- **Keypad:** 320 wide `Wrap`, spacing 16.
  - 80×80 circles in `neutralSubtleOpacity`, digits in Young Serif 40.
  - Last row: [help | 0 | backspace].
  - Haptics: `digit()` on digits, `auxiliaryKey()` on delete.
  - Mismatch error: "Passcodes didn't match. Try again."
- The passcode **is** the wallet password on mobile (min length 6, per `password_policy.dart`).

**Biometrics**
- Hero art, then "Unlock your wallet\nwith Face ID" / "This is an easy and fast way to sign in.\nYou can switch back to passcode anytime."
- Primary "Enable Face ID" (faceId icon), ghost "Not now".

**Unlock**
- "Welcome Back" / "Enter your passcode to open Vizor", then dots and numpad.
- `PasscodeBiometricButton` ("Sign in with Face ID"): ghost pill, 36h.
- Help key → "Forgot Passcode?" sheet → "Are you sure?" sheet with a countdown destructive "Reset Vizor".

**Import**
- "Import Wallet": 24 slots in a dark card; "Paste secret phrase" / "Enter manually".
- Manual word entry: word field in `displayLarge` with suggestion chips.
- Birthday: date or block height tabs; "I don’t remember".

### 4.10 Accounts

**Accounts sheet** (from the home top nav)
- Centred 56 avatar and name.
- "Other accounts": up to 4 rows of 48h.
  - Row: 40 avatar and name. A trailing copy icon copies the shielded address (toast "Address copied"). Tapping a row switches account.
- Bottom row: secondary "Manage accounts" (expanded) + an 80×50 primary `addNew` pill → `/add-account`.

**Accounts screen**
- Groups "Current" / "Other".
- Each row has a ⋯ menu: a dark inverse popover, radius 16, with "Copy address", "Send ZEC", "Edit account", divider, "Remove account" (in destructiveLight).

### 4.11 `design_suggestion/` (repo root)

`index.html` is a Korean-language design-request memo (2026-06-12). It lists nine screens the developers built without Figma frames and asks the designer to formalize them:

1. Address book.
2. The Keystone signing flow (QR, then camera with progress %).
3. Swap review/detail.
4. The onboarding progress rule `step/(count+1)`.
5. Keystone badge placement.
6. The send-amount "validating" state.
7. The Face ID row and numpad glyph.
8. The birthday input: native calendar on iOS, sheet on Android; custom keypad removed.
9. Swap copy in sentence case.

The images are all iPhone 16 Pro, light theme, and **older than the code**:

| Image | Shows |
|---|---|
| `32_intro.png` | Intro step: progress track about 1/6, serif "The Shielded World", dark patterned card with a shield-Z icon, centred paragraph, dark pill "Tell me how Zcash works ›", skip link. |
| `35_passphrase_hidden.png` | Secret passphrase pre-reveal: large dark card with a crimson key tile, white heading, grey body, "Reveal phrase ›". |
| `31b_keystone_intro.png` | "Connect Keystone" / "Prepare your Keystone wallet": white card with "1. Check Keystone firmware" + link, divider, "2. Prepare to connect" with 4 numbered steps, "Continue ›". |
| `30f_address_book_empty.png` | Older "Address book" empty state: dark users tile, "No contacts yet", "⊕ Add contact"; floating tab bar with the crimson-tinted active pill. |
| `30g_address_book_add.png` | "Add contact" sheet over a scrim: knight avatar with pencil badge; Network / Name / Address fields; dark "Add contact" button; "Cancel". |
| `S0_address_book_list.png` | Contact row card with avatar, name + crimson Zcash badge, truncated t-address, trash icon; pinned "⊕ Add contact"; 4-tab bar. |
| `22_send_amount_ready.png` | "Enter amount": serif "0.25", "Max: 1.25 TAZ", "Sending to" + avatar + address, "Finish & Review", **custom grey keypad** (since replaced by the system keyboard). |
| `S1_swap_composer.png` | Swap composer (not relevant to Zafe). |
| `S2_keystone_sign_qr.png` | **Keystone QR sign screen:** black full screen, ✕ top right, "Confirm transaction" / "Use your Keystone wallet to scan this transaction QR code. Follow the steps on your device.", white rounded card with a square-module QR, bottom "Cancel" + dark outlined pill "Next step ›". The shield flow still uses this black layout; the send flow now uses the modal-card version (5.3). |

The `onboarding/` folder at the repo root is Korean developer docs (architecture, FFI, sync engine, Keystone, gotchas). It has no UI content.

---

## 5. Keystone / external-signer UX (the analogue of Zafe's approval flow)

### 5.1 Concept

A Keystone account is **watch-only in the wallet DB**. It holds a UFVK imported from the device, plus `seedFingerprint` and `zip32Index`. `AccountInfo.isHardware = true`, and `accountProvider.notifier.isHardwareAccount(uuid)` gates the flow. Every spend goes through PCZT (ZIP 374 partially created Zcash transactions):

```
proposeSend → createPcztFromProposal → redactPcztForSigner → encodePcztUrParts
   → [animated QR shown to device]
   ║ in parallel: addProofsToPczt(original)
   → scan signed UR "zcash-pczt" (decodeUrPart loop) → decodePcztFromCbor
   → wait for proofs → extractAndBroadcastPczt(proofs, signatures)
```

### 5.2 Dart → Rust call sequence (`send/screens/mobile/mobile_keystone_sign_screen.dart`, `_preparePczt`)

| # | Call (Dart binding) | Returns / notes |
|---|---|---|
| 0 | `rust_sync.proposeSend(dbPath, network, accountUuid, sendFlowId, toAddress, amountZatoshi, memo)` | `proposalId`, fee, `needsSaplingParams` |
| 1 | Sapling params check | If needed, `MobileSaplingParamsSheet`. Cancel throws `MobileKeystonePcztSigningAborted` and pops null. |
| 2 | `rust_sync.createPcztFromProposal(dbPath, network, BigInt proposalId, String sendFlowId)` | `Uint8List` PCZT (consumes the proposal) |
| 3 | `rust_sync.redactPcztForSigner(pcztBytes)` | Strips witnesses and metadata the signer does not need |
| 4 | `rust_keystone.encodePcztUrParts(pcztBytes: redacted, maxFragmentLen: BigInt.from(140))` | `List<String>` of `ur:zcash-pczt/...` fountain parts |
| 5 | `rust_sync.addProofsToPczt(pcztBytes: original, spendParamsPath?, outputParamsPath?)` | **Started without await.** Proving runs while the user signs on the device. |
| 6 | `rust_keystone.resetUrSession()` (sync), then `rust_keystone.decodeUrPart(part, expectedUrType: 'zcash-pczt')` for each scanned frame | `UrDecodeResult{complete, progress, data?, urType?}`. Frames are de-duplicated with a lowercase set. |
| 7 | `rust_keystone.decodePcztFromCbor(cbor)` | Signed PCZT bytes |
| 8 | Poll the proofs future every 200ms | |
| 9 | Pop `KeystoneBroadcastArgs{reviewArgs, pcztWithProofsBytes, pcztWithSignaturesBytes}`, then the status screen calls `rust_sync.extractAndBroadcastPczt(dbPath, lightwalletdUrl, network, pcztWithProofsBytes, pcztWithSignaturesBytes, spendParamsPath?, outputParamsPath?)` | `{txid, status: 'broadcasted' | 'broadcast_unknown' | 'broadcasted_storage_failed' | ...}` |

- **Cancel:** the pushed route pops `null`, and the review screen calls `discardSendProposal(...)` (idempotent) and stays on review.
- Other bindings (`lib/src/rust/api/keystone.dart`): `encodePcztToUr`, `decodeUrToPczt`, `decodeAccountsFromCbor(cbor) → List<KeystoneAccountInfo{name, ufvk, index, seedFingerprint}>`, `decodeAccountsUr`.
- Rust UR errors (`rust/src/wallet/keystone.rs`):
  - "Invalid UR: missing type prefix" (ignored by Dart)
  - "Unexpected UR type: got …, expected …"
  - "UR session reset: …" (Dart resets the session)
- Crates: `ur` (KeystoneHQ/ur-rs 0.3.3), `ur-registry` (KeystoneHQ/keystone-sdk-rust), and `pczt 0.7 [orchard, io-finalizer, signer, zcp-builder]`.

### 5.3 Signing state machine and screens (`keystone/widgets/mobile_keystone_pczt_signing_flow.dart`)

`MobileKeystonePcztSigningFlow`: `_SignStage {preparing, showQr, scanning, failed}` plus the flags `_decoding`, `_proofsFailed`, `_scanProgress`, `_scanHint`.

```
preparing ──ok──▶ showQr ──"Get signature"──▶ scanning ──complete──▶ (decoding) ──▶ onSigned → pop(result)
    │                                            │  bad frame → hint, stay; CBOR fail → reset UR session
    └──error──▶ failed ◀──── onSigned/proofs error (camera stopped)
Cancel = pop(null); blocked while _decoding (PopScope canPop:false, close disabled)
```

**QR stage.** A transparent route with a scrim, containing a bottom-aligned `MobileModalCard` (max width 393), padding `(16,32,16,32)`:
- Title "Confirm with Keystone" (`bodyLarge` w600). On failure: "Confirm transaction".
- 4 gap, then "Scan with your Keystone" (`bodyMedium` secondary).
- Close circle at top 15.5 / right 16, Semantics "Close Keystone signing".
- 44 gap, then the QR (`KeystonePcztQrStage`):
  - Animated UR frames at 100ms (10 fps), each cached as a `QrImage`.
  - Square modules, EC M, quiet zone 3, on white.
  - Size = available height, clamped to 200–321.
  - While preparing: `_MobileKeystoneQrPlaceholder`, a radius-24 shimmering square.
- 16 gap, then a one-line instruction (`bodyMedium`, centred): "Loading the QR code ..." → "After you scanned, click Get signature." → the error in `text.destructive`.
- 32 gap, then primary **"Get signature"** (enabled only in showQr), 12 gap, then ghost **"Close"**.

**Scanning stage.** The same modal frame containing `MobileQrScanCard`:
- Card height = screen height − (top inset + 71 + 32), minimum 420.
- Full-bleed camera under a `0xB3000000` scrim with a 256px clear window (radius 32) and corner brackets 56/32/4.
- Torch (top-left) and close ✕ (top-right), 30px white icons in 40×40 boxes.
- Caption (152 wide, bottom 110, `bodyMediumStrong`, white), in priority order:
  - "Reading signature..."
  - hint
  - "Scanning... N%"
  - "Scan a Zcash QR code to continue"
- Hints:
  - "Open the signed transaction QR on Keystone, then scan again." (wrong UR type)
  - "Keep the QR code steady and fully visible."
  - "This QR code could not be decoded as a Keystone signature."
- Camera loading: blur-37 veil with "Loading...".
- Permission card (`MobileModalScaffold` "Scan the signed Keystone QR"):
  - "You've denied camera access" / "Request again, or enable manually\nin the System settings." with secondary "Request again" (`renew`), which opens OS settings.
  - "Camera unavailable" / "Keystone signing needs a camera on this device." with "Try again".
  - Ghost "Cancel".
- Prepare errors:
  - "Transaction expired before it could be signed."
  - "Required proving parameters could not be prepared."
  - "Keystone signing could not be prepared. Go back and try again."

The send flow has no path back from scanning to the QR. The shield flow adds a "Show QR" button for this, which is recommended for Zafe.

### 5.4 Keystone shield screen (`home/screens/mobile/mobile_keystone_shield_screen.dart`)

The older full-screen **black** layout (matches `S2_keystone_sign_qr.png`):
- Stages: `_ShieldSignStage {preparing, showQr, scanning, broadcasting, failed, broadcastWarning}`.
- `Scaffold(bg: Colors.black)`. Top bar: torch (while scanning) and ✕ "Cancel signing".
- 72 gap, then the title (`headlineSmall`, white): "Shield transparent balance", or "Broadcasting shield tx".
- Subtitle (`bodyMedium` @ 80% white): "Use your Keystone wallet to scan this shielding QR code. Follow the steps on your device." / "Keep Vizor open while the transaction is sent."
- The QR sits in a `ground` container, padding 16, radius 24, size 220–280.
- Scanner: full screen with a 260 window.
- Bottom bar, padding `(24,12,24,24)`:
  - Ghost "Cancel" with white text.
  - Primary "Next step ›", which becomes "Broadcasting..." with a loader. While scanning it becomes secondary "Show QR".
  - Terminal states show a single "Back to wallet" button.
- Uses `rust_sync.createShieldTransparentPczt(dbPath, network, accountUuid) → {pcztBytes, needsSaplingParams}` and then the same pipeline, broadcasting in-screen.
- Result toasts on home: "Shielding complete" or a warning.
- Uncertain broadcast: "The shield transaction status is uncertain. Check activity before trying again."

### 5.5 Keystone onboarding (`onboarding/mobile/mobile_keystone_screens.dart`)

All screens use `MobileOnboardingStepScaffold`, subtitle "Prepare your Keystone wallet", progress 0.2 / 0.4 / 0.6 / 0.8.

1. **"Connect Keystone"** (intro): two `ground` cards (radius 24, padding 20) 16 apart.
   - Card 1: "1. Check Keystone firmware" (`importWallet` icon), body "Make sure your Keystone is on the latest Cypherpunk firmware.", and an inline link to keyst.one/firmware.
   - Card 2: "2. Prepare to connect".
     - "On your Keystone": "1. Tap ••• (top right), then Connect software wallet." / "2. Select Vizor (or ZODL)".
     - "On Vizor": "3. Scan the dynamic QR code on your Keystone."
     - Numbers sit in a 20px column.
   - CTA "Continue ›".
2. **"Scan QR Code"**: the step scaffold behind a scrim with a bottom `MobileModalCard(MobileQrScanCard)`.
   - Scans UR `zcash-accounts`, then `decodeAccountsFromCbor`.
   - Caption "Scan the Keystone account QR" / "Reading accounts..." / "Scanning... N%".
   - Errors: "No Zcash accounts were found on this Keystone QR." / "This QR code could not be decoded as a Keystone Zcash account." / "Open the Zcash account QR on Keystone, then scan again."
   - Permission: "Enable camera access" / "A camera is required to connect Keystone.\nYou can revert this in settings anytime later."
3. **"Select account"**: "N accounts found".
   - Radio cards: min height 64, `ground`, radius 16, 2px `border.strong` when selected.
   - Each card: `user` icon, name (fallback "Account {index+1}"), UFVK truncated 12 + " ... " + 10, and a 24 radio.
   - CTA "Select account".
4. **Birthday**: reuses `MobileImportBirthdayScreen`.
   - If no passcode exists yet, push `/onboarding/set-passcode` with `SetPasswordScreenArgs.importKeystone(name, ufvk, seedFingerprint, zip32Index, birthdayHeight)`.
   - Otherwise `accountNotifier.importKeystoneAccount(...)` inside `runWithSyncPausedForAccountMutation`, then `go('/home')`.

---

## 6. State and data flow in Dart

### 6.1 Startup (`lib/main.dart`, `lib/app.dart`, `lib/src/app_bootstrap.dart`)

```
main(): assert(debugCheckFormFactorMatchesPlatform()); runZcashWalletApp();
initializeZcashWalletRuntime(): WidgetsFlutterBinding.ensureInitialized(); await RustLib.init(); (desktop window init)
bootstrap = await loadAppBootstrap();
runApp(BootstrappedZcashWalletApp) → ProviderScope(key: ValueKey(generation), overrides: [
    appBootstrapProvider.overrideWithValue(bootstrap),
    appBootstrapRetryProvider.overrideWithValue(reload)])   // retry bumps key → fresh container
```

**`loadAppBootstrap()`** does, in order:
1. Ensures the wallet DB name.
2. Recovers any interrupted password rotation.
3. Reads settings from secure storage: network, RPC endpoint, theme, privacy mode, biometric flag, password configured.
4. Sets `isUnlocked = hasSessionPassword`.
5. Calls `rust_wallet.walletExists`. If the wallet exists, calls `ensureWalletDbMigrated`.
6. Calls `listAccounts` and merges the result with the secure-store `zcash_accounts` JSON. Rust is authoritative for existence; Dart owns the name, order, `profilePictureId` and `isHardware`.
7. Resolves the active account.
8. If unlocked, preloads the sync snapshot: `getSyncStatus`, `getBalance`, `getTransactionHistory(limit: 10)`.
9. Computes `initialLocation`: `/welcome`, `/unlock` or `/home`.

Failures produce `AppBootstrapState.blocked(kind)`, where `kind` is `secureStorageUnavailable | startupFailure | walletDbMigrationFailed`. The router then shows `/storage-unavailable` (`StorageUnavailableScreen`) with a retry. Example copy: "Vizor needs access to secure storage before it can open your wallet."

**`ZcashWalletApp`** is `MaterialApp.router(theme: buildLegacyLightTheme(), darkTheme: ..., themeMode: ref.watch(themeModeProvider), routerConfig: router, builder: ...)`. The `builder` wraps the child as `AppThemeHost(themeMode) → … → NetworkFallbackToastHost → GestureDetector(unfocus on outside tap)`.

### 6.2 Riverpod pattern

**Riverpod setup**
- `flutter_riverpod ^3.3.1`.
- All providers are hand-written: no codegen, no `@riverpod`, no `family`.
- All providers are global and non-autoDispose, except price data, which is `.autoDispose`.
- Most notifiers seed `build()` from `ref.watch(appBootstrapProvider)`, so a bootstrap retry resets everything.

```dart
// lib/src/providers/privacy_mode_provider.dart (complete, representative)
class PrivacyModeNotifier extends Notifier<bool> {
  static final _store = AppSecureStore.instance;
  @override
  bool build() => ref.watch(appBootstrapProvider).privacyModeEnabled;
  Future<void> set(bool enabled) async {
    await _store.writePlain(kPrivacyModeEnabledKey, enabled ? 'true' : 'false');
    state = enabled;
  }
  Future<void> toggle() => set(!state);
}
final privacyModeProvider =
    NotifierProvider<PrivacyModeNotifier, bool>(PrivacyModeNotifier.new);
```

| Provider | Type | Role |
|---|---|---|
| `walletProvider` | `AsyncNotifierProvider<WalletNotifier, WalletState>` | Derived: `hasWallet`, `unifiedAddress`, `activeAccountUuid` |
| `accountProvider` | `AsyncNotifierProvider<AccountNotifier, AccountState>` | Create, import, import Keystone, switch, rename, remove, reset, mnemonic access; persists `zcash_accounts` / `zcash_active_account` |
| `syncProvider` | `AsyncNotifierProvider<SyncNotifier, SyncState>` | Full sync over an FRB `Stream` (see below), mempool observer, 10s poll, lifecycle hooks |
| `appSecurityProvider` | `NotifierProvider<AppSecurityNotifier, AppSecurityState{isPasswordConfigured, isUnlocked}>` | Two-phase `preparePasswordSetup` / `commitPasswordSetup` / `rollbackPasswordSetup` (so the router never sees half-done onboarding), `unlock`, `lock`, `changePassword` |
| `biometricUnlockProvider` | `AsyncNotifierProvider` | `enable(passcode)`, `disable`, `readPasscode(reason)` |
| `privacyModeProvider`, `themeModeProvider` | `NotifierProvider` | Persisted UI prefs |
| `receiveAddressServiceProvider` | `Provider<ReceiveAddressService>` | Retries a DB lock after 300ms, 1s and 2s, then "Wallet is busy. Try again in a moment." |
| `routerRefreshProvider` | `Provider<RouterRefreshController>` | `requestRefresh()`, `pauseWhile(action)` |
| `rpcEndpointProvider`, `rpcEndpointFailoverProvider`, `rpcEndpointLatencyProvider` | `NotifierProvider` | Endpoint selection, chain-name verification, automatic failover + toast |
| `zecHomeMarketDataProvider` etc. | `NotifierProvider.autoDispose` | CoinGecko price and 24h change |

**Sync pipeline**
- `rust_sync.startFullSync(dbPath, lightwalletdUrl, network, mode: 1)` returns a Dart `Stream<ApiSyncProgressEvent>`, fed by a Rust `StreamSink`.
- A generation counter `_syncGen` drops stale events after a restart.
- `runWithSyncPausedForAccountMutation(ref, action)` (`providers/wallet_mutation_guard.dart`) pauses sync around DB-mutating operations.

### 6.3 Calling Rust and mapping errors

**Call style**
- Providers and services call the generated bindings directly: `import '../rust/api/sync.dart' as rust_sync;` (likewise `rust_wallet`, `rust_keystone`, `rust_secret`).
- There are no isolates and no `compute()`. FRB runs normal functions on its thread pool and returns `Future`s.
- `#[frb(sync)]` is used for trivial getters and cancels (`set_sync_mode`, `cancel_full_sync`, `reset_ur_session`, …).

**Rust side**
- API functions return `Result<T, String>` and wrap their bodies in a `catch_unwind` helper, so panics become `"Rust panic: …"` errors.

**Dart side: error mapping is string matching**
- `onboarding_error_messages.dart` strips `Exception: ` and `AnyhowException(...)`, then maps known substrings to friendly copy.
- `providers/sync_failure.dart` has `classifySyncFailure(Object) → SyncFailure{kind: network|endpoint|databaseBusy|databaseFatal|chainRecovery|parseFatal|unknown, userMessage, showSettingsAction}`.
- Send has its own friendly-error mapper (4.6).

*For Zafe: prefer typed errors (an FRB-mirrored enum) over substring matching.*

### 6.4 Secure storage and secrets (`core/storage/app_secure_store.dart`, singleton `AppSecureStore.instance`)

**Library and options**
- `flutter_secure_storage ^10.0.0`.
- Service name: `com.keplr.vizor.secure_store` on mainnet, `com.keplr.vizor.<net>.secure_store` on other networks.
- iOS: `IOSOptions(accountName: service, accessibility: KeychainAccessibility.first_unlock)`.
- Android: `AndroidOptions.defaultOptions` (mainnet), or `sharedPreferencesName: service` on other networks.

**Keys**

| Key | Content |
|---|---|
| `zcash_wallet_db_name` | `zcash_wallet_<24hex>.db` |
| `zcash_accounts` / `zcash_active_account` | account metadata JSON / active uuid |
| `zcash_wallet_network` | network |
| `zcash_theme_mode`, `zcash_privacy_mode_enabled`, `zcash_biometric_unlock_enabled` | plain prefs |
| `zcash_rpc_endpoint_url`, `zcash_rpc_endpoint_preset` | endpoint |
| `zcash_secure_store_salt` | salt for the encryption KDF |
| `zcash_password_verifier`, `zcash_password_verifier_salt` | PBKDF2-HMAC-SHA256 (100k iterations, 32 bytes) verifier, computed in Rust |
| `zcash_rotation_in_progress` | password-change journal |
| `zcash_account_mnemonic_<uuid>` | mnemonic, AES-256-GCM encrypted with a PBKDF2 key derived from the passcode. Payload JSON: `{"v":1,"n":nonce,"c":ciphertext,"m":mac}`. Encrypted and decrypted by `rust_secret.encryptSecretPayload` / `decryptSecretPayload`. |

**Behaviour**
- The session password is held in memory only. `lock()` clears it.
- **The SQLite wallet DB is not encrypted.**
- The DB lives at `getApplicationSupportDirectory()/zcash_wallet_<hex>.db`. Sapling params live in `<appSupport>/sapling_params/`.

**Biometrics**
- There is **no `local_auth`**. A custom channel `com.zcash.wallet/biometric_unlock` escrows the passcode:
  - iOS: a keychain item with `.biometryCurrentSet`.
  - Android: an AndroidKeyStore AES-GCM key with `setUserAuthenticationRequired(true)` and `setInvalidatedByBiometricEnrollment(true)`, unlocked via `BiometricPrompt`. This requires `MainActivity` to be a `FlutterFragmentActivity`.
- `device_owner_auth` gates a wallet reset.

**Privacy**
- Android sets `FLAG_SECURE` through `com.zcash.wallet/privacy_shield`.
- iOS screenshot detection uses `EventChannel('com.zcash.wallet/screenshots')`.
- Sensitive clipboard: `com.zcash.wallet/sensitive_clipboard`, iOS only, 1 minute expiry.

**Background sync**
- iOS only: `BGContinuedProcessingTask` (iOS 26), which calls a separate C FFI (`rust/src/ffi.rs`, `zcash_run_full_sync`).
- Android has no background sync.

---

## 7. flutter_rust_bridge setup

**`flutter_rust_bridge.yaml`** (entire file):

```yaml
rust_input: crate::api,zcash_voting::wire
rust_root: rust/
dart_output: lib/src/rust
```

**Codegen:** run `flutter_rust_bridge_codegen generate` from the repo root. No wrapper script exists. FRB is pinned exactly on both sides: `flutter_rust_bridge = "=2.11.1"` in Rust and `flutter_rust_bridge: 2.11.1` in pubspec.

**Generated output**
- Dart: `lib/src/rust/frb_generated{,.io,.web}.dart` and `lib/src/rust/api/{keystone,secret,simple,sync,voting,voting_config,wallet}.dart`, plus mirrored types.
- Rust: `rust/src/frb_generated.rs`.
- Dart startup must call `await RustLib.init()`.

**`rust/Cargo.toml`**
- Package `rust_lib_zcash_wallet`, `crate-type = ["cdylib", "staticlib", "rlib"]`. `staticlib` is used by iOS and `rlib` by tests.
- `[profile.dev] opt-level = 3`, because proving is unusable at opt-level 0.
- Key dependencies:
  - `zcash_client_backend 0.23 [orchard, transparent-inputs, lightwalletd-tonic, lightwalletd-tonic-tls-webpki-roots, sync, pczt]`
  - `zcash_client_sqlite 0.21 [orchard, transparent-inputs, unstable, serde]`
  - `zcash_primitives 0.28`, `zcash_keys 0.14`, `zcash_protocol 0.9`, `zcash_address 0.12`, `zcash_transparent 0.8`, `zcash_proofs 0.28`
  - `orchard 0.14`, `sapling-crypto 0.7`, `pczt 0.7 [orchard, io-finalizer, signer, zcp-builder]`, `zip32 0.2`
  - `tonic 0.14` + `rustls 0.23` (ring provider installed in `init_app`), `tokio 1`
  - `rusqlite 0.37`, `aes-gcm 0.10`, `pbkdf2 0.12`, `zeroize`, `secrecy`
  - `ur` (git KeystoneHQ/ur-rs 0.3.3), `ur-registry` (git KeystoneHQ/keystone-sdk-rust)

**`rust/src` layout: the api/ vs wallet/ split**

| Module | Purpose |
|---|---|
| `lib.rs` | `pub mod api; pub mod ffi; mod frb_generated; pub mod wallet;` |
| `api/` | **The FRB surface only.** Flat functions taking primitives, `Vec<u8>`, `String` and simple structs, returning `Result<T, String>`. `simple.rs` (`#[frb(init)] init_app()`), `wallet.rs` (create/import/list/delete accounts, addresses, mnemonic, migrate), `sync.rs` (sync stream, balance, validate address, propose/estimate/execute, PCZT create/prove/redact/extract+broadcast, history), `secret.rs`, `keystone.rs`, `voting.rs`. |
| `wallet/` | **Internals, not scanned by codegen:** `db.rs`, `keys.rs`, `keystone.rs` (UR session state), `network.rs`, `secret_payload.rs`, `sync/{pczt,send,transactions}.rs`, `sync_engine/{block_source,enhance,lwd,mempool,error}.rs`. |
| `ffi.rs` | A hand-written C ABI for the Swift background task. It sits outside `api/` on purpose so FRB ignores it. |

**`rust_builder/`** is the standard FRB cargokit plugin template:
- `rust_builder/pubspec.yaml`: plugin `rust_lib_zcash_wallet`, `ffiPlugin: true` for android, ios, linux, macos, windows. The app depends on it via `rust_lib_zcash_wallet: {path: rust_builder}`.
- `rust_builder/cargokit/` contains `build_pod.sh`, `build_tool/`, `cmake/`, `gradle/plugin.gradle` and `run_build_tool.sh`. Cargokit is MIT-licensed; its `LICENSE` file is kept in that directory.
- **Android** (`rust_builder/android/build.gradle`): `apply from: "../cargokit/gradle/plugin.gradle"` with `cargokit { manifestDir = "../../rust"; libname = "rust_lib_zcash_wallet" }`. Gradle builds the `.so` for each ABI with the NDK; `ndkVersion` is inherited from the app.
  - The app's `android/app/build.gradle.kts` uses the `flutter.*` SDK defaults, Java 17 and `androidx.biometric:biometric:1.1.0`.
- **iOS** (`rust_builder/ios/rust_lib_zcash_wallet.podspec`): platform iOS 15.0.
  - A script phase `sh "$PODS_TARGET_SRCROOT/../cargokit/build_pod.sh" ../../rust rust_lib_zcash_wallet` runs before compile.
  - `OTHER_LDFLAGS = -force_load ${BUILT_PRODUCTS_DIR}/librust_lib_zcash_wallet.a`.
  - The Podfile uses `platform :ios, '15.0'` and `use_frameworks!`.
- **Build commands:**
  - Development: `fvm flutter run --dart-define=VIZOR_FORM_FACTOR=mobile`.
  - Release: `fvm flutter build ipa|appbundle --dart-define=VIZOR_FORM_FACTOR=mobile --dart-define=VIZOR_RELEASE_VERSION=...` (fastlane lanes in `fastlane/{ios,android}/Fastfile`).
  - Flutter is pinned by `.fvmrc` to **3.41.6**; the Dart SDK constraint is `^3.11.4`.
- **lightwalletd protos:** there is a Dart gRPC stack (`protos/*.proto`, `lib/src/generated/*.pb*.dart`, `grpc ^5.1.0`), but it is used only by the import birthday estimator. All other network traffic goes through Rust tonic.

Other notable pubspec dependencies:
- `go_router ^17.1.0`, `flutter_svg ^2.0.10`, `pretty_qr_code ^3.6.0`, `qr_flutter ^4.1.0`, `share_plus ^12.0.1`, `path_provider`, `url_launcher`, `shared_preferences`.
- `mobile_scanner` (git fork `chainapsis/mobile_scanner`).
- Dev: `widgetbook ^3.14.3`.

---

## 8. Copy conventions and tone

From `AGENTS.md` "UI Copy Conventions", confirmed across the code.

**Casing rules**
- **Sentence case everywhere** for buttons, toasts, sheet titles, labels, errors and empty states. Capitalize only the first word and proper nouns.
- Canonical acronyms and brands keep their casing: `ZEC`, `Zcash`, `Keystone`, `Vizor`.
- Interpolated labels stay sentence case: `'$symbol deposit tx'`.
- **Exceptions:** screen titles, onboarding step titles and Figma display headings may be Title Case. Examples: "Select Recipient", "Review Send", "Secret Passphrase", "Create Passcode", "Welcome Back".
- The code is not perfectly consistent: "Confirm & Send", "Add Memo" and "Continue Anyway" still exist.

**Tone**
- Short, calm and direct, often second person.
- Errors say what happened and what to do: "Couldn't load activity. Try again in a moment."
- Progress labels use a trailing ellipsis made of three dots ("Sending...", "Preparing...", "Reading signature..."). Vizor sometimes adds a space before it ("Loading the QR code ...").
- Privacy is framed positively: "Strong privacy by default.", "Only the recipient can read this".

**CTA vocabulary**

| Situation | Label |
|---|---|
| Next step | "Continue" (often with a trailing chevron) |
| Final step | "Finish & review", "Confirm & Send", "Confirm & import" |
| Hardware | "Get signature", "Scan signature", "Show QR", "Next step" |
| Dismissal | "Cancel", "Close", "Not now", "Back to wallet", "Go back" |
| Retry | "Try again", "Request again", "Retry" |
| Disabled CTA carrying its reason | "Enter address to continue", "Enter amount to continue", "Not enough ZEC" |

**Toasts** are past-tense confirmations: "Address copied", "Passcode updated", "Shielding complete", "Endpoint updated".

**Labels** are nouns: "Amount", "To", "From", "Status", "Message", "Timestamp", "Tx ID", "Tx fee", "Sending to".

**Status words:** "In progress", "Completed", "Failed, funds returned", "Pending".

**Audit files:** `qa-copy-review.csv` and `copy-review-20260528-1554.csv` are referenced by AGENTS.md but are **not in this checkout**.

---

## 9. What to copy vs. adapt for Zafe

| Vizor | Zafe | Action |
|---|---|---|
| Token system (`primitives.dart`, `colors/*`, `AppTheme`/`context.colors`, `AppSpacing`, `AppRadii`, `AppTypographyMobile`, sizing) | Same | **Copy** (with attribution). Consider a different brand hue than crimson so Zafe is not mistaken for Vizor; changing the Crimson ladder alone re-skins the brand. |
| `AppButton`, `MobileTextField`, `MobileSurfaceCard`, `MobileListRow`, `MobileReviewInfoRow`, `ReviewWrapCard`, `showAppMobileSheet` + `MobileModalCard` + `MobileModalScaffold`, `AppToast`, `AppIcon`/`AppIcons`, `AppHaptics`, `DotQrShape`, `MobileTopNav`, `AppMobileTabBar`, `AppMobileShell`, `MobileBottomSafeArea`, `MobileTopScrollFade` | Same | **Copy** these nearly verbatim. They are self-contained and depend only on the tokens. |
| Tab shell (Home / Swap / Activity / Settings) | Home (vault) / Activity (incl. pending approvals) / Settings, perhaps an Approvals tab | **Adapt.** Keep `StatefulShellRoute.indexedStack` and the single `_allMobileTabs` list. |
| Keystone account (`isHardware`, UFVK-only watch account, `MobileAccountAvatar` Keystone badge) | FROST vault account (group UFVK, participant role, threshold) | **Direct map.** The vault is a watch-only account whose spends require external signatures. Reuse the badge pattern ("vault" glyph, "t-of-n" chip). |
| PCZT pipeline: propose → `createPcztFromProposal` → `redactPcztForSigner` → *(signing)* → prove in parallel → `extractAndBroadcastPczt` | Same pipeline; signing = FROST rounds over the blind relay instead of a QR round-trip | **Copy the pipeline and the "prove while signing" trick.** Replace UR/QR transport with relay messages. Keep `sendFlowId`, `discardSendProposal` on cancel, and the broadcast status set (`broadcasted` / `broadcast_unknown` / `broadcasted_storage_failed`). |
| `MobileKeystonePcztSigningFlow` states `{preparing, showQr, scanning, failed}` + modal card | `{preparing, awaitingApprovals(k of t), aggregating, failed}` | **Adapt.** Keep the modal-card frame, the title/subtitle, the one-line instruction slot, a primary action, "Close", and pop-null-means-cancel. Replace the QR stage with a signer list (avatar + name + status chip "Approved" / "Waiting"). |
| Send wizard (recipient → amount → review → status) | Proposal wizard: create proposal → co-signers approve → broadcast | **Copy the layouts** (floating address field, serif amount, review rows, detail card, status chip). Add an approvals section to review/status. |
| Status chip vocabulary ("In progress" / "Completed" / "Failed, funds returned") | Add "Awaiting approvals (1 of 2)", "Rejected", "Expired" | Adapt. |
| Receive screen (shielded/transparent tabs, dot QR, renew diversified address) | Vault receive address | Copy. Consider dropping the transparent tab if vaults are shielded-only. |
| Activity feed sections and rows | Same, plus proposal rows | Copy. |
| Passcode + biometrics (custom channel escrow), privacy overlay, sensitive clipboard, `FLAG_SECURE` | Same needs for local key-share protection | Copy the patterns. The native channel code must be rewritten or ported (Swift/Kotlin handlers). |
| Secure store (PBKDF2 verifier, AES-GCM payloads, rotation journal) | Store FROST key shares instead of mnemonics | Adapt. Also consider encrypting the wallet DB, which Vizor does not. |
| Mnemonic onboarding (create / import / seed display / birthday) | Not applicable as-is (FROST DKG or share import instead) | **Drop.** Keep the step-scaffold, progress bar and passcode steps; reuse the Keystone "select account" radio cards and the birthday screen for vault import. |
| Swap (NEAR Intents), voting (`zcash_voting`), desktop shell, Windows/Linux updaters, iOS 26 background task | Not needed | **Drop.** |
| Knight illustrations, profile pictures, Vizor wordmark, crimson shield QR badge | Must be Zafe's own | **Do not copy** (section 10). |

---

## 10. Licensing

**What Vizor's license covers**
- Vizor is **Apache License 2.0** ("Copyright 2026 Vizor contributors", `LICENSE`).
- There is **no `NOTICE` file**, and source files carry **no per-file license headers**.

**Obligations when Zafe copies Vizor code (Apache-2.0 §4)**
1. Include a copy of the Apache-2.0 license with Zafe's distribution, e.g. `third_party/vizor/LICENSE` or in-app via `LicenseRegistry`.
2. Mark modified files with a prominent notice. Recommended header on each copied or derived Dart file:
   `// Portions derived from Vizor (https://github.com/chainapsis/vizor-wallet), Copyright 2026 Vizor contributors, Apache-2.0. Modified by Zafe.`
3. Keep any existing copyright or attribution notices. There are none in-file today, but keep the cargokit `LICENSE` if its code is copied.
4. Vizor has no NOTICE file, so nothing must be propagated from one. Adding a Zafe `NOTICE` that credits Vizor is good practice.
5. **Trademarks are not licensed (§6).** Do not use "Vizor", the wordmark (`AppIcons.vizor`, `VizorWordmark`), Keplr branding, or confusingly similar trade dress. Also rename channel names (`com.zcash.wallet/*`), bundle IDs (`com.keplr.vizor`), secure-store service names, and every piece of copy containing "Vizor".

**Assets with separate or unclear licensing**
- **Fonts: OFL 1.1, not Apache.** Geist, Geist Mono and Young Serif are verified from the embedded name tables; Inter's embedded strings point to openfontlicense.org. The repo omits the OFL texts, so Zafe must add the OFL license and copyright lines for each bundled family.
  - Obtaining fonts from their upstream releases (vercel/geist-font, noirblancrouge/YoungSerif) is cleaner than copying them from Vizor.
  - The OFL forbids selling the fonts alone and restricts use of the Reserved Font Names in modified versions. Bundling them unmodified in an app is fine.
- **Illustrations and profile pictures** (`assets/illustrations/`, `assets/profile_pictures/`): the bespoke knight artwork has no separate license and is technically covered by the repo's Apache-2.0. It is, however, Vizor's brand identity, and its provenance is undocumented (it may be commissioned or AI-assisted).
  - **Recommendation:** do not reuse it. Commission or create Zafe art for the same slots.
- **Icons** (`assets/icons/`): Apache-2.0 via the repo. They are generic and reusable with attribution, **except** the brand-bearing ones: `vizor.svg`, `keystone.svg` (Keystone's trademark, which is fine to use nominatively when referring to Keystone), `github.svg`, `near_intents_*`, `zcash*.svg` and `network_zec.png` (Zcash marks, governed by the Zcash trademark policy), and the `receive_qr_*` badges.
- **Token and chain logos** (`assets/swap/`): third-party trademarks. Not needed by Zafe.
- **Cargokit** (`rust_builder/cargokit/`): has its own `LICENSE`. Zafe will likely generate its own `rust_builder` with `flutter_rust_bridge_codegen create`/`integrate` rather than copy Vizor's.
- **`mobile_scanner` and `desktop_window_bootstrap`:** git forks under `chainapsis/`. Check each fork's license before depending on it; upstream `mobile_scanner` is BSD-3-Clause. Prefer the pub.dev release unless the fork's fix is needed.

**Practical recommendation**
- Copy the Dart design-system and widget code with headers plus a Zafe `NOTICE`.
- Take fonts from upstream with their OFL files.
- Do not copy art or brand assets.
- Re-derive the Rust side against Zafe's own crates rather than copying `rust/src`, which is heavily Vizor-specific.
