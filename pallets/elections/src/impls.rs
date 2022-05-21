use crate::{Config, ErasReserved, MembersPerSession, Pallet, SessionValidatorBlockCount};
use frame_election_provider_support::sp_arithmetic::Perquintill;
use frame_support::{pallet_prelude::Get, traits::Currency};
use sp_staking::{EraIndex, SessionIndex};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

const MAX_REWARD: u32 = 1_000_000_000;
const LENIENT_THRESHOLD: Perquintill = Perquintill::from_percent(90);

fn calculate_adjusted_session_points(
    sessions_per_era: EraIndex,
    blocks_to_produce_per_session: u32,
    blocks_created: u32,
    total_possible_reward: u32,
) -> u32 {
    let performance =
        Perquintill::from_rational(blocks_created as u64, blocks_to_produce_per_session as u64);

    // when produced between 90% to 100% expected blocks get 100% possible reward for session
    if performance >= LENIENT_THRESHOLD && blocks_to_produce_per_session >= blocks_created {
        return (Perquintill::from_rational(1, sessions_per_era as u64)
            * total_possible_reward as u64) as u32;
    }

    (Perquintill::from_rational(
        blocks_created as u64,
        (blocks_to_produce_per_session * sessions_per_era) as u64,
    ) * total_possible_reward as u64) as u32
}

fn compute_validator_scaled_totals<V>(validator_totals: Vec<(V, u128)>) -> Vec<(V, u32)> {
    let sum_totals: u128 = validator_totals.iter().map(|(_, t)| t).sum();

    // scaled_total = total * (MAX_REWARD / sum_totals)
    // for maximum possible value of the total sum_totals the scaled_total is equal to MAX_REWARD
    validator_totals
        .into_iter()
        .map(|(v, t)| {
            (
                v,
                (t.saturating_mul(MAX_REWARD as u128) / sum_totals) as u32,
            )
        })
        .collect()
}

fn rotate<T: Clone + PartialEq>(
    current_era: EraIndex,
    current_session: SessionIndex,
    n_validators: usize,
    all_validators: Vec<T>,
    reserved: Vec<T>,
) -> Option<Vec<T>> {
    if current_era == 0 {
        return None;
    }

    let validators_without_reserved: Vec<_> = all_validators
        .into_iter()
        .filter(|v| !reserved.contains(v))
        .collect();
    let n_all_validators_without_reserved = validators_without_reserved.len();

    // The validators for the committee at the session `n` are chosen as follow:
    // 1. Reserved validators are always chosen.
    // 2. Given non-reserved list of validators the chosen ones are from the range:
    // `n * free_seats` to `(n + 1) * free_seats` where free_seats is equal to free number of free
    // seats in the committee after reserved nodes are added.
    let free_seats = n_validators.saturating_sub(reserved.len());
    let first_validator = current_session as usize * free_seats;

    let committee =
        reserved
            .into_iter()
            .chain((first_validator..first_validator + free_seats).map(|i| {
                validators_without_reserved[i % n_all_validators_without_reserved].clone()
            }))
            .collect();

    Some(committee)
}

impl<T> Pallet<T>
    where
        T: Config + pallet_session::Config + pallet_staking::Config,
        <<T as pallet_staking::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance: Into<u128>,
        <T as pallet_session::Config>::ValidatorId: From<T::AccountId>,
        <T as pallet_session::Config>::ValidatorId: Into<T::AccountId>,
{
    fn compute_validator_scaled_totals(era: EraIndex) -> BTreeMap<T::AccountId, u32> {
        let validator_totals = pallet_staking::ErasStakers::<T>::iter_prefix(era)
            .map(|(validator, exposure)| (validator, exposure.total.into()))
            .collect();

        compute_validator_scaled_totals(validator_totals).into_iter().collect()
    }

    fn get_committee_and_non_committee(current_era: EraIndex) -> (Vec<T::AccountId>, Vec<T::AccountId>) {
        let committee: Vec<T::AccountId> = pallet_session::Validators::<T>::get().into_iter().map(|a| a.into()).collect();
        let non_committee = pallet_staking::ErasStakers::<T>::iter_key_prefix(current_era)
            .filter(|a| !committee.contains(a))
            .collect();

        (committee, non_committee)
    }

    fn blocks_to_produce_per_session() -> u32 {
        T::SessionPeriod::get() / MembersPerSession::<T>::get()
    }

    fn reward_for_session_non_committee(
        non_committee: Vec<T::AccountId>,
        nr_of_sessions: SessionIndex,
        blocks_per_session: u32,
        validator_totals: &BTreeMap<T::AccountId, u32>
    ) -> impl IntoIterator<Item=(T::AccountId, u32)> + '_ {
        non_committee.into_iter().map(move |validator| {
            let total = BTreeMap::<_,_>::get(validator_totals,&validator).unwrap_or(&0);
            (
                validator,
                calculate_adjusted_session_points(
                    nr_of_sessions,
                    blocks_per_session,
                    blocks_per_session,
                    *total,
                ),
            )
        })
    }

    fn reward_for_session_committee(
        committee: Vec<T::AccountId>,
        nr_of_sessions: SessionIndex,
        blocks_per_session: u32,
        validator_totals: &BTreeMap<T::AccountId, u32>
    ) -> impl IntoIterator<Item=(T::AccountId, u32)> + '_ {
        committee.into_iter().map(move |validator| {
            let total = BTreeMap::<_,_>::get(validator_totals,&validator).unwrap_or(&0);
            let blocks_created = SessionValidatorBlockCount::<T>::get(&validator);
            (
                validator,
                calculate_adjusted_session_points(
                    nr_of_sessions,
                    blocks_per_session,
                    blocks_created,
                    *total,
                ),
            )
        })
    }

    // Choose a subset of all the validators for current era that contains all the
    // reserved nodes. Non reserved ones are chosen in consecutive batches for every session
    fn rotate_committee() -> Option<Vec<T::AccountId>> {
        let current_era = match pallet_staking::ActiveEra::<T>::get() {
            Some(ae) if ae.index > 0 => ae.index,
            _ => return None,
        };
        let all_validators: Vec<T::AccountId> =
            pallet_staking::ErasStakers::<T>::iter_key_prefix(current_era).collect();
        let reserved = ErasReserved::<T>::get();
        let n_validators = MembersPerSession::<T>::get() as usize;
        let current_session = pallet_session::Pallet::<T>::current_index();

        rotate(
            current_era,
            current_session,
            n_validators,
            all_validators,
            reserved,
        )
    }

    fn populate_reserved_on_next_era_start(start_index: SessionIndex) {
        let current_era = match pallet_staking::ActiveEra::<T>::get() {
            Some(ae) => ae.index,
            _ => return,
        };
        // this will be populated once for the session `n+1` on the start of the session `n` where session
        // `n+1` starts a new era.
        if let Some(era_index) = pallet_staking::ErasStartSessionIndex::<T>::get(current_era + 1) {
            if era_index == start_index {
                let reserved_validators = pallet_staking::Invulnerables::<T>::get();
                ErasReserved::<T>::put(reserved_validators);
            }
        }
    }

    fn adjust_rewards_for_session() {
        let active_era = match pallet_staking::ActiveEra::<T>::get() {
            Some(ae) if ae.index > 0 => ae.index,
            _ => return,
        };

        let (committee, non_committee) = Self::get_committee_and_non_committee(active_era);
        let nr_of_sessions = T::SessionsPerEra::get();
        let blocks_per_session = Self::blocks_to_produce_per_session();
        let validator_totals = Self::compute_validator_scaled_totals(active_era);

        let rewards =
            Self::reward_for_session_non_committee(non_committee, nr_of_sessions, blocks_per_session, &validator_totals)
                .into_iter()
                .chain(Self::reward_for_session_committee(committee, nr_of_sessions, blocks_per_session, &validator_totals).into_iter());

        pallet_staking::Pallet::<T>::reward_by_ids(rewards);
    }
}

impl<T> pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Pallet<T>
    where
        T: Config + pallet_session::Config + pallet_staking::Config,
        <<T as pallet_staking::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance: Into<u128>,
        <T as pallet_session::Config>::ValidatorId: From<T::AccountId>,
        <T as pallet_session::Config>::ValidatorId: Into<T::AccountId>, {
    fn note_author(validator: T::AccountId) {
        SessionValidatorBlockCount::<T>::mutate(&validator, |count| {
            *count += 1;
        });
    }

    fn note_uncle(_author: T::AccountId, _age: T::BlockNumber) {}
}

impl<T> pallet_session::SessionManager<T::AccountId> for Pallet<T>
    where
        T: Config + pallet_session::Config + pallet_staking::Config,
        <<T as pallet_staking::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance: Into<u128>,
        <T as pallet_session::Config>::ValidatorId: From<T::AccountId>,
        <T as pallet_session::Config>::ValidatorId: Into<T::AccountId>, {
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        <T as Config>::SessionManager::new_session(new_index);
        // new session is always called before the end_session of the previous session
        // so we need to populate reserved set here not on start_session nor end_session
        let committee = Self::rotate_committee();
        Self::populate_reserved_on_next_era_start(new_index);

        committee
    }

    fn new_session_genesis(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        <T as Config>::SessionManager::new_session_genesis(new_index)
    }

    fn end_session(end_index: SessionIndex) {
        <T as Config>::SessionManager::end_session(end_index);
        Self::adjust_rewards_for_session();

        // clear block count
        SessionValidatorBlockCount::<T>::remove_all(None);
    }

    fn start_session(start_index: SessionIndex) {
        <T as Config>::SessionManager::start_session(start_index);
    }
}

#[cfg(test)]
mod tests {
    use crate::impls::{
        calculate_adjusted_session_points, compute_validator_scaled_totals, rotate, MAX_REWARD,
    };
    use std::collections::VecDeque;

    #[test]
    fn given_era_zero_when_rotating_committee_then_committee_is_empty() {
        assert_eq!(None, rotate(0, 0, 4, (0..10).collect(), vec![1, 2, 3, 4]));
    }

    #[test]
    fn adjusted_session_points_all_blocks_created_are_calculated_correctly() {
        assert_eq!(5000, calculate_adjusted_session_points(5, 30, 30, 25_000));

        assert_eq!(
            6250000,
            calculate_adjusted_session_points(96, 900, 900, 600_000_000)
        );

        assert_eq!(
            6145833,
            calculate_adjusted_session_points(96, 900, 900, 590_000_000)
        );
    }

    #[test]
    fn adjusted_session_points_above_90_perc_are_calculated_correctly() {
        assert_eq!(5000, calculate_adjusted_session_points(5, 30, 27, 25_000));

        assert_eq!(
            6250000,
            calculate_adjusted_session_points(96, 900, 811, 600_000_000)
        );

        assert_eq!(
            6145833,
            calculate_adjusted_session_points(96, 900, 899, 590_000_000)
        );
    }

    #[test]
    fn adjusted_session_points_more_than_all_blocks_created_are_calculated_correctly() {
        assert_eq!(
            2 * 5000,
            calculate_adjusted_session_points(5, 30, 2 * 30, 25_000)
        );

        assert_eq!(
            3 * 6250000,
            calculate_adjusted_session_points(96, 900, 3 * 900, 600_000_000)
        );

        assert_eq!(
            6152662,
            calculate_adjusted_session_points(96, 900, 901, 590_000_000)
        );
    }

    #[test]
    fn scale_points_correctly_when_under_u32() {
        assert_eq!(
            vec![(1, MAX_REWARD / 2), (2, MAX_REWARD / 2)],
            compute_validator_scaled_totals(vec![(1, 10), (2, 10)])
        );
        assert_eq!(
            vec![(1, MAX_REWARD), (2, 0)],
            compute_validator_scaled_totals(vec![(1, 10), (2, 0)])
        );
        assert_eq!(
            vec![
                (1, MAX_REWARD / 3),
                (2, MAX_REWARD / 6),
                (3, MAX_REWARD / 2)
            ],
            compute_validator_scaled_totals(vec![(1, 20), (2, 10), (3, 30)])
        );
    }

    #[test]
    fn scale_points_correctly_when_above_u32() {
        let max: u128 = u32::MAX as u128;

        assert_eq!(
            vec![(1, MAX_REWARD / 2), (2, MAX_REWARD / 2)],
            compute_validator_scaled_totals(vec![(1, 10 * max), (2, 10 * max)])
        );
        assert_eq!(
            vec![(1, MAX_REWARD), (2, 0)],
            compute_validator_scaled_totals(vec![(1, 10 * max), (2, 0)])
        );
        assert_eq!(
            vec![
                (1, MAX_REWARD / 3),
                (2, MAX_REWARD / 6),
                (3, MAX_REWARD / 2)
            ],
            compute_validator_scaled_totals(vec![(1, 20 * max), (2, 10 * max), (3, 30 * max)])
        );
    }

    #[test]
    fn given_non_zero_era_and_prime_number_of_validators_when_rotating_committee_then_rotate_is_correct(
    ) {
        let all_validators: Vec<_> = (0..101).collect();
        let reserved: Vec<_> = (0..11).collect();
        let total_validators = 53;
        let mut rotated_free_seats_validators: VecDeque<_> = (11..101).collect();

        for session_index in 0u32..100u32 {
            let mut expected_rotated_free_seats = vec![];
            for _ in 0..total_validators - reserved.len() {
                let first = rotated_free_seats_validators.pop_front().unwrap();
                expected_rotated_free_seats.push(first);
                rotated_free_seats_validators.push_back(first);
            }
            let mut expected_rotated_committee = reserved.clone();
            expected_rotated_committee.append(&mut expected_rotated_free_seats);
            assert_eq!(
                expected_rotated_committee,
                rotate(
                    1,
                    session_index,
                    total_validators,
                    all_validators.clone(),
                    reserved.clone(),
                )
                .expect("Expected non-empty rotated committee!")
            );
        }
    }
}
