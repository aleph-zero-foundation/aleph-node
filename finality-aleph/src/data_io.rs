use crate::{Error, Metrics};
use aleph_bft::OrderedBatch;
use futures::channel::mpsc;
use log::debug;
use sp_consensus::SelectChain;
use sp_runtime::traits::{Block, Header};

#[derive(Clone)]
pub(crate) struct DataIO<B: Block, SC: SelectChain<B>> {
    pub(crate) select_chain: SC,
    pub(crate) ordered_batch_tx: mpsc::UnboundedSender<OrderedBatch<B::Hash>>,
    pub(crate) metrics: Option<Metrics<B::Header>>,
}

impl<B: Block, SC: SelectChain<B>> aleph_bft::DataIO<B::Hash> for DataIO<B, SC> {
    type Error = Error;

    fn get_data(&self) -> B::Hash {
        let header = self.select_chain.best_chain().expect("No best chain");

        if let Some(m) = &self.metrics {
            m.report_block(header.hash(), std::time::Instant::now(), "get_data");
        }
        debug!(target: "afa", "Outputting {:?} in get_data", header.hash());
        header.hash()
    }

    fn send_ordered_batch(&mut self, batch: OrderedBatch<B::Hash>) -> Result<(), Self::Error> {
        // TODO: add better conversion
        self.ordered_batch_tx
            .unbounded_send(batch)
            .map_err(|_| Error::SendData)
    }
}
