pub use nomination_pools::CustomMigrateToV2;

mod nomination_pools {
    use codec::{Decode, DecodeAll, Encode, Error, Input};
    use frame_support::{
        log,
        traits::{OnRuntimeUpgrade, StorageVersion},
    };
    use pallet_nomination_pools::{
        BalanceOf, BondedPools, Config, Metadata, Pallet, PoolId, PoolMember, PoolMembers,
        ReversePoolIdLookup, RewardPool, RewardPools, SubPoolsStorage,
    };
    use sp_core::{Get, U256};
    use sp_std::{
        collections::{btree_map::BTreeMap, btree_set::BTreeSet},
        prelude::*,
    };

    use crate::{
        sp_api_hidden_includes_construct_runtime::hidden_include::dispatch::GetStorageVersion,
        Weight,
    };

    #[derive(Decode)]
    pub struct OldRewardPool<B> {
        pub balance: B,
        pub total_earnings: B,
        pub points: U256,
    }

    enum EitherRewardPool<T: Config, B> {
        Old(OldRewardPool<B>),
        New(RewardPool<T>),
    }

    impl<T: Config, B: Decode> Decode for EitherRewardPool<T, B> {
        fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
            let len = input.remaining_len()?.unwrap_or_default();
            let mut buffer = vec![0; len];
            input.read(&mut buffer)?;
            if let Ok(new) = OldRewardPool::<B>::decode_all(&mut buffer.clone().as_slice()) {
                return Ok(EitherRewardPool::Old(new));
            }

            RewardPool::<T>::decode_all(&mut buffer.as_slice())
                .map(|old| EitherRewardPool::New(old))
        }
    }

    fn dissolve_pool<T: Config>(id: PoolId) {
        let bonded_account = Pallet::<T>::create_bonded_account(id);
        ReversePoolIdLookup::<T>::remove(&bonded_account);
        SubPoolsStorage::<T>::remove(id);
        Metadata::<T>::remove(id);
        BondedPools::<T>::remove(id);
    }

    /// Delete pools, members and their bonded pool in the old scheme
    /// <https://github.com/paritytech/substrate/pull/11669.>.
    pub struct CustomMigrateToV2<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> CustomMigrateToV2<T> {
        fn run() -> Weight {
            let mut old_ids = BTreeSet::new();
            let mut members = BTreeMap::<PoolId, Vec<(T::AccountId, PoolMember<T>)>>::new();
            let mut members_deleted = 0;
            let members_read = PoolMembers::<T>::count();
            let mut reward_pools_deleted = 0;
            let pools_read = RewardPools::<T>::count();

            PoolMembers::<T>::translate::<PoolMember<T>, _>(|key, member: PoolMember<T>| {
                members
                    .entry(member.pool_id)
                    .or_default()
                    .push((key, member.clone()));
                Some(member)
            });

            // delete old pools
            RewardPools::<T>::translate::<EitherRewardPool<T, BalanceOf<T>>, _>(|key, either| {
                match either {
                    EitherRewardPool::Old(_) => {
                        old_ids.insert(key);
                        log::info!(target: "runtime::nomination-pools", "deleting pool with id {}", key);
                        for (account, member) in members.remove(&key).unwrap_or_default() {
                            // encode to be able see AccountId in the log
                            log::info!(target: "runtime::nomination-pools", "deleting member with id {:?}, Member points {:?}", account.encode(), member.points);
                            PoolMembers::<T>::remove(account);
                            members_deleted += 1;
                        }

                        dissolve_pool::<T>(key);
                        reward_pools_deleted += 1;
                        None
                    }
                    EitherRewardPool::New(new) => Some(new),
                }
            });

            log::info!(target: "runtime::nomination-pools", "deleted pools {:?}", old_ids);
            StorageVersion::new(2).put::<Pallet<T>>();

            T::DbWeight::get().reads_writes(
                members_read as u64 + pools_read as u64 + 1,
                members_deleted + 5 * reward_pools_deleted + 1, // times 5 because for each pool we remove 4 other associated entries.
            )
        }
    }

    impl<T: Config> OnRuntimeUpgrade for CustomMigrateToV2<T> {
        fn on_runtime_upgrade() -> Weight {
            let current = Pallet::<T>::current_storage_version();
            let onchain = Pallet::<T>::on_chain_storage_version();

            log::info!(target: "runtime::nomination-pools",
                "Running migration with current storage version {:?} / onchain {:?}",
                current,
                onchain
            );

            //on testnet we have storage_version set to 0
            if onchain == 0 {
                Self::run()
            } else {
                log::info!(target: "runtime::nomination-pools",
                    "MigrateToV2 did not executed. This probably should be removed"
                );
                T::DbWeight::get().reads(1)
            }
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_: Vec<u8>) -> Result<(), &'static str> {
            // new version must be set.
            assert_eq!(Pallet::<T>::on_chain_storage_version(), 2);

            // no reward or bonded pool has been skipped.
            assert_eq!(
                RewardPools::<T>::iter().count() as u32,
                RewardPools::<T>::count()
            );
            assert_eq!(
                BondedPools::<T>::iter().count() as u32,
                BondedPools::<T>::count()
            );
            assert_eq!(
                PoolMembers::<T>::iter().count() as u32,
                PoolMembers::<T>::count()
            );

            // all members belongs to existing pool
            for (_, member) in PoolMembers::<T>::iter() {
                assert!(RewardPools::<T>::contains_key(member.pool_id));
            }

            log::info!(target: "runtime::nomination-pools", "post upgrade hook for MigrateToV2 executed.");
            Ok(())
        }
    }
}
