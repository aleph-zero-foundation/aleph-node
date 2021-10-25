use aleph_bft::{
    KeyBox as AlephKeyBox, MultiKeychain, NodeCount, NodeIndex, PartialMultisignature, SignatureSet,
};
use aleph_primitives::{AuthorityId, AuthoritySignature, KEY_TYPE};
use codec::{Decode, Encode};
use sp_core::crypto::KeyTypeId;
use sp_keystore::{CryptoStore, Error as KeystoreError};
use sp_runtime::RuntimeAppPublic;
use std::{convert::TryInto, sync::Arc};

#[derive(Debug)]
pub enum Error {
    KeyMissing,
    Keystore(KeystoreError),
    Conversion,
}

#[derive(PartialEq, Eq, Clone, Debug, Decode, Encode)]
pub struct Signature(AuthoritySignature);

/// Ties an authority identification and a cryptography keystore together for use in
/// signing that requires an authority.
#[derive(Clone)]
pub struct AuthorityPen {
    key_type_id: KeyTypeId,
    authority_id: AuthorityId,
    keystore: Arc<dyn CryptoStore>,
}

impl AuthorityPen {
    /// Constructs a new authority cryptography keystore for the given ID.
    /// Will attempt to sign a test message to verify that signing works.
    /// Returns errors if anything goes wrong during this attempt, otherwise we assume the
    /// AuthorityPen will work for any future attempts at signing.
    pub async fn new(
        authority_id: AuthorityId,
        keystore: Arc<dyn CryptoStore>,
    ) -> Result<Self, Error> {
        // Check whether this signing setup works
        let _: AuthoritySignature = keystore
            .sign_with(KEY_TYPE, &authority_id.clone().into(), b"test")
            .await
            .map_err(Error::Keystore)?
            .ok_or(Error::KeyMissing)?
            .try_into()
            .map_err(|_| Error::Conversion)?;
        Ok(AuthorityPen {
            key_type_id: KEY_TYPE,
            authority_id,
            keystore,
        })
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
            Some(authority) => authority.verify(&msg.to_vec(), &sgn.0),
            None => false,
        }
    }

    fn node_count(&self) -> NodeCount {
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

/// KeyBox combines an AuthorityPen and AuthorityVerifier into one object implementing the AlephBFT
/// MultiKeychain trait.
#[derive(Clone)]
pub struct KeyBox {
    id: NodeIndex,
    authority_pen: AuthorityPen,
    authority_verifier: AuthorityVerifier,
}

impl KeyBox {
    /// Constructs a new keybox from a signing contraption and verifier, with the specified node
    /// index.
    pub fn new(
        id: NodeIndex,
        authority_verifier: AuthorityVerifier,
        authority_pen: AuthorityPen,
    ) -> Self {
        KeyBox {
            id,
            authority_pen,
            authority_verifier,
        }
    }
}

impl aleph_bft::Index for KeyBox {
    fn index(&self) -> NodeIndex {
        self.id
    }
}

#[async_trait::async_trait]
impl AlephKeyBox for KeyBox {
    type Signature = Signature;

    fn node_count(&self) -> NodeCount {
        self.authority_verifier.node_count()
    }

    async fn sign(&self, msg: &[u8]) -> Signature {
        self.authority_pen.sign(msg).await
    }

    fn verify(&self, msg: &[u8], sgn: &Signature, index: NodeIndex) -> bool {
        self.authority_verifier.verify(msg, sgn, index)
    }
}

impl MultiKeychain for KeyBox {
    // Using `SignatureSet` is slow, but Substrate has not yet implemented aggregation.
    // We probably should do this for them at some point.
    type PartialMultisignature = SignatureSet<Signature>;

    fn from_signature(
        &self,
        signature: &Signature,
        index: NodeIndex,
    ) -> Self::PartialMultisignature {
        SignatureSet::add_signature(SignatureSet::with_size(self.node_count()), signature, index)
    }

    fn is_complete(&self, msg: &[u8], partial: &Self::PartialMultisignature) -> bool {
        self.authority_verifier.is_complete(msg, partial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_keystore::{testing::KeyStore, CryptoStore};

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
