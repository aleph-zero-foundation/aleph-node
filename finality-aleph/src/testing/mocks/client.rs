use std::sync::Arc;

use primitives::{fake_runtime_api, BlockHash, BlockNumber};
use sp_blockchain::HeaderBackend;
use substrate_test_client::{client, sc_client_db, sc_executor};
use substrate_test_runtime_client::{GenesisParameters, LocalExecutorDispatch};

use crate::{
    block::HeaderBackend as AlephHeaderBackend,
    testing::mocks::{TBlock, THeader},
    BlockId,
};

// /// A `TestClient` with `test-runtime` builder.
pub type TestClientBuilder<E, B> =
    substrate_test_client::TestClientBuilder<TBlock, E, B, GenesisParameters>;

/// Call executor for `kitchensink-runtime` `TestClient`.
pub type ExecutorDispatch = sc_executor::NativeElseWasmExecutor<LocalExecutorDispatch>;

/// Default backend type.
pub type Backend = sc_client_db::Backend<TBlock>;

/// Test client type.
pub type TestClient = client::Client<
    Backend,
    client::LocalCallExecutor<TBlock, Backend, ExecutorDispatch>,
    TBlock,
    fake_runtime_api::fake_runtime::RuntimeApi,
>;

/// A `test-runtime` extensions to `TestClientBuilder`.
pub trait TestClientBuilderExt: Sized {
    /// Create test client builder.
    fn new() -> Self;

    /// Build the test client.
    fn build(self) -> TestClient;

    /// Build the test client and longest chain selector.
    fn build_with_longest_chain(self) -> (TestClient, sc_consensus::LongestChain<Backend, TBlock>);

    /// Build the test client and the backend.
    fn build_with_backend(self) -> (TestClient, Arc<Backend>);
}

impl TestClientBuilderExt
    for substrate_test_client::TestClientBuilder<
        TBlock,
        client::LocalCallExecutor<TBlock, Backend, ExecutorDispatch>,
        Backend,
        GenesisParameters,
    >
{
    fn new() -> Self {
        Self::default()
    }

    fn build(self) -> TestClient {
        self.build_with_native_executor(None).0
    }

    fn build_with_longest_chain(self) -> (TestClient, sc_consensus::LongestChain<Backend, TBlock>) {
        self.build_with_native_executor(None)
    }

    fn build_with_backend(self) -> (TestClient, Arc<Backend>) {
        let backend = self.backend();
        (self.build_with_native_executor(None).0, backend)
    }
}

// TODO: remove when we will have abstraction for block import and use block::mock::Backend
// instead of TestClient.
impl AlephHeaderBackend<THeader> for Arc<TestClient> {
    type Error = sp_blockchain::Error;

    fn header(&self, id: &BlockId) -> Result<Option<THeader>, Self::Error> {
        TestClient::header(self, id.hash())
    }

    fn header_of_finalized_at(&self, number: BlockNumber) -> Result<Option<THeader>, Self::Error> {
        match TestClient::hash(self, number) {
            Ok(Some(hash)) => {
                if self.top_finalized_id().number() >= number {
                    Ok(Some(
                        self.header(&(hash, number).into())?
                            .expect("header must exist"),
                    ))
                } else {
                    Ok(None)
                }
            }
            Ok(None) => Err(sp_blockchain::Error::UnknownBlocks("{number}".into())),
            Err(e) => Err(e),
        }
    }

    fn top_finalized_id(&self) -> BlockId {
        let info = self.chain_info();
        (info.finalized_hash, info.finalized_number).into()
    }

    fn hash_to_id(&self, hash: BlockHash) -> Result<Option<BlockId>, Self::Error> {
        match TestClient::number(self, hash) {
            Ok(Some(number)) => Ok(Some((hash, number).into())),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
