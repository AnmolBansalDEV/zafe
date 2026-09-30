import 'package:flutter_test/flutter_test.dart';
import 'package:zafe/src/features/proposals/proposal_status.dart';
import 'package:zafe/src/features/send/send_screen.dart' show zecDecimal;
import 'package:zafe/src/notifications/vault_updates.dart';
import 'package:zafe/src/rust/api/proposals.dart';

import 'vault_updates_test.dart' show proposal;

void main() {
  group('expiry', () {
    final open = proposal('p', expiryHeight: 1000);

    test(
      'an unsent proposal expires once the tip reaches its expiry height',
      () {
        expect(proposalExpired(open, null), isFalse, reason: 'height unknown');
        expect(proposalExpired(open, 999), isFalse);
        expect(proposalExpired(open, 1000), isTrue);
        expect(proposalTitle(open, height: 1000), 'Expired');
        expect(proposalTitle(open, height: 999), 'Needs your approval');
      },
    );

    test(
      'finished proposals and ones without an expiry never show as expired',
      () {
        for (final stage in [
          ProposalStage.sent,
          ProposalStage.rejected,
          ProposalStage.cancelled,
        ]) {
          expect(
            proposalExpired(proposal('p', stage: stage, expiryHeight: 10), 99),
            isFalse,
          );
        }
        expect(proposalExpired(proposal('p'), 99), isFalse);
      },
    );

    test('time left reads in minutes, hours or days (75 s blocks)', () {
      expect(expiresIn(open, 999), 'about 1 minute');
      expect(expiresIn(open, 1000 - 40), 'about 50 minutes');
      expect(expiresIn(open, 1000 - 48), 'about 1 hour');
      expect(expiresIn(open, 1000 - 8064 ~/ 8), 'about 21 hours');
      final week = proposal('p', expiryHeight: 8064);
      expect(expiresIn(week, 0), 'about 7 days');
    });

    test('expired proposals need no action', () {
      final items = [
        proposal('a', expiryHeight: 1000),
        proposal('b', stage: ProposalStage.approved, expiryHeight: 1000),
      ];
      expect(actionableCount(items, height: 999), 2);
      expect(actionableCount(items, height: 1000), 0);
      expect(actionableCount(items), 2);
    });
  });

  test('propose again fills the amount field with plain decimal ZEC', () {
    expect(zecDecimal(BigInt.from(150000000)), '1.5');
    expect(zecDecimal(BigInt.from(100000000)), '1');
    expect(zecDecimal(BigInt.one), '0.00000001');
    expect(zecDecimal(BigInt.zero), '0');
  });
}
