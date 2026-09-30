import 'dart:async';

import 'package:app_links/app_links.dart';
import 'package:flutter/foundation.dart';

import '../features/onboarding/invite_link.dart';

/// Invites from opened invite links (`zafe://join?invite=...`, or
/// `https://<ZAFE_LINK_HOST>/join#...` through App Links / Universal Links), for the router
/// to show on the Join screen. Holds the raw invite, or the whole link when it doesn't carry
/// a valid one (the Join screen then says it isn't an invite). Links never join by
/// themselves.
final inviteLinks = ValueNotifier<String?>(null);

StreamSubscription<Uri>? _subscription;

/// Listens for invite links. The stream also delivers the link that launched the app
/// (cold start), so there is no separate initial-link read.
void initInviteLinks() {
  _subscription ??= AppLinks().uriLinkStream.listen((uri) {
    if (!isInviteLink(uri)) return;
    inviteLinks.value = inviteFromLink(uri) ?? uri.toString();
  }, onError: (Object e) => debugPrint('invite link: $e'));
}
