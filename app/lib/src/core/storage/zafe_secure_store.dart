import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// One vault's secrets on this device. Each vault has its own member identity, so the relay
/// can't link one person's memberships across vaults.
class StoredVault {
  const StoredVault({
    required this.id,
    this.identity,
    this.invite,
    this.material,
  });

  /// The vault id (hex), from the invite onward.
  final String id;
  final Uint8List? identity;
  final String? invite;

  /// FROST share, vault secret, log key and descriptor; set after key generation.
  final Uint8List? material;

  bool get ready => material != null;
}

/// Platform secure storage (iOS Keychain / Android Keystore) for Zafe's secrets, per vault:
/// identity seeds, the invite (not secret, kept so setup can resume) and the vault material.
/// `zafe_vaults` lists the vault ids in the order they were added.
class ZafeSecureStore {
  ZafeSecureStore._();
  static final instance = ZafeSecureStore._();

  static const _registryKey = 'zafe_vaults';

  // The single-vault layout before multiple vaults (migrated on first read).
  static const _legacyIdentityKey = 'zafe_identity_seeds';
  static const _legacyInviteKey = 'zafe_invite';
  static const _legacyMaterialKey = 'zafe_vault_material';

  final FlutterSecureStorage _storage = const FlutterSecureStorage(
    iOptions: IOSOptions(
      accessibility: KeychainAccessibility.first_unlock_this_device,
    ),
  );

  String _key(String id, String field) => 'zafe_vault_${id}_$field';

  Future<Uint8List?> _readBytes(String key) async {
    final value = await _storage.read(key: key);
    return value == null ? null : base64Decode(value);
  }

  Future<void> _writeBytes(String key, List<int> bytes) =>
      _storage.write(key: key, value: base64Encode(bytes));

  Future<List<String>> vaultIds() async {
    final raw = await _storage.read(key: _registryKey);
    return raw == null ? [] : List<String>.from(jsonDecode(raw) as List);
  }

  Future<void> _writeIds(List<String> ids) =>
      _storage.write(key: _registryKey, value: jsonEncode(ids));

  Future<StoredVault> read(String id) async => StoredVault(
    id: id,
    identity: await _readBytes(_key(id, 'identity')),
    invite: await _storage.read(key: _key(id, 'invite')),
    material: await _readBytes(_key(id, 'material')),
  );

  /// Every vault on this device, in the order added.
  Future<List<StoredVault>> readAll() async => [
    for (final id in await vaultIds()) await read(id),
  ];

  /// Adds a vault (identity and invite; material comes after key generation).
  Future<void> add({
    required String id,
    required List<int> identity,
    required String invite,
  }) async {
    await _writeBytes(_key(id, 'identity'), identity);
    await _storage.write(key: _key(id, 'invite'), value: invite);
    final ids = await vaultIds();
    if (!ids.contains(id)) await _writeIds([...ids, id]);
  }

  Future<void> writeMaterial(String id, List<int> material) =>
      _writeBytes(_key(id, 'material'), material);

  /// Removes a vault's secrets from this device (it stays a member on the relay).
  Future<void> remove(String id) async {
    for (final field in ['identity', 'invite', 'material']) {
      await _storage.delete(key: _key(id, field));
    }
    await _writeIds([
      for (final x in await vaultIds())
        if (x != id) x,
    ]);
  }

  /// Moves the pre-multi-vault single slot into the registry under `id` (the invite's vault
  /// id, which the caller parses). Returns the migrated vault, or null if there was none.
  Future<StoredVault?> migrateLegacy(
    String Function(String invite) vaultIdOf,
  ) async {
    final invite = await _storage.read(key: _legacyInviteKey);
    final identity = await _readBytes(_legacyIdentityKey);
    if (invite == null || identity == null) return null;
    final id = vaultIdOf(invite);
    await add(id: id, identity: identity, invite: invite);
    final material = await _readBytes(_legacyMaterialKey);
    if (material != null) await writeMaterial(id, material);
    for (final key in [
      _legacyIdentityKey,
      _legacyInviteKey,
      _legacyMaterialKey,
    ]) {
      await _storage.delete(key: key);
    }
    return read(id);
  }
}
