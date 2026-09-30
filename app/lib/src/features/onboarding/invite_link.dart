/// Invite links: `zafe://join?invite=<invite>`, the tappable and scannable form of a raw
/// invite (`zafe-invite-v1:<hex>`). Pure string handling; the invite itself is validated
/// by Rust (`parseInvite`) wherever it's used.
library;

const kInviteLinkScheme = 'zafe';
const kInviteLinkHost = 'join';

/// Every raw invite starts with this (the version follows it).
const _rawInvitePrefix = 'zafe-invite-';

/// Characters that may appear unescaped in a query value (RFC 3986 unreserved plus ':').
final _urlSafe = RegExp(r'^[A-Za-z0-9._~:-]+$');

/// The link that opens Zafe's Join screen with [invite] filled in. Today's invites
/// (prefix + hex) are URL-safe and embedded as they are.
String inviteLink(String invite) {
  final raw = invite.trim();
  final value = _urlSafe.hasMatch(raw) ? raw : Uri.encodeQueryComponent(raw);
  return '$kInviteLinkScheme://$kInviteLinkHost?invite=$value';
}

/// The raw invite carried by [uri], or null when it isn't a Zafe invite link.
String? inviteFromLink(Uri uri) {
  if (uri.scheme.toLowerCase() != kInviteLinkScheme) return null;
  if (uri.host.toLowerCase() != kInviteLinkHost) return null;
  if (uri.path.isNotEmpty && uri.path != '/') return null;
  final List<String> values;
  try {
    values = uri.queryParametersAll['invite'] ?? const [];
  } on FormatException {
    return null; // bad percent-encoding
  }
  if (values.length != 1) return null;
  final invite = values.single.trim();
  return invite.startsWith(_rawInvitePrefix) ? invite : null;
}

/// The raw invite in [text] (pasted, scanned or typed): a raw invite or an invite link,
/// alone or inside a shared message. Null when there's none. The result still needs
/// `parseInvite`.
String? extractInvite(String text) {
  for (final token in text.trim().split(RegExp(r'\s+'))) {
    final invite = _inviteIn(token);
    if (invite != null) return invite;
  }
  return null;
}

String? _inviteIn(String token) {
  if (token.startsWith(_rawInvitePrefix)) return token;
  if (!token.toLowerCase().startsWith('$kInviteLinkScheme:')) return null;
  final uri = Uri.tryParse(token);
  return uri == null ? null : inviteFromLink(uri);
}
