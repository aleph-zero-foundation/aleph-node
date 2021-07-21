use crate::{AuthorityId, AuthorityKeystore, AuthoritySignature, JustificationNotification};
use codec::{Decode, Encode};
use futures::channel::mpsc;
use sp_api::{BlockT, NumberFor};
use sp_application_crypto::RuntimeAppPublic;
use sp_blockchain::Error;
use tokio::stream::StreamExt;

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

    pub(crate) fn _decode_and_verify<Block: BlockT>(
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

        log::debug!(target: "afa", "Bad justification decoded for block number #{:?}", number);
        Err(Error::BadJustification(String::from(
            "No known AuthorityId was used to sign justification",
        )))
    }
}

// For now it is glorified proxy channel not doing anything useful
pub struct JustificationHandler<Block: BlockT> {
    finalization_proposals_tx: mpsc::UnboundedSender<JustificationNotification<Block>>,
    justification_rx: mpsc::UnboundedReceiver<JustificationNotification<Block>>,
}

impl<Block: BlockT> JustificationHandler<Block> {
    pub(crate) fn new(
        finalization_proposals_tx: mpsc::UnboundedSender<JustificationNotification<Block>>,
        justification_rx: mpsc::UnboundedReceiver<JustificationNotification<Block>>,
    ) -> Self {
        Self {
            finalization_proposals_tx,
            justification_rx,
        }
    }

    pub(crate) async fn run(mut self) {
        while let Some(notification) = self.justification_rx.next().await {
            self.finalization_proposals_tx
                .unbounded_send(notification)
                .expect("Notification should succeed");
        }

        log::error!(target: "afa", "Notification channel closed unexpectedly");
    }
}
