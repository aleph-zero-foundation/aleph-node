use sp_blockchain::Error;
use sp_runtime::{traits::Block, Justification};

use crate::{
    finalization::BlockFinalizer,
    testing::mocks::{single_action_mock::SingleActionMock, TBlock},
    BlockId,
};
type CallArgs = (BlockId, Justification);

#[derive(Clone, Default)]
pub struct MockedBlockFinalizer {
    mock: SingleActionMock<CallArgs>,
}

impl MockedBlockFinalizer {
    pub fn new() -> Self {
        Self {
            mock: Default::default(),
        }
    }

    pub async fn has_not_been_invoked(&self) -> bool {
        self.mock.has_not_been_invoked().await
    }

    pub async fn has_been_invoked_with(&self, block: TBlock) -> bool {
        self.mock
            .has_been_invoked_with(|(BlockId { hash, number }, _)| {
                block.hash() == hash && block.header.number == number
            })
            .await
    }
}

impl BlockFinalizer for MockedBlockFinalizer {
    fn finalize_block(&self, block_id: BlockId, justification: Justification) -> Result<(), Error> {
        self.mock.invoke_with((block_id, justification));
        Ok(())
    }
}
