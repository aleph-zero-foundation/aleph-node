#[ink_lang::contract(env = snarcos_extension::DefaultEnvironment)]
#[allow(clippy::let_unit_value)] // Clippy shouts about returning anything from messages.
mod blender {
    use core::ops::Not;

    use ark_ff::BigInteger256;
    use ark_serialize::CanonicalSerialize;
    use ink_env::call::{build_call, Call, ExecutionInput, Selector};
    #[allow(unused_imports)]
    use ink_env::*;
    use ink_lang::{
        codegen::{EmitEvent, Env},
        reflect::ContractEventBase,
    };
    use ink_prelude::{vec, vec::Vec};
    use ink_storage::{traits::SpreadAllocate, Mapping};
    use openbrush::{
        contracts::{ownable::*, psp22::PSP22Error},
        modifiers,
        traits::Storage,
    };
    use scale::{Decode, Encode};

    use crate::{
        crypto::compute_parent_hash, error::BlenderError, CircuitField, MerkleHash, MerkleRoot,
        Note, Nullifier, Set, TokenAmount, TokenId, DEPOSIT_VK_IDENTIFIER,
        PSP22_TRANSFER_FROM_SELECTOR, PSP22_TRANSFER_SELECTOR, SYSTEM, WITHDRAW_VK_IDENTIFIER,
    };

    /// Supported relations - used for registering verifying keys.
    #[derive(Eq, PartialEq, Debug, Decode, Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Relation {
        Deposit,
        Withdraw,
    }

    #[ink(event)]
    pub struct Deposited {
        #[ink(topic)]
        token_id: TokenId,
        value: TokenAmount,
        leaf_idx: u32,
        note: Note,
    }

    #[ink(event)]
    pub struct Withdrawn {
        #[ink(topic)]
        token_id: TokenId,
        value: TokenAmount,
        #[ink(topic)]
        recipient: AccountId,
        leaf_idx: u32,
        new_note: Note,
    }

    type Result<T> = core::result::Result<T, BlenderError>;
    type Event = <Blender as ContractEventBase>::Type;

    /// Describes a path from a leaf to the root.
    ///
    /// The path is given in a ~optimized way:
    ///  - it does not contain leaf (it is the note that you have submitted)
    ///  - it does not contain parents (i.e. results of hashing intermediate children)
    ///
    /// So effectively it is just siblings, from bottom to top - the first one is the leaf sibling,
    /// the next one is their uncle and so forth. You can recreate shape of this path knowing leaf
    /// index.
    pub type MerklePath = Vec<MerkleHash>;

    #[ink(storage)]
    #[derive(SpreadAllocate, Storage)]
    pub struct Blender {
        /// Merkle tree holding notes in its leaves.
        ///
        /// Root is at [1], children are at [2n] and [2n+1].
        notes: Mapping<u32, MerkleHash>,
        /// Marker of the first 'non-occupied' leaf.
        next_free_leaf: u32,
        /// Tree capacity.
        max_leaves: u32,

        /// All the observed Merkle roots (including the current, excluding the initial).
        merkle_roots: Set<MerkleRoot>,
        /// Set of presented nullifiers.
        nullifiers: Set<Nullifier>,

        /// List of registered (supported) token contracts.
        registered_tokens: Mapping<TokenId, AccountId>,

        /// `Openbrush::Ownable` data.
        #[storage_field]
        ownable: ownable::Data,
    }

    impl Ownable for Blender {}

    impl Blender {
        /// Instantiate the contract. Set the caller as the owner.
        #[ink(constructor)]
        pub fn new(max_leaves: u32) -> Self {
            if !max_leaves.is_power_of_two() {
                panic!("Please have 2^n leaves")
            }

            ink_lang::utils::initialize_contract(|blender: &mut Self| {
                blender._init_with_owner(Self::env().caller());
                blender.max_leaves = max_leaves;
                blender.next_free_leaf = max_leaves;
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

            self.create_new_leaf(note)?;
            self.merkle_roots.insert(self.current_root(), &());

            Self::emit_event(
                self.env(),
                Event::Deposited(Deposited {
                    token_id,
                    value,
                    leaf_idx: self.next_free_leaf - 1,
                    note,
                }),
            );

            Ok(())
        }

        /// Trigger withdraw action (see ADR for detailed description).
        #[allow(clippy::too_many_arguments)]
        #[ink(message, selector = 2)]
        pub fn withdraw(
            &mut self,
            token_id: TokenId,
            value: TokenAmount,
            recipient: AccountId,
            fee_for_caller: Option<TokenAmount>,
            merkle_root: MerkleRoot,
            nullifier: Nullifier,
            new_note: Note,
            proof: Vec<u8>,
        ) -> Result<()> {
            self.verify_fee(fee_for_caller, value)?;
            self.verify_merkle_root(merkle_root)?;
            self.verify_nullifier(nullifier)?;
            self.verify_withdrawal(
                token_id,
                value,
                merkle_root,
                nullifier,
                new_note,
                proof,
                fee_for_caller.unwrap_or_default(),
                recipient,
            )?;

            self.create_new_leaf(new_note)?;
            self.nullifiers.insert(nullifier, &());

            self.withdraw_funds(token_id, value, fee_for_caller, recipient)?;

            Self::emit_event(
                self.env(),
                Event::Withdrawn(Withdrawn {
                    token_id,
                    value,
                    recipient,
                    leaf_idx: self.next_free_leaf - 1,
                    new_note,
                }),
            );

            Ok(())
        }

        /// Read the current root of the Merkle tree with notes.
        #[ink(message, selector = 3)]
        pub fn current_merkle_root(&self) -> MerkleRoot {
            self.current_root()
        }

        /// Retrieve the path from the leaf to the root. `None` if the leaf does not exist.
        #[ink(message, selector = 4)]
        pub fn merkle_path(&self, leaf_idx: u32) -> Option<MerklePath> {
            if self.max_leaves > leaf_idx || leaf_idx >= self.next_free_leaf {
                return None;
            }

            let mut auth_path = vec![self.tree_value(leaf_idx ^ 1)];

            let mut current_idx = leaf_idx / 2;
            while current_idx > 1 {
                auth_path.push(self.tree_value(current_idx ^ 1));
                current_idx /= 2;
            }

            Some(auth_path)
        }

        /// Check whether `nullifier` has been already used.
        #[ink(message, selector = 5)]
        pub fn contains_nullifier(&self, nullifier: Nullifier) -> bool {
            self.nullifiers.contains(nullifier)
        }

        /// Register a verifying key for one of the `Relation`.
        ///
        /// For owner use only.
        #[ink(message, selector = 8)]
        #[modifiers(only_owner)]
        pub fn register_vk(&mut self, relation: Relation, vk: Vec<u8>) -> Result<()> {
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
        /// For owner use only.
        #[ink(message, selector = 10)]
        #[modifiers(only_owner)]
        pub fn register_new_token(
            &mut self,
            token_id: TokenId,
            token_address: AccountId,
        ) -> Result<()> {
            self.registered_tokens
                .contains(token_id)
                .not()
                .then(|| self.registered_tokens.insert(token_id, &token_address))
                .ok_or(BlenderError::TokenIdAlreadyRegistered)
        }
    }

    /// Auxiliary contract methods.
    impl Blender {
        /// Get the value at this node idx or the clean hash (`[0u64; 4]`).
        fn tree_value(&self, idx: u32) -> MerkleHash {
            self.notes.get(idx).unwrap_or_default()
        }

        /// Get the value from the root node.
        fn current_root(&self) -> MerkleRoot {
            self.tree_value(1)
        }

        /// Add `value` to the first 'non-occupied' leaf.
        ///
        /// Returns `Err(_)` iff there are no free leafs.
        fn create_new_leaf(&mut self, value: Note) -> Result<()> {
            if self.next_free_leaf == 2 * self.max_leaves {
                return Err(BlenderError::TooManyNotes);
            }

            self.notes.insert(self.next_free_leaf, &value);

            let mut parent = self.next_free_leaf / 2;
            while parent > 0 {
                let left_child = self.tree_value(2 * parent);
                let right_child = self.tree_value(2 * parent + 1);
                self.notes
                    .insert(parent, &compute_parent_hash(&left_child, &right_child));
                parent /= 2;
            }

            self.next_free_leaf += 1;
            Ok(())
        }

        /// Serialize with `ark-serialize::CanonicalSerialize`.
        pub fn serialize<T: CanonicalSerialize + ?Sized>(t: &T) -> Vec<u8> {
            let mut bytes = vec![0; t.serialized_size()];
            t.serialize(&mut bytes[..]).expect("Failed to serialize");
            bytes.to_vec()
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

        /// Call `pallet_snarcos::verify` for the `deposit` relation with `(token_id, value, note)`
        /// as public input.
        fn verify_deposit(
            &self,
            token_id: TokenId,
            value: TokenAmount,
            note: Note,
            proof: Vec<u8>,
        ) -> Result<()> {
            let input = [
                CircuitField::from(BigInteger256::new(note)),
                CircuitField::from(token_id),
                CircuitField::from(value),
            ];

            self.env().extension().verify(
                DEPOSIT_VK_IDENTIFIER,
                proof,
                Self::serialize(input.as_ref()),
                SYSTEM,
            )?;

            Ok(())
        }

        fn verify_fee(
            &self,
            fee_for_caller: Option<TokenAmount>,
            value_to_withdraw: TokenAmount,
        ) -> Result<()> {
            match fee_for_caller {
                Some(fee) if fee > value_to_withdraw => Err(BlenderError::TooHighFee),
                _ => Ok(()),
            }
        }

        fn verify_merkle_root(&self, merkle_root: MerkleRoot) -> Result<()> {
            self.merkle_roots
                .contains(merkle_root)
                .then_some(())
                .ok_or(BlenderError::UnknownMerkleRoot)
        }

        fn verify_nullifier(&self, nullifier: Nullifier) -> Result<()> {
            self.nullifiers
                .contains(nullifier)
                .not()
                .then_some(())
                .ok_or(BlenderError::NullifierAlreadyUsed)
        }

        #[allow(clippy::too_many_arguments)]
        fn verify_withdrawal(
            &self,
            token_id: TokenId,
            value_out: TokenAmount,
            merkle_root: MerkleRoot,
            nullifier: Nullifier,
            new_note: Note,
            proof: Vec<u8>,
            fee: TokenAmount,
            recipient: AccountId,
        ) -> Result<()> {
            let recipient_bytes: &[u8; 32] = recipient.as_ref();

            let input = [
                CircuitField::from(fee),
                CircuitField::from(BigInteger256::new([
                    u64::from_le_bytes(recipient_bytes[0..8].try_into().unwrap()),
                    u64::from_le_bytes(recipient_bytes[8..16].try_into().unwrap()),
                    u64::from_le_bytes(recipient_bytes[16..24].try_into().unwrap()),
                    u64::from_le_bytes(recipient_bytes[24..32].try_into().unwrap()),
                ])),
                CircuitField::from(token_id),
                CircuitField::from(nullifier),
                CircuitField::from(BigInteger256::new(new_note)),
                CircuitField::from(value_out),
                CircuitField::from(BigInteger256::new(merkle_root)),
            ];

            self.env().extension().verify(
                WITHDRAW_VK_IDENTIFIER,
                proof,
                Self::serialize(input.as_ref()),
                SYSTEM,
            )?;

            Ok(())
        }

        fn withdraw_funds(
            &self,
            token_id: TokenId,
            value: TokenAmount,
            fee_for_caller: Option<TokenAmount>,
            recipient: AccountId,
        ) -> Result<()> {
            let token_contract = self
                .registered_token_address(token_id)
                .ok_or(BlenderError::TokenIdNotRegistered)?;

            match fee_for_caller {
                Some(fee) => {
                    self.transfer(token_contract, fee, self.env().caller())?;
                    self.transfer(token_contract, value - fee, recipient)
                }
                None => self.transfer(token_contract, value, recipient),
            }
        }

        fn transfer(
            &self,
            token_contract: AccountId,
            value: TokenAmount,
            recipient: AccountId,
        ) -> Result<()> {
            build_call::<super::blender::Environment>()
                .call_type(Call::new().callee(token_contract))
                .exec_input(
                    ExecutionInput::new(Selector::new(PSP22_TRANSFER_SELECTOR))
                        .push_arg(recipient)
                        .push_arg(value as Balance)
                        .push_arg::<Vec<u8>>(vec![]),
                )
                .returns::<core::result::Result<(), PSP22Error>>()
                .fire()??;
            Ok(())
        }

        /// Emit event with correct type boundaries.
        fn emit_event<EE: EmitEvent<Blender>>(emitter: EE, event: Event) {
            emitter.emit_event(event);
        }
    }
}
