use core::result::Result;
use log::{debug, error, warn};
use sc_client_api::Backend;
use sp_api::{BlockId, NumberFor};
use sp_runtime::{
    traits::{Block, Header},
    Justification,
};

use std::{collections::VecDeque, sync::Arc};

pub(crate) fn finalize_block<BE, B, C>(
    client: Arc<C>,
    hash: B::Hash,
    block_number: NumberFor<B>,
    justification: Option<Justification>,
) -> Result<(), sp_blockchain::Error>
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let status = client.info();
    if status.finalized_number >= block_number {
        warn!(target: "afa", "trying to finalize a block with hash {} and number {}
               that is not greater than already finalized {}", hash, block_number, status.finalized_number);
    }

    debug!(target: "afa", "Finalizing block with hash {:?} and number {:?}. Previous best: #{:?}.", hash, block_number, status.finalized_number);

    let update_res = client.lock_import_and_run(|import_op| {
        // NOTE: all other finalization logic should come here, inside the lock
        client.apply_finality(import_op, BlockId::Hash(hash), justification, true)
    });
    let status = client.info();
    debug!(target: "afa", "Attempted to finalize block with hash {:?}. Current best: #{:?}.", hash, status.finalized_number);
    update_res
}

/// Given hashes `last_finalized` and `new_hash` of two block, returns
/// the sequence of headers of the blocks on the path from `last_finalized` to `new_hash`
/// excluding the header corresponding to `last_finalized`, or an empty sequence if
/// `new_hash` is not a descendant of `last_finalized`.
pub(crate) fn chain_extension_step<BE, B, C>(
    last_finalized: B::Hash,
    new_hash: B::Hash,
    client: &C,
) -> VecDeque<B::Header>
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    // this early return is for optimization reasons only.
    if new_hash == last_finalized {
        return VecDeque::new();
    }

    let last_finalized_number = match client.number(last_finalized) {
        Ok(Some(number)) => number,
        _ => {
            error!(target: "afa", "No block number for {}", last_finalized);
            return VecDeque::new();
        }
    };

    let mut extension = VecDeque::new();
    // iterate ancestors of `new_hash` and push their headers to the front of `extension`
    // until reaching a block with number <= last_finalized_number.
    let mut hash = new_hash;
    loop {
        let header = match client.header(BlockId::Hash(hash)) {
            Ok(Some(header)) => header,
            _ => {
                error!(target: "afa", "no header for hash {}", hash);
                return VecDeque::new();
            }
        };

        if header.number() <= &last_finalized_number {
            if hash != last_finalized {
                // `new_hash` is not an ancestor of `last_finalized`
                return VecDeque::new();
            }
            break;
        }
        hash = *header.parent_hash();
        extension.push_front(header);
    }
    extension
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_block_builder::BlockBuilderProvider;
    use sp_consensus::BlockOrigin;
    use substrate_test_runtime::Extrinsic;
    use substrate_test_runtime_client::{
        ClientBlockImportExt, ClientExt, DefaultTestClientBuilderExt, TestClient,
        TestClientBuilder, TestClientBuilderExt,
    };

    fn create_chain(client: &mut Arc<TestClient>, n: u64) -> Vec<sp_core::H256> {
        let mut blocks = vec![client.genesis_hash()];

        for _ in 1..=n {
            let block = client
                .new_block(Default::default())
                .unwrap()
                .build()
                .unwrap()
                .block;

            blocks.push(block.header.hash());
            futures::executor::block_on(client.import(BlockOrigin::Own, block)).unwrap();
        }

        blocks
    }

    #[test]
    fn chain_extenstion_step_for_descendant() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let n = 5;
        let blocks = create_chain(&mut client, n as u64);
        for i in 0..n {
            for j in i..n {
                let extension = chain_extension_step(blocks[i], blocks[j], client.as_ref());
                assert!(extension
                    .iter()
                    .map(|header| header.hash())
                    .eq(blocks[i + 1..j + 1].iter().cloned()));
            }
        }
    }

    #[test]
    fn chain_extenstion_step_for_non_descendant() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let n = 5;
        let blocks = create_chain(&mut client, n as u64);

        for i in 0..=n {
            for j in 0..i {
                let extension = chain_extension_step(blocks[i], blocks[j], client.as_ref());
                assert!(extension.is_empty());
            }
        }

        let extra_children: Vec<_> = blocks
            .iter()
            .map(|hash| {
                let mut builder = client
                    .new_block_at(&BlockId::Hash(*hash), Default::default(), false)
                    .unwrap();
                // Add a dummy extrinsic to make the block distinct from the one on chain
                builder
                    .push(Extrinsic::AuthoritiesChange(Vec::new()))
                    .unwrap();
                let block = builder.build().unwrap().block;
                let hash = block.header.hash();
                futures::executor::block_on(client.import(BlockOrigin::Own, block)).unwrap();
                hash
            })
            .collect();

        for i in 0..=n {
            for j in 0..=n {
                if i != j {
                    let extension =
                        chain_extension_step(extra_children[i], extra_children[j], client.as_ref());
                    assert!(extension.is_empty());
                }
            }
        }
    }
}
