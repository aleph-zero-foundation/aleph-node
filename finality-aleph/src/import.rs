use crate::{data_io::finalize_block, justification::AlephJustification, AuthorityId};
use aleph_primitives::{AuthoritiesLog, ALEPH_ENGINE_ID};
use codec::Encode;
use sc_client_api::backend::Backend;
use sp_api::TransactionFor;
use sp_consensus::{
    BlockCheckParams, BlockImport, BlockImportParams, Error as ConsensusError, ImportResult,
    JustificationImport,
};
use sp_runtime::{
    generic::OpaqueDigestItemId,
    traits::{Block as BlockT, Header, NumberFor},
    Justification,
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};

pub struct AlephBlockImport<Block, Be, I>
where
    Block: BlockT,
    Be: Backend<Block>,
    I: crate::ClientForAleph<Block, Be>,
{
    inner: Arc<I>,
    authorities: Vec<AuthorityId>,
    _phantom: PhantomData<(Be, Block)>,
}

impl<Block, Be, I> AlephBlockImport<Block, Be, I>
where
    Block: BlockT,
    Be: Backend<Block>,
    I: crate::ClientForAleph<Block, Be>,
{
    pub fn new(inner: Arc<I>, authorities: Vec<AuthorityId>) -> AlephBlockImport<Block, Be, I> {
        AlephBlockImport {
            inner,
            authorities,
            _phantom: PhantomData,
        }
    }

    fn log_change(header: &Block::Header) {
        let id = OpaqueDigestItemId::Consensus(&ALEPH_ENGINE_ID);

        let log = header.digest().convert_first(|l| {
            l.try_to(id).map(
                |log: AuthoritiesLog<AuthorityId, NumberFor<Block>>| match log {
                    AuthoritiesLog::WillChange {
                        session_id,
                        when,
                        next_authorities,
                    } => (session_id, when, next_authorities),
                },
            )
        });

        if let Some((session_id, when, _)) = log {
            log::debug!(
                target: "afa",
                "Got new authorities for session #{:?} scheduled for block #{:?}", session_id, when
            );
        }
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
            authorities: self.authorities.clone(),
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
        let hash = block.header.hash();
        let justifications = block.justifications.take();

        Self::log_change(&block.header);

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
            let res = self.import_justification(hash, number, (ALEPH_ENGINE_ID, justification));
            res.unwrap_or_else(|_err| {
                imported_aux.bad_justification = true;
                imported_aux.needs_justification = true;
            });
        }

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
        log::debug!(target: "afa", "Importing justification for block #{:?}", number);
        if justification.0 != ALEPH_ENGINE_ID {
            return Err(ConsensusError::ClientImport(
                "Aleph can import only Aleph justifications.".into(),
            ));
        }

        if let Ok(justification) = AlephJustification::decode_and_verify::<Block>(
            &justification.1,
            hash,
            &self.authorities,
            number,
        ) {
            log::debug!(target: "afa", "Finalizing block #{:?} from justification import", number);
            finalize_block(
                Arc::clone(&self.inner),
                hash,
                number,
                Some((ALEPH_ENGINE_ID, justification.encode())),
            );
            Ok(())
        } else {
            Err(ConsensusError::ClientImport("Bad justification".into()))
        }
    }
}
