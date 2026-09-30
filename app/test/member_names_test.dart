import 'package:flutter_test/flutter_test.dart';
import 'package:zafe/src/core/formatting/member_label.dart';
import 'package:zafe/src/core/storage/member_names.dart';
import 'package:zafe/src/rust/api/names.dart';

void main() {
  const key =
      '97c9ce75aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa123e';

  test('a member shows as You, their local name, or a short key', () {
    expect(memberLabel(key, me: key, names: {key: 'Alice'}), 'You');
    expect(memberLabel(key, names: {key: 'Alice'}), 'Alice');
    expect(memberLabel(key, names: {key: ''}), '97c9ce...123e');
    expect(memberLabel(key), '97c9ce...123e');
  });

  test('names are trimmed, single-spaced and capped', () {
    expect(MemberNames.clean('  Alice \n  (treasurer) '), 'Alice (treasurer)');
    expect(MemberNames.clean('   '), '');
    expect(MemberNames.clean('x' * 50).length, MemberNames.maxLength);
  });

  test('names round-trip through backups, cleaned on restore', () {
    final other = 'b' * 64;
    final signers = MemberNames.toSigners({key: 'Alice'});
    expect(signers, [SignerName(keyHex: key, name: 'Alice')]);
    expect(
      MemberNames.fromSigners([
        ...signers,
        SignerName(keyHex: other, name: '  Bob   the  builder '),
        SignerName(keyHex: 'c' * 64, name: '   '),
        const SignerName(keyHex: 'not-hex', name: 'Mallory'),
      ]),
      {key: 'Alice', other: 'Bob the builder'},
    );
  });
}
