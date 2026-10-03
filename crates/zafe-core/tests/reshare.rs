//! Changing the threshold without changing the key (spec §10.4.4): resharing in the style
//! of Desmedt-Jajodia / GRR98, the same scheme as Human Network's Algorithm 4. Not in
//! `frost-core` (its refresh keeps `min_signers`), so this test builds the packages with
//! the `internals` feature to show it works on the redpallas ciphersuite Zafe uses. It is
//! a reference for the spec, not the protocol Zafe runs.

use std::collections::{BTreeMap, BTreeSet};

use frost_core::{
    self as fc,
    keys::{dkg, KeyPackage, PublicKeyPackage, SigningShare, VerifyingShare},
    Ciphersuite, Field, Group, Identifier, VerifyingKey,
};
use rand::thread_rng;
use reddsa::frost::redpallas::{keys::EvenY, PallasBlake2b512 as C};

type F = <<C as Ciphersuite>::Group as Group>::Field;
type Scalar = <F as Field>::Scalar;
type Element = <<C as Ciphersuite>::Group as Group>::Element;
type Keys = BTreeMap<Identifier<C>, KeyPackage<C>>;

fn id(i: u16) -> Identifier<C> {
    i.try_into().unwrap()
}

fn g(s: Scalar) -> Element {
    <C as Ciphersuite>::Group::generator() * s
}

fn keygen(n: u16, t: u16) -> (Keys, PublicKeyPackage<C>) {
    let mut rng = thread_rng();
    let ids: Vec<_> = (1..=n).map(id).collect();
    let mut r1_secret = BTreeMap::new();
    let mut r1 = BTreeMap::new();
    for i in &ids {
        let (s, p) = dkg::part1(*i, n, t, &mut rng).unwrap();
        r1_secret.insert(*i, s);
        r1.insert(*i, p);
    }
    let others = |i: &Identifier<C>| -> BTreeMap<_, _> {
        r1.iter()
            .filter(|(k, _)| *k != i)
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    };
    let mut r2_secret = BTreeMap::new();
    let mut r2: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
    for i in &ids {
        let (s, out) = dkg::part2(r1_secret.remove(i).unwrap(), &others(i)).unwrap();
        r2_secret.insert(*i, s);
        for (to, package) in out {
            r2.entry(to).or_default().insert(*i, package);
        }
    }
    let mut keys = BTreeMap::new();
    let mut public = None;
    for i in &ids {
        let (key, pkp) = dkg::part3(&r2_secret[i], &others(i), &r2[i]).unwrap();
        keys.insert(*i, key);
        public = Some(pkp);
    }
    (keys, public.unwrap())
}

/// What one helper sends: Feldman commitments to its new polynomial (broadcast) and one
/// evaluation per new member (sealed to that member).
struct Deal {
    commitments: Vec<Element>,
    shares: BTreeMap<Identifier<C>, Scalar>,
}

/// Helper `h` (one of at least the old t) deals `lambda_h * share_h` on a random polynomial
/// of degree `new_t - 1`.
fn deal(
    old: &KeyPackage<C>,
    helpers: &BTreeSet<Identifier<C>>,
    new_ids: &[u16],
    new_t: u16,
) -> Deal {
    let mut rng = thread_rng();
    let lambda = fc::compute_lagrange_coefficient(helpers, None, *old.identifier()).unwrap();
    let mut coefficients = vec![lambda * old.signing_share().to_scalar()];
    coefficients.extend((1..new_t).map(|_| F::random(&mut rng)));
    let shares = new_ids
        .iter()
        .map(|j| (id(*j), eval(&coefficients, id(*j).to_scalar())))
        .collect();
    Deal {
        commitments: coefficients.iter().map(|c| g(*c)).collect(),
        shares,
    }
}

fn eval(coefficients: &[Scalar], x: Scalar) -> Scalar {
    coefficients
        .iter()
        .rev()
        .fold(F::zero(), |acc, c| acc * x + *c)
}

fn eval_commitments(commitments: &[Element], x: Scalar) -> Element {
    commitments
        .iter()
        .rev()
        .fold(<C as Ciphersuite>::Group::identity(), |acc, c| acc * x + *c)
}

#[derive(Debug, PartialEq)]
enum ReshareError {
    /// A helper's share doesn't match its own commitments.
    BadShare(Identifier<C>),
    /// A helper's constant term isn't its weighted old verifying share: it dealt
    /// something other than its real share.
    WrongSecret(Identifier<C>),
    /// The constant terms don't add up to the group key.
    WrongKey,
}

/// Every new member checks every deal against the *old* public key package, then sums its
/// evaluations. Returns the new key packages and public key package.
fn reshare(
    deals: &BTreeMap<Identifier<C>, Deal>,
    old_public: &PublicKeyPackage<C>,
    new_ids: &[u16],
    new_t: u16,
) -> Result<(Keys, PublicKeyPackage<C>), ReshareError> {
    let helpers: BTreeSet<_> = deals.keys().copied().collect();
    let mut key = <C as Ciphersuite>::Group::identity();
    for (h, d) in deals {
        let lambda = fc::compute_lagrange_coefficient(&helpers, None, *h).unwrap();
        if d.commitments[0] != old_public.verifying_shares()[h].to_element() * lambda {
            return Err(ReshareError::WrongSecret(*h));
        }
        key += d.commitments[0];
    }
    if key != old_public.verifying_key().to_element() {
        return Err(ReshareError::WrongKey);
    }
    let vk: VerifyingKey<C> = *old_public.verifying_key();
    let mut keys = BTreeMap::new();
    let mut shares = BTreeMap::new();
    for j in new_ids.iter().map(|j| id(*j)) {
        let mut sum = F::zero();
        for (h, d) in deals {
            let s = d.shares[&j];
            if g(s) != eval_commitments(&d.commitments, j.to_scalar()) {
                return Err(ReshareError::BadShare(*h));
            }
            sum += s;
        }
        let signing = SigningShare::new(sum);
        let verifying: VerifyingShare<C> = signing.into();
        shares.insert(j, verifying);
        keys.insert(j, KeyPackage::new(j, signing, verifying, vk, new_t));
    }
    Ok((keys, PublicKeyPackage::new(shares, vk, Some(new_t))))
}

fn deals(
    old: &Keys,
    helpers: &[u16],
    new_ids: &[u16],
    new_t: u16,
) -> BTreeMap<Identifier<C>, Deal> {
    let set: BTreeSet<_> = helpers.iter().map(|h| id(*h)).collect();
    set.iter()
        .map(|h| (*h, deal(&old[h], &set, new_ids, new_t)))
        .collect()
}

fn signs(keys: &Keys, signers: &[u16], public: &PublicKeyPackage<C>) -> bool {
    let mut rng = thread_rng();
    let message = b"zafe reshare";
    let mut nonces = BTreeMap::new();
    let mut commitments = BTreeMap::new();
    for s in signers.iter().map(|s| id(*s)) {
        let (n, c) = fc::round1::commit(keys[&s].signing_share(), &mut rng);
        nonces.insert(s, n);
        commitments.insert(s, c);
    }
    let package = fc::SigningPackage::new(commitments, message);
    let mut shares = BTreeMap::new();
    for s in signers.iter().map(|s| id(*s)) {
        match fc::round2::sign(&package, &nonces[&s], &keys[&s]) {
            Ok(share) => shares.insert(s, share),
            Err(_) => return false,
        };
    }
    fc::aggregate(&package, &shares, public)
        .map(|sig| public.verifying_key().verify(message, &sig).is_ok())
        .unwrap_or(false)
}

/// Whether these shares interpolate to the group key, ignoring `min_signers`: the real
/// threshold, not the software check.
fn recovers_key(keys: &Keys, who: &[u16], vk: &VerifyingKey<C>) -> bool {
    let set: BTreeSet<_> = who.iter().map(|w| id(*w)).collect();
    let secret = set.iter().fold(F::zero(), |acc, w| {
        acc + fc::compute_lagrange_coefficient(&set, None, *w).unwrap()
            * keys[w].signing_share().to_scalar()
    });
    g(secret) == vk.to_element()
}

#[test]
fn raise_then_lower_the_threshold_keeps_the_key() {
    let (old, old_public) = keygen(3, 2);
    let vk = *old_public.verifying_key();
    assert!(old_public.has_even_y());

    // 2-of-3 -> 3-of-4: two old members deal, four new members receive.
    let (new, new_public) = reshare(
        &deals(&old, &[1, 2], &[1, 2, 3, 4], 3),
        &old_public,
        &[1, 2, 3, 4],
        3,
    )
    .unwrap();
    assert_eq!(new_public.verifying_key(), &vk);
    assert_eq!(new_public.min_signers(), Some(3));
    assert!(signs(&new, &[1, 2, 3], &new_public));
    assert!(signs(&new, &[2, 3, 4], &new_public));
    assert!(!signs(&new, &[1, 4], &new_public));
    assert!(!recovers_key(&new, &[1, 4], &vk));
    assert!(!recovers_key(&new, &[1, 2], &vk));

    // Old shares are still shares of the same key: whoever kept two can still sign at the
    // old threshold. Nothing in the protocol can prevent this (spec §10.4.4).
    assert!(recovers_key(&old, &[1, 3], &vk));
    assert!(signs(&old, &[1, 3], &old_public));

    // 3-of-4 -> 2-of-3 without member 4: three members of the new set deal.
    let (low, low_public) = reshare(
        &deals(&new, &[2, 3, 4], &[1, 2, 3], 2),
        &new_public,
        &[1, 2, 3],
        2,
    )
    .unwrap();
    assert_eq!(low_public.verifying_key(), &vk);
    assert!(signs(&low, &[1, 3], &low_public));
    assert!(signs(&low, &[2, 3], &low_public));

    // Shares from different rounds don't combine.
    let mixed: Keys = [(id(1), new[&id(1)].clone()), (id(3), low[&id(3)].clone())].into();
    assert!(!recovers_key(&mixed, &[1, 3], &vk));
}

#[test]
fn fewer_helpers_than_the_old_threshold_cannot_reshare() {
    let (old, old_public) = keygen(4, 3);
    // Two of a 3-of-4 group try to deal a new 2-of-2: their weighted shares don't add up
    // to the key, so every receiver rejects it.
    let result = reshare(&deals(&old, &[1, 2], &[1, 2], 2), &old_public, &[1, 2], 2);
    assert!(matches!(
        result,
        Err(ReshareError::WrongSecret(_)) | Err(ReshareError::WrongKey)
    ));
}

#[test]
fn a_dishonest_helper_is_caught() {
    let (old, old_public) = keygen(3, 2);

    // Deals a random secret with honest-looking commitments: caught by its constant term.
    let mut d = deals(&old, &[1, 2], &[1, 2, 3], 2);
    let fake = KeyPackage::new(
        id(2),
        SigningShare::new(F::random(&mut thread_rng())),
        *old[&id(2)].verifying_share(),
        *old_public.verifying_key(),
        2,
    );
    let helpers: BTreeSet<_> = [id(1), id(2)].into();
    d.insert(id(2), deal(&fake, &helpers, &[1, 2, 3], 2));
    assert_eq!(
        reshare(&d, &old_public, &[1, 2, 3], 2).err(),
        Some(ReshareError::WrongSecret(id(2)))
    );

    // Sends one member a share off its own polynomial: caught by the commitments.
    let mut d = deals(&old, &[1, 2], &[1, 2, 3], 2);
    let s = d.get_mut(&id(1)).unwrap().shares.get_mut(&id(3)).unwrap();
    *s += F::one();
    assert_eq!(
        reshare(&d, &old_public, &[1, 2, 3], 2).err(),
        Some(ReshareError::BadShare(id(1)))
    );
}
