use std::marker::PhantomData;

use crate::{
    network::RequestBlocks as OldRequestBlocks, sync::RequestBlocks as NewRequestBlocks,
    BlockIdentifier,
};

/// BlockRequester that uses both new and old RequestBlocks so that
/// every request goes into old and new engine.
#[derive(Clone)]
pub struct OldSyncCompatibleRequestBlocks<OBR, NBR, BI>
where
    OBR: OldRequestBlocks<BI>,
    NBR: NewRequestBlocks<BI>,
    BI: BlockIdentifier,
{
    new: NBR,
    old: OBR,
    _phantom: PhantomData<BI>,
}

impl<OBR, NBR, BI> OldSyncCompatibleRequestBlocks<OBR, NBR, BI>
where
    OBR: OldRequestBlocks<BI>,
    NBR: NewRequestBlocks<BI>,
    BI: BlockIdentifier,
{
    pub fn new(old: OBR, new: NBR) -> Self {
        Self {
            new,
            old,
            _phantom: PhantomData,
        }
    }
}

impl<OBR, NBR, BI> NewRequestBlocks<BI> for OldSyncCompatibleRequestBlocks<OBR, NBR, BI>
where
    OBR: OldRequestBlocks<BI>,
    NBR: NewRequestBlocks<BI>,
    BI: BlockIdentifier,
{
    type Error = NBR::Error;

    fn request_block(&self, block_id: BI) -> Result<(), Self::Error> {
        self.old.request_stale_block(block_id.clone());

        self.new.request_block(block_id)
    }
}
