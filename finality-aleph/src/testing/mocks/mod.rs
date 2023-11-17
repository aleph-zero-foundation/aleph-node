use std::fmt::{Display, Error as FmtError, Formatter};

pub use acceptance_policy::AcceptancePolicy;
pub use block_finalizer::MockedBlockFinalizer;
pub use client::{TestClient, TestClientBuilder, TestClientBuilderExt};
pub use proposal::{
    aleph_data_from_blocks, aleph_data_from_headers, unvalidated_proposal_from_headers,
};
use sp_runtime::traits::BlakeTwo256;
use substrate_test_runtime::Extrinsic;

use crate::{
    aleph_primitives::BlockNumber,
    block::{EquivocationProof, HeaderVerifier, VerifiedHeader},
};

type Hashing = BlakeTwo256;
pub type TBlock = sp_runtime::generic::Block<THeader, Extrinsic>;
pub type THeader = sp_runtime::generic::Header<BlockNumber, Hashing>;
pub type THash = substrate_test_runtime::Hash;

#[derive(Clone)]
pub struct TestVerifier;

pub struct TestEquivocationProof;

impl Display for TestEquivocationProof {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "this should never get created")
    }
}

impl EquivocationProof for TestEquivocationProof {
    fn are_we_equivocating(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct TestVerificationError;

impl Display for TestVerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "this should never get created")
    }
}

impl HeaderVerifier<THeader> for TestVerifier {
    type EquivocationProof = TestEquivocationProof;
    type Error = TestVerificationError;

    fn verify_header(
        &mut self,
        header: THeader,
        _just_created: bool,
    ) -> Result<VerifiedHeader<THeader, Self::EquivocationProof>, Self::Error> {
        Ok(VerifiedHeader {
            header,
            maybe_equivocation_proof: None,
        })
    }
}

mod acceptance_policy;
mod block_finalizer;
mod client;
mod proposal;
mod single_action_mock;
