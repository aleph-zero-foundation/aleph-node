//! Provides "fake" runtime API implementations
//!
//! These are used to provide a type that implements these runtime APIs without requiring to import
//! the native runtimes.

use frame_support::weights::Weight;
use pallet_aleph_runtime_api::*;
use pallet_transaction_payment::FeeDetails;
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use primitives::{
    AccountId, ApiError as AlephApiError, AuraId, AuthorityId as AlephId, Balance, Block, Nonce,
    Perbill, SessionAuthorityData, SessionCommittee, SessionIndex, SessionValidatorError,
    Version as FinalityVersion,
};
use sp_consensus_aura::SlotDuration;
use sp_core::OpaqueMetadata;
use sp_runtime::{
    traits::Block as BlockT,
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult,
};
use sp_std::vec::Vec;
use sp_version::RuntimeVersion;

#[cfg(feature = "std")]
pub mod fake_runtime {

    pub struct Runtime;

    use super::*;

    sp_api::impl_runtime_apis! {
        impl sp_api::Core<Block> for Runtime {
            fn version() -> RuntimeVersion {
                unimplemented!()
            }

            fn execute_block(_: Block) {
                unimplemented!()
            }

            fn initialize_block(_: &<Block as BlockT>::Header) {
                unimplemented!()
            }
        }

        impl sp_api::Metadata<Block> for Runtime {
            fn metadata() -> OpaqueMetadata {
                unimplemented!()
            }

            fn metadata_at_version(_: u32) -> Option<OpaqueMetadata> {
                unimplemented!()
            }

            fn metadata_versions() -> Vec<u32> {
                unimplemented!()
            }
        }

        impl sp_block_builder::BlockBuilder<Block> for Runtime {
            fn apply_extrinsic(_: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
                unimplemented!()
            }

            fn finalize_block() -> <Block as BlockT>::Header {
                unimplemented!()
            }

            fn inherent_extrinsics(_: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
                unimplemented!()
            }

            fn check_inherents(
                _: Block,
                _: sp_inherents::InherentData,
            ) -> sp_inherents::CheckInherentsResult {
                unimplemented!()
            }
        }

        impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
            fn validate_transaction(
                _: TransactionSource,
                _: <Block as BlockT>::Extrinsic,
                _: <Block as BlockT>::Hash,
            ) -> TransactionValidity {
                unimplemented!()
            }
        }

        impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
            fn slot_duration() -> SlotDuration {
                unimplemented!()
            }

            fn authorities() -> Vec<AuraId> {
                unimplemented!()
            }
        }

        impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
            fn offchain_worker(_: &<Block as BlockT>::Header) {
                unimplemented!()
            }
        }

        impl sp_session::SessionKeys<Block> for Runtime {
            fn generate_session_keys(_: Option<Vec<u8>>) -> Vec<u8> {
                unimplemented!()
            }

            fn decode_session_keys(
                _: Vec<u8>,
            ) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
                unimplemented!()
            }
        }

        impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
            fn account_nonce(_: AccountId) -> Nonce {
                unimplemented!()
            }
        }

        impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
            Block,
            Balance,
        > for Runtime {
            fn query_info(_: <Block as BlockT>::Extrinsic, _: u32) -> RuntimeDispatchInfo<Balance> {
                unimplemented!()
            }
            fn query_fee_details(_: <Block as BlockT>::Extrinsic, _: u32) -> FeeDetails<Balance> {
                unimplemented!()
            }
            fn query_weight_to_fee(_: Weight) -> Balance {
                unimplemented!()
            }
            fn query_length_to_fee(_: u32) -> Balance {
                unimplemented!()
            }
        }

         impl crate::AlephSessionApi<Block> for Runtime {
            fn millisecs_per_block() -> u64 {
                unimplemented!()
            }

            fn session_period() -> u32 {
                unimplemented!()
            }

            fn authorities() -> Vec<AlephId> {
                unimplemented!()
            }

            fn next_session_authorities() -> Result<Vec<AlephId>, AlephApiError> {
                unimplemented!()
            }

            fn authority_data() -> SessionAuthorityData {
                unimplemented!()
            }

            fn next_session_authority_data() -> Result<SessionAuthorityData, AlephApiError> {
                unimplemented!()
            }

            fn finality_version() -> FinalityVersion {
                unimplemented!()
            }

            fn next_session_finality_version() -> FinalityVersion {
                unimplemented!()
            }

            fn predict_session_committee(
                _session: SessionIndex,
            ) -> Result<SessionCommittee<AccountId>, SessionValidatorError> {
                unimplemented!()
            }

            fn next_session_aura_authorities() -> Vec<(AccountId, AuraId)> {
                unimplemented!()
            }

            fn key_owner(_key: AlephId) -> Option<AccountId> {
                unimplemented!()
            }

            fn yearly_inflation() -> Perbill {
                unimplemented!()
            }

            fn current_era_payout() -> (Balance, Balance) {
                unimplemented!()
            }
        }

        /// There’s an important remark on how this fake runtime must be implemented - it does not need to
        /// have all the same entries like `impl_runtime_apis!` has - in particular, it does not need an
        /// implementation for
        ///  * `pallet_nomination_pools_runtime_api::NominationPoolsApi`
        ///  * `pallet_staking_runtime_api::StakingApi`
        ///  * `pallet_contracts::ContractsApi`
        /// ie, code compiles without them, even though real runtime has those.
        /// Why? Because this fake runtime API is only used only for sake of compilation, so as long
        /// as `fake_runtime_api` implements no less than real runtime API, we’re good.

        #[cfg(feature = "try-runtime")]
        impl frame_try_runtime::TryRuntime<Block> for Runtime {
            fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
                 unimplemented!()
            }

            fn execute_block(
                block: Block,
                state_root_check: bool,
                checks: bool,
                select: frame_try_runtime::TryStateSelect,
            ) -> Weight {
                 unimplemented!()
            }
         }

        #[cfg(feature = "runtime-benchmarks")]
        impl frame_benchmarking::Benchmark<Block> for Runtime {
            fn benchmark_metadata(extra: bool) -> (
                Vec<frame_benchmarking::BenchmarkList>,
                Vec<frame_support::traits::StorageInfo>,
            ) {
                 unimplemented!()
            }

            fn dispatch_benchmark(
                config: frame_benchmarking::BenchmarkConfig
            ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
                 unimplemented!()
            }
         }
    }
}
