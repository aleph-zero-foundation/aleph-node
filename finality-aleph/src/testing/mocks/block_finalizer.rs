use sp_blockchain::Error;
use sp_runtime::{traits::Block, Justification};

use crate::{
    finalization::BlockFinalizer,
    testing::mocks::{single_action_mock::SingleActionMock, TBlock, THash, TNumber},
};

type CallArgs = (THash, TNumber, Option<Justification>);

#[derive(Clone)]
pub(crate) struct MockedBlockFinalizer {
    mock: SingleActionMock<CallArgs>,
}

impl MockedBlockFinalizer {
    pub(crate) fn new() -> Self {
        Self {
            mock: Default::default(),
        }
    }

    pub(crate) async fn has_not_been_invoked(&self) -> bool {
        self.mock.has_not_been_invoked().await
    }

    pub(crate) async fn has_been_invoked_with(&self, block: TBlock) -> bool {
        self.mock
            .has_been_invoked_with(|(hash, number, _)| {
                block.hash() == hash && block.header.number == number
            })
            .await
    }
}

impl BlockFinalizer<TBlock> for MockedBlockFinalizer {
    fn finalize_block(
        &self,
        hash: THash,
        block_number: TNumber,
        justification: Option<Justification>,
    ) -> Result<(), Error> {
        self.mock.invoke_with((hash, block_number, justification));
        Ok(())
    }
}
