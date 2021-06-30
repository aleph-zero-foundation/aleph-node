use crate::Error;
use aleph_bft::OrderedBatch;
use futures::channel::mpsc;
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block, Header};
use std::{future::Future, pin::Pin};

#[derive(Clone)]
pub(crate) struct DataIO<B: Block, SC: SelectChain<B>> {
    pub(crate) select_chain: SC,
    pub(crate) ordered_batch_tx: mpsc::UnboundedSender<OrderedBatch<B::Hash>>,
}

impl<B: Block, SC: SelectChain<B>> aleph_bft::DataIO<B::Hash> for DataIO<B, SC> {
    type Error = Error;

    fn get_data(&self) -> B::Hash {
        self.select_chain
            .best_chain()
            .expect("No best chain")
            .hash()
    }

    #[allow(clippy::type_complexity)]
    fn check_availability(
        &self,
        _data: &<B as Block>::Hash,
    ) -> Option<Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>> {
        // TODO: implement actual logic
        None
    }

    fn send_ordered_batch(&mut self, batch: OrderedBatch<B::Hash>) -> Result<(), Self::Error> {
        // TODO: add better conversion
        self.ordered_batch_tx
            .unbounded_send(batch)
            .map_err(|_| Error::SendData)
    }
}
