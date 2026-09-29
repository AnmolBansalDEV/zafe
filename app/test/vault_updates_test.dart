import 'package:flutter_test/flutter_test.dart';
import 'package:zafe/src/notifications/vault_updates.dart';
import 'package:zafe/src/rust/api/proposals.dart';

ProposalInfo proposal(
  String id, {
  ProposalStage stage = ProposalStage.open,
  MyVote myVote = MyVote.none,
  bool isMine = false,
  bool ready = false,
  bool autoSend = true,
}) => ProposalInfo(
  id: id,
  author: 'aa',
  isMine: isMine,
  payments: [
    PaymentInfo(
      address: 'uregtest1abcdefghijklmnopqrstuvwxyz0123456789',
      amountZat: BigInt.from(150000000),
      memo: '',
    ),
  ],
  totalZat: BigInt.from(150000000),
  stage: stage,
  approvals: const [],
  rejections: const [],
  myVote: myVote,
  threshold: 2,
  rejectionThreshold: 2,
  createdAt: BigInt.zero,
  signingStarted: false,
  oneTap: true,
  ready: ready,
  completedByMe: false,
  autoSend: autoSend,
);

List<VaultUpdate> updates(
  SeenSnapshot? previous,
  List<ProposalInfo> now, {
  bool hide = false,
}) => vaultUpdates(
  previous: previous,
  proposals: now,
  vaultName: 'Grants',
  hideAmounts: hide,
);

void main() {
  test('a fresh install announces nothing', () {
    expect(updates(null, [proposal('p1')]), isEmpty);
  });

  test('a new proposal from someone else needs approval', () {
    final u = updates({}, [proposal('p1')]);
    expect(u, hasLength(1));
    expect(u.single.title, 'Grants: payment needs your approval');
    expect(u.single.body, '1.5 TAZ to uregtes .... 3456789');
    expect(u.single.proposalId, 'p1');
  });

  test('my own proposal is not announced to me', () {
    expect(updates({}, [proposal('p1', isMine: true)]), isEmpty);
  });

  test('nothing changes, nothing is announced', () {
    final p = proposal('p1');
    expect(updates(snapshotOf([p]), [p]), isEmpty);
  });

  test(
    'ready to send is announced only when the proposer chose manual send',
    () {
      final before = snapshotOf([proposal('p1', myVote: MyVote.approved)]);
      final manual = proposal(
        'p1',
        stage: ProposalStage.approved,
        myVote: MyVote.approved,
        ready: true,
        autoSend: false,
      );
      final auto = proposal(
        'p1',
        stage: ProposalStage.approved,
        myVote: MyVote.approved,
        ready: true,
      );
      expect(
        updates(before, [manual]).single.title,
        'Grants: payment ready to send',
      );
      expect(
        updates(before, [auto]),
        isEmpty,
        reason: 'the completing signer sends it',
      );
    },
  );

  test('sent and rejected are announced', () {
    final before = snapshotOf([proposal('p1'), proposal('p2')]);
    final u = updates(before, [
      proposal('p1', stage: ProposalStage.sent),
      proposal('p2', stage: ProposalStage.rejected),
    ]);
    expect(u.map((x) => x.title), [
      'Grants: payment sent',
      'Grants: payment rejected',
    ]);
  });

  test('privacy mode leaves amounts and addresses out', () {
    final u = updates({}, [proposal('p1')], hide: true);
    expect(u.single.body, 'A payment');
  });
}
