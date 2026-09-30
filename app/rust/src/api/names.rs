//! Local signer names (this device's labels for the vault's members) crossing the bridge.

use std::collections::BTreeMap;

/// One local name: the signer's signing key (hex) and the name this device gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignerName {
    pub key_hex: String,
    pub name: String,
}

pub(crate) fn to_map(names: Vec<SignerName>) -> BTreeMap<String, String> {
    names.into_iter().map(|n| (n.key_hex, n.name)).collect()
}

pub(crate) fn from_map(names: &BTreeMap<String, String>) -> Vec<SignerName> {
    names
        .iter()
        .map(|(k, v)| SignerName {
            key_hex: k.clone(),
            name: v.clone(),
        })
        .collect()
}
