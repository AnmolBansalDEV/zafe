import 'zec_amount.dart';

/// What the balance card says about money in the total that can't be spent yet, one
/// line per kind: received money confirming, the vault's own change confirming, and
/// notes held by open payments. Amounts are compact ZEC text without the ticker.
List<String> balanceNotes({
  required BigInt incomingPendingZat,
  required BigInt changePendingZat,
  required BigInt lockedZat,
  required String ticker,
}) {
  String amount(BigInt zat) => ZecAmount.fromZatoshi(zat).balance.amountText;
  return [
    if (incomingPendingZat > BigInt.zero)
      '+${amount(incomingPendingZat)} $ticker incoming, confirming',
    if (changePendingZat > BigInt.zero)
      '${amount(changePendingZat)} $ticker change confirming',
    if (lockedZat > BigInt.zero)
      '${amount(lockedZat)} $ticker held for payments in progress',
  ];
}
