use clap::Subcommand;
use relations::{
    CircuitField, ConstraintSynthesizer, ConstraintSystemRef, DepositRelation, FrontendAccount,
    FrontendLeafIndex, FrontendMerklePath, FrontendMerkleRoot, FrontendNote, FrontendNullifier,
    FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor, GetPublicInput, LinearEquationRelation,
    MerkleTreeRelation, Result as R1CsResult, WithdrawRelation, XorRelation,
};

use crate::snark_relations::parsing::{
    parse_frontend_account, parse_frontend_merkle_path_single, parse_frontend_merkle_root,
    parse_frontend_note,
};

/// All available relations from `relations` crate.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Subcommand)]
pub enum RelationArgs {
    Xor {
        #[clap(long, short = 'a', default_value = "2")]
        public_xoree: u8,
        #[clap(long, short = 'b', default_value = "3")]
        private_xoree: u8,
        #[clap(long, short = 'c', default_value = "1")]
        result: u8,
    },
    LinearEquation {
        /// constant (a slope)
        #[clap(long, default_value = "2")]
        a: u32,
        /// private witness
        #[clap(long, default_value = "7")]
        x: u32,
        /// constant(an intercept)
        #[clap(long, default_value = "5")]
        b: u32,
        /// constant
        #[clap(long, default_value = "19")]
        y: u32,
    },
    MerkleTree {
        /// Seed bytes for rng, the more the merrier
        #[clap(long)]
        seed: Option<String>,
        /// Tree leaves, used to calculate the tree root
        #[clap(long, value_delimiter = ',')]
        leaves: Vec<u8>,
        /// Leaf of which membership is to be proven, must be one of the leaves
        #[clap(long)]
        leaf: u8,
    },
    Deposit {
        #[clap(long, value_parser = parse_frontend_note)]
        note: FrontendNote,
        #[clap(long)]
        token_id: FrontendTokenId,
        #[clap(long)]
        token_amount: FrontendTokenAmount,

        #[clap(long)]
        trapdoor: FrontendTrapdoor,
        #[clap(long)]
        nullifier: FrontendNullifier,
    },
    Withdraw {
        #[clap(long)]
        old_nullifier: FrontendNullifier,
        #[clap(long, value_parser = parse_frontend_merkle_root)]
        merkle_root: FrontendMerkleRoot,
        #[clap(long, value_parser = parse_frontend_note)]
        new_note: FrontendNote,
        #[clap(long)]
        token_id: FrontendTokenId,
        #[clap(long)]
        token_amount_out: FrontendTokenAmount,
        #[clap(long)]
        fee: FrontendTokenAmount,
        #[clap(long, value_parser = parse_frontend_account)]
        recipient: FrontendAccount,

        #[clap(long)]
        old_trapdoor: FrontendTrapdoor,
        #[clap(long)]
        new_trapdoor: FrontendTrapdoor,
        #[clap(long)]
        new_nullifier: FrontendNullifier,
        #[clap(long, value_delimiter = ',', value_parser = parse_frontend_merkle_path_single)]
        merkle_path: FrontendMerklePath,
        #[clap(long)]
        leaf_index: FrontendLeafIndex,
        #[clap(long, value_parser = parse_frontend_note)]
        old_note: FrontendNote,
        #[clap(long)]
        whole_token_amount: FrontendTokenAmount,
        #[clap(long)]
        new_token_amount: FrontendTokenAmount,
    },
}

impl RelationArgs {
    /// Relation identifier.
    #[allow(dead_code)]
    pub fn id(&self) -> String {
        match &self {
            RelationArgs::Xor { .. } => String::from("xor"),
            RelationArgs::LinearEquation { .. } => String::from("linear_equation"),
            RelationArgs::MerkleTree { .. } => String::from("merkle_tree"),
            RelationArgs::Deposit { .. } => String::from("deposit"),
            RelationArgs::Withdraw { .. } => String::from("withdraw"),
        }
    }
}

impl ConstraintSynthesizer<CircuitField> for RelationArgs {
    fn generate_constraints(self, cs: ConstraintSystemRef<CircuitField>) -> R1CsResult<()> {
        match self {
            RelationArgs::Xor {
                public_xoree,
                private_xoree,
                result,
            } => XorRelation::new(public_xoree, private_xoree, result).generate_constraints(cs),
            RelationArgs::LinearEquation { a, x, b, y } => {
                LinearEquationRelation::new(a, x, b, y).generate_constraints(cs)
            }
            RelationArgs::MerkleTree { seed, leaf, leaves } => {
                MerkleTreeRelation::new(leaves, leaf, seed).generate_constraints(cs)
            }
            RelationArgs::Deposit {
                note,
                token_id,
                token_amount,
                trapdoor,
                nullifier,
            } => DepositRelation::new(note, token_id, token_amount, trapdoor, nullifier)
                .generate_constraints(cs),
            RelationArgs::Withdraw {
                old_nullifier,
                merkle_root,
                new_note,
                token_id,
                token_amount_out,
                fee,
                recipient,
                old_trapdoor,
                new_trapdoor,
                new_nullifier,
                merkle_path,
                leaf_index,
                old_note,
                whole_token_amount,
                new_token_amount,
            } => WithdrawRelation::new(
                old_nullifier,
                merkle_root,
                new_note,
                token_id,
                token_amount_out,
                old_trapdoor,
                new_trapdoor,
                new_nullifier,
                merkle_path,
                leaf_index,
                old_note,
                whole_token_amount,
                new_token_amount,
                fee,
                recipient,
            )
            .generate_constraints(cs),
        }
    }
}

impl GetPublicInput<CircuitField> for RelationArgs {
    fn public_input(&self) -> Vec<CircuitField> {
        // todo: deduplicate casting
        match self {
            RelationArgs::Xor {
                public_xoree,
                private_xoree,
                result,
            } => XorRelation::new(*public_xoree, *private_xoree, *result).public_input(),
            RelationArgs::LinearEquation { a, x, b, y } => {
                LinearEquationRelation::new(*a, *x, *b, *y).public_input()
            }
            RelationArgs::MerkleTree { seed, leaf, leaves } => {
                MerkleTreeRelation::new(leaves.clone(), *leaf, seed.clone()).public_input()
            }
            RelationArgs::Deposit {
                note,
                token_id,
                token_amount,
                trapdoor,
                nullifier,
            } => DepositRelation::new(*note, *token_id, *token_amount, *trapdoor, *nullifier)
                .public_input(),
            RelationArgs::Withdraw {
                old_nullifier,
                merkle_root,
                new_note,
                token_id,
                token_amount_out,
                fee,
                recipient,
                old_trapdoor,
                new_trapdoor,
                new_nullifier,
                merkle_path,
                leaf_index,
                old_note,
                whole_token_amount,
                new_token_amount,
            } => WithdrawRelation::new(
                *old_nullifier,
                *merkle_root,
                *new_note,
                *token_id,
                *token_amount_out,
                *old_trapdoor,
                *new_trapdoor,
                *new_nullifier,
                merkle_path.clone(),
                *leaf_index,
                *old_note,
                *whole_token_amount,
                *new_token_amount,
                *fee,
                *recipient,
            )
            .public_input(),
        }
    }
}
