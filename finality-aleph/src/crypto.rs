use std::{convert::TryInto, sync::Arc};

use aleph_primitives::{AuthorityId, AuthoritySignature, KEY_TYPE};
use codec::{Decode, Encode};
use sp_core::{crypto::KeyTypeId, ed25519::Signature as RawSignature};
use sp_keystore::{CryptoStore, Error as KeystoreError};
use sp_runtime::RuntimeAppPublic;

use crate::abft::{NodeCount, NodeIndex, SignatureSet};

#[derive(Debug)]
pub enum Error {
    KeyMissing(AuthorityId),
    Keystore(KeystoreError),
    Conversion,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash, Decode, Encode)]
pub struct Signature(AuthoritySignature);

impl From<AuthoritySignature> for Signature {
    fn from(authority_signature: AuthoritySignature) -> Signature {
        Signature(authority_signature)
    }
}

// This is here just for a compatibility hack, remove when removing legacy/v1 authentications.
impl From<[u8; 64]> for Signature {
    fn from(bytes: [u8; 64]) -> Signature {
        Signature(RawSignature::from_raw(bytes).into())
    }
}

/// Ties an authority identification and a cryptography keystore together for use in
/// signing that requires an authority.
#[derive(Clone)]
pub struct AuthorityPen {
    key_type_id: KeyTypeId,
    authority_id: AuthorityId,
    keystore: Arc<dyn CryptoStore>,
}

impl AuthorityPen {
    /// Constructs a new authority cryptography keystore for the given ID and key type.
    /// Will attempt to sign a test message to verify that signing works.
    /// Returns errors if anything goes wrong during this attempt, otherwise we assume the
    /// AuthorityPen will work for any future attempts at signing.
    pub async fn new_with_key_type(
        authority_id: AuthorityId,
        keystore: Arc<dyn CryptoStore>,
        key_type: KeyTypeId,
    ) -> Result<Self, Error> {
        // Check whether this signing setup works
        let _: AuthoritySignature = keystore
            .sign_with(key_type, &authority_id.clone().into(), b"test")
            .await
            .map_err(Error::Keystore)?
            .ok_or_else(|| Error::KeyMissing(authority_id.clone()))?
            .try_into()
            .map_err(|_| Error::Conversion)?;
        Ok(AuthorityPen {
            key_type_id: key_type,
            authority_id,
            keystore,
        })
    }

    /// Constructs a new authority cryptography keystore for the given ID and the aleph key type.
    /// Will attempt to sign a test message to verify that signing works.
    /// Returns errors if anything goes wrong during this attempt, otherwise we assume the
    /// AuthorityPen will work for any future attempts at signing.
    pub async fn new(
        authority_id: AuthorityId,
        keystore: Arc<dyn CryptoStore>,
    ) -> Result<Self, Error> {
        Self::new_with_key_type(authority_id, keystore, KEY_TYPE).await
    }

    /// Cryptographically signs the message.
    pub async fn sign(&self, msg: &[u8]) -> Signature {
        Signature(
            self.keystore
                .sign_with(self.key_type_id, &self.authority_id.clone().into(), msg)
                .await
                .expect("the keystore works")
                .expect("we have the required key")
                .try_into()
                .expect("the bytes encode a signature"),
        )
    }

    /// Return the associated AuthorityId.
    pub fn authority_id(&self) -> AuthorityId {
        self.authority_id.clone()
    }
}

/// Verify the signature given an authority id.
pub fn verify(authority: &AuthorityId, message: &[u8], signature: &Signature) -> bool {
    authority.verify(&message, &signature.0)
}

/// Holds the public authority keys for a session allowing for verification of messages from that
/// session.
#[derive(Clone)]
pub struct AuthorityVerifier {
    authorities: Vec<AuthorityId>,
}

impl AuthorityVerifier {
    /// Constructs a new authority verifier from a set of public keys.
    pub fn new(authorities: Vec<AuthorityId>) -> Self {
        AuthorityVerifier { authorities }
    }

    /// Verifies whether the message is correctly signed with the signature assumed to be made by a
    /// node of the given index.
    pub fn verify(&self, msg: &[u8], sgn: &Signature, index: NodeIndex) -> bool {
        match self.authorities.get(index.0) {
            Some(authority) => verify(authority, msg, sgn),
            None => false,
        }
    }

    pub fn node_count(&self) -> NodeCount {
        self.authorities.len().into()
    }

    fn threshold(&self) -> usize {
        2 * self.node_count().0 / 3 + 1
    }

    /// Verifies whether the given signature set is a correct and complete multisignature of the
    /// message. Completeness requires more than 2/3 of all authorities.
    pub fn is_complete(&self, msg: &[u8], partial: &SignatureSet<Signature>) -> bool {
        let signature_count = partial.iter().count();
        if signature_count < self.threshold() {
            return false;
        }
        partial.iter().all(|(i, sgn)| self.verify(msg, sgn, i))
    }
}

/// Old format of signatures, needed for backwards compatibility.
#[derive(PartialEq, Eq, Clone, Debug, Decode, Encode)]
pub struct SignatureV1 {
    pub _id: NodeIndex,
    pub sgn: AuthoritySignature,
}

impl From<SignatureV1> for Signature {
    fn from(sig_v1: SignatureV1) -> Signature {
        Signature(sig_v1.sgn)
    }
}

#[cfg(test)]
mod tests {
    use sp_keystore::{testing::KeyStore, CryptoStore};

    use super::*;
    use crate::abft::NodeIndex;

    async fn generate_keys(names: &[String]) -> (Vec<AuthorityPen>, AuthorityVerifier) {
        let key_store = Arc::new(KeyStore::new());
        let mut authority_ids = Vec::with_capacity(names.len());
        for name in names {
            let pk = key_store
                .ed25519_generate_new(KEY_TYPE, Some(name))
                .await
                .unwrap();
            authority_ids.push(AuthorityId::from(pk));
        }
        let mut pens = Vec::with_capacity(names.len());
        for authority_id in authority_ids.clone() {
            pens.push(
                AuthorityPen::new(authority_id, key_store.clone())
                    .await
                    .expect("The keys should sign successfully"),
            );
        }
        assert_eq!(
            key_store.keys(KEY_TYPE).await.unwrap().len(),
            3 * names.len()
        );
        (pens, AuthorityVerifier::new(authority_ids))
    }

    async fn prepare_test() -> (Vec<AuthorityPen>, AuthorityVerifier) {
        let authority_names: Vec<_> = ["//Alice", "//Bob", "//Charlie"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        generate_keys(&authority_names).await
    }

    #[tokio::test]
    async fn produces_and_verifies_correct_signatures() {
        let (pens, verifier) = prepare_test().await;
        let msg = b"test";
        for (i, pen) in pens.into_iter().enumerate() {
            let signature = pen.sign(msg).await;
            assert!(verifier.verify(msg, &signature, NodeIndex(i)));
        }
    }

    #[tokio::test]
    async fn does_not_accept_signatures_from_wrong_sources() {
        let (pens, verifier) = prepare_test().await;
        let msg = b"test";
        for pen in &pens[1..] {
            let signature = pen.sign(msg).await;
            assert!(!verifier.verify(msg, &signature, NodeIndex(0)));
        }
    }

    #[tokio::test]
    async fn does_not_accept_signatures_from_unknown_sources() {
        let (pens, verifier) = prepare_test().await;
        let msg = b"test";
        for pen in &pens {
            let signature = pen.sign(msg).await;
            assert!(!verifier.verify(msg, &signature, NodeIndex(pens.len())));
        }
    }

    #[tokio::test]
    async fn does_not_accept_signatures_for_different_messages() {
        let (pens, verifier) = prepare_test().await;
        let msg = b"test";
        let not_msg = b"not test";
        for (i, pen) in pens.into_iter().enumerate() {
            let signature = pen.sign(msg).await;
            assert!(!verifier.verify(not_msg, &signature, NodeIndex(i)));
        }
    }
}
