use aleph_primitives::BlockNumber;
use sp_runtime::traits::Block;

use crate::{
    network::RequestBlocks,
    testing::mocks::{single_action_mock::SingleActionMock, TBlock, THash},
};

type CallArgs = (THash, BlockNumber);

#[derive(Clone, Default)]
pub struct MockedBlockRequester {
    mock: SingleActionMock<CallArgs>,
}

impl MockedBlockRequester {
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
            .has_been_invoked_with(|(hash, number)| {
                block.hash() == hash && block.header.number == number
            })
            .await
    }
}

impl RequestBlocks<TBlock> for MockedBlockRequester {
    fn request_justification(&self, hash: &THash, number: BlockNumber) {
        self.mock.invoke_with((*hash, number))
    }

    fn request_stale_block(&self, _hash: THash, _number: BlockNumber) {
        panic!("`request_stale_block` not implemented!")
    }

    /// Clear all pending justification requests.
    fn clear_justification_requests(&self) {
        panic!("`clear_justification_requests` not implemented!")
    }
}
