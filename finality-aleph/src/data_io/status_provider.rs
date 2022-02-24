use crate::data_io::{
    chain_info::ChainInfoProvider,
    proposal::{AlephProposal, ProposalStatus},
};
use sp_runtime::traits::{Block as BlockT, NumberFor};
use sp_runtime::SaturatedConversion;

pub fn get_proposal_status<B, CIP>(
    chain_info_provider: &mut CIP,
    proposal: &AlephProposal<B>,
    old_status: Option<&ProposalStatus<B>>,
) -> ProposalStatus<B>
where
    B: BlockT,
    CIP: ChainInfoProvider<B>,
{
    use crate::data_io::proposal::PendingProposalStatus::*;
    use crate::data_io::proposal::ProposalStatus::*;

    if is_hopeless_fork(chain_info_provider, proposal) {
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
                        Finalize(proposal.top_block())
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
                Finalize(proposal.top_block())
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
