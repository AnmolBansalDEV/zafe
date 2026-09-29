import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/config/network_config.dart';
import '../core/errors/zafe_error_copy.dart';
import '../core/storage/zafe_paths.dart';
import '../core/storage/vault_summaries.dart';
import '../notifications/vault_updates.dart' show actionableCount;
import '../notifications/vault_watch.dart' show recordSeen;
import '../rust/api/proposals.dart' as rust;
import 'vault_provider.dart';

/// A send in progress (or just failed) on this device.
class SendState {
  const SendState({this.progress, this.error});

  /// Latest progress; `null` until the first event.
  final rust.SendProgress? progress;

  /// Friendly error once a send attempt failed (the send is then no longer running).
  final String? error;

  bool get running => error == null && progress?.stage != rust.SendStage.sent;
}

class ProposalsState {
  const ProposalsState({
    this.items = const [],
    this.loaded = false,
    this.error,
    this.sends = const {},
  });

  final List<rust.ProposalInfo> items;
  final bool loaded;
  final Object? error;

  /// Sends started on this device, by proposal id.
  final Map<String, SendState> sends;

  rust.ProposalInfo? byId(String id) {
    for (final p in items) {
      if (p.id == id) return p;
    }
    return null;
  }

  ProposalsState copyWith({
    List<rust.ProposalInfo>? items,
    bool? loaded,
    Object? error,
    bool clearError = false,
    Map<String, SendState>? sends,
  }) => ProposalsState(
    items: items ?? this.items,
    loaded: loaded ?? this.loaded,
    error: clearError ? null : (error ?? this.error),
    sends: sends ?? this.sends,
  );
}

/// Vault proposals from the log, plus the member actions on them.
///
/// Each refresh also keeps this device's one-tap commitments topped up (in Rust), answers
/// interactive signing requests, and starts the auto-send for any proposal whose
/// signatures this member's approval completed (so it survives leaving the screen or
/// restarting the app).
class ProposalsNotifier extends Notifier<ProposalsState> {
  bool _refreshing = false;
  final _subscriptions = <String, StreamSubscription<rust.SendProgress>>{};

  @override
  ProposalsState build() {
    // A different vault on screen starts from scratch (and cancels this one's listeners).
    ref.watch(vaultProvider.select((v) => v.activeId));
    ref.onDispose(() {
      for (final s in _subscriptions.values) {
        s.cancel();
      }
    });
    return const ProposalsState();
  }

  VaultState get _vault => ref.read(vaultProvider);

  Future<void> refresh() async {
    final vault = _vault;
    if (!vault.hasVault || _refreshing) return;
    _refreshing = true;
    try {
      final paths = await ZafePaths.get();
      final items = await rust.listProposals(
        relayUrl: kZafeRelayUrl,
        stateDir: await paths.stateDir(vault.activeId!),
        seeds: vault.identity!,
        material: vault.material!,
      );
      state = state.copyWith(items: items, loaded: true, clearError: true);
      // Seen on screen: never announced from the background. Only while the app is in
      // the foreground; a refresh running in the background must not swallow news.
      if (WidgetsBinding.instance.lifecycleState == AppLifecycleState.resumed) {
        unawaited(recordSeen(vault.activeId!, items));
      }
      unawaited(
        VaultSummaries.write(
          vault.activeId!,
          actionable: actionableCount(items),
        ),
      );
      _autoSend();
      await _answerRequests(paths);
    } catch (e) {
      debugPrint('refresh failed: ${describeError(e)}');
      state = state.copyWith(error: e);
    } finally {
      _refreshing = false;
    }
  }

  /// The member whose approval completed the signatures sends, if the proposer asked for
  /// it. A failed attempt is not retried automatically (the screen offers "Try again").
  void _autoSend() {
    for (final p in state.items) {
      final due =
          p.stage == rust.ProposalStage.approved &&
          p.ready &&
          p.autoSend &&
          p.completedByMe;
      if (due && !state.sends.containsKey(p.id)) startSend(p.id);
    }
  }

  Future<void> _answerRequests(ZafePaths paths) async {
    final vault = _vault;
    try {
      await rust.answerSigningRequests(
        relayUrl: kZafeRelayUrl,
        lightwalletdUrl: kZafeLightwalletdUrl,
        dbDir: paths.dbDir,
        stateDir: await paths.stateDir(vault.activeId!),
        seeds: vault.identity!,
        material: vault.material!,
      );
    } catch (_) {
      // Not synced yet or offline: the next poll retries.
    }
  }

  Future<String> propose({
    required String address,
    required BigInt amountZat,
    required String memo,
    required bool autoSend,
  }) async {
    final vault = _vault;
    final paths = await ZafePaths.get();
    final id = await rust.proposePayment(
      relayUrl: kZafeRelayUrl,
      lightwalletdUrl: kZafeLightwalletdUrl,
      dbDir: paths.dbDir,
      seeds: vault.identity!,
      material: vault.material!,
      payments: [
        rust.PaymentInput(address: address, amountZat: amountZat, memo: memo),
      ],
      autoSend: autoSend,
    );
    await refresh();
    return id;
  }

  Future<rust.ReviewInfo> review(String id) async {
    final vault = _vault;
    final paths = await ZafePaths.get();
    return rust.reviewProposal(
      relayUrl: kZafeRelayUrl,
      lightwalletdUrl: kZafeLightwalletdUrl,
      dbDir: paths.dbDir,
      seeds: vault.identity!,
      material: vault.material!,
      proposalId: id,
    );
  }

  /// Approves (and, for one-tap proposals, signs). If this approval completed the
  /// signatures and the proposer asked for auto-send, the refresh starts sending.
  Future<rust.ApproveResult> approve(String id) async {
    final vault = _vault;
    final paths = await ZafePaths.get();
    final result = await rust.approveProposal(
      relayUrl: kZafeRelayUrl,
      lightwalletdUrl: kZafeLightwalletdUrl,
      dbDir: paths.dbDir,
      stateDir: await paths.stateDir(vault.activeId!),
      seeds: vault.identity!,
      material: vault.material!,
      proposalId: id,
    );
    await refresh();
    return result;
  }

  Future<void> reject(String id) async {
    final vault = _vault;
    await rust.rejectProposal(
      relayUrl: kZafeRelayUrl,
      seeds: vault.identity!,
      material: vault.material!,
      proposalId: id,
    );
    await refresh();
  }

  /// Sends a proposal: directly from the approvals' signatures when they are complete
  /// (one tap), otherwise by asking the approvers to sign (interactive). Progress and
  /// errors land in `state.sends[id]`.
  Future<void> startSend(String id) async {
    if (state.sends[id]?.running ?? false) return;
    _setSend(id, const SendState());
    final vault = _vault;
    final paths = await ZafePaths.get();
    await _subscriptions.remove(id)?.cancel();
    _subscriptions[id] = rust
        .sendProposal(
          relayUrl: kZafeRelayUrl,
          lightwalletdUrl: kZafeLightwalletdUrl,
          dbDir: paths.dbDir,
          stateDir: await paths.stateDir(vault.activeId!),
          seeds: vault.identity!,
          material: vault.material!,
          proposalId: id,
        )
        .listen(
          (p) {
            final error = p.error;
            if (p.stage == rust.SendStage.failed && error != null) {
              _fail(id, error);
            } else {
              _setSend(id, SendState(progress: p));
            }
          },
          onError: (Object e) => _fail(id, e),
          onDone: () {
            _subscriptions.remove(id);
            unawaited(refresh());
            unawaited(ref.read(vaultProvider.notifier).sync());
          },
        );
  }

  void _fail(String id, Object e) {
    debugPrint('send failed: ${describeError(e)}');
    _setSend(
      id,
      SendState(
        error: zafeErrorMessage(e, fallback: 'Send failed. Try again.'),
      ),
    );
  }

  void _setSend(String id, SendState send) {
    state = state.copyWith(sends: {...state.sends, id: send});
  }
}

final proposalsProvider = NotifierProvider<ProposalsNotifier, ProposalsState>(
  ProposalsNotifier.new,
);

/// This device's independent check of one proposal (re-run when the page opens).
final proposalReviewProvider = FutureProvider.autoDispose
    .family<rust.ReviewInfo, String>(
      (ref, id) => ref.read(proposalsProvider.notifier).review(id),
    );
