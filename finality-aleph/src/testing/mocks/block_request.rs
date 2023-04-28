use sp_runtime::traits::Block;

use crate::{
    network::RequestBlocks,
    testing::mocks::{single_action_mock::SingleActionMock, TBlock},
    IdentifierFor,
};

type CallArgs = IdentifierFor<TBlock>;

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
            .has_been_invoked_with(|block_id| {
                block.hash() == block_id.hash && block.header.number == block_id.number
            })
            .await
    }
}

impl RequestBlocks<IdentifierFor<TBlock>> for MockedBlockRequester {
    fn request_justification(&self, block_id: IdentifierFor<TBlock>) {
        self.mock.invoke_with(block_id)
    }

    fn request_stale_block(&self, _block_id: IdentifierFor<TBlock>) {
        panic!("`request_stale_block` not implemented!")
    }

    /// Clear all pending justification requests.
    fn clear_justification_requests(&self) {
        panic!("`clear_justification_requests` not implemented!")
    }
}
