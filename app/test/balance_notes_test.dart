import 'package:flutter_test/flutter_test.dart';
import 'package:zafe/src/core/formatting/balance_notes.dart';

void main() {
  List<String> notes({int incoming = 0, int change = 0, int locked = 0}) =>
      balanceNotes(
        incomingPendingZat: BigInt.from(incoming),
        changePendingZat: BigInt.from(change),
        lockedZat: BigInt.from(locked),
        ticker: 'TAZ',
      );

  test('nothing pending says nothing', () => expect(notes(), isEmpty));

  test('change after a payment is not shown as incoming money', () {
    expect(notes(change: 11380000), ['0.1138 TAZ change confirming']);
  });

  test('each kind gets its own line, incoming first', () {
    expect(notes(incoming: 20000000, change: 500000, locked: 1000000), [
      '+0.20 TAZ incoming, confirming',
      '0.005 TAZ change confirming',
      '0.01 TAZ held for payments in progress',
    ]);
  });
}
