use sp_runtime::traits::Block as BlockT;
use substrate_test_runtime_client::runtime::{Block, Header};

use crate::data_io::{AlephData, UnvalidatedAlephProposal};

pub fn unvalidated_proposal_from_headers(headers: Vec<Header>) -> UnvalidatedAlephProposal<Block> {
    let num = headers.last().unwrap().number;
    let hashes = headers.into_iter().map(|header| header.hash()).collect();
    UnvalidatedAlephProposal::new(hashes, num)
}

pub fn aleph_data_from_blocks(blocks: Vec<Block>) -> AlephData<Block> {
    let headers = blocks.into_iter().map(|b| b.header().clone()).collect();
    aleph_data_from_headers(headers)
}

pub fn aleph_data_from_headers(headers: Vec<Header>) -> AlephData<Block> {
    AlephData {
        head_proposal: unvalidated_proposal_from_headers(headers),
    }
}
