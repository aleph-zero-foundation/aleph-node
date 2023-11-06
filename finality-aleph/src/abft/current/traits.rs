//! Implementations and definitions of traits used in current abft
use crate::data_io::{AlephData, ChainInfoProvider, DataProvider, OrderedDataInterpreter};

#[async_trait::async_trait]
impl current_aleph_bft::DataProvider<AlephData> for DataProvider {
    async fn get_data(&mut self) -> Option<AlephData> {
        DataProvider::get_data(self).await
    }
}

impl<CIP> current_aleph_bft::FinalizationHandler<AlephData> for OrderedDataInterpreter<CIP>
where
    CIP: ChainInfoProvider,
{
    fn data_finalized(&mut self, data: AlephData) {
        OrderedDataInterpreter::data_finalized(self, data)
    }
}
