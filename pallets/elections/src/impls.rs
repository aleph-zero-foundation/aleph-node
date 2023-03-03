use frame_election_provider_support::sp_arithmetic::Perquintill;
use frame_support::{
    log::{debug, info},
    pallet_prelude::Get,
};
use primitives::{BanConfig as BanConfigStruct, BanInfo, BanReason, CommitteeSeats, EraValidators};
use sp_runtime::Perbill;
use sp_staking::{EraIndex, SessionIndex};
use sp_std::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    vec::Vec,
};

use crate::{
    traits::{EraInfoProvider, SessionInfoProvider, ValidatorExtractor, ValidatorRewardsHandler},
    BanConfig, Banned, CommitteeSize, Config, CurrentEraValidators, NextEraCommitteeSize,
    NextEraNonReservedValidators, NextEraReservedValidators, Pallet, SessionValidatorBlockCount,
    UnderperformedValidatorSessionCount, ValidatorEraTotalReward, ValidatorTotalRewards,
};

const MAX_REWARD: u32 = 1_000_000_000;
pub const LENIENT_THRESHOLD: Perquintill = Perquintill::from_percent(90);

/// We assume that block `B` ends session nr `S`, and current era index is `E`.
///
/// 1. Block `B` initialized
/// 2. `end_session(S)` is called
/// *  Based on block count we might mark the session for a given validator as underperformed
/// *  We update rewards and clear block count for the session `S`.
/// 3. `start_session(S + 1)` is called.
/// *  if session `S+1` starts new era we populate totals and unban all validators whose ban expired.
/// *  if session `S+1` % [`BanConfig::clean_session_counter_delay`] == 0, we
///    clean up underperformed session counter
/// 4. `new_session(S + 2)` is called.
/// *  If session `S+2` starts new era:
///    * during elections, we choose validators eligible for elections depending on the openness of the process
///       * `permsionless`: all validators that bonded sufficient amount are chosen
///       * `permissioned`: we choose only validators from allow lists
///    * in both cases, we exclude banned validators from the elections
///    * then we update the reserved and non reserved validators.
/// *  We rotate the validators for session `S + 2` using the information about reserved and non reserved validators.
///

fn calculate_adjusted_session_points(
    sessions_per_era: EraIndex,
    blocks_to_produce_per_session: u32,
    blocks_created: u32,
    total_possible_reward: u32,
) -> u32 {
    let performance =
        Perquintill::from_rational(blocks_created as u64, blocks_to_produce_per_session as u64);

    // when produced more than 90% expected blocks, get 100% possible reward for session
    if performance >= LENIENT_THRESHOLD {
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

fn choose_for_session<T: Clone>(
    validators: Vec<T>,
    count: usize,
    session: usize,
) -> Option<Vec<T>> {
    if validators.is_empty() || count == 0 {
        return None;
    }

    let validators_len = validators.len();
    let first_index = session.saturating_mul(count) % validators_len;
    let mut chosen = Vec::new();

    for i in 0..count.min(validators_len) {
        chosen.push(validators[first_index.saturating_add(i) % validators_len].clone());
    }

    Some(chosen)
}

fn rotate<T: Clone + PartialEq>(
    current_session: SessionIndex,
    reserved_seats: usize,
    non_reserved_seats: usize,
    reserved: Vec<T>,
    non_reserved: Vec<T>,
) -> Option<Vec<T>> {
    // The validators for the committee at the session `n` are chosen as follow:
    // 1. `reserved_seats` validators are chosen from the reserved set while `non_reserved_seats` from the non_reserved set.
    // 2. Given a set of validators the chosen ones are from the range:
    // `n * seats` to `(n + 1) * seats` where seats is equal to reserved_seats(non_reserved_seats) for reserved(non_reserved) validators.

    let reserved_committee = choose_for_session(reserved, reserved_seats, current_session as usize);
    let non_reserved_committee =
        choose_for_session(non_reserved, non_reserved_seats, current_session as usize);

    match (reserved_committee, non_reserved_committee) {
        (Some(rc), Some(nrc)) => Some(rc.into_iter().chain(nrc.into_iter()).collect()),
        (Some(rc), _) => Some(rc),
        (_, Some(nrc)) => Some(nrc),
        _ => None,
    }
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
        let EraValidators {
            reserved,
            non_reserved,
        } = CurrentEraValidators::<T>::get();

        let non_committee = non_reserved
            .into_iter()
            .chain(reserved.into_iter())
            .filter(|a| !committee.contains(a))
            .collect();

        (committee.into_iter().collect(), non_committee)
    }

    fn blocks_to_produce_per_session() -> u32 {
        T::SessionPeriod::get().saturating_div(CommitteeSize::<T>::get().size())
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
        let CommitteeSeats {
            reserved_seats,
            non_reserved_seats,
        } = CommitteeSize::<T>::get();

        rotate(
            current_session,
            reserved_seats as usize,
            non_reserved_seats as usize,
            reserved,
            non_reserved,
        )
    }

    pub fn ban_expired(start: EraIndex, period: EraIndex, active_era: EraIndex) -> bool {
        start + period <= active_era
    }

    fn if_era_starts_do<F: Fn()>(era: EraIndex, start_index: SessionIndex, on_era_start: F) {
        if let Some(era_start_index) = T::EraInfoProvider::era_start_session_index(era) {
            if era_start_index == start_index {
                on_era_start()
            }
        }
    }

    fn clear_expired_bans_on_new_era_start(session: SessionIndex) {
        let active_era = match T::EraInfoProvider::active_era() {
            Some(ae) => ae,
            _ => return,
        };

        Self::if_era_starts_do(active_era, session, || {
            let ban_period = BanConfig::<T>::get().ban_period;
            let unban = Banned::<T>::iter().filter_map(|(v, ban_info)| {
                if Self::ban_expired(ban_info.start, ban_period, active_era) {
                    return Some(v);
                }
                None
            });
            unban.for_each(Banned::<T>::remove);
        });
    }

    fn populate_next_era_validators_on_next_era_start(session: SessionIndex) {
        let active_era = match T::EraInfoProvider::active_era() {
            Some(ae) => ae,
            _ => return,
        };

        // this will be populated once for the session `n+1` on the start of the session `n` where session
        // `n+1` starts a new era.
        Self::if_era_starts_do(active_era + 1, session, || {
            let elected_committee =
                BTreeSet::from_iter(T::EraInfoProvider::elected_validators(active_era + 1));

            let retain_elected = |vals: Vec<T::AccountId>| -> Vec<T::AccountId> {
                vals.into_iter()
                    .filter(|v| elected_committee.contains(v))
                    .collect()
            };

            let reserved_validators = NextEraReservedValidators::<T>::get();
            let non_reserved_validators = NextEraNonReservedValidators::<T>::get();
            let committee_size = NextEraCommitteeSize::<T>::get();

            CurrentEraValidators::<T>::put(EraValidators {
                reserved: retain_elected(reserved_validators),
                non_reserved: retain_elected(non_reserved_validators),
            });
            CommitteeSize::<T>::put(committee_size);
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

    fn calculate_underperforming_validators() {
        let thresholds = BanConfig::<T>::get();
        let current_committee = T::SessionInfoProvider::current_committee();
        let expected_blocks_per_validator = Self::blocks_to_produce_per_session();
        for validator in current_committee {
            let underperformance = match SessionValidatorBlockCount::<T>::try_get(&validator) {
                Ok(block_count) => {
                    Perbill::from_rational(block_count, expected_blocks_per_validator)
                        <= thresholds.minimal_expected_performance
                }
                Err(_) => true,
            };
            if underperformance {
                Self::mark_validator_underperformance(&thresholds, &validator);
            }
        }
    }

    pub fn ban_validator(validator: &T::AccountId, reason: BanReason) {
        // we do not ban reserved validators
        if NextEraReservedValidators::<T>::get().contains(validator) {
            return;
        }
        // current era is the latest planned era for which validators are already chosen
        // so we ban from the next era
        let start: EraIndex = T::EraInfoProvider::current_era()
            .unwrap_or(0)
            .saturating_add(1);
        T::ValidatorExtractor::remove_validator(validator);
        Banned::<T>::insert(validator, BanInfo { reason, start });
    }

    fn mark_validator_underperformance(thresholds: &BanConfigStruct, validator: &T::AccountId) {
        let counter = UnderperformedValidatorSessionCount::<T>::mutate(validator, |count| {
            *count += 1;
            *count
        });
        if counter >= thresholds.underperformed_session_count_threshold {
            let reason = BanReason::InsufficientUptime(counter);
            Self::ban_validator(validator, reason);
            UnderperformedValidatorSessionCount::<T>::remove(validator);
        }
    }

    fn clear_underperformance_session_counter(session: SessionIndex) {
        let clean_session_counter_delay = BanConfig::<T>::get().clean_session_counter_delay;
        if session % clean_session_counter_delay == 0 {
            info!(target: "pallet_elections", "Clearing UnderperformedValidatorSessionCount");
            let _result = UnderperformedValidatorSessionCount::<T>::clear(u32::MAX, None);
        }
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
        Self::calculate_underperforming_validators();
        // clear block count after calculating stats for underperforming validators, as they use
        // SessionValidatorBlockCount for that
        let result = SessionValidatorBlockCount::<T>::clear(u32::MAX, None);
        debug!(target: "pallet_elections", "Result of clearing the `SessionValidatorBlockCount`, {:?}", result.deconstruct());
    }

    fn start_session(start_index: SessionIndex) {
        <T as Config>::SessionManager::start_session(start_index);
        Self::populate_totals_on_new_era_start(start_index);
        Self::clear_underperformance_session_counter(start_index);
        Self::clear_expired_bans_on_new_era_start(start_index);
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
            5000,
            calculate_adjusted_session_points(5, 30, 2 * 30, 25_000)
        );

        assert_eq!(
            6250000,
            calculate_adjusted_session_points(96, 900, 3 * 900, 600_000_000)
        );

        assert_eq!(
            6145833,
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
                (3, 30 * max),
            ])
        );
    }

    #[test]
    fn given_non_zero_era_and_prime_number_of_validators_when_rotating_committee_then_rotate_is_correct(
    ) {
        let reserved: Vec<_> = (0..11).collect();
        let non_reserved: Vec<_> = (11..101).collect();
        let reserved_seats = 7;
        let non_reserved_seats = 13;
        let mut rotated_non_reserved_validators: VecDeque<_> = (11..101).collect();
        let mut rotated_reserved_validators: VecDeque<_> = (0..11).collect();

        for session_index in 0u32..100u32 {
            let mut expected_committee = vec![];
            for _ in 0..reserved_seats {
                let first = rotated_reserved_validators.pop_front().unwrap();
                expected_committee.push(first);
                rotated_reserved_validators.push_back(first);
            }
            for _ in 0..non_reserved_seats {
                let first = rotated_non_reserved_validators.pop_front().unwrap();
                expected_committee.push(first);
                rotated_non_reserved_validators.push_back(first);
            }

            assert_eq!(
                expected_committee,
                rotate(
                    session_index,
                    reserved_seats,
                    non_reserved_seats,
                    reserved.clone(),
                    non_reserved.clone(),
                )
                .expect("Expected non-empty rotated committee!")
            );
        }
    }
}
