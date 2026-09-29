import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../core/config/network_config.dart';
import '../core/errors/zafe_error_copy.dart';
import '../core/storage/vault_summaries.dart';
import '../core/storage/zafe_paths.dart';
import '../core/storage/zafe_secure_store.dart';
import '../rust/api/error.dart';
import '../rust/api/vault.dart' as rust;
import 'privacy_mode_provider.dart' show kPrivacyModeKey;
import 'theme_mode_provider.dart' show kThemeModeKey, themeModeFromName;

const kActiveVaultKey = 'zafe_active_vault';

/// Snapshot read before the first frame (Vizor's bootstrap pattern), so the router can
/// start on the right screen without a flash.
class VaultBootstrap {
  const VaultBootstrap({
    this.vaults = const [],
    this.activeId,
    this.privacyMode = false,
    this.themeMode = ThemeMode.system,
  });
  final List<StoredVault> vaults;
  final String? activeId;
  final bool privacyMode;
  final ThemeMode themeMode;

  /// Needs Rust initialized (parses the legacy invite when migrating).
  static Future<VaultBootstrap> load() async {
    final store = ZafeSecureStore.instance;
    final prefs = await SharedPreferences.getInstance();
    final migrated = await store.migrateLegacy(
      (invite) => rust.parseInvite(invite: invite).vaultId,
    );
    if (migrated != null) {
      await (await ZafePaths.get()).migrateLegacy(migrated.id);
      await prefs.setString(kActiveVaultKey, migrated.id);
    }
    final vaults = await store.readAll();
    final saved = prefs.getString(kActiveVaultKey);
    final activeId = vaults.any((v) => v.id == saved)
        ? saved
        : (vaults.where((v) => v.ready).firstOrNull ?? vaults.firstOrNull)?.id;
    return VaultBootstrap(
      vaults: vaults,
      activeId: activeId,
      privacyMode: prefs.getBool(kPrivacyModeKey) ?? false,
      themeMode: themeModeFromName(prefs.getString(kThemeModeKey)),
    );
  }
}

final vaultBootstrapProvider = Provider<VaultBootstrap>(
  (ref) => const VaultBootstrap(),
);

class VaultState {
  const VaultState({
    this.vaults = const [],
    this.activeId,
    this.returnTo,
    this.membership,
    this.balances = const {},
    this.syncing = false,
    this.syncError,
  });

  /// Every vault on this device, in the order added (ready and still setting up).
  final List<StoredVault> vaults;

  /// The vault the screens show; null on the welcome screen (first run, or adding one).
  final String? activeId;

  /// While adding another vault: the vault to go back to if the user cancels.
  final String? returnTo;

  final rust.MembershipInfo? membership;

  /// Last known balance per vault id (this session).
  final Map<String, rust.Balance> balances;
  final bool syncing;

  /// Last sync failure (a `ZafeError` from Rust), cleared by the next successful sync.
  final Object? syncError;

  StoredVault? get active {
    for (final v in vaults) {
      if (v.id == activeId) return v;
    }
    return null;
  }

  Uint8List? get identity => active?.identity;
  String? get invite => active?.invite;
  Uint8List? get material => active?.material;
  rust.Balance? get balance => activeId == null ? null : balances[activeId];

  bool get syncOffline =>
      syncError is ZafeError &&
      (syncError as ZafeError).kind == ZafeErrorKind.network;

  bool get hasVault => material != null;
  bool get isSettingUp => !hasVault && invite != null;
  bool get isAdding => returnTo != null;

  rust.InviteInfo? get inviteInfo =>
      invite == null ? null : rust.parseInvite(invite: invite!);
  rust.VaultSummary? get summary =>
      material == null ? null : rust.vaultSummary(material: material!);
  String? get myKeyHex =>
      identity == null ? null : rust.identityPublicKey(seeds: identity!);

  VaultState copyWith({
    List<StoredVault>? vaults,
    String? activeId,
    bool clearActive = false,
    String? returnTo,
    bool clearReturnTo = false,
    rust.MembershipInfo? membership,
    bool clearMembership = false,
    Map<String, rust.Balance>? balances,
    bool? syncing,
    Object? syncError,
    bool clearSyncError = false,
  }) => VaultState(
    vaults: vaults ?? this.vaults,
    activeId: clearActive ? null : (activeId ?? this.activeId),
    returnTo: clearReturnTo ? null : (returnTo ?? this.returnTo),
    membership: clearMembership ? null : (membership ?? this.membership),
    balances: balances ?? this.balances,
    syncing: syncing ?? this.syncing,
    syncError: clearSyncError ? null : (syncError ?? this.syncError),
  );
}

/// Owns the device's vaults: which one is active, adding (create/join, membership, key
/// generation), removing, and wallet sync of the active vault.
class VaultNotifier extends Notifier<VaultState> {
  final _store = ZafeSecureStore.instance;

  @override
  VaultState build() {
    final boot = ref.watch(vaultBootstrapProvider);
    return VaultState(vaults: boot.vaults, activeId: boot.activeId);
  }

  Future<void> _reload({String? activeId, bool clearActive = false}) async {
    final vaults = await _store.readAll();
    state = state.copyWith(
      vaults: vaults,
      activeId: activeId,
      clearActive: clearActive,
      clearMembership: true,
      clearSyncError: true,
    );
    final prefs = await SharedPreferences.getInstance();
    final id = state.activeId;
    id == null
        ? await prefs.remove(kActiveVaultKey)
        : await prefs.setString(kActiveVaultKey, id);
  }

  /// Shows another vault.
  Future<void> switchTo(String id) async {
    if (id == state.activeId && !state.isAdding) return;
    state = state.copyWith(clearReturnTo: true);
    await _reload(activeId: id);
    unawaited(sync());
  }

  /// Leaves the current vault on screen to create or join another (onboarding shows).
  void beginAddVault() {
    final current = state.activeId;
    if (current == null) return;
    state = state.copyWith(
      returnTo: current,
      clearActive: true,
      clearMembership: true,
    );
  }

  /// Back to the vault that was on screen before "Add vault".
  Future<void> cancelAddVault() async {
    final back = state.returnTo;
    if (back == null) return;
    state = state.copyWith(clearReturnTo: true);
    await _reload(activeId: back);
  }

  Future<void> createVault({
    required String name,
    required int threshold,
    required int members,
  }) async {
    // A fresh member identity per vault, so the relay can't link memberships.
    final id = rust.generateIdentity();
    final invite = await rust.createVault(
      relayUrl: kZafeRelayUrl,
      seeds: id.seeds,
      name: name,
      threshold: threshold,
      members: members,
    );
    final vaultId = rust.parseInvite(invite: invite).vaultId;
    await _store.add(id: vaultId, identity: id.seeds, invite: invite);
    state = state.copyWith(clearReturnTo: true);
    await _reload(activeId: vaultId);
  }

  Future<void> joinVault(String invite) async {
    final trimmed = invite.trim();
    final vaultId = rust
        .parseInvite(invite: trimmed)
        .vaultId; // validates first
    if (state.vaults.any((v) => v.id == vaultId)) {
      await switchTo(vaultId); // already on this device
      return;
    }
    final id = rust.generateIdentity();
    await rust.joinVault(
      relayUrl: kZafeRelayUrl,
      seeds: id.seeds,
      invite: trimmed,
    );
    await _store.add(id: vaultId, identity: id.seeds, invite: trimmed);
    state = state.copyWith(clearReturnTo: true);
    await _reload(activeId: vaultId);
  }

  /// Adds a vault restored from a backup and shows it. This device starts with no signing
  /// nonces (backups never carry them) and publishes a fresh pool on its first refresh.
  Future<void> addRestoredVault({
    required String vaultId,
    required List<int> identity,
    required List<int> material,
    required String invite,
  }) async {
    if (state.vaults.any((v) => v.id == vaultId)) {
      throw StateError('already on this device');
    }
    await _store.add(id: vaultId, identity: identity, invite: invite);
    await _store.writeMaterial(vaultId, material);
    state = state.copyWith(clearReturnTo: true);
    await _reload(activeId: vaultId);
    unawaited(sync());
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
    await rust.sealVault(
      relayUrl: kZafeRelayUrl,
      seeds: state.identity!,
      invite: state.invite!,
    );
    await refreshMembership();
  }

  /// Runs key generation with every member; blocks until done. `safetyNumber` is the
  /// number the user confirmed out of band.
  Future<void> createKeys(String safetyNumber) async {
    final vaultId = state.activeId!;
    final material = await rust.runKeygen(
      relayUrl: kZafeRelayUrl,
      lightwalletdUrl: kZafeLightwalletdUrl,
      networkName: kZafeNetwork,
      seeds: state.identity!,
      invite: state.invite!,
      confirmedSafetyNumber: safetyNumber,
      timeoutSecs: 600,
      birthdayHeight: null,
    );
    await _store.writeMaterial(vaultId, material);
    await _reload(activeId: vaultId);
    unawaited(sync());
  }

  /// Removes a vault from this device: its secrets, signing state, notification state and
  /// wallet database. The vault and this member's seat stay on the relay.
  Future<void> removeVault(String id) async {
    await _store.remove(id);
    await (await ZafePaths.get()).deleteVault(id);
    final rest = state.vaults.where((v) => v.id != id).toList();
    final next = state.activeId == id
        ? (rest.where((v) => v.ready).firstOrNull ?? rest.firstOrNull)?.id
        : state.activeId;
    state = state.copyWith(
      balances: {...state.balances}..remove(id),
      clearReturnTo: true,
    );
    await _reload(activeId: next, clearActive: next == null);
    if (next != null) unawaited(sync());
  }

  Future<void> sync() async {
    final material = state.material;
    final vaultId = state.activeId;
    if (material == null || vaultId == null || state.syncing) return;
    state = state.copyWith(syncing: true, clearSyncError: true);
    try {
      final paths = await ZafePaths.get();
      final balance = await rust.syncVault(
        dbDir: paths.dbDir,
        lightwalletdUrl: kZafeLightwalletdUrl,
        material: material,
      );
      state = state.copyWith(
        balances: {...state.balances, vaultId: balance},
        syncing: false,
      );
      unawaited(VaultSummaries.write(vaultId, balanceZat: balance.totalZat));
    } catch (e) {
      debugPrint('sync failed: ${describeError(e)}');
      state = state.copyWith(syncing: false, syncError: e);
    }
  }
}

final vaultProvider = NotifierProvider<VaultNotifier, VaultState>(
  VaultNotifier.new,
);
