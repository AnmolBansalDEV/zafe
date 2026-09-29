import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Platform secure storage (iOS Keychain / Android Keystore) for Zafe's secrets:
/// the member identity seeds and the vault material (FROST share, vault secret, log key).
/// The invite is not secret but is kept alongside so setup can resume.
class ZafeSecureStore {
  ZafeSecureStore._();
  static final instance = ZafeSecureStore._();

  static const _identityKey = 'zafe_identity_seeds';
  static const _inviteKey = 'zafe_invite';
  static const _materialKey = 'zafe_vault_material';

  final FlutterSecureStorage _storage = const FlutterSecureStorage(
    iOptions: IOSOptions(
      accessibility: KeychainAccessibility.first_unlock_this_device,
    ),
  );

  Future<Uint8List?> _readBytes(String key) async {
    final value = await _storage.read(key: key);
    return value == null ? null : base64Decode(value);
  }

  Future<void> _writeBytes(String key, List<int> bytes) =>
      _storage.write(key: key, value: base64Encode(bytes));

  Future<Uint8List?> readIdentity() => _readBytes(_identityKey);
  Future<void> writeIdentity(List<int> seeds) =>
      _writeBytes(_identityKey, seeds);

  Future<String?> readInvite() => _storage.read(key: _inviteKey);
  Future<void> writeInvite(String invite) =>
      _storage.write(key: _inviteKey, value: invite);

  Future<Uint8List?> readMaterial() => _readBytes(_materialKey);
  Future<void> writeMaterial(List<int> material) =>
      _writeBytes(_materialKey, material);

  /// Removes everything (used by "Reset this device").
  Future<void> deleteAll() async {
    for (final key in [_identityKey, _inviteKey, _materialKey]) {
      await _storage.delete(key: key);
    }
  }
}
