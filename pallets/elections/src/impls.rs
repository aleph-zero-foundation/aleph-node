use frame_election_provider_support::sp_arithmetic::Perquintill;
use frame_support::pallet_prelude::Get;
use sp_staking::{EraIndex, SessionIndex};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    traits::{EraInfoProvider, SessionInfoProvider, ValidatorRewardsHandler},
    CommitteeSize, Config, CurrentEraValidators, EraValidators, NextEraNonReservedValidators,
    NextEraReservedValidators, Pallet, SessionValidatorBlockCount, ValidatorEraTotalReward,
    ValidatorTotalRewards,
};

const MAX_REWARD: u32 = 1_000_000_000;
pub const LENIENT_THRESHOLD: Perquintill = Perquintill::from_percent(90);

/// We assume that block `B` ends session nr `S`, and current era index is `E`.
///
/// 1. Block `B` initialized
/// 2. `end_session(S)` is called
/// -  We update rewards and clear block count for the session `S`.
/// 3. `start_session(S + 1)` is called.
/// -  if session `S+1` starts new era we populate totals.
/// 4. `new_session(S + 2)` is called.
/// -  If session `S+2` starts new era then we update the reserved and non_reserved validators.
/// -  We rotate the validators for session `S + 2` using the information about reserved and non_reserved validators.
///

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

pub fn compute_validator_scaled_total_rewards<V>(
    validator_totals: Vec<(V, u128)>,
) -> Vec<(V, u32)> {
    let sum_totals: u128 = validator_totals.iter().map(|(_, t)| t).sum();

    if sum_totals == 0 {
        return validator_totals.into_iter().map(|(v, _)| (v, 0)).collect();
    }

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
    current_session: SessionIndex,
    n_validators: usize,
    reserved: Vec<T>,
    non_reserved: Vec<T>,
) -> Option<Vec<T>> {
    if non_reserved.is_empty() {
        return Some(reserved);
    }

    // The validators for the committee at the session `n` are chosen as follow:
    // 1. Reserved validators are always chosen.
    // 2. Given non-reserved list of validators the chosen ones are from the range:
    // `n * free_seats` to `(n + 1) * free_seats` where free_seats is equal to free number of free
    // seats in the committee after reserved nodes are added.
    let free_seats = n_validators.saturating_sub(reserved.len());

    let non_reserved_len = non_reserved.len();
    let first_validator = (current_session as usize).saturating_mul(free_seats) % non_reserved_len;

    let committee = reserved
        .into_iter()
        .chain(
            (first_validator..first_validator + free_seats)
                .map(|i| non_reserved[i % non_reserved_len].clone()),
        )
        .collect();

    Some(committee)
}

impl<T> Pallet<T>
where
    T: Config,
{
    fn update_validator_total_rewards(era: EraIndex) {
        let validator_totals = T::ValidatorRewardsHandler::validator_totals(era);
        let scaled_totals = compute_validator_scaled_total_rewards(validator_totals).into_iter();

        ValidatorEraTotalReward::<T>::put(ValidatorTotalRewards(scaled_totals.collect()));
    }

    fn get_committee_and_non_committee() -> (Vec<T::AccountId>, Vec<T::AccountId>) {
        let committee = T::SessionInfoProvider::current_committee();
        let non_committee = CurrentEraValidators::<T>::get()
            .non_reserved
            .into_iter()
            .filter(|a| !committee.contains(a))
            .collect();

        (committee.into_iter().collect(), non_committee)
    }

    fn blocks_to_produce_per_session() -> u32 {
        T::SessionPeriod::get() / CommitteeSize::<T>::get()
    }

    fn reward_for_session_non_committee(
        non_committee: Vec<T::AccountId>,
        nr_of_sessions: SessionIndex,
        blocks_per_session: u32,
        validator_totals: &BTreeMap<T::AccountId, u32>,
    ) -> impl IntoIterator<Item = (T::AccountId, u32)> + '_ {
        non_committee.into_iter().map(move |validator| {
            let total = BTreeMap::<_, _>::get(validator_totals, &validator).unwrap_or(&0);
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
        validator_totals: &BTreeMap<T::AccountId, u32>,
    ) -> impl IntoIterator<Item = (T::AccountId, u32)> + '_ {
        committee.into_iter().map(move |validator| {
            let total = BTreeMap::<_, _>::get(validator_totals, &validator).unwrap_or(&0);
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
    fn rotate_committee(current_session: SessionIndex) -> Option<Vec<T::AccountId>> {
        if T::EraInfoProvider::active_era().unwrap_or(0) == 0 {
            return None;
        }

        let EraValidators {
            reserved,
            non_reserved,
        } = CurrentEraValidators::<T>::get();
        let n_validators = CommitteeSize::<T>::get() as usize;

        rotate(current_session, n_validators, reserved, non_reserved)
    }

    fn if_era_starts_do<F: Fn()>(era: EraIndex, start_index: SessionIndex, on_era_start: F) {
        if let Some(era_start_index) = T::EraInfoProvider::era_start_session_index(era) {
            if era_start_index == start_index {
                on_era_start()
            }
        }
    }

    fn populate_next_era_validators_on_next_era_start(session: SessionIndex) {
        let active_era = match T::EraInfoProvider::active_era() {
            Some(ae) => ae,
            _ => return,
        };

        // this will be populated once for the session `n+1` on the start of the session `n` where session
        // `n+1` starts a new era.
        Self::if_era_starts_do(active_era + 1, session, || {
            let reserved_validators = NextEraReservedValidators::<T>::get();
            let non_reserved_validators = NextEraNonReservedValidators::<T>::get();
            CurrentEraValidators::<T>::put(EraValidators {
                reserved: reserved_validators,
                non_reserved: non_reserved_validators,
            });
        });
    }

    fn populate_totals_on_new_era_start(session: SessionIndex) {
        let active_era = match T::EraInfoProvider::active_era() {
            Some(ae) => ae,
            _ => return,
        };

        Self::if_era_starts_do(active_era, session, || {
            Self::update_validator_total_rewards(active_era)
        });
    }

    fn adjust_rewards_for_session() {
        if T::EraInfoProvider::active_era().unwrap_or(0) == 0 {
            return;
        }

        let (committee, non_committee) = Self::get_committee_and_non_committee();
        let nr_of_sessions = T::EraInfoProvider::sessions_per_era();
        let blocks_per_session = Self::blocks_to_produce_per_session();
        let validator_total_rewards = ValidatorEraTotalReward::<T>::get()
            .unwrap_or_else(|| ValidatorTotalRewards(BTreeMap::new()))
            .0;

        let rewards = Self::reward_for_session_non_committee(
            non_committee,
            nr_of_sessions,
            blocks_per_session,
            &validator_total_rewards,
        )
        .into_iter()
        .chain(
            Self::reward_for_session_committee(
                committee,
                nr_of_sessions,
                blocks_per_session,
                &validator_total_rewards,
            )
            .into_iter(),
        );

        T::ValidatorRewardsHandler::add_rewards(rewards);
    }
}

impl<T> pallet_authorship::EventHandler<T::AccountId, T::BlockNumber> for Pallet<T>
where
    T: Config,
{
    fn note_author(validator: T::AccountId) {
        SessionValidatorBlockCount::<T>::mutate(&validator, |count| {
            *count += 1;
        });
    }

    fn note_uncle(_author: T::AccountId, _age: T::BlockNumber) {}
}

impl<T> pallet_session::SessionManager<T::AccountId> for Pallet<T>
where
    T: Config,
{
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        <T as Config>::SessionManager::new_session(new_index);
        // new session is always called before the end_session of the previous session
        // so we need to populate reserved set here not on start_session nor end_session
        Self::populate_next_era_validators_on_next_era_start(new_index);
        Self::rotate_committee(new_index)
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
        Self::populate_totals_on_new_era_start(start_index);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::impls::{
        calculate_adjusted_session_points, compute_validator_scaled_total_rewards, rotate,
        MAX_REWARD,
    };

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
            compute_validator_scaled_total_rewards(vec![(1, 10), (2, 10)])
        );
        assert_eq!(
            vec![(1, MAX_REWARD), (2, 0)],
            compute_validator_scaled_total_rewards(vec![(1, 10), (2, 0)])
        );
        assert_eq!(
            vec![
                (1, MAX_REWARD / 3),
                (2, MAX_REWARD / 6),
                (3, MAX_REWARD / 2),
            ],
            compute_validator_scaled_total_rewards(vec![(1, 20), (2, 10), (3, 30)])
        );
    }

    #[test]
    fn scale_points_correctly_when_above_u32() {
        let max: u128 = u32::MAX as u128;

        assert_eq!(
            vec![(1, MAX_REWARD / 2), (2, MAX_REWARD / 2)],
            compute_validator_scaled_total_rewards(vec![(1, 10 * max), (2, 10 * max)])
        );
        assert_eq!(
            vec![(1, MAX_REWARD), (2, 0)],
            compute_validator_scaled_total_rewards(vec![(1, 10 * max), (2, 0)])
        );
        assert_eq!(
            vec![
                (1, MAX_REWARD / 3),
                (2, MAX_REWARD / 6),
                (3, MAX_REWARD / 2),
            ],
            compute_validator_scaled_total_rewards(vec![
                (1, 20 * max),
                (2, 10 * max),
                (3, 30 * max)
            ])
        );
    }

    #[test]
    fn given_non_zero_era_and_prime_number_of_validators_when_rotating_committee_then_rotate_is_correct(
    ) {
        let reserved: Vec<_> = (0..11).collect();
        let non_reserved: Vec<_> = (11..101).collect();
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
                    session_index,
                    total_validators,
                    reserved.clone(),
                    non_reserved.clone(),
                )
                .expect("Expected non-empty rotated committee!")
            );
        }
    }
}
