import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

import '../core/config/network_config.dart';
import '../core/storage/zafe_secure_store.dart';
import '../rust/api/vault.dart' as rust;

/// Snapshot read before the first frame (Vizor's bootstrap pattern), so the router can
/// start on the right screen without a flash.
class VaultBootstrap {
  const VaultBootstrap({this.identity, this.invite, this.material});
  final Uint8List? identity;
  final String? invite;
  final Uint8List? material;

  static Future<VaultBootstrap> load() async {
    final store = ZafeSecureStore.instance;
    return VaultBootstrap(
      identity: await store.readIdentity(),
      invite: await store.readInvite(),
      material: await store.readMaterial(),
    );
  }
}

final vaultBootstrapProvider = Provider<VaultBootstrap>((ref) => const VaultBootstrap());

class VaultState {
  const VaultState({
    this.identity,
    this.invite,
    this.material,
    this.membership,
    this.balance,
    this.syncing = false,
    this.syncError,
  });

  final Uint8List? identity;
  final String? invite;
  final Uint8List? material;
  final rust.MembershipInfo? membership;
  final rust.Balance? balance;
  final bool syncing;
  final String? syncError;

  bool get hasVault => material != null;
  bool get isSettingUp => !hasVault && invite != null;

  rust.InviteInfo? get inviteInfo => invite == null ? null : rust.parseInvite(invite: invite!);
  rust.VaultSummary? get summary => material == null ? null : rust.vaultSummary(material: material!);

  VaultState copyWith({
    Uint8List? identity,
    String? invite,
    Uint8List? material,
    rust.MembershipInfo? membership,
    rust.Balance? balance,
    bool? syncing,
    String? syncError,
    bool clearSyncError = false,
  }) => VaultState(
    identity: identity ?? this.identity,
    invite: invite ?? this.invite,
    material: material ?? this.material,
    membership: membership ?? this.membership,
    balance: balance ?? this.balance,
    syncing: syncing ?? this.syncing,
    syncError: clearSyncError ? null : (syncError ?? this.syncError),
  );
}

/// Owns vault setup (create/join, membership, key generation) and wallet sync.
class VaultNotifier extends Notifier<VaultState> {
  final _store = ZafeSecureStore.instance;

  @override
  VaultState build() {
    final boot = ref.watch(vaultBootstrapProvider);
    return VaultState(identity: boot.identity, invite: boot.invite, material: boot.material);
  }

  Future<Uint8List> _ensureIdentity() async {
    final existing = state.identity;
    if (existing != null) return existing;
    final id = rust.generateIdentity();
    await _store.writeIdentity(id.seeds);
    final seeds = Uint8List.fromList(id.seeds);
    state = state.copyWith(identity: seeds);
    return seeds;
  }

  Future<void> createVault({required String name, required int threshold, required int members}) async {
    final seeds = await _ensureIdentity();
    final invite = await rust.createVault(
      relayUrl: kZafeRelayUrl,
      seeds: seeds,
      name: name,
      threshold: threshold,
      members: members,
    );
    await _store.writeInvite(invite);
    state = state.copyWith(invite: invite);
  }

  Future<void> joinVault(String invite) async {
    rust.parseInvite(invite: invite.trim()); // validates before touching the relay
    final seeds = await _ensureIdentity();
    await rust.joinVault(relayUrl: kZafeRelayUrl, seeds: seeds, invite: invite.trim());
    await _store.writeInvite(invite.trim());
    state = state.copyWith(invite: invite.trim());
  }

  Future<rust.MembershipInfo> refreshMembership() async {
    final membership = await rust.vaultMembership(
      relayUrl: kZafeRelayUrl,
      seeds: state.identity!,
      invite: state.invite!,
    );
    state = state.copyWith(membership: membership);
    return membership;
  }

  Future<void> seal() async {
    await rust.sealVault(relayUrl: kZafeRelayUrl, seeds: state.identity!, invite: state.invite!);
    await refreshMembership();
  }

  /// Runs key generation with every member; blocks until done. `safetyNumber` is the
  /// number the user confirmed out of band.
  Future<void> createKeys(String safetyNumber) async {
    final material = await rust.runKeygen(
      relayUrl: kZafeRelayUrl,
      lightwalletdUrl: kZafeLightwalletdUrl,
      networkName: kZafeNetwork,
      seeds: state.identity!,
      invite: state.invite!,
      confirmedSafetyNumber: safetyNumber,
      timeoutSecs: 600,
    );
    await _store.writeMaterial(material);
    state = state.copyWith(material: material);
    unawaited(sync());
  }

  Future<void> sync() async {
    final material = state.material;
    if (material == null || state.syncing) return;
    state = state.copyWith(syncing: true, clearSyncError: true);
    try {
      final dir = await getApplicationSupportDirectory();
      final balance = await rust.syncVault(
        dbDir: dir.path,
        lightwalletdUrl: kZafeLightwalletdUrl,
        material: material,
      );
      state = state.copyWith(balance: balance, syncing: false);
    } catch (e) {
      state = state.copyWith(syncing: false, syncError: '$e');
    }
  }
}

final vaultProvider = NotifierProvider<VaultNotifier, VaultState>(VaultNotifier.new);
