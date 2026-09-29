/// Members are shown by a short key until the address book lands.
String memberLabel(String keyHex, {String? me}) {
  if (keyHex == me) return 'You';
  if (keyHex.length < 14) return keyHex;
  return '${keyHex.substring(0, 6)}...${keyHex.substring(keyHex.length - 4)}';
}

/// Vizor's compact address: 7 + " .... " + 7.
String compactAddress(String address) {
  if (address.length <= 18) return address;
  return '${address.substring(0, 7)} .... ${address.substring(address.length - 7)}';
}
