// Invite landing page: https://<host>/join#<invite>. Reached only when Zafe isn't
// installed (or App Links / Universal Links didn't take it). The invite is read from the
// fragment, which browsers never send to the server, and never leaves this page except
// through "Open in Zafe" and "Copy invite".
'use strict';

(function () {
  var PREFIX = 'zafe-invite-';
  var KEY = 'zafe-invite';
  var URL_SAFE = /^[A-Za-z0-9._~:-]+$/;

  function valid(invite) {
    return typeof invite === 'string' && invite.indexOf(PREFIX) === 0 &&
      invite.length < 4096 && !/\s/.test(invite);
  }

  function fromFragment() {
    var hash = location.hash.slice(1);
    if (!hash) return null;
    try {
      return decodeURIComponent(hash).trim();
    } catch (e) {
      return null; // bad percent-encoding
    }
  }

  // Tab-scoped, so coming back from the download page still has the invite, while the
  // address bar (and so browser history and history sync) no longer shows it.
  function remember(invite) {
    try { sessionStorage.setItem(KEY, invite); } catch (e) { /* private mode */ }
  }

  function remembered() {
    try { return sessionStorage.getItem(KEY); } catch (e) { return null; }
  }

  var invite = fromFragment();
  if (valid(invite)) {
    remember(invite);
  } else if (invite === null) {
    invite = remembered();
  }
  if (location.hash) history.replaceState(null, '', location.pathname);

  if (!valid(invite)) {
    document.getElementById('missing').hidden = false;
    return;
  }

  // The same form the app shares when it has no link host (invite_link.dart).
  var value = URL_SAFE.test(invite) ? invite : encodeURIComponent(invite);
  document.getElementById('open').href = 'zafe://join?invite=' + value;

  var status = document.getElementById('copied');
  document.getElementById('copy').addEventListener('click', function () {
    function done() { status.textContent = 'Copied. Paste it on Zafe\'s Join screen.'; }
    function failed() { status.textContent = 'Couldn\'t copy. Paste the whole link instead.'; }
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(invite).then(done, failed);
    } else {
      failed();
    }
  });

  document.getElementById('invite').hidden = false;
})();
