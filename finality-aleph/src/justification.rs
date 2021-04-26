use crate::{AuthorityId, AuthorityKeystore, AuthoritySignature};
use codec::{Decode, Encode};
use sp_api::{BlockT, NumberFor};
use sp_application_crypto::RuntimeAppPublic;
use sp_blockchain::Error;

#[derive(Clone, Encode, Decode, PartialEq, Eq, Debug)]
pub struct AlephJustification {
    pub(crate) signature: AuthoritySignature,
}

impl AlephJustification {
    pub fn new<Block: BlockT>(auth_crypto_store: &AuthorityKeystore, hash: Block::Hash) -> Self {
        Self {
            signature: auth_crypto_store.sign(&hash.encode()[..]),
        }
    }

    pub(crate) fn decode_and_verify<Block: BlockT>(
        justification: &[u8],
        block_hash: Block::Hash,
        authorities: &[AuthorityId],
        number: NumberFor<Block>,
    ) -> Result<AlephJustification, Error> {
        let aleph_justification = AlephJustification::decode(&mut &*justification)
            .map_err(|_| Error::JustificationDecode)?;

        let encoded_hash = &block_hash.encode()[..];
        for x in authorities.iter() {
            if x.verify(&encoded_hash, &aleph_justification.signature) {
                return Ok(aleph_justification);
            };
        }

        log::debug!(target: "afg", "Bad justification decoded for block number #{:?}", number);
        Err(Error::BadJustification(String::from(
            "No known AuthorityId was used to sign justification",
        )))
    }
}
