//! Benchmarking suite for the chain extension.
//!
//! We make use of the FRAME benchmarking framework. Although it is dedicated specifically to FRAME pallets, with a few
//! tricks we can use it to benchmark our chain extension as well.
//!
//! # Tricks
//!
//! The benchmarking framework expects to see two things here:
//! - A `Config` trait that defines the pallet's configuration.
//! - A `Pallet` struct that implements the `Config` trait.
//!
//! Since we don't have a pallet, we have to provide these two things ourselves. We do this by defining two dummy items.
//! To avoid confusion outside this module, we reexport the `Pallet` as `ChainExtensionBenchmarking` type, which can be
//! then used in the runtime benchmarking setup.
//!
//! # Expectations from the runtime
//!
//! Benchmarks are run for a specific runtime instance. We can refer to it via the `T` type in the benchmark body. Since
//! sometimes we might require that `T` includes some pallet (e.g. `pallet_vk_storage`). We can put this constraint on
//! our artificial `Config` trait.
//!
//! ## Note
//!
//! Please note, that in the current form, it would be sufficient to just use the `VkStorage` pallet as the `Pallet`
//! type and `pallete_vk_storage::Config` as the `Config` trait. However, we want to keep the benchmarking of the
//! chain extension abstracted from the pallets that it uses. This is why we define our own dummy pallet and config.
//!
//! # Macro-generated benchmark suite
//!
//! Since we want to run benchmarks for different circuit parameters, we use a macro to generate all the corresponding
//! benchmark functions.
//!
//! However, since the whole benchmark suite is defined with `#[benchmarks]` macro, we cannot use an auxiliary macro
//! within it -- this is due to the macro expansion order. To overcome this problem, we provide a macro that generates
//! the whole benchmark suite.

#![allow(unused_imports)]

/// Dummy trait that defines the pallet's configuration. Since `auto trait` is not stable yet, we just provide a full
/// blanket implementation for all runtimes that contain the `pallet_vk_storage` pallet.
trait Config: pallet_vk_storage::Config {}
impl<T: pallet_vk_storage::Config> Config for T {}

/// Dummy pallet struct. The only thing that actually matters is that it is generic over some type `T` that implements
/// the `Config` trait.
pub struct Pallet<T> {
    _phantom: sp_std::marker::PhantomData<T>,
}

/// A type alias for the pallet struct. This is the type that should be used in the runtime benchmarking setup and
/// limits the confusion to this module only.
pub type ChainExtensionBenchmarking<T> = Pallet<T>;

/// Get the verification artifacts from the benchmark resources.
///
/// Since the benchmarks are run within the runtime, we don't have access to the common `std::fs` utilities.
/// Fortunately, we can still make use of the `include_bytes` macro.
///
/// We use a macro here, because a function cannot put literal variables in the `include_bytes` macro.
macro_rules! get_artifact {
    ($instances:literal, $row_blowup:literal, $art:literal) => {
        include_bytes!(concat!(
            "../../benchmark-resources/",
            $instances,
            "_",
            $row_blowup,
            "_",
            $art
        ))
        .to_vec()
    };
}

/// Generate the benchmark suite for the given circuit parameters.
macro_rules! generate_benchmarks {
    (
        circuit_parameters: $(($instances:literal, $row_blowup:literal)),*
    ) => {
        paste::paste! {
            use frame_benchmarking::v2::*;
            use frame_support::{sp_runtime::traits::Hash, BoundedVec};
            use pallet_vk_storage::{KeyHasher, VerificationKeys};
            use sp_std::vec;

            #[benchmarks]
            mod benchmarks {
                use parity_scale_codec::{Decode, Encode};

                use super::*;
                use crate::{args::VerifyArgs, backend::BackendExecutorT};

                /// Benchmark `verify` arguments decoding.
                #[benchmark]
                fn verify_read_args(
                    // Check input length up to ~10MB
                    x: Linear<0, 10_000_000>,
                ) {
                    let args = VerifyArgs {
                        verification_key_hash: Default::default(),
                        proof: vec![1; (x / 2) as usize],
                        public_input: vec![2; (x / 2) as usize],
                    }
                    .encode();

                    #[block]
                    {
                        VerifyArgs::decode(&mut &args[..]).unwrap();
                    }
                }


                $(#[benchmark]
                fn [<verify_ $instances _ $row_blowup>] () {
                    let verification_key = get_artifact!($instances, $row_blowup, "vk");
                    let args = VerifyArgs {
                        verification_key_hash: KeyHasher::hash(&verification_key),
                        proof: get_artifact!($instances, $row_blowup, "proof"),
                        public_input: get_artifact!($instances, $row_blowup, "input"),
                    };

                    let verification_key: BoundedVec<_, _> = verification_key.try_into().unwrap();
                    VerificationKeys::<T>::insert(args.verification_key_hash, verification_key);

                    #[block]
                    {
                        <T as BackendExecutorT>::verify(args).unwrap();
                    }
                })*
            }
        }
    };
}

generate_benchmarks!(
    circuit_parameters:
        (1, 1), (1, 8), (1, 64), (1, 512), (1, 4096),
        (2, 1), (2, 8), (2, 64), (2, 512), (2, 4096),
        (8, 1), (8, 8), (8, 64), (8, 512), (8, 4096),
        (16, 1), (16, 8), (16, 64), (16, 512), (16, 4096),
        (64, 1), (64, 8), (64, 64), (64, 512), (64, 4096),
        (128, 1), (128, 8), (128, 64), (128, 512), (128, 4096)
);
