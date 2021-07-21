use crate::metrics::Metrics;
use aleph_primitives::ALEPH_ENGINE_ID;
use futures::channel::mpsc::{TrySendError, UnboundedSender};
use sc_client_api::backend::Backend;
use sp_api::TransactionFor;
use sp_consensus::{
    BlockCheckParams, BlockImport, BlockImportParams, Error as ConsensusError, ImportResult,
    JustificationImport,
};
use sp_runtime::{
    traits::{Block as BlockT, Header, NumberFor},
    Justification,
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Instant};

pub struct AlephBlockImport<Block, Be, I>
where
    Block: BlockT,
    Be: Backend<Block>,
    I: crate::ClientForAleph<Block, Be>,
{
    inner: Arc<I>,
    justification_tx: UnboundedSender<JustificationNotification<Block>>,
    metrics: Option<Metrics<Block::Header>>,
    _phantom: PhantomData<Be>,
}

enum SendJustificationError<Block>
where
    Block: BlockT,
{
    Send(TrySendError<JustificationNotification<Block>>),
    Consensus(Box<ConsensusError>),
}

impl<Block, Be, I> AlephBlockImport<Block, Be, I>
where
    Block: BlockT,
    Be: Backend<Block>,
    I: crate::ClientForAleph<Block, Be>,
{
    pub fn new(
        inner: Arc<I>,
        justification_tx: UnboundedSender<JustificationNotification<Block>>,
        metrics: Option<Metrics<Block::Header>>,
    ) -> AlephBlockImport<Block, Be, I> {
        AlephBlockImport {
            inner,
            justification_tx,
            metrics,
            _phantom: PhantomData,
        }
    }

    fn send_justification(
        &mut self,
        hash: Block::Hash,
        number: NumberFor<Block>,
        justification: Justification,
    ) -> Result<(), SendJustificationError<Block>> {
        log::debug!(target: "afa", "Importing justification for block #{:?}", number);
        if justification.0 != ALEPH_ENGINE_ID {
            return Err(SendJustificationError::Consensus(Box::new(
                ConsensusError::ClientImport("Aleph can import only Aleph justifications.".into()),
            )));
        }
        self.justification_tx
            .unbounded_send(JustificationNotification {
                hash,
                number,
                justification: justification.1,
            })
            .map_err(SendJustificationError::Send)
    }
}

impl<Block, Be, I> Clone for AlephBlockImport<Block, Be, I>
where
    Block: BlockT,
    Be: Backend<Block>,
    I: crate::ClientForAleph<Block, Be>,
{
    fn clone(&self) -> Self {
        AlephBlockImport {
            inner: self.inner.clone(),
            justification_tx: self.justification_tx.clone(),
            metrics: self.metrics.clone(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<Block, Be, I> BlockImport<Block> for AlephBlockImport<Block, Be, I>
where
    Block: BlockT,
    Be: Backend<Block>,
    I: crate::ClientForAleph<Block, Be> + Send,
    for<'a> &'a I:
        BlockImport<Block, Error = ConsensusError, Transaction = TransactionFor<I, Block>>,
    TransactionFor<I, Block>: Send + 'static,
{
    type Error = <I as BlockImport<Block>>::Error;
    type Transaction = TransactionFor<I, Block>;

    async fn check_block(
        &mut self,
        block: BlockCheckParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        self.inner.check_block(block).await
    }

    async fn import_block(
        &mut self,
        mut block: BlockImportParams<Block, Self::Transaction>,
        cache: HashMap<[u8; 4], Vec<u8>>,
    ) -> Result<ImportResult, Self::Error> {
        let number = *block.header.number();
        let hash = block.post_hash();
        let justifications = block.justifications.take();

        if let Some(m) = &self.metrics {
            m.report_block(hash, Instant::now(), "importing");
        };

        log::debug!(target: "afa", "Importing block #{:?}", number);
        let import_result = self.inner.import_block(block, cache).await;

        let mut imported_aux = match import_result {
            Ok(ImportResult::Imported(aux)) => aux,
            Ok(r) => return Ok(r),
            Err(e) => return Err(e),
        };

        if let Some(justification) =
            justifications.and_then(|just| just.into_justification(ALEPH_ENGINE_ID))
        {
            match self.send_justification(hash, number, (ALEPH_ENGINE_ID, justification)) {
                Err(SendJustificationError::Send(_)) => {
                    imported_aux.needs_justification = true;
                }
                Err(SendJustificationError::Consensus(_)) => {
                    imported_aux.bad_justification = true;
                    imported_aux.needs_justification = true
                }
                Ok(_) => (),
            };
        } else {
            imported_aux.needs_justification = true;
        }

        if let Some(m) = &self.metrics {
            m.report_block(hash, Instant::now(), "imported");
        };

        Ok(ImportResult::Imported(imported_aux))
    }
}

impl<Block, Be, I> JustificationImport<Block> for AlephBlockImport<Block, Be, I>
where
    Block: BlockT,
    Be: Backend<Block>,
    I: crate::ClientForAleph<Block, Be>,
{
    type Error = ConsensusError;

    fn on_start(&mut self) -> Vec<(Block::Hash, NumberFor<Block>)> {
        log::debug!(target: "afa", "On start called");
        Vec::new()
    }

    fn import_justification(
        &mut self,
        hash: Block::Hash,
        number: NumberFor<Block>,
        justification: Justification,
    ) -> Result<(), Self::Error> {
        match self.send_justification(hash, number, justification) {
            Err(SendJustificationError::Send(_)) => Err(ConsensusError::ClientImport(
                String::from("Could not send justification to ConsensusParty"),
            )),
            Err(SendJustificationError::Consensus(error)) => Err(*error),
            Ok(()) => Ok(()),
        }
    }
}

pub struct JustificationNotification<Block>
where
    Block: BlockT,
{
    pub justification: Vec<u8>,
    pub hash: Block::Hash,
    pub number: NumberFor<Block>,
}
