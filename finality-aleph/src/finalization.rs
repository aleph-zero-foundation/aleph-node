use crate::{justification::AlephJustification, AuthorityKeystore};
use aleph_primitives::ALEPH_ENGINE_ID;
use codec::Encode;
use log::{debug, error};
use sc_client_api::Backend;
use sp_api::{BlockId, NumberFor};
use sp_runtime::{
    traits::{Block, Header},
    Justification,
};
use std::sync::Arc;

pub(crate) fn finalize_block_as_authority<BE, B, C>(
    client: Arc<C>,
    h: B::Hash,
    auth_keystore: &AuthorityKeystore,
) where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let block_number = match client.number(h) {
        Ok(Some(number)) => number,
        _ => {
            error!(target: "afa", "a block with hash {} should already be in chain", h);
            return;
        }
    };
    finalize_block(
        client,
        h,
        block_number,
        Some((
            ALEPH_ENGINE_ID,
            AlephJustification::new::<B>(&auth_keystore, h).encode(),
        )),
    );
}

pub(crate) fn finalize_block<BE, B, C>(
    client: Arc<C>,
    hash: B::Hash,
    block_number: NumberFor<B>,
    justification: Option<Justification>,
) where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let status = client.info();
    if status.finalized_number >= block_number {
        error!(target: "afa", "trying to finalized a block with hash {} and number {}
               that is not greater than already finalized {}", hash, block_number, status.finalized_number);
        return;
    }

    debug!(target: "afa", "Finalizing block with hash {:?}. Previous best: #{:?}.", hash, status.finalized_number);

    let _update_res = client.lock_import_and_run(|import_op| {
        // NOTE: all other finalization logic should come here, inside the lock
        client.apply_finality(import_op, BlockId::Hash(hash), justification, true)
    });

    let status = client.info();
    debug!(target: "afa", "Finalized block with hash {:?}. Current best: #{:?}.", hash, status.finalized_number);
}

// Returns true if and only if h is descended from the last finalized block.
// The last finalized block therefore does not extends finalized.
pub(crate) fn check_extends_last_finalized<BE, B, C>(client: Arc<C>, h: B::Hash) -> bool
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    let head_finalized = client.info().finalized_hash;
    if h == head_finalized {
        return false;
    }
    let lca = sp_blockchain::lowest_common_ancestor(client.as_ref(), h, head_finalized)
        .expect("No lowest common ancestor");
    lca.hash == head_finalized
}

pub(crate) fn reduce_block_up_to<BE, B, C>(
    client: Arc<C>,
    mut h: B::Hash,
    max_h: NumberFor<B>,
) -> Option<B::Hash>
where
    B: Block,
    BE: Backend<B>,
    C: crate::ClientForAleph<B, BE>,
{
    while let Ok(Some(number)) = client.number(h) {
        if number <= max_h {
            return Some(h);
        }

        if let Ok(Some(header)) = client.header(BlockId::Hash(h)) {
            h = *header.parent_hash();
        } else {
            return None;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_block_builder::BlockBuilderProvider;
    use sp_blockchain::HeaderBackend;
    use sp_consensus::BlockOrigin;
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
    fn reduce_return_without_reduction() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let blocks = create_chain(&mut client, 100);

        let reduce_up_to = 10u64;

        // blocks nr are equal to index + 1;
        for nr in 0..=reduce_up_to {
            assert_eq!(
                Some(blocks[nr as usize]),
                reduce_block_up_to(client.clone(), blocks[nr as usize], reduce_up_to + 1),
                "Expected block #{} to not be reduced",
                nr + 1
            );
        }
    }

    #[test]
    fn reduce_to_max_h() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let blocks = create_chain(&mut client, 100);

        let reduce_up_to = 10u64;

        for nr in reduce_up_to..100 {
            assert_eq!(
                Some(blocks[reduce_up_to as usize]),
                reduce_block_up_to(client.clone(), blocks[nr as usize], reduce_up_to),
                "Expected block #{} to be reduced up to #{}",
                nr,
                reduce_up_to
            );
        }
    }

    #[test]
    fn reduce_to_0() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let blocks = create_chain(&mut client, 100);

        let reduce_up_to = 0;

        for nr in reduce_up_to..100 {
            assert_eq!(
                Some(blocks[reduce_up_to as usize]),
                reduce_block_up_to(client.clone(), blocks[nr as usize], reduce_up_to),
                "Expected block #{} to be reduced up to #{}",
                nr,
                reduce_up_to
            );
        }
    }

    #[test]
    fn not_finalizing_non_existing_block() {
        let client = Arc::new(TestClientBuilder::new().build());
        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        // Not existing because we did not imported it yet
        finalize_block(client.clone(), block.header.hash(), 1, None);
        assert_eq!(client.info().finalized_number, 0)
    }

    #[test]
    fn finalizing_existing_block() {
        let mut client = Arc::new(TestClientBuilder::new().build());
        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        futures::executor::block_on(client.import(BlockOrigin::Own, block.clone())).unwrap();

        finalize_block(client.clone(), block.header.hash(), 1, None);
        assert_eq!(client.info().finalized_number, 1)
    }

    #[test]
    fn not_finalizing_existing_nonsense_block() {
        let mut client = Arc::new(TestClientBuilder::new().build());
        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        futures::executor::block_on(client.import(BlockOrigin::Own, block)).unwrap();

        let block = client
            .new_block(Default::default())
            .unwrap()
            .build()
            .unwrap()
            .block;

        finalize_block(client.clone(), block.header.hash(), 1, None);
        assert_eq!(client.info().finalized_number, 0)
    }

    #[test]
    fn simple_extends_finalized_true_cases() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let blocks = create_chain(&mut client, 100);

        finalize_block(client.clone(), blocks[10], 10, None);

        for hash in blocks.iter().skip(11) {
            assert!(check_extends_last_finalized(client.clone(), *hash))
        }
    }

    #[test]
    fn last_finalized_not_extending_finalized() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let blocks = create_chain(&mut client, 100);

        finalize_block(client.clone(), blocks[10], 10, None);
        assert!(!check_extends_last_finalized(client, blocks[10]))
    }

    #[test]
    fn simple_extends_finalized_false_cases() {
        let mut client = Arc::new(TestClientBuilder::new().build());

        let blocks = create_chain(&mut client, 100);

        finalize_block(client.clone(), blocks[10], 10, None);

        for nr in 0..10 {
            let block = client
                .new_block_at(&BlockId::Number(nr), Default::default(), false)
                .unwrap()
                .build()
                .unwrap()
                .block;
            assert!(!check_extends_last_finalized(
                client.clone(),
                blocks[nr as usize]
            ));
            assert!(!check_extends_last_finalized(
                client.clone(),
                block.header.hash()
            ));
        }
    }
}
