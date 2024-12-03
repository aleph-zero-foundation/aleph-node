//! Implementations and definitions of traits used in current abft
use crate::{
    block::{Header, HeaderVerifier, UnverifiedHeader},
    data_io::{AlephData, ChainInfoProvider, DataProvider, OrderedDataInterpreter},
    Hasher,
};

#[async_trait::async_trait]
impl<UH: UnverifiedHeader> current_aleph_bft::DataProvider for DataProvider<UH> {
    type Output = AlephData<UH>;

    async fn get_data(&mut self) -> Option<AlephData<UH>> {
        DataProvider::get_data(self).await
    }
}

impl<CIP, H, V> current_aleph_bft::UnitFinalizationHandler for OrderedDataInterpreter<CIP, H, V>
where
    CIP: ChainInfoProvider,
    H: Header,
    V: HeaderVerifier<H>,
{
    type Data = AlephData<H::Unverified>;
    type Hasher = Hasher;

    fn batch_finalized(
        &mut self,
        batch: Vec<current_aleph_bft::OrderedUnit<Self::Data, Self::Hasher>>,
    ) {
        // TODO(A0-4575): compute performance scores.
        for unit in batch {
            if let Some(data) = unit.data {
                self.data_finalized(data)
            }
        }
    }
}
