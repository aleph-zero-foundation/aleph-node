use primitives::{CommitteeSeats, EraValidators};
use rand::{seq::SliceRandom, SeedableRng};
use rand_pcg::Pcg32;
use sp_staking::EraIndex;
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};

use crate::{
    traits::ValidatorProvider, CommitteeSize, Config, CurrentEraValidators, NextEraCommitteeSize,
    NextEraNonReservedValidators, NextEraReservedValidators, Pallet,
};

impl<T> Pallet<T>
where
    T: Config,
{
    fn populate_next_era_validators_on_next_era_start(era: EraIndex) {
        let mut rng = Pcg32::seed_from_u64(era as u64);
        let elected_committee = BTreeSet::from_iter(T::ValidatorProvider::elected_validators(era));

        let mut retain_shuffle_elected = |vals: Vec<T::AccountId>| -> Vec<T::AccountId> {
            let mut vals: Vec<_> = vals
                .into_iter()
                .filter(|v| elected_committee.contains(v))
                .collect();
            vals.shuffle(&mut rng);

            vals
        };

        let reserved_validators = NextEraReservedValidators::<T>::get();
        let non_reserved_validators = NextEraNonReservedValidators::<T>::get();
        let committee_size = NextEraCommitteeSize::<T>::get();

        CurrentEraValidators::<T>::put(EraValidators {
            reserved: retain_shuffle_elected(reserved_validators),
            non_reserved: retain_shuffle_elected(non_reserved_validators),
        });
        CommitteeSize::<T>::put(committee_size);
    }
}

impl<T: Config> primitives::EraManager for Pallet<T> {
    fn on_new_era(era: EraIndex) {
        Self::populate_next_era_validators_on_next_era_start(era);
    }
}

impl<T: Config> primitives::BanHandler for Pallet<T> {
    type AccountId = T::AccountId;
    fn can_ban(account_id: &Self::AccountId) -> bool {
        !NextEraReservedValidators::<T>::get().contains(account_id)
    }
}

impl<T: Config + pallet_staking::Config> primitives::ValidatorProvider for Pallet<T> {
    type AccountId = T::AccountId;
    fn current_era_validators() -> EraValidators<Self::AccountId> {
        CurrentEraValidators::<T>::get()
    }
    fn current_era_committee_size() -> CommitteeSeats {
        CommitteeSize::<T>::get()
    }
}
