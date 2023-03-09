use clap::Subcommand;
use liminal_ark_relations::{
    CircuitField, ConstraintSynthesizer, ConstraintSystemRef, DepositAndMergeRelationWithFullInput,
    DepositAndMergeRelationWithPublicInput, DepositAndMergeRelationWithoutInput,
    DepositRelationWithFullInput, DepositRelationWithPublicInput, DepositRelationWithoutInput,
    FrontendAccount, FrontendLeafIndex, FrontendMerklePath, FrontendMerkleRoot, FrontendNote,
    FrontendNullifier, FrontendTokenAmount, FrontendTokenId, FrontendTrapdoor, GetPublicInput,
    LinearEquationRelationWithFullInput, LinearEquationRelationWithPublicInput,
    MergeRelationWithFullInput, MergeRelationWithPublicInput, MergeRelationWithoutInput,
    PreimageRelationWithFullInput, PreimageRelationWithPublicInput, Result as R1CsResult,
    WithdrawRelationWithFullInput, WithdrawRelationWithPublicInput, WithdrawRelationWithoutInput,
    XorRelationWithFullInput, XorRelationWithPublicInput,
};

use crate::snark_relations::parsing::{
    parse_frontend_account, parse_frontend_merkle_path, parse_frontend_note,
};

/// All available relations from `relations` crate.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Eq, PartialEq, Hash, Debug, Subcommand)]
pub enum RelationArgs {
    Preimage {
        /// hash to commit to (public input)
        #[clap(long, value_parser = parse_frontend_note)]
        hash: Option<[u64; 4]>,
        /// preimage (private witness)
        #[clap(long, value_parser = parse_frontend_note)]
        preimage: Option<[u64; 4]>,
    },

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

    DepositAndMerge {
        /// The upper bound for Merkle tree height (circuit constant).
        #[clap(long, default_value = "16")]
        max_path_len: u8,

        /// The identifier of the token being unshielded (public input).
        #[clap(long)]
        token_id: Option<FrontendTokenId>,
        /// The nullifier that was used for the old note (public input).
        #[clap(long)]
        old_nullifier: Option<FrontendNullifier>,
        /// The new note (public input).
        #[clap(long, value_parser = parse_frontend_note)]
        new_note: Option<FrontendNote>,
        /// The amount being shielded (public input).
        #[clap(long)]
        token_amount: Option<FrontendTokenAmount>,
        /// The Merkle root of the tree containing the old note (public input).
        #[clap(long, value_parser = parse_frontend_note)]
        merkle_root: Option<FrontendMerkleRoot>,

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
        /// The original amount that was shielded (private input).
        #[clap(long)]
        old_token_amount: Option<FrontendTokenAmount>,
        /// The token amount that will be shielded in the new note (private input).
        #[clap(long)]
        new_token_amount: Option<FrontendTokenAmount>,
    },

    Merge {
        /// The upper bound for Merkle tree height (circuit constant).
        #[clap(long, default_value = "16")]
        max_path_len: u8,

        /// The identifier of the token being unshielded (public input).
        #[clap(long)]
        token_id: Option<FrontendTokenId>,
        /// The nullifier that was used for the first old note (public input).
        #[clap(long)]
        first_old_nullifier: Option<FrontendNullifier>,
        /// The nullifier that was used for the second old note (public input).
        #[clap(long)]
        second_old_nullifier: Option<FrontendNullifier>,
        /// The new note (public input).
        #[clap(long, value_parser = parse_frontend_note)]
        new_note: Option<FrontendNote>,
        /// The Merkle root of the tree containing the first and second old notes (public input).
        #[clap(long, value_parser = parse_frontend_note)]
        merkle_root: Option<FrontendMerkleRoot>,

        /// The trapdoor that was used for the first old note (private input).
        #[clap(long)]
        first_old_trapdoor: Option<FrontendTrapdoor>,
        /// The trapdoor that was used for the second old note (private input).
        #[clap(long)]
        second_old_trapdoor: Option<FrontendTrapdoor>,
        /// The trapdoor that was used for the new note (private input).
        #[clap(long)]
        new_trapdoor: Option<FrontendTrapdoor>,
        /// The nullifier that was used for the new note (private input).
        #[clap(long)]
        new_nullifier: Option<FrontendNullifier>,
        /// The Merkle path proving that the first old note is under `merkle_root` (private input).
        #[clap(long, value_parser = parse_frontend_merkle_path)]
        first_merkle_path: Option<FrontendMerklePath>,
        /// The Merkle path proving that the second old note is under `merkle_root` (private input).
        #[clap(long, value_parser = parse_frontend_merkle_path)]
        second_merkle_path: Option<FrontendMerklePath>,
        /// The index of the first old note in the Merkle tree (private input).
        #[clap(long)]
        first_leaf_index: Option<FrontendLeafIndex>,
        /// The index of the second old note in the Merkle tree (private input).
        #[clap(long)]
        second_leaf_index: Option<FrontendLeafIndex>,
        /// The first old note (private input).
        #[clap(long, value_parser = parse_frontend_note)]
        first_old_note: Option<FrontendNote>,
        /// The second old note (private input).
        #[clap(long, value_parser = parse_frontend_note)]
        second_old_note: Option<FrontendNote>,
        /// The original amount that was shielded in the first old note (private input).
        #[clap(long)]
        first_old_token_amount: Option<FrontendTokenAmount>,
        /// The original amount that was shielded in the second old note (private input).
        #[clap(long)]
        second_old_token_amount: Option<FrontendTokenAmount>,
        /// The token amount that will be shielded in the new note (private input).
        #[clap(long)]
        new_token_amount: Option<FrontendTokenAmount>,
    },

    Withdraw {
        /// The upper bound for Merkle tree height (circuit constant).
        #[clap(long, default_value = "16")]
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
            RelationArgs::Deposit { .. } => String::from("deposit"),
            RelationArgs::DepositAndMerge { .. } => String::from("deposit_and_merge"),
            RelationArgs::Merge { .. } => String::from("merge"),
            RelationArgs::Withdraw { .. } => String::from("withdraw"),
            RelationArgs::Preimage { .. } => String::from("preimage"),
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
            } => XorRelationWithFullInput::new(result, public_xoree, private_xoree)
                .generate_constraints(cs),

            RelationArgs::LinearEquation { a, x, b, y } => {
                LinearEquationRelationWithFullInput::new(a, b, y, x).generate_constraints(cs)
            }

            RelationArgs::Deposit {
                note,
                token_id,
                token_amount,
                trapdoor,
                nullifier,
            } => {
                if cs.is_in_setup_mode() {
                    return DepositRelationWithoutInput::new().generate_constraints(cs);
                }

                DepositRelationWithFullInput::new(
                    note.unwrap_or_else(|| panic!("You must provide note")),
                    token_id.unwrap_or_else(|| panic!("You must provide token id")),
                    token_amount.unwrap_or_else(|| panic!("You must provide token amount")),
                    trapdoor.unwrap_or_else(|| panic!("You must provide trapdoor")),
                    nullifier.unwrap_or_else(|| panic!("You must provide nullifier")),
                )
                .generate_constraints(cs)
            }

            RelationArgs::DepositAndMerge {
                max_path_len,
                token_id,
                old_nullifier,
                new_note,
                token_amount,
                merkle_root,
                old_trapdoor,
                new_trapdoor,
                new_nullifier,
                merkle_path,
                leaf_index,
                old_note,
                old_token_amount,
                new_token_amount,
            } => {
                if cs.is_in_setup_mode() {
                    return DepositAndMergeRelationWithoutInput::new(max_path_len)
                        .generate_constraints(cs);
                }

                DepositAndMergeRelationWithFullInput::new(
                    max_path_len,
                    token_id.unwrap_or_else(|| panic!("You must provide token id")),
                    old_nullifier.unwrap_or_else(|| panic!("You must provide old nullifier")),
                    new_note.unwrap_or_else(|| panic!("You must provide new note")),
                    token_amount.unwrap_or_else(|| panic!("You must provide token amount")),
                    merkle_root.unwrap_or_else(|| panic!("You must provide merkle root")),
                    old_trapdoor.unwrap_or_else(|| panic!("You must provide old trapdoor")),
                    new_trapdoor.unwrap_or_else(|| panic!("You must provide new trapdoor")),
                    new_nullifier.unwrap_or_else(|| panic!("You must provide new nullifier")),
                    merkle_path.unwrap_or_else(|| panic!("You must provide merkle path")),
                    leaf_index.unwrap_or_else(|| panic!("You must provide leaf index")),
                    old_note.unwrap_or_else(|| panic!("You must provide old note")),
                    old_token_amount.unwrap_or_else(|| panic!("You must provide old token amount")),
                    new_token_amount.unwrap_or_else(|| panic!("You must provide new token amount")),
                )
                .generate_constraints(cs)
            }

            RelationArgs::Merge {
                max_path_len,
                token_id,
                first_old_nullifier,
                second_old_nullifier,
                new_note,
                merkle_root,
                first_old_trapdoor,
                second_old_trapdoor,
                new_trapdoor,
                new_nullifier,
                first_merkle_path,
                second_merkle_path,
                first_leaf_index,
                second_leaf_index,
                first_old_note,
                second_old_note,
                first_old_token_amount,
                second_old_token_amount,
                new_token_amount,
            } => {
                if cs.is_in_setup_mode() {
                    return MergeRelationWithoutInput::new(max_path_len).generate_constraints(cs);
                }

                MergeRelationWithFullInput::new(
                    max_path_len,
                    token_id.unwrap_or_else(|| panic!("You must provide token id")),
                    first_old_nullifier
                        .unwrap_or_else(|| panic!("You must provide first old nullifier")),
                    second_old_nullifier
                        .unwrap_or_else(|| panic!("You must provide second old nullifier")),
                    new_note.unwrap_or_else(|| panic!("You must provide new note")),
                    merkle_root.unwrap_or_else(|| panic!("You must provide merkle root")),
                    first_old_trapdoor
                        .unwrap_or_else(|| panic!("You must provide first old trapdoor")),
                    second_old_trapdoor
                        .unwrap_or_else(|| panic!("You must provide second old trapdoor")),
                    new_trapdoor.unwrap_or_else(|| panic!("You must provide new trapdoor")),
                    new_nullifier.unwrap_or_else(|| panic!("You must provide new nullifier")),
                    first_merkle_path
                        .unwrap_or_else(|| panic!("You must provide first merkle path")),
                    second_merkle_path
                        .unwrap_or_else(|| panic!("You must provide second merkle path")),
                    first_leaf_index.unwrap_or_else(|| panic!("You must provide first leaf index")),
                    second_leaf_index
                        .unwrap_or_else(|| panic!("You must provide second leaf index")),
                    first_old_note.unwrap_or_else(|| panic!("You must provide first old note")),
                    second_old_note.unwrap_or_else(|| panic!("You must provide second old note")),
                    first_old_token_amount
                        .unwrap_or_else(|| panic!("You must provide first old token amount")),
                    second_old_token_amount
                        .unwrap_or_else(|| panic!("You must provide second old token amount")),
                    new_token_amount.unwrap_or_else(|| panic!("You must provide new token amount")),
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
                    return WithdrawRelationWithoutInput::new(max_path_len)
                        .generate_constraints(cs);
                }

                WithdrawRelationWithFullInput::new(
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

            RelationArgs::Preimage { hash, preimage } => PreimageRelationWithFullInput::new(
                preimage.unwrap_or_else(|| panic!("You must provide preimage")),
                hash.unwrap_or_else(|| panic!("You must provide hash")),
            )
            .generate_constraints(cs),
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
            } => XorRelationWithPublicInput::new(*result, *public_xoree).serialize_public_input(),

            RelationArgs::LinearEquation { a, b, y, .. } => {
                LinearEquationRelationWithPublicInput::new(*a, *b, *y).serialize_public_input()
            }

            RelationArgs::Deposit {
                note,
                token_id,
                token_amount,
                ..
            } => match (note, token_id, token_amount) {
                (Some(note), Some(token_id), Some(token_amount)) => {
                    DepositRelationWithPublicInput::new(*note, *token_id, *token_amount)
                        .serialize_public_input()
                }
                _ => panic!("Provide at least public (note, token id and token amount)"),
            },

            RelationArgs::DepositAndMerge{
                max_path_len,
                token_id,
                old_nullifier,
                new_note,
                token_amount,
                merkle_root,
                ..
            } => {
                match (
                    token_id,
                    old_nullifier,
                    new_note,
                    token_amount,
                    merkle_root,
                ) {
                    (
                        Some(token_id),
                        Some(old_nullifier),
                        Some(new_note),
                        Some(token_amount),
                        Some(merkle_root),
                    ) => DepositAndMergeRelationWithPublicInput::new(
                        *max_path_len,
                        *token_id,
                        *old_nullifier,
                        *new_note,
                        *token_amount,
                        *merkle_root,
                    ).serialize_public_input(),
                    _ => panic!("Provide at least public (token id, old token amount, old nullifier, merkle root, and new note)"),
                }
            }

            RelationArgs::Merge{
                max_path_len,
                token_id,
                first_old_nullifier,
                second_old_nullifier,
                new_note,
                merkle_root,
                ..
            } => {
                match (
                    token_id,
                    first_old_nullifier,
                    second_old_nullifier,
                    new_note,
                    merkle_root,
                ) {
                    (
                        Some(token_id),
                        Some(first_old_nullifier),
                        Some(second_old_nullifier),
                        Some(new_note),
                        Some(merkle_root),
                    ) => MergeRelationWithPublicInput::new(
                        *max_path_len,
                        *token_id,
                        *first_old_nullifier,
                        *second_old_nullifier,
                        *new_note,
                        *merkle_root,
                    ).serialize_public_input(),
                    _ => panic!("Provide at least public (token id, first old nullifier, second old nullifier, new note, Merkle root)."),
                }
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
                    ) => WithdrawRelationWithPublicInput::new(
                        *max_path_len,
                        *fee,
                        *recipient,
                        *token_id,
                        *old_nullifier,
                        *new_note,
                        *token_amount_out,
                        *merkle_root,
                    )
                    .serialize_public_input(),
                    _ => panic!("Provide at least public (fee, recipient, token id, old nullifier, new note, token amount out and merkle root)"),
                }
            }
            RelationArgs::Preimage { hash, .. } => PreimageRelationWithPublicInput::new(hash.unwrap_or_else(|| panic!("You must provide hash"))).serialize_public_input(),
        }
    }
}
