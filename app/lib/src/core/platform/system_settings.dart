import 'package:flutter/services.dart';

const _channel = MethodChannel('xyz.zafe/system_settings');

/// Opens the phone's security settings, where a screen lock is set. False where it
/// couldn't (no handler: tests, iOS for now).
Future<bool> openSecuritySettings() async {
  try {
    return await _channel.invokeMethod<bool>('openSecurity') ?? false;
  } on MissingPluginException {
    return false;
  } on PlatformException {
    return false;
  }
}
