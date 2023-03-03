use aleph_primitives::{BlockNumber, ALEPH_ENGINE_ID};
use sc_client_api::{Backend, Finalizer as SubstrateFinalizer, HeaderBackend, LockImportRun};
use sp_blockchain::Error as ClientError;
use sp_runtime::traits::{Block as BlockT, Header as SubstrateHeader};

use crate::{
    finalization::{AlephFinalizer, BlockFinalizer},
    justification::versioned_encode,
    sync::{substrate::Justification, Finalizer},
};

impl<B, BE, C> Finalizer<Justification<B::Header>> for AlephFinalizer<B, BE, C>
where
    B: BlockT,
    B::Header: SubstrateHeader<Number = BlockNumber>,
    BE: Backend<B>,
    C: HeaderBackend<B> + LockImportRun<B, BE> + SubstrateFinalizer<B, BE>,
{
    type Error = ClientError;

    fn finalize(&self, justification: Justification<B::Header>) -> Result<(), Self::Error> {
        self.finalize_block(
            justification.header.hash(),
            *justification.header.number(),
            Some((
                ALEPH_ENGINE_ID,
                versioned_encode(justification.raw_justification),
            )),
        )
    }
}
