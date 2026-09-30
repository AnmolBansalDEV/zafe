import 'package:flutter/widgets.dart';
import 'package:flutter_svg/flutter_svg.dart';

import '../theme/app_theme.dart';
import 'app_loading_icon.dart';

/// Name-based handles for every design-system icon.
///
/// Keeping the asset paths behind typed constants means a rename /
/// re-export only touches this file; call sites reference
/// `AppIcons.book` instead of a string literal.
///
/// Names mostly follow the curated export set under `assets/icons/`: Figma
/// Icons page node names normalized to snake_case, with a few
/// compatibility-preserving aliases where older app code already shipped
/// stable names (`import_wallet`, `shield_keyhole`, `arrow_upward`, etc.).
/// [loader] is code-drawn rather than loaded from SVG.
abstract final class AppIcons {
  static const addNew = 'add_new';
  static const arrowBack = 'arrow_back';
  static const arrowBottomLeft = 'arrow_bottom_left';
  static const arrowDown = 'arrow_down';
  static const arrowDownCircle = 'arrow_down_circle';
  static const arrowDownward = 'arrow_downward';
  static const arrowForwardIos = 'arrow_forward_ios';
  static const arrowTopRight = 'arrow_top_right';
  static const arrowUpward = 'arrow_upward';
  static const backspace = 'backspace';
  static const bell = 'bell';
  static const block = 'block';
  static const book = 'book';
  static const calendar = 'calendar';
  static const camera = 'camera';
  static const cameraDenied = 'camera_denied';
  static const cancel = 'cancel';
  static const check = 'check';
  static const checkCircle = 'check_circle';
  static const chevronBackward = 'chevron_backward';
  static const chevronForward = 'chevron_forward';
  static const collapsed = 'collapsed';
  static const cog = 'cog';
  static const coins = 'coins';
  static const coinholderVoting = 'coinholder_voting';
  static const copy = 'copy';
  static const cross = 'cross';
  static const crystalBall = 'crystal_ball';
  static const day = 'day';
  static const doubleArrowVertical = 'double_arrow_vertical';
  static const donation = 'donation';
  static const dragon = 'dragon';
  static const edit = 'edit';
  static const editFilled = 'edit_filled';
  static const endpoint = 'endpoint';
  static const eye = 'eye';
  static const eyeClosed = 'eye_closed';
  static const expand = 'expand';
  static const faceId = 'face_id';
  static const touchId = 'touch_id';
  static const filter = 'filter';
  static const github = 'github';
  static const giftCard = 'gift_card';
  static const giftCardOutline = 'gift_card_outline';
  static const globe = 'globe';
  static const help = 'help';
  static const history = 'history';
  static const home = 'home';
  static const importWallet = 'import_wallet';
  static const key = 'key';
  static const keystone = 'keystone';
  static const keystoneScan = 'keystone_scan';
  static const ledger = 'ledger';
  static const ledgerBrand = 'ledger_brand';
  static const link = 'link';
  static const loader = 'loader';
  static const lock = 'lock';
  static const logOut = 'log_out';
  static const monitor = 'monitor';
  static const moneyBag = 'money_bag';
  static const migrationFast = 'migration_fast';
  static const ironwoodMigrationLoader = 'ironwood_migration_loader';
  static const migrationSign = 'migration_sign';
  static const migrationSplit = 'migration_split';
  static const migrationTimer = 'migration_timer';
  static const minus = 'minus';
  static const night = 'night';
  static const notificationBell = 'notification_bell';
  static const options = 'options';
  static const paid = 'paid';
  static const paste = 'paste';
  static const pause = 'pause';
  static const plane = 'plane';
  static const play = 'play';
  static const plus = 'plus';
  static const qr = 'qr';
  static const qrCodeFill = 'qr_code_fill';
  static const renew = 'renew';
  static const scroll = 'scroll';
  static const search = 'search';
  static const share = 'share';
  static const shieldAsset = 'shield_asset';
  static const shieldKeyhole = 'shield_keyhole';
  static const shieldKeyholeOutline = 'shield_keyhole_outline';
  static const skip = 'skip';
  static const skull = 'skull';
  static const swapArrows = 'swap_arrows';
  static const subArrow = 'sub_arrow';
  static const sync = 'sync';
  static const theme = 'theme';
  static const time = 'time';
  static const tor = 'tor';
  static const transparentBalance = 'transparent_balance';
  static const unlock = 'unlock';
  static const trash = 'trash';
  static const user = 'user';
  static const users = 'users';
  static const uturnUp = 'uturn_up';
  static const vote = 'vote';
  static const wallet = 'wallet';
  static const warning = 'warning';
  static const warningCircle = 'warning_circle';
  static const wrench = 'wrench';
  static const zcash = 'zcash';
  static const zcashCurrency = 'zcash_currency';

  /// Icons drawn in the Patina two-tone style (docs/brand.md §4), from Phosphor
  /// Icons (MIT) by `scripts/brand/patina_icons.py` into `assets/icons/patina/`.
  /// Every other name still renders its filled `assets/icons/<name>.svg`.
  /// Keep in sync with the script's `MAP`.
  static const patina = {
    home,
    history,
    users,
    cog,
    plane,
    arrowDownCircle,
    addNew,
    warning,
    warningCircle,
    share,
    chevronForward,
    chevronBackward,
    eye,
    eyeClosed,
    shieldKeyhole,
    editFilled,
    time,
    cross,
    check,
    checkCircle,
    arrowDown,
  };
}

/// Renders a Figma-exported icon from `assets/icons/<name>.svg`, or a Patina
/// two-tone icon from `assets/icons/patina/` for names in [AppIcons.patina]:
/// idle, a rounded 1.7 px outline in [color] with closed shapes filled with
/// [patina] (default: the brand accent) at 38%; [active], the solid Phosphor
/// fill in [color] (pass the accent: current tab, a payment that needs you).
///
/// SVGs preserve their Figma icon frame in the viewBox, so the art's
/// position inside the frame is preserved when Flutter scales the SVG to any
/// [size]. Colour is swapped at paint time via
/// `ColorFilter.mode(color, BlendMode.srcIn)`:
/// the SVG keeps its own alpha shape, every non-transparent pixel gets
/// re-colored to [color]. Works for the single-fill monochrome icons
/// the design system currently ships; multi-fill / gradient icons
/// would need a different render path. [AppIcons.loader] is code-drawn so it
/// can animate while keeping the same public [AppIcon] call pattern.
///
/// Color resolution order, first non-null wins:
/// 1. Explicit [color] argument at the call site.
/// 2. `IconTheme.of(context).color` — ambient theme. This is what lets
///    a widget like [AppButton] merge its label color into an
///    `IconTheme` wrapper and have every `AppIcon` child pick it up
///    without the caller having to thread the color through.
/// 3. [AppColors.icon.regular] from the ambient [AppTheme] — default
///    "neutral icon" when no other context has spoken.
class AppIcon extends StatelessWidget {
  const AppIcon(
    this.name, {
    this.size = AppIconSize.medium,
    this.color,
    this.animated = true,
    this.semanticLabel,
    this.active = false,
    this.patina,
    super.key,
  });

  /// Patina icons only: draw the solid (filled) form, for state.
  final bool active;

  /// Patina icons only: colour of the closed shapes (drawn at 38%). Defaults to
  /// the brand accent; pass the icon colour for semantic icons (warning, value).
  final Color? patina;

  /// Pass a constant from [AppIcons] — e.g. `AppIcons.book`.
  final String name;

  /// Rendered pixel size (width == height). Defaults to
  /// [AppIconSize.medium] (16); use [AppIconSize.large] (24) for the
  /// asset's native pixel-perfect size.
  final double size;

  /// Tint applied via `BlendMode.srcIn`. When null, falls back through
  /// ambient `IconTheme` and then [AppColors.icon.regular] — see the
  /// class doc for the full resolution order.
  final Color? color;

  /// Controls the pulse animation for [AppIcons.loader]. Ignored by SVG icons.
  final bool animated;

  /// Optional accessibility label forwarded to [SvgPicture.asset].
  final String? semanticLabel;

  @override
  Widget build(BuildContext context) {
    final resolved =
        color ?? IconTheme.of(context).color ?? context.colors.icon.regular;
    if (name == AppIcons.loader) {
      return AppLoadingIcon(
        size: size,
        color: resolved,
        animated: animated,
        semanticLabel: semanticLabel,
      );
    }
    if (AppIcons.patina.contains(name)) {
      if (active) {
        return SvgPicture.asset(
          'assets/icons/patina/${name}_active.svg',
          width: size,
          height: size,
          colorFilter: ColorFilter.mode(resolved, BlendMode.srcIn),
          semanticsLabel: semanticLabel,
        );
      }
      final fill = patina ?? context.colors.icon.brand;
      return SvgPicture.asset(
        'assets/icons/patina/$name.svg',
        width: size,
        height: size,
        colorMapper: PatinaColorMapper(
          outline: resolved,
          fill: fill.withValues(alpha: fill.a * kPatinaFillOpacity),
        ),
        semanticsLabel: semanticLabel,
      );
    }
    return SvgPicture.asset(
      'assets/icons/$name.svg',
      width: size,
      height: size,
      colorFilter: ColorFilter.mode(resolved, BlendMode.srcIn),
      semanticsLabel: semanticLabel,
    );
  }
}

/// Opacity of a Patina icon's closed shapes.
const kPatinaFillOpacity = 0.38;

/// Recolours a Patina SVG: black (#000000) is the outline, magenta (#FF00FF) the
/// closed shapes. Value equality keeps flutter_svg's picture cache working.
@immutable
class PatinaColorMapper extends ColorMapper {
  const PatinaColorMapper({required this.outline, required this.fill});
  final Color outline;
  final Color fill;

  static const _outlineKey = Color(0xFF000000);
  static const _fillKey = Color(0xFFFF00FF);

  @override
  Color substitute(
    String? id,
    String elementName,
    String attributeName,
    Color color,
  ) {
    if (color == _outlineKey) return outline;
    if (color == _fillKey) return fill;
    return color;
  }

  @override
  bool operator ==(Object other) =>
      other is PatinaColorMapper &&
      other.outline == outline &&
      other.fill == fill;

  @override
  int get hashCode => Object.hash(outline, fill);
}
