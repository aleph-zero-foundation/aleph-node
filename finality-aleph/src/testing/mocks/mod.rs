pub use acceptance_policy::AcceptancePolicy;
use aleph_primitives::BlockNumber;
pub use backend::{create_block, Backend};
pub use block_finalizer::MockedBlockFinalizer;
pub use block_request::MockedBlockRequester;
pub use client::{TestClient, TestClientBuilder, TestClientBuilderExt};
pub use justification_handler_config::JustificationRequestSchedulerImpl;
pub use proposal::{
    aleph_data_from_blocks, aleph_data_from_headers, unvalidated_proposal_from_headers,
};
pub use session_info::{SessionInfoProviderImpl, VerifierWrapper};
use sp_runtime::traits::BlakeTwo256;
use substrate_test_runtime::Extrinsic;

type Hashing = BlakeTwo256;
pub type TBlock = sp_runtime::generic::Block<THeader, Extrinsic>;
pub type THeader = sp_runtime::generic::Header<BlockNumber, Hashing>;
pub type THash = substrate_test_runtime::Hash;

mod acceptance_policy;
mod backend;
mod block_finalizer;
mod block_request;
mod client;
mod justification_handler_config;
mod proposal;
mod session_info;
mod single_action_mock;
