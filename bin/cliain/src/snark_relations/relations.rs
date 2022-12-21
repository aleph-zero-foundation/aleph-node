use clap::Subcommand;
use relations::{
    CircuitField, ConstraintSynthesizer, ConstraintSystemRef, DepositRelation, FrontendAccount,
    FrontendLeafIndex, FrontendMerklePath, FrontendMerkleRoot, FrontendNote, FrontendNullifier,
    FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor, GetPublicInput, LinearEquationRelation,
    MerkleTreeRelation, Result as R1CsResult, Root, WithdrawRelation, XorRelation,
};

use crate::snark_relations::parsing::{
    parse_circuit_field, parse_frontend_account, parse_frontend_merkle_path, parse_frontend_note,
};

/// All available relations from `relations` crate.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Eq, PartialEq, Hash, Debug, Subcommand)]
pub enum RelationArgs {
    Xor {
        /// The first xoree (public input).
        #[clap(long, short = 'a', default_value = "2")]
        public_xoree: u8,
        /// The second xoree (private input).
        #[clap(long, short = 'b', default_value = "3")]
        private_xoree: u8,
        /// The xor result (circuit constant).
        #[clap(long, short = 'c', default_value = "1")]
        result: u8,
    },

    LinearEquation {
        /// The equation slope (circuit constant).
        #[clap(long, default_value = "2")]
        a: u32,
        /// The equation variable (private input).
        #[clap(long, default_value = "7")]
        x: u32,
        /// The equation intercept (circuit constant).
        #[clap(long, default_value = "5")]
        b: u32,
        /// The equation right-hand side (circuit constant).
        #[clap(long, default_value = "19")]
        y: u32,
    },

    MerkleTree {
        /// Seed bytes for rng, the more the merrier.
        #[clap(long)]
        seed: Option<String>,
        /// Tree leaves (private input).
        #[clap(long, value_delimiter = ',')]
        leaves: Option<Vec<u8>>,
        /// Tree root (public input).
        ///
        /// If you know the leaves, then don't provide root - it can be computed from the leaves.
        #[clap(long, conflicts_with = "leaves", value_parser = parse_circuit_field)]
        root: Option<Root>,
        /// The leaf of which membership is to be proven (public input).
        #[clap(long)]
        leaf: Option<u8>,
    },

    Deposit {
        /// The note encoding token id, token amount, trapdoor and nullifier (public input).
        #[clap(long, value_parser = parse_frontend_note)]
        note: Option<FrontendNote>,
        /// The identifier of the token being shielded (public input).
        #[clap(long)]
        token_id: Option<FrontendTokenId>,
        /// The amount being shielded (public input).
        #[clap(long)]
        token_amount: Option<FrontendTokenAmount>,

        /// The trapdoor, that keeps the note private even after revealing the nullifier (private
        /// input).
        #[clap(long)]
        trapdoor: Option<FrontendTrapdoor>,
        /// The nullifier for invalidating the note in the future (private input).
        #[clap(long)]
        nullifier: Option<FrontendNullifier>,
    },

    Withdraw {
        /// The upper bound for Merkle tree height (circuit constant).
        #[clap(long, default_value = "10")]
        max_path_len: u8,

        /// The nullifier that was used for the old note (public input).
        #[clap(long)]
        old_nullifier: Option<FrontendNullifier>,
        /// The Merkle root of the tree containing the old note (public input).
        #[clap(long, value_parser = parse_frontend_note)]
        merkle_root: Option<FrontendMerkleRoot>,
        /// The new note (public input).
        #[clap(long, value_parser = parse_frontend_note)]
        new_note: Option<FrontendNote>,
        /// The identifier of the token being unshielded (public input).
        #[clap(long)]
        token_id: Option<FrontendTokenId>,
        /// The amount being unshielded (public input).
        #[clap(long)]
        token_amount_out: Option<FrontendTokenAmount>,
        /// The fee for the caller (public input).
        #[clap(long)]
        fee: Option<FrontendTokenAmount>,
        /// The recipient of unshielded tokens, excluding fee (public input).
        #[clap(long, value_parser = parse_frontend_account)]
        recipient: Option<FrontendAccount>,

        /// The trapdoor that was used for the old note (private input).
        #[clap(long)]
        old_trapdoor: Option<FrontendTrapdoor>,
        /// The trapdoor that was used for the new note (private input).
        #[clap(long)]
        new_trapdoor: Option<FrontendTrapdoor>,
        /// The nullifier that was used for the new note (private input).
        #[clap(long)]
        new_nullifier: Option<FrontendNullifier>,
        /// The Merkle path proving that the old note is under `merkle_root` (private input).
        #[clap(long, value_parser = parse_frontend_merkle_path)]
        merkle_path: Option<FrontendMerklePath>,
        /// The index of the old note in the Merkle tree (private input).
        #[clap(long)]
        leaf_index: Option<FrontendLeafIndex>,
        /// The old note (private input).
        #[clap(long, value_parser = parse_frontend_note)]
        old_note: Option<FrontendNote>,
        /// The token amount that was originally shielded (private input).
        #[clap(long)]
        whole_token_amount: Option<FrontendTokenAmount>,
        /// The token amount that will still be shielded in the new note (private input).
        #[clap(long)]
        new_token_amount: Option<FrontendTokenAmount>,
    },
}

impl RelationArgs {
    /// Relation identifier.
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
            } => XorRelation::with_full_input(public_xoree, private_xoree, result)
                .generate_constraints(cs),

            RelationArgs::LinearEquation { a, x, b, y } => {
                LinearEquationRelation::with_full_input(a, x, b, y).generate_constraints(cs)
            }

            RelationArgs::MerkleTree {
                seed, leaf, leaves, ..
            } => MerkleTreeRelation::with_full_input(
                leaves.unwrap_or_else(|| panic!("You must provide `leaves`")),
                leaf.expect("You must provide `leaf` input"),
                seed,
            )
            .generate_constraints(cs),

            RelationArgs::Deposit {
                note,
                token_id,
                token_amount,
                trapdoor,
                nullifier,
            } => {
                if cs.is_in_setup_mode() {
                    return DepositRelation::without_input().generate_constraints(cs);
                }

                DepositRelation::with_full_input(
                    note.unwrap_or_else(|| panic!("You must provide note")),
                    token_id.unwrap_or_else(|| panic!("You must provide token id")),
                    token_amount.unwrap_or_else(|| panic!("You must provide token amount")),
                    trapdoor.unwrap_or_else(|| panic!("You must provide trapdoor")),
                    nullifier.unwrap_or_else(|| panic!("You must provide nullifier")),
                )
                .generate_constraints(cs)
            }

            RelationArgs::Withdraw {
                max_path_len,
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
            } => {
                if cs.is_in_setup_mode() {
                    return WithdrawRelation::without_input(max_path_len).generate_constraints(cs);
                }

                WithdrawRelation::with_full_input(
                    max_path_len,
                    fee.unwrap_or_else(|| panic!("You must provide fee")),
                    recipient.unwrap_or_else(|| panic!("You must provide recipient")),
                    token_id.unwrap_or_else(|| panic!("You must provide token id")),
                    old_nullifier.unwrap_or_else(|| panic!("You must provide old nullifier")),
                    new_note.unwrap_or_else(|| panic!("You must provide new note")),
                    token_amount_out.unwrap_or_else(|| panic!("You must provide token amount out")),
                    merkle_root.unwrap_or_else(|| panic!("You must provide merkle root")),
                    old_trapdoor.unwrap_or_else(|| panic!("You must provide old trapdoor")),
                    new_trapdoor.unwrap_or_else(|| panic!("You must provide new trapdoor")),
                    new_nullifier.unwrap_or_else(|| panic!("You must provide new nullifier")),
                    merkle_path.unwrap_or_else(|| panic!("You must provide merkle path")),
                    leaf_index.unwrap_or_else(|| panic!("You must provide leaf index")),
                    old_note.unwrap_or_else(|| panic!("You must provide old note")),
                    whole_token_amount
                        .unwrap_or_else(|| panic!("You must provide whole token amount")),
                    new_token_amount.unwrap_or_else(|| panic!("You must provide new token amount")),
                )
                .generate_constraints(cs)
            }
        }
    }
}

impl GetPublicInput<CircuitField> for RelationArgs {
    fn public_input(&self) -> Vec<CircuitField> {
        match self {
            RelationArgs::Xor {
                public_xoree,
                result,
                ..
            } => XorRelation::with_public_input(*public_xoree, *result).public_input(),

            RelationArgs::LinearEquation { a, b, y, .. } => {
                LinearEquationRelation::without_input(*a, *b, *y).public_input()
            }

            RelationArgs::MerkleTree {
                seed,
                root,
                leaf,
                leaves,
            } => {
                let leaf = leaf.expect("You must provide `leaf` input");
                if let Some(root) = root {
                    MerkleTreeRelation::with_public_input(*root, leaf, seed.clone()).public_input()
                } else if let Some(leaves) = leaves {
                    MerkleTreeRelation::with_full_input(leaves.clone(), leaf, seed.clone())
                        .public_input()
                } else {
                    panic!("You must provide either `root` or `leaves` input")
                }
            }

            RelationArgs::Deposit {
                note,
                token_id,
                token_amount,
                ..
            } => match (note, token_id, token_amount) {
                (Some(note), Some(token_id), Some(token_amount)) => {
                    DepositRelation::with_public_input(*note, *token_id, *token_amount)
                        .public_input()
                }
                _ => panic!("Provide at least public (note, token id and token amount)"),
            },

            RelationArgs::Withdraw {
                max_path_len,
                old_nullifier,
                merkle_root,
                new_note,
                token_id,
                token_amount_out,
                fee,
                recipient,
                ..
            } => {
                match (
                    fee,
                    recipient,
                    token_id,
                    old_nullifier,
                    new_note,
                    token_amount_out,
                    merkle_root,
                ) {
                    (
                        Some(fee),
                        Some(recipient),
                        Some(token_id),
                        Some(old_nullifier),
                        Some(new_note),
                        Some(token_amount_out),
                        Some(merkle_root),
                    ) => WithdrawRelation::with_public_input(
                        *max_path_len,
                        *fee,
                        *recipient,
                        *token_id,
                        *old_nullifier,
                        *new_note,
                        *token_amount_out,
                        *merkle_root,
                    )
                    .public_input(),
                    _ => panic!("Provide at least public (fee, recipient, token id, old nullifier, new note, token amount out and merkle root)"),
                }
            }
        }
    }
}
