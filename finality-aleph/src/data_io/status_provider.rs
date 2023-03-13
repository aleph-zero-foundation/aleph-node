use log::debug;
use sp_runtime::{
    traits::{Block as BlockT, NumberFor, One},
    SaturatedConversion,
};

use crate::data_io::{
    chain_info::ChainInfoProvider,
    proposal::{AlephProposal, ProposalStatus},
};

pub fn get_proposal_status<B, CIP>(
    chain_info_provider: &mut CIP,
    proposal: &AlephProposal<B>,
    old_status: Option<&ProposalStatus<B>>,
) -> ProposalStatus<B>
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    use crate::data_io::proposal::{PendingProposalStatus::*, ProposalStatus::*};

    let current_highest_finalized = chain_info_provider.get_highest_finalized().num;

    if current_highest_finalized >= proposal.number_top_block() {
        return Ignore;
    }

    if is_hopeless_fork(chain_info_provider, proposal) {
        debug!(target: "aleph-finality", "Encountered a hopeless fork proposal {:?}.", proposal);
        return Ignore;
    }

    let old_status = match old_status {
        Some(status) => status,
        None => &Pending(PendingTopBlock),
    };
    match old_status {
        Pending(PendingTopBlock) => {
            let top_block = proposal.top_block();
            if chain_info_provider.is_block_imported(&top_block) {
                // Note that the above also makes sure that the `number` claimed in the proposal is correct.
                // That's why checking the branch correctness now boils down to checking the parent-child
                // relation on the branch.
                if is_branch_ancestry_correct(chain_info_provider, proposal) {
                    if is_ancestor_finalized(chain_info_provider, proposal) {
                        Finalize(
                            proposal
                                .blocks_from_num(current_highest_finalized + NumberFor::<B>::one())
                                .collect(),
                        )
                    } else {
                        // This could also be a hopeless fork, but we have checked before that it isn't (yet).
                        Pending(TopBlockImportedButNotFinalizedAncestor)
                    }
                } else {
                    // This could also be a hopeless fork, but we have checked before that it isn't (yet).
                    Pending(TopBlockImportedButIncorrectBranch)
                }
            } else {
                // This could also be a hopeless fork, but we have checked before that it isn't (yet).
                Pending(PendingTopBlock)
            }
        }
        Pending(TopBlockImportedButNotFinalizedAncestor) => {
            if is_ancestor_finalized(chain_info_provider, proposal) {
                Finalize(
                    proposal
                        .blocks_from_num(current_highest_finalized + NumberFor::<B>::one())
                        .collect(),
                )
            } else {
                // This could also be a hopeless fork, but we have checked before that it isn't (yet).
                Pending(TopBlockImportedButNotFinalizedAncestor)
            }
        }
        Pending(TopBlockImportedButIncorrectBranch) => {
            // This could also be a hopeless fork, but we have checked before that it isn't (yet).
            Pending(TopBlockImportedButIncorrectBranch)
        }
        _ => old_status.clone(),
    }
}

fn is_hopeless_fork<B, CIP>(chain_info_provider: &mut CIP, proposal: &AlephProposal<B>) -> bool
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    let bottom_num = proposal.number_bottom_block();
    for i in 0..proposal.len() {
        if let Ok(finalized_block) =
            chain_info_provider.get_finalized_at(bottom_num + <NumberFor<B>>::saturated_from(i))
        {
            if finalized_block.hash != proposal[i] {
                return true;
            }
        } else {
            // We don't know the finalized block at this height
            break;
        }
    }
    false
}

fn is_ancestor_finalized<B, CIP>(chain_info_provider: &mut CIP, proposal: &AlephProposal<B>) -> bool
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    let bottom = proposal.bottom_block();
    let parent_hash = if let Ok(hash) = chain_info_provider.get_parent_hash(&bottom) {
        hash
    } else {
        return false;
    };
    let finalized =
        if let Ok(hash) = chain_info_provider.get_finalized_at(proposal.number_below_branch()) {
            hash
        } else {
            return false;
        };
    parent_hash == finalized.hash
}

// Checks that the subsequent blocks in the branch are in the parent-child relation, as required.
fn is_branch_ancestry_correct<B, CIP>(
    chain_info_provider: &mut CIP,
    proposal: &AlephProposal<B>,
) -> bool
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    let bottom_num = proposal.number_bottom_block();
    for i in 1..proposal.len() {
        let curr_num = bottom_num + <NumberFor<B>>::saturated_from(i);
        let curr_block = proposal.block_at_num(curr_num).expect("is within bounds");
        match chain_info_provider.get_parent_hash(&curr_block) {
            Ok(parent_hash) => {
                if parent_hash != proposal[i - 1] {
                    return false;
                }
            }
            Err(()) => {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sp_runtime::traits::Block as BlockT;

    use crate::{
        data_io::{
            chain_info::{AuxFinalizationChainInfoProvider, CachedChainInfoProvider},
            proposal::{
                AlephProposal,
                PendingProposalStatus::*,
                ProposalStatus::{self, *},
            },
            status_provider::get_proposal_status,
            ChainInfoCacheConfig, MAX_DATA_BRANCH_LEN,
        },
        testing::{
            client_chain_builder::ClientChainBuilder,
            mocks::{
                unvalidated_proposal_from_headers, TBlock, THeader, TestClient, TestClientBuilder,
                TestClientBuilderExt,
            },
        },
        SessionBoundaries, SessionId, SessionPeriod,
    };

    // A large number only for the purpose of creating `AlephProposal`s
    const DUMMY_SESSION_LEN: u32 = 1_000_000;

    fn proposal_from_headers(headers: Vec<THeader>) -> AlephProposal<TBlock> {
        let unvalidated = unvalidated_proposal_from_headers(headers);
        let session_boundaries =
            SessionBoundaries::new(SessionId(0), SessionPeriod(DUMMY_SESSION_LEN));
        unvalidated.validate_bounds(&session_boundaries).unwrap()
    }

    fn proposal_from_blocks(blocks: Vec<TBlock>) -> AlephProposal<TBlock> {
        let headers = blocks.into_iter().map(|b| b.header().clone()).collect();
        proposal_from_headers(headers)
    }

    type TestCachedChainInfo = CachedChainInfoProvider<TBlock, Arc<TestClient>>;
    type TestAuxChainInfo = AuxFinalizationChainInfoProvider<TBlock, Arc<TestClient>>;

    fn prepare_proposal_test() -> (ClientChainBuilder, TestCachedChainInfo, TestAuxChainInfo) {
        let client = Arc::new(TestClientBuilder::new().build());

        let config = ChainInfoCacheConfig {
            block_cache_capacity: 2,
        };
        let cached_chain_info_provider = CachedChainInfoProvider::new(client.clone(), config);

        let chain_builder =
            ClientChainBuilder::new(client.clone(), Arc::new(TestClientBuilder::new().build()));

        let aux_chain_info_provider =
            AuxFinalizationChainInfoProvider::new(client, chain_builder.genesis_hash_num());

        (
            chain_builder,
            cached_chain_info_provider,
            aux_chain_info_provider,
        )
    }

    fn verify_proposal_status(
        cached_cip: &mut TestCachedChainInfo,
        aux_cip: &mut TestAuxChainInfo,
        proposal: &AlephProposal<TBlock>,
        correct_status: ProposalStatus<TBlock>,
    ) {
        let status_a = get_proposal_status(aux_cip, proposal, None);
        assert_eq!(
            status_a, correct_status,
            "Aux chain info gives wrong status for proposal {:?}",
            proposal
        );
        let status_c = get_proposal_status(cached_cip, proposal, None);
        assert_eq!(
            status_c, correct_status,
            "Cached chain info gives wrong status for proposal {:?}",
            proposal
        );
    }

    fn verify_proposal_of_all_lens_finalizable(
        blocks: Vec<TBlock>,
        cached_cip: &mut TestCachedChainInfo,
        aux_cip: &mut TestAuxChainInfo,
    ) {
        for len in 1..=MAX_DATA_BRANCH_LEN {
            let blocks_branch = blocks[0..len].to_vec();
            let proposal = proposal_from_blocks(blocks_branch);
            verify_proposal_status(
                cached_cip,
                aux_cip,
                &proposal,
                ProposalStatus::Finalize(proposal.blocks_from_num(0).collect()),
            );
        }
    }

    #[tokio::test]
    async fn correct_proposals_are_finalizable_even_with_forks() {
        let (mut chain_builder, mut cached_cip, mut aux_cip) = prepare_proposal_test();
        let blocks = chain_builder
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        verify_proposal_of_all_lens_finalizable(blocks.clone(), &mut cached_cip, &mut aux_cip);

        let _fork = chain_builder
            .build_and_import_branch_above(&blocks[2].header.hash(), MAX_DATA_BRANCH_LEN * 10)
            .await;

        verify_proposal_of_all_lens_finalizable(blocks.clone(), &mut cached_cip, &mut aux_cip);
    }

    #[tokio::test]
    async fn not_finalized_ancestors_handled_correctly() {
        let (mut chain_builder, mut cached_cip, mut aux_cip) = prepare_proposal_test();
        let blocks = chain_builder
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        let fork = chain_builder
            .build_and_import_branch_above(&blocks[2].header.hash(), MAX_DATA_BRANCH_LEN * 10)
            .await;

        for len in 1..=MAX_DATA_BRANCH_LEN {
            let blocks_branch = blocks[1..(len + 1)].to_vec();
            let proposal = proposal_from_blocks(blocks_branch);
            verify_proposal_status(
                &mut cached_cip,
                &mut aux_cip,
                &proposal,
                Pending(TopBlockImportedButNotFinalizedAncestor),
            );
            let blocks_branch = fork[1..(len + 1)].to_vec();
            let proposal = proposal_from_blocks(blocks_branch);
            verify_proposal_status(
                &mut cached_cip,
                &mut aux_cip,
                &proposal,
                Pending(TopBlockImportedButNotFinalizedAncestor),
            );
        }
    }

    #[tokio::test]
    async fn incorrect_branch_handled_correctly() {
        let (mut chain_builder, mut cached_cip, mut aux_cip) = prepare_proposal_test();
        let blocks = chain_builder
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        let incorrect_branch = vec![
            blocks[0].clone(),
            blocks[1].clone(),
            blocks[3].clone(),
            blocks[5].clone(),
        ];
        let proposal = proposal_from_blocks(incorrect_branch);
        verify_proposal_status(
            &mut cached_cip,
            &mut aux_cip,
            &proposal,
            Pending(TopBlockImportedButIncorrectBranch),
        );

        chain_builder.finalize_block(&blocks[1].header.hash());
        verify_proposal_status(
            &mut cached_cip,
            &mut aux_cip,
            &proposal,
            Pending(TopBlockImportedButIncorrectBranch),
        );

        chain_builder.finalize_block(&blocks[10].header.hash());
        verify_proposal_status(&mut cached_cip, &mut aux_cip, &proposal, Ignore);
    }

    #[tokio::test]
    async fn pending_top_block_handled_correctly() {
        let (mut chain_builder, mut cached_cip, mut aux_cip) = prepare_proposal_test();
        let blocks = chain_builder
            .initialize_single_branch(MAX_DATA_BRANCH_LEN * 10)
            .await;

        for len in 1..=MAX_DATA_BRANCH_LEN {
            let blocks_branch = blocks[0..len].to_vec();
            let proposal = proposal_from_blocks(blocks_branch);
            verify_proposal_status(
                &mut cached_cip,
                &mut aux_cip,
                &proposal,
                Pending(PendingTopBlock),
            );
        }
        chain_builder.import_branch(blocks.clone()).await;

        verify_proposal_of_all_lens_finalizable(blocks, &mut cached_cip, &mut aux_cip);
    }

    #[tokio::test]
    async fn hopeless_forks_handled_correctly() {
        let (mut chain_builder, mut cached_cip, mut aux_cip) = prepare_proposal_test();
        let blocks = chain_builder
            .initialize_single_branch_and_import(MAX_DATA_BRANCH_LEN * 10)
            .await;

        let fork = chain_builder
            .build_branch_above(&blocks[2].header.hash(), MAX_DATA_BRANCH_LEN * 10)
            .await;

        for len in 1..=MAX_DATA_BRANCH_LEN {
            let fork_branch = fork[0..len].to_vec();
            let proposal = proposal_from_blocks(fork_branch);
            verify_proposal_status(
                &mut cached_cip,
                &mut aux_cip,
                &proposal,
                Pending(PendingTopBlock),
            );
        }

        chain_builder.finalize_block(&blocks[2].header.hash());

        for len in 1..=MAX_DATA_BRANCH_LEN {
            let fork_branch = fork[0..len].to_vec();
            let proposal = proposal_from_blocks(fork_branch);
            verify_proposal_status(
                &mut cached_cip,
                &mut aux_cip,
                &proposal,
                Pending(PendingTopBlock),
            );
        }

        chain_builder.finalize_block(&blocks[3].header.hash());

        for len in 1..=MAX_DATA_BRANCH_LEN {
            let fork_branch = fork[0..len].to_vec();
            let proposal = proposal_from_blocks(fork_branch);
            verify_proposal_status(&mut cached_cip, &mut aux_cip, &proposal, Ignore);
        }
        // Proposal below finalized should be ignored
        verify_proposal_status(
            &mut cached_cip,
            &mut aux_cip,
            &proposal_from_blocks(blocks[0..4].to_vec()),
            Ignore,
        );

        // New proposals above finalized should be finalizable.
        let fresh_proposal = proposal_from_blocks(blocks[4..6].to_vec());
        verify_proposal_status(
            &mut cached_cip,
            &mut aux_cip,
            &fresh_proposal,
            Finalize(fresh_proposal.blocks_from_num(0).collect()),
        );

        // Long proposals should finalize the appropriate suffix.
        let long_proposal = proposal_from_blocks(blocks[0..6].to_vec());
        verify_proposal_status(
            &mut cached_cip,
            &mut aux_cip,
            &long_proposal,
            // We are using fresh_proposal here on purpose, to only check the expected blocks.
            Finalize(fresh_proposal.blocks_from_num(0).collect()),
        );
    }
}
