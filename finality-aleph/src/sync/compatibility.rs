use crate::{
    network::RequestBlocks as OldRequestBlocks, sync::RequestBlocks as NewRequestBlocks, BlockId,
};

/// BlockRequester that uses both new and old RequestBlocks so that
/// every request goes into old and new engine.
#[derive(Clone)]
pub struct OldSyncCompatibleRequestBlocks<OBR, NBR>
where
    OBR: OldRequestBlocks,
    NBR: NewRequestBlocks,
{
    new: NBR,
    old: OBR,
}

impl<OBR, NBR> OldSyncCompatibleRequestBlocks<OBR, NBR>
where
    OBR: OldRequestBlocks,
    NBR: NewRequestBlocks,
{
    pub fn new(old: OBR, new: NBR) -> Self {
        Self { new, old }
    }
}

impl<OBR, NBR> NewRequestBlocks for OldSyncCompatibleRequestBlocks<OBR, NBR>
where
    OBR: OldRequestBlocks,
    NBR: NewRequestBlocks,
{
    type Error = NBR::Error;

    fn request_block(&self, block_id: BlockId) -> Result<(), Self::Error> {
        self.old.request_stale_block(block_id.clone());

        self.new.request_block(block_id)
    }
}
