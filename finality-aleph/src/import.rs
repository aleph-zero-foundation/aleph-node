use std::{collections::HashMap, time::Instant};

use aleph_primitives::ALEPH_ENGINE_ID;
use futures::channel::mpsc::{TrySendError, UnboundedSender};
use log::{debug, warn};
use sc_consensus::{
    BlockCheckParams, BlockImport, BlockImportParams, ImportResult, JustificationImport,
};
use sp_consensus::Error as ConsensusError;
use sp_runtime::{
    traits::{Block as BlockT, Header, NumberFor},
    Justification,
};

use crate::{
    justification::{backwards_compatible_decode, DecodeError, JustificationNotification},
    metrics::{Checkpoint, Metrics},
};

/// A wrapper around a block import that also marks the start and end of the import of every block
/// in the metrics, if provided.
#[derive(Clone)]
pub struct TracingBlockImport<B, I>
where
    B: BlockT,
    I: BlockImport<B> + Send + Sync,
{
    inner: I,
    metrics: Option<Metrics<<B::Header as Header>::Hash>>,
}

impl<B, I> TracingBlockImport<B, I>
where
    B: BlockT,
    I: BlockImport<B> + Send + Sync,
{
    pub fn new(inner: I, metrics: Option<Metrics<<B::Header as Header>::Hash>>) -> Self {
        TracingBlockImport { inner, metrics }
    }
}
#[async_trait::async_trait]
impl<B, I> BlockImport<B> for TracingBlockImport<B, I>
where
    B: BlockT,
    I: BlockImport<B> + Send + Sync,
{
    type Error = I::Error;
    type Transaction = I::Transaction;

    async fn check_block(
        &mut self,
        block: BlockCheckParams<B>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner.check_block(block).await
    }

    async fn import_block(
        &mut self,
        block: BlockImportParams<B, Self::Transaction>,
        cache: HashMap<[u8; 4], Vec<u8>>,
    ) -> Result<ImportResult, Self::Error> {
        let post_hash = block.post_hash();
        if let Some(m) = &self.metrics {
            m.report_block(post_hash, Instant::now(), Checkpoint::Importing);
        };

        let result = self.inner.import_block(block, cache).await;

        if let (Some(m), Ok(ImportResult::Imported(_))) = (&self.metrics, &result) {
            m.report_block(post_hash, Instant::now(), Checkpoint::Imported);
        }
        result
    }
}

/// A wrapper around a block import that also extracts any present jsutifications and send them to
/// our components which will process them further and possibly finalize the block.
#[derive(Clone)]
pub struct AlephBlockImport<B, I>
where
    B: BlockT,
    I: BlockImport<B> + Clone + Send,
{
    inner: I,
    justification_tx: UnboundedSender<JustificationNotification<B>>,
}

#[derive(Debug)]
enum SendJustificationError<B: BlockT> {
    Send(TrySendError<JustificationNotification<B>>),
    Consensus(Box<ConsensusError>),
    Decode(DecodeError),
}

impl<B: BlockT> From<DecodeError> for SendJustificationError<B> {
    fn from(decode_error: DecodeError) -> Self {
        Self::Decode(decode_error)
    }
}

impl<B, I> AlephBlockImport<B, I>
where
    B: BlockT,
    I: BlockImport<B> + Clone + Send,
{
    pub fn new(
        inner: I,
        justification_tx: UnboundedSender<JustificationNotification<B>>,
    ) -> AlephBlockImport<B, I> {
        AlephBlockImport {
            inner,
            justification_tx,
        }
    }

    fn send_justification(
        &mut self,
        hash: B::Hash,
        number: NumberFor<B>,
        justification: Justification,
    ) -> Result<(), SendJustificationError<B>> {
        debug!(target: "aleph-justification", "Importing justification for block {:?}", number);
        if justification.0 != ALEPH_ENGINE_ID {
            return Err(SendJustificationError::Consensus(Box::new(
                ConsensusError::ClientImport("Aleph can import only Aleph justifications.".into()),
            )));
        }
        let justification_raw = justification.1;
        let aleph_justification = backwards_compatible_decode(justification_raw)?;

        self.justification_tx
            .unbounded_send(JustificationNotification {
                hash,
                number,
                justification: aleph_justification,
            })
            .map_err(SendJustificationError::Send)
    }
}

#[async_trait::async_trait]
impl<B, I> BlockImport<B> for AlephBlockImport<B, I>
where
    B: BlockT,
    I: BlockImport<B> + Clone + Send,
{
    type Error = I::Error;
    type Transaction = I::Transaction;

    async fn check_block(
        &mut self,
        block: BlockCheckParams<B>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner.check_block(block).await
    }

    async fn import_block(
        &mut self,
        mut block: BlockImportParams<B, Self::Transaction>,
        cache: HashMap<[u8; 4], Vec<u8>>,
    ) -> Result<ImportResult, Self::Error> {
        let number = *block.header.number();
        let post_hash = block.post_hash();

        let justifications = block.justifications.take();

        debug!(target: "aleph-justification", "Importing block {:?} {:?} {:?}", number, block.header.hash(), block.post_hash());
        let result = self.inner.import_block(block, cache).await;

        if let Ok(ImportResult::Imported(_)) = result {
            if let Some(justification) =
                justifications.and_then(|just| just.into_justification(ALEPH_ENGINE_ID))
            {
                debug!(target: "aleph-justification", "Got justification along imported block {:?}", number);

                if let Err(e) =
                    self.send_justification(post_hash, number, (ALEPH_ENGINE_ID, justification))
                {
                    warn!(target: "aleph-justification", "Error while receiving justification for block {:?}: {:?}", post_hash, e);
                }
            }
        }

        result
    }
}

#[async_trait::async_trait]
impl<B, I> JustificationImport<B> for AlephBlockImport<B, I>
where
    B: BlockT,
    I: BlockImport<B> + Clone + Send,
{
    type Error = ConsensusError;

    async fn on_start(&mut self) -> Vec<(B::Hash, NumberFor<B>)> {
        debug!(target: "aleph-justification", "On start called");
        Vec::new()
    }

    async fn import_justification(
        &mut self,
        hash: B::Hash,
        number: NumberFor<B>,
        justification: Justification,
    ) -> Result<(), Self::Error> {
        debug!(target: "aleph-justification", "import_justification called on {:?}", justification);
        self.send_justification(hash, number, justification)
            .map_err(|error| match error {
                SendJustificationError::Send(_) => ConsensusError::ClientImport(String::from(
                    "Could not send justification to ConsensusParty",
                )),
                SendJustificationError::Consensus(e) => *e,
                SendJustificationError::Decode(e) => {
                    warn!(target: "aleph-justification", "Justification for block {:?} decoded incorrectly: {}", number, e);
                    ConsensusError::ClientImport(String::from("Could not decode justification"))
                }
            })
    }
}
