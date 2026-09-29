import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/config/network_config.dart';
import '../core/storage/zafe_paths.dart';
import '../rust/api/proposals.dart' as rust;
import 'vault_provider.dart';

class ProposalsState {
  const ProposalsState({
    this.items = const [],
    this.loaded = false,
    this.error,
  });

  final List<rust.ProposalInfo> items;
  final bool loaded;
  final Object? error;

  rust.ProposalInfo? byId(String id) {
    for (final p in items) {
      if (p.id == id) return p;
    }
    return null;
  }

  /// Proposals that need something from this member (vote, or send once approved).
  int get actionable => items
      .where(
        (p) =>
            (p.stage == rust.ProposalStage.open &&
                p.myVote == rust.MyVote.none) ||
            p.stage == rust.ProposalStage.approved,
      )
      .length;
}

/// Vault proposals from the log, plus the member actions on them. Also answers signing
/// requests on each refresh, so a member who approved signs as soon as the app is open.
class ProposalsNotifier extends Notifier<ProposalsState> {
  bool _refreshing = false;

  @override
  ProposalsState build() => const ProposalsState();

  VaultState get _vault => ref.read(vaultProvider);

  Future<void> refresh() async {
    final vault = _vault;
    if (!vault.hasVault || _refreshing) return;
    _refreshing = true;
    try {
      final paths = await ZafePaths.get();
      final items = await rust.listProposals(
        relayUrl: kZafeRelayUrl,
        stateDir: paths.stateDir,
        seeds: vault.identity!,
        material: vault.material!,
      );
      state = ProposalsState(items: items, loaded: true);
      await _answerRequests(paths);
    } catch (e) {
      state = ProposalsState(
        items: state.items,
        loaded: state.loaded,
        error: e,
      );
    } finally {
      _refreshing = false;
    }
  }

  Future<void> _answerRequests(ZafePaths paths) async {
    final vault = _vault;
    try {
      await rust.answerSigningRequests(
        relayUrl: kZafeRelayUrl,
        lightwalletdUrl: kZafeLightwalletdUrl,
        dbDir: paths.dbDir,
        stateDir: paths.stateDir,
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

  Future<void> approve(String id) async {
    final vault = _vault;
    final paths = await ZafePaths.get();
    await rust.approveProposal(
      relayUrl: kZafeRelayUrl,
      lightwalletdUrl: kZafeLightwalletdUrl,
      dbDir: paths.dbDir,
      stateDir: paths.stateDir,
      seeds: vault.identity!,
      material: vault.material!,
      proposalId: id,
    );
    await refresh();
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

  /// Leader: collect signatures and broadcast. Emits progress; completes after `sent`.
  Stream<rust.SendProgress> send(String id) async* {
    final vault = _vault;
    final paths = await ZafePaths.get();
    yield* rust.sendProposal(
      relayUrl: kZafeRelayUrl,
      lightwalletdUrl: kZafeLightwalletdUrl,
      dbDir: paths.dbDir,
      stateDir: paths.stateDir,
      seeds: vault.identity!,
      material: vault.material!,
      proposalId: id,
    );
    unawaited(refresh());
    unawaited(ref.read(vaultProvider.notifier).sync());
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
