/// ZIP 321 payment links (`zcash:...`) opened from other apps or websites ("Pay with
/// Zcash"). Pure, so the rules are unit-tested without Rust or a router.
library;

/// A `zcash:` link waiting to be shown on the payment request screen.
class PendingPaymentLink {
  const PendingPaymentLink(this.raw, this.receivedAt);

  /// The link exactly as the system delivered it (trimmed); parsed by Rust later.
  final String raw;
  final DateTime receivedAt;
}

/// A link that waited longer than this (the app was locked, or a vault was being set
/// up) is dropped instead of shown: paying a request opened long ago, maybe by accident,
/// is worse than asking for the link again.
const kPaymentLinkTtl = Duration(minutes: 10);

/// Shown when [kPaymentLinkTtl] ran out before the link could be shown.
const kPaymentLinkExpiredMessage =
    'A payment link expired before you unlocked. Open it again to pay';

/// Shown when a link arrives on a device with no vault to pay from.
const kPaymentLinkNoVaultMessage =
    'Set up or join a vault before paying a Zcash payment link';

/// Whether [raw] is a ZIP 321 payment link (case-insensitive `zcash:` scheme). Matched
/// on the string, since a malformed link must still reach the screen that says so.
bool isPaymentLink(String raw) =>
    raw.trimLeft().toLowerCase().startsWith('zcash:');

/// Whether [link] waited too long to be shown at [now].
bool paymentLinkExpired(PendingPaymentLink link, DateTime now) =>
    now.difference(link.receivedAt) > kPaymentLinkTtl;
