use crate::data_io::AlephDataFor;
use core::result::Result;
use log::{debug, error, warn};
use sc_client_api::Backend;
use sp_api::{BlockId, NumberFor};
use sp_runtime::{
    traits::{Block, Header},
    Justification,
};
use std::sync::Arc;

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

/// Given hash `last_finalized` and `AlephDataFor` `new_data` of two blocks, returns
/// Some(new_data) if the block hash represented by new_data is a descendant of last_finalized
/// (and the new_data.number is correct). Otherwise it outputs None.
pub(crate) fn should_finalize<BE, B, C>(
    last_finalized: B::Hash,
    new_data: AlephDataFor<B>,
    client: &C,
    last_block_in_session: NumberFor<B>,
) -> Option<AlephDataFor<B>>
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    // this early return is for optimization reasons only.
    if new_data.hash == last_finalized {
        return None;
    }

    if new_data.number > last_block_in_session {
        return None;
    }

    let last_finalized_number = match client.number(last_finalized) {
        Ok(Some(number)) => number,
        _ => {
            error!(target: "afa", "No block number for {}", last_finalized);
            return None;
        }
    };

    if let Ok(Some(header)) = client.header(BlockId::Hash(new_data.hash)) {
        if *header.number() != new_data.number {
            warn!(target: "afa", "Incorrect number for hash {}. Got {}, should be {}", new_data.hash, new_data.number, header.number());
            return None;
        }
    } else {
        warn!(target: "afa", "No header for hash {}", new_data.hash);
        return None;
    }

    // Iterate ancestors of `new_hash` until reaching a block with number <= last_finalized_number
    // in order to check if new_data.hash is an ancestor of last_finalized
    let mut hash = new_data.hash;
    loop {
        let header = match client.header(BlockId::Hash(hash)) {
            Ok(Some(header)) => header,
            _ => {
                error!(target: "afa", "No header for hash {}", hash);
                return None;
            }
        };

        if header.number() <= &last_finalized_number {
            if hash != last_finalized {
                // `new_hash` is not an ancestor of `last_finalized`
                return None;
            }
            break;
        }
        hash = *header.parent_hash();
    }
    Some(new_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_io::AlephData;
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
    fn should_finalize_for_descendant() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let n = 5;
        let blocks = create_chain(&mut client, n as u64);
        for i in 0..n {
            for j in i..n {
                let maybe_data = should_finalize(
                    blocks[i],
                    AlephData::new(blocks[j], j as u64),
                    client.as_ref(),
                    100u64,
                );
                let correct_result = if i == j {
                    None
                } else {
                    Some(AlephData::new(blocks[j], j as u64))
                };
                assert!(maybe_data == correct_result);
            }
        }
    }

    #[test]
    fn should_finalize_for_non_descendant() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let n = 5;
        let blocks = create_chain(&mut client, n as u64);

        for i in 0..=n {
            for j in 0..i {
                let maybe_data = should_finalize(
                    blocks[i],
                    AlephData::new(blocks[j], j as u64),
                    client.as_ref(),
                    100u64,
                );
                assert!(maybe_data.is_none());
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
                    let maybe_data = should_finalize(
                        extra_children[i],
                        AlephData::new(extra_children[j], j as u64),
                        client.as_ref(),
                        100u64,
                    );
                    assert!(maybe_data.is_none());
                }
            }
        }
    }

    #[test]
    fn should_finalize_for_incorrect_aleph_data() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let n = 5;
        let blocks = create_chain(&mut client, n as u64);

        for i in 0..n {
            for j in i..n {
                let maybe_data = should_finalize(
                    blocks[i],
                    AlephData::new(blocks[j], (j + 1) as u64),
                    client.as_ref(),
                    100u64,
                );
                assert!(maybe_data.is_none());
            }
        }
    }
}
