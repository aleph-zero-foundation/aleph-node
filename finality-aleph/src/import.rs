use std::{
    collections::HashMap,
    fmt::Debug,
    time::{Duration, Instant},
};

use aleph_primitives::{BlockNumber, ALEPH_ENGINE_ID};
use futures::channel::mpsc::{TrySendError, UnboundedSender};
use log::{debug, warn};
use sc_consensus::{
    BlockCheckParams, BlockImport, BlockImportParams, ImportResult, JustificationImport,
};
use sp_consensus::{BlockOrigin, Error as ConsensusError};
use sp_runtime::{
    traits::{Block as BlockT, Header},
    Justification as SubstrateJustification,
};
use tokio::time::sleep;

use crate::{
    justification::{backwards_compatible_decode, DecodeError},
    metrics::{Checkpoint, Metrics},
    sync::substrate::{Justification, JustificationTranslator},
    BlockId,
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
    metrics: Metrics<<B::Header as Header>::Hash>,
}

impl<B, I> TracingBlockImport<B, I>
where
    B: BlockT,
    I: BlockImport<B> + Send + Sync,
{
    pub fn new(inner: I, metrics: Metrics<<B::Header as Header>::Hash>) -> Self {
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
        self.metrics
            .report_block(post_hash, Instant::now(), Checkpoint::Importing);

        let result = self.inner.import_block(block, cache).await;

        if let Ok(ImportResult::Imported(_)) = &result {
            self.metrics
                .report_block(post_hash, Instant::now(), Checkpoint::Imported);
        }
        result
    }
}

/// A wrapper around a block import that also extracts any present jsutifications and send them to
/// our components which will process them further and possibly finalize the block. It also makes
/// blocks from major sync import slightly slower than they normally would, to avoid breaking the
/// new justificaiton sync. The last part will be removed once we finish rewriting the block sync.
#[derive(Clone)]
pub struct AlephBlockImport<B, I, JT>
where
    B: BlockT,
    B::Header: Header<Number = BlockNumber>,
    I: BlockImport<B> + Clone + Send,
    JT: JustificationTranslator<B::Header>,
{
    inner: I,
    justification_tx: UnboundedSender<Justification<<B as BlockT>::Header>>,
    translator: JT,
}

#[derive(Debug)]
enum SendJustificationError<H: Header<Number = BlockNumber>, TE: Debug> {
    Send(TrySendError<Justification<H>>),
    Consensus(Box<ConsensusError>),
    Decode(DecodeError),
    Translate(TE),
}

impl<H: Header<Number = BlockNumber>, TE: Debug> From<DecodeError>
    for SendJustificationError<H, TE>
{
    fn from(decode_error: DecodeError) -> Self {
        Self::Decode(decode_error)
    }
}

impl<B, I, JT> AlephBlockImport<B, I, JT>
where
    B: BlockT,
    B::Header: Header<Number = BlockNumber>,
    I: BlockImport<B> + Clone + Send,
    JT: JustificationTranslator<B::Header>,
{
    pub fn new(
        inner: I,
        justification_tx: UnboundedSender<Justification<B::Header>>,
        translator: JT,
    ) -> AlephBlockImport<B, I, JT> {
        AlephBlockImport {
            inner,
            justification_tx,
            translator,
        }
    }

    fn send_justification(
        &mut self,
        block_id: BlockId<B::Header>,
        justification: SubstrateJustification,
    ) -> Result<(), SendJustificationError<B::Header, JT::Error>> {
        debug!(target: "aleph-justification", "Importing justification for block {}.", block_id);
        if justification.0 != ALEPH_ENGINE_ID {
            return Err(SendJustificationError::Consensus(Box::new(
                ConsensusError::ClientImport("Aleph can import only Aleph justifications.".into()),
            )));
        }
        let justification_raw = justification.1;
        let aleph_justification = backwards_compatible_decode(justification_raw)?;
        let justification = self
            .translator
            .translate(aleph_justification, block_id)
            .map_err(SendJustificationError::Translate)?;

        self.justification_tx
            .unbounded_send(justification)
            .map_err(SendJustificationError::Send)
    }
}

#[async_trait::async_trait]
impl<B, I, JT> BlockImport<B> for AlephBlockImport<B, I, JT>
where
    B: BlockT,
    B::Header: Header<Number = BlockNumber>,
    I: BlockImport<B> + Clone + Send,
    JT: JustificationTranslator<B::Header>,
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

        if matches!(block.origin, BlockOrigin::NetworkInitialSync) {
            sleep(Duration::from_millis(2)).await;
        }
        debug!(target: "aleph-justification", "Importing block {:?} {:?} {:?}", number, block.header.hash(), block.post_hash());
        let result = self.inner.import_block(block, cache).await;

        if let Ok(ImportResult::Imported(_)) = result {
            if let Some(justification) =
                justifications.and_then(|just| just.into_justification(ALEPH_ENGINE_ID))
            {
                debug!(target: "aleph-justification", "Got justification along imported block {:?}", number);

                if let Err(e) = self.send_justification(
                    BlockId::new(post_hash, number),
                    (ALEPH_ENGINE_ID, justification),
                ) {
                    warn!(target: "aleph-justification", "Error while receiving justification for block {:?}: {:?}", post_hash, e);
                }
            }
        }

        result
    }
}

#[async_trait::async_trait]
impl<B, I, JT> JustificationImport<B> for AlephBlockImport<B, I, JT>
where
    B: BlockT,
    B::Header: Header<Number = BlockNumber>,
    I: BlockImport<B> + Clone + Send,
    JT: JustificationTranslator<B::Header>,
{
    type Error = ConsensusError;

    async fn on_start(&mut self) -> Vec<(B::Hash, BlockNumber)> {
        debug!(target: "aleph-justification", "On start called");
        Vec::new()
    }

    async fn import_justification(
        &mut self,
        hash: B::Hash,
        number: BlockNumber,
        justification: SubstrateJustification,
    ) -> Result<(), Self::Error> {
        use SendJustificationError::*;
        debug!(target: "aleph-justification", "import_justification called on {:?}", justification);
        self.send_justification(BlockId::new(hash, number), justification)
            .map_err(|error| match error {
                Send(_) => ConsensusError::ClientImport(String::from(
                    "Could not send justification to ConsensusParty",
                )),
                Consensus(e) => *e,
                Decode(e) => ConsensusError::ClientImport(format!(
                    "Justification for block {:?} decoded incorrectly: {}",
                    number, e
                )),
                Translate(e) => ConsensusError::ClientImport(format!(
                    "Could not translate justification: {}",
                    e
                )),
            })
    }
}
