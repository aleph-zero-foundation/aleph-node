use sp_runtime::traits::Block as BlockT;

use crate::{
    data_io::{AlephData, UnvalidatedAlephProposal},
    testing::mocks::{TBlock, THeader},
};

pub fn unvalidated_proposal_from_headers(
    headers: Vec<THeader>,
) -> UnvalidatedAlephProposal<TBlock> {
    let num = headers.last().unwrap().number;
    let hashes = headers.into_iter().map(|header| header.hash()).collect();
    UnvalidatedAlephProposal::new(hashes, num)
}

pub fn aleph_data_from_blocks(blocks: Vec<TBlock>) -> AlephData<TBlock> {
    let headers = blocks.into_iter().map(|b| b.header().clone()).collect();
    aleph_data_from_headers(headers)
}

pub fn aleph_data_from_headers(headers: Vec<THeader>) -> AlephData<TBlock> {
    AlephData {
        head_proposal: unvalidated_proposal_from_headers(headers),
    }
}
