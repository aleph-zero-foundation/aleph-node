use std::{
    cmp::max,
    hash::{Hash, Hasher},
    iter,
};

use parity_scale_codec::{Decode, Encode};
use sp_runtime::SaturatedConversion;

use crate::{
    aleph_primitives::{BlockHash, BlockNumber},
    block::UnverifiedHeader,
    data_io::MAX_DATA_BRANCH_LEN,
    BlockId, SessionBoundaries,
};

/// Represents a proposal we obtain from another node. Note that since the proposal might come from
/// a malicious node there is no guarantee that the block hashes in the proposal correspond to real blocks
/// or encompass a branch within a session. Hence we perform initial validation of the block number and
/// the branch length before we transform it into a safer `AlephProposal` type that guarantees we
/// will not fail on  any integer over- or underflows. We expect that honest nodes create
/// UnvalidatedAlephProposal {head: hd_n, tail: [h_0, h_1, ..., h_(n-1)]} objects that represent
/// an ascending sequence of blocks b_0, b_1, ..., b_n satisfying the following conditions:
///     1) hash(b_i) = h_i for i = 0, 1, ..., n-1,
///     2) parent(b_{i+1}) = b_i for i = 0, 1, ..., (n-1),
///     3) header(b_n) = hd_n,
///     4) The parent of b_0 has been finalized (prior to creating this AlephData).
/// Such an UnvalidatedAlephProposal  object should be thought of as a proposal for block b_n to be finalized.
/// We refer for to `DataProvider` for a precise description of honest nodes' algorithm of creating proposals.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub struct UnvalidatedAlephProposal<UH: UnverifiedHeader> {
    head: UH,
    tail: Vec<BlockHash>,
}

impl<UH: UnverifiedHeader> Hash for UnvalidatedAlephProposal<UH> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.head.encode().hash(state);
        self.tail.hash(state);
    }
}

/// Represents possible invalid states as described in [UnvalidatedAlephProposal].
#[derive(Debug, PartialEq, Eq)]
pub enum ValidationError {
    BranchTooLong {
        branch_size: usize,
    },
    BlockNumberOutOfBounds {
        branch_size: usize,
        block_number: BlockNumber,
    },
    BlockOutsideSessionBoundaries {
        session_start: BlockNumber,
        session_end: BlockNumber,
        top_block: BlockNumber,
        bottom_block: BlockNumber,
    },
}

impl<UH: UnverifiedHeader> UnvalidatedAlephProposal<UH> {
    pub fn new(head: UH, tail: Vec<BlockHash>) -> Self {
        UnvalidatedAlephProposal { head, tail }
    }

    fn top_number(&self) -> BlockNumber {
        self.top_block().number()
    }

    /// Outputs the highest block in the branch.
    pub fn top_block(&self) -> BlockId {
        self.head.id()
    }

    fn branch_len(&self) -> usize {
        self.tail.len() + 1
    }

    pub fn validate_bounds(
        &self,
        session_boundaries: &SessionBoundaries,
    ) -> Result<AlephProposal<UH>, ValidationError> {
        use ValidationError::*;

        if self.branch_len() > MAX_DATA_BRANCH_LEN {
            return Err(BranchTooLong {
                branch_size: self.branch_len(),
            });
        }
        if self.top_number() < <BlockNumber>::saturated_from(self.branch_len()) {
            // Note that this also excludes branches starting at the genesis (0th) block.
            return Err(BlockNumberOutOfBounds {
                branch_size: self.branch_len(),
                block_number: self.top_number(),
            });
        }

        let bottom_block = self.top_number() - <BlockNumber>::saturated_from(self.branch_len() - 1);
        let top_block = self.top_number();
        let session_start = session_boundaries.first_block();
        let session_end = session_boundaries.last_block();
        if session_start > bottom_block || top_block > session_end {
            return Err(BlockOutsideSessionBoundaries {
                session_start,
                session_end,
                top_block,
                bottom_block,
            });
        }

        Ok(AlephProposal {
            head: self.head.clone(),
            tail: self.tail.clone(),
        })
    }
}

/// A version of UnvalidatedAlephProposal that has been initially validated and fits
/// within session bounds.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub struct AlephProposal<UH: UnverifiedHeader> {
    head: UH,
    tail: Vec<BlockHash>,
}

impl<UH: UnverifiedHeader> Hash for AlephProposal<UH> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.head.encode().hash(state);
        self.tail.hash(state);
    }
}

impl<UH: UnverifiedHeader> AlephProposal<UH> {
    /// Outputs the length the branch.
    pub fn len(&self) -> usize {
        self.tail.len() + 1
    }

    pub fn top_block_header(&self) -> UH {
        self.head.clone()
    }

    /// Outputs the highest block in the branch.
    pub fn top_block(&self) -> BlockId {
        self.top_block_header().id()
    }

    /// Outputs the lowest block in the branch.
    pub fn bottom_block(&self) -> BlockId {
        match self.tail.first() {
            Some(hash) => BlockId::new(*hash, self.number_bottom_block()),
            None => self.top_block(),
        }
    }

    /// Outputs the number one below the lowest block in the branch.
    pub fn number_below_branch(&self) -> BlockNumber {
        // Assumes that data is within bounds
        self.number_top_block() - <BlockNumber>::saturated_from(self.len())
    }

    /// Outputs the number of the lowest block in the branch.
    pub fn number_bottom_block(&self) -> BlockNumber {
        // Assumes that data is within bounds
        self.number_top_block() - <BlockNumber>::saturated_from(self.len() - 1)
    }

    /// Outputs the number of the highest block in the branch.
    pub fn number_top_block(&self) -> BlockNumber {
        self.top_block().number()
    }

    /// Outputs the block corresponding to the number in the proposed branch in case num is
    /// between the lowest and highest block number of the branch. Otherwise returns None.
    pub fn block_at_num(&self, num: BlockNumber) -> Option<BlockId> {
        if num == self.number_top_block() {
            return Some(self.top_block());
        }
        if self.number_bottom_block() <= num && num < self.number_top_block() {
            let ind: usize = (num - self.number_bottom_block()).saturated_into();
            return Some(BlockId::new(self.tail[ind], num));
        }
        None
    }

    /// Outputs an iterator over blocks starting at num. If num is too high, the iterator is
    /// empty, if it's too low the whole branch is returned.
    pub fn blocks_from_num(&self, num: BlockNumber) -> impl Iterator<Item = BlockId> + '_ {
        let num = max(num, self.number_bottom_block());
        self.tail
            .iter()
            .cloned()
            .chain(iter::once(self.head.id().hash()))
            .skip((num - self.number_bottom_block()).saturated_into())
            .zip(0u32..)
            .map(move |(hash, index)| (hash, num + index).into())
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum PendingProposalStatus {
    PendingTopBlock,
    TopBlockImportedButIncorrectBranch,
    TopBlockImportedButNotFinalizedAncestor,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ProposalStatus {
    Finalize(Vec<BlockId>),
    Ignore,
    Pending(PendingProposalStatus),
}

#[cfg(test)]
mod tests {
    use super::{UnvalidatedAlephProposal, ValidationError::*};
    use crate::{
        block::{mock::MockHeader, Header},
        data_io::MAX_DATA_BRANCH_LEN,
        BlockId, SessionBoundaryInfo, SessionId, SessionPeriod,
    };

    #[test]
    fn too_long_proposal_is_invalid() {
        let session_boundaries =
            SessionBoundaryInfo::new(SessionPeriod(20)).boundaries_for_session(SessionId(1));
        let session_start = session_boundaries.first_block();
        let tail: Vec<_> = BlockId::new_random(session_start)
            .random_branch()
            .take(MAX_DATA_BRANCH_LEN)
            .collect();
        let head = tail.last().unwrap().random_child();
        let tail = tail.into_iter().map(|header| header.id().hash()).collect();
        let proposal = UnvalidatedAlephProposal::new(head, tail);
        let branch_size = MAX_DATA_BRANCH_LEN + 1;
        assert_eq!(
            proposal.validate_bounds(&session_boundaries),
            Err(BranchTooLong { branch_size })
        );
    }

    #[test]
    fn proposal_not_within_session_is_invalid() {
        let session_boundaries =
            SessionBoundaryInfo::new(SessionPeriod(20)).boundaries_for_session(SessionId(1));
        let session_start = session_boundaries.first_block();
        let session_end = session_boundaries.last_block();
        let prev_session_block = BlockId::new_random(session_start - 1);
        let head = prev_session_block.random_child();
        let tail = vec![prev_session_block.hash()];

        let proposal = UnvalidatedAlephProposal::new(head, tail);
        assert_eq!(
            proposal.validate_bounds(&session_boundaries),
            Err(BlockOutsideSessionBoundaries {
                session_start,
                session_end,
                bottom_block: session_start - 1,
                top_block: session_start
            })
        );

        let last_session_block = BlockId::new_random(session_end);
        let head = last_session_block.random_child();
        let tail = vec![last_session_block.hash()];
        let proposal = UnvalidatedAlephProposal::new(head, tail);
        assert_eq!(
            proposal.validate_bounds(&session_boundaries),
            Err(BlockOutsideSessionBoundaries {
                session_start,
                session_end,
                bottom_block: session_end,
                top_block: session_end + 1
            })
        );
    }

    #[test]
    fn proposal_starting_at_zero_block_is_invalid() {
        let session_boundaries =
            SessionBoundaryInfo::new(SessionPeriod(20)).boundaries_for_session(SessionId(0));
        let genesis = MockHeader::genesis();
        let head = genesis.random_child();
        let tail = vec![genesis.id().hash()];
        let proposal = UnvalidatedAlephProposal::new(head, tail);
        assert_eq!(
            proposal.validate_bounds(&session_boundaries),
            Err(BlockNumberOutOfBounds {
                branch_size: 2,
                block_number: 1
            })
        );
    }

    #[test]
    fn valid_proposal_is_validated_positively() {
        let session_boundaries =
            SessionBoundaryInfo::new(SessionPeriod(20)).boundaries_for_session(SessionId(0));

        let genesis = MockHeader::genesis();
        let branch: Vec<_> = genesis
            .random_branch()
            .take(MAX_DATA_BRANCH_LEN - 1)
            .collect();
        let head = branch.last().unwrap().random_child();
        let tail = branch.iter().map(|header| header.id().hash()).collect();
        let proposal = UnvalidatedAlephProposal::new(head, tail);
        assert!(proposal.validate_bounds(&session_boundaries).is_ok());

        let head = branch.last().unwrap().random_child();
        let proposal = UnvalidatedAlephProposal::new(head, Vec::new());
        assert!(proposal.validate_bounds(&session_boundaries).is_ok());
    }
}
