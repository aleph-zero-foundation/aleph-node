#[ink_lang::contract(env = snarcos_extension::DefaultEnvironment)]
#[allow(clippy::let_unit_value)] // Clippy shouts about returning anything from messages.
mod blender {
    use core::ops::Not;

    use ark_serialize::CanonicalSerialize;
    use ink_env::call::{build_call, Call, ExecutionInput, Selector};
    #[allow(unused_imports)]
    use ink_env::*;
    use ink_prelude::{vec, vec::Vec};
    use ink_storage::{traits::SpreadAllocate, Mapping};
    use openbrush::contracts::psp22::PSP22Error;
    use scale::{Decode, Encode};
    #[cfg(feature = "std")]
    use scale_info::TypeInfo;

    use crate::{
        error::BlenderError, merkle_tree::MerkleTree, MerkleRoot, Note, Nullifier, Set,
        TokenAmount, TokenId, DEPOSIT_VK_IDENTIFIER, PSP22_TRANSFER_FROM_SELECTOR, SYSTEM,
        WITHDRAW_VK_IDENTIFIER,
    };

    /// Supported relations - used for registering verifying keys.
    #[derive(Eq, PartialEq, Debug, Decode, Encode)]
    #[cfg_attr(feature = "std", derive(TypeInfo))]
    pub enum Relation {
        Deposit,
        Withdraw,
    }

    #[ink(event)]
    pub struct Deposited {
        #[ink(topic)]
        token_id: TokenId,
        value: TokenAmount,
        note: Note,
        leaf_idx: u32,
    }

    type Result<T> = core::result::Result<T, BlenderError>;

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Blender {
        /// Merkle tree holding all the notes.
        notes: MerkleTree<1024>,
        /// All the seen Merkle roots (including the current).
        merkle_roots: Set<MerkleRoot>,
        /// Set of presented nullifiers.
        nullifiers: Set<Nullifier>,

        /// List of registered (supported) token contracts.
        registered_tokens: Mapping<TokenId, AccountId>,

        /// Mister Blendermaster (contract admin).
        blendermaster: AccountId,
    }

    impl Default for Blender {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Blender {
        /// Instantiate contract. Set caller as blendermaster.
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|blender: &mut Self| {
                blender.blendermaster = Self::env().caller();
            })
        }

        /// Trigger deposit action (see ADR for detailed description).
        #[ink(message, selector = 1)]
        pub fn deposit(
            &mut self,
            token_id: TokenId,
            value: TokenAmount,
            note: Note,
            proof: Vec<u8>,
        ) -> Result<()> {
            self.acquire_deposit(token_id, value)?;
            self.verify_deposit(token_id, value, note, proof)?;
            let leaf_idx = self
                .notes
                .add(note)
                .map_err(|_| BlenderError::TooManyNotes)?;
            self.merkle_roots.insert(self.notes.root(), &());

            self.env().emit_event(Deposited {
                token_id,
                value,
                note,
                leaf_idx,
            });

            Ok(())
        }

        /// Transfer `deposit` tokens of type `token_id` from the caller to this contract.
        fn acquire_deposit(&self, token_id: TokenId, deposit: TokenAmount) -> Result<()> {
            let token_contract = self
                .registered_token_address(token_id)
                .ok_or(BlenderError::TokenIdNotRegistered)?;

            build_call::<super::blender::Environment>()
                .call_type(Call::new().callee(token_contract))
                .exec_input(
                    ExecutionInput::new(Selector::new(PSP22_TRANSFER_FROM_SELECTOR))
                        .push_arg(self.env().caller())
                        .push_arg(self.env().account_id())
                        .push_arg(deposit as Balance)
                        .push_arg::<Vec<u8>>(vec![]),
                )
                .call_flags(CallFlags::default().set_allow_reentry(true))
                .returns::<core::result::Result<(), PSP22Error>>()
                .fire()??;
            Ok(())
        }

        /// Serialize with `ark-serialize::CanonicalSerialize`.
        pub fn serialize<T: CanonicalSerialize + ?Sized>(t: &T) -> Vec<u8> {
            let mut bytes = vec![0; t.serialized_size()];
            t.serialize(&mut bytes[..]).expect("Failed to serialize");
            bytes.to_vec()
        }

        /// Call `pallet_snarcos::verify` for the `deposit` relation with `(token_id, value, note)`
        /// as public input.
        fn verify_deposit(
            &self,
            token_id: TokenId,
            value: TokenAmount,
            note: Note,
            proof: Vec<u8>,
        ) -> Result<()> {
            // For now we assume naive input encoding (from typed arguments).
            let serialized_input = [
                Self::serialize(&token_id),
                Self::serialize(&value),
                Self::serialize(note.as_ref()),
            ]
            .concat();

            self.env().extension().verify(
                DEPOSIT_VK_IDENTIFIER,
                proof,
                serialized_input,
                SYSTEM,
            )?;

            Ok(())
        }

        /// Register a verifying key for one of the `Relation`.
        ///
        /// For blendermaster use only.
        #[ink(message, selector = 8)]
        pub fn register_vk(&mut self, relation: Relation, vk: Vec<u8>) -> Result<()> {
            self.ensure_mr_blendermaster()?;
            let identifier = match relation {
                Relation::Deposit => DEPOSIT_VK_IDENTIFIER,
                Relation::Withdraw => WITHDRAW_VK_IDENTIFIER,
            };
            self.env().extension().store_key(identifier, vk)?;
            Ok(())
        }

        /// Check if there is a token address registered at `token_id`.
        #[ink(message, selector = 9)]
        pub fn registered_token_address(&self, token_id: TokenId) -> Option<AccountId> {
            self.registered_tokens.get(token_id)
        }

        /// Register a token contract (`token_address`) at `token_id`.
        ///
        /// For blendermaster use only.
        #[ink(message, selector = 10)]
        pub fn register_new_token(
            &mut self,
            token_id: TokenId,
            token_address: AccountId,
        ) -> Result<()> {
            self.ensure_mr_blendermaster()?;
            self.registered_tokens
                .contains(token_id)
                .not()
                .then(|| self.registered_tokens.insert(token_id, &token_address))
                .ok_or(BlenderError::TokenIdAlreadyRegistered)
        }

        /// Check if the caller is the blendermaster.
        fn ensure_mr_blendermaster(&self) -> Result<()> {
            (self.env().caller() == self.blendermaster)
                .then_some(())
                .ok_or(BlenderError::InsufficientPermission)
        }
    }
}
