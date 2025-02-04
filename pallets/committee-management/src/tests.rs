use std::collections::BTreeSet;

use pallet_aleph::AbftScores;
use primitives::{BanInfo, BannedValidators, Score};

use crate::{
    mock::{
        active_era, advance_era, committee_management_events, start_session, AccountId,
        CommitteeManagement, Elections, SessionPeriod, TestBuilderConfig, TestExtBuilder,
        TestRuntime,
    },
    CurrentAndNextSessionValidatorsStorage, Event, FinalityBanConfig, ProductionBanConfig,
    SessionValidatorBlockCount,
};

fn gen_config() -> TestBuilderConfig {
    TestBuilderConfig {
        reserved_validators: (0..10).collect(),
        non_reserved_validators: (10..100).collect(),
        non_reserved_seats: 50,
        non_reserved_finality_seats: 4,
    }
}

fn add_underperformer(
    validators: &mut Vec<AccountId>,
    underperformer: AccountId,
    reserved: &BTreeSet<AccountId>,
) -> Vec<AccountId> {
    if !validators.contains(&underperformer) {
        validators.retain(|p| !reserved.contains(p));
        validators.pop();
        validators.extend(reserved.iter());
        validators.push(underperformer);
    }

    validators.clone()
}

#[test]
fn new_poducers_every_session() {
    TestExtBuilder::new(gen_config()).build().execute_with(|| {
        let mut producers_in_all_sessions = BTreeSet::<BTreeSet<AccountId>>::new();
        for session_index in 2..=6 {
            start_session(session_index);
            let producers = CommitteeManagement::current_session_validators()
                .current
                .producers
                .into_iter()
                .collect();

            assert!(producers_in_all_sessions.insert(producers));
        }
    })
}

#[test]
fn new_finalizers_every_session() {
    TestExtBuilder::new(gen_config()).build().execute_with(|| {
        let mut finalizers_in_all_sessions = BTreeSet::<BTreeSet<AccountId>>::new();
        for session_index in 2..=6 {
            start_session(session_index);
            let finalizers = CommitteeManagement::current_session_validators()
                .current
                .finalizers
                .into_iter()
                .collect();
            assert!(finalizers_in_all_sessions.insert(finalizers));
        }
    })
}

#[test]
fn all_reserved_validators_are_chosen() {
    TestExtBuilder::new(gen_config()).build().execute_with(|| {
        let reserved = Elections::current_era_validators().reserved;
        start_session(2);
        let producers: BTreeSet<AccountId> = CommitteeManagement::current_session_validators()
            .current
            .producers
            .into_iter()
            .collect();
        assert!(reserved.iter().all(|rv| producers.contains(rv)));
        let finalizers: BTreeSet<AccountId> = CommitteeManagement::current_session_validators()
            .current
            .finalizers
            .into_iter()
            .collect();
        assert!(reserved.iter().all(|rv| finalizers.contains(rv)));
    })
}

#[test]
fn ban_underperforming_producers() {
    TestExtBuilder::new(gen_config()).build().execute_with(|| {
        let underperformer = 10;
        let mut ban_config = CommitteeManagement::production_ban_config();
        let underperformed_session_count_threshold =
            ban_config.underperformed_session_count_threshold;
        let reserved: BTreeSet<AccountId> = Elections::current_era_validators()
            .reserved
            .into_iter()
            .collect();
        let blocks_to_produce_per_session = SessionPeriod::get();
        let mut underperf_count = 0;
        let mut session_index = 2;
        loop {
            start_session(session_index);
            if underperf_count == underperformed_session_count_threshold {
                break;
            }
            assert_eq!(CommitteeManagement::banned(), Vec::<AccountId>::new());

            // Make sure underperformer is a producer in every session.
            let producers = CurrentAndNextSessionValidatorsStorage::<TestRuntime>::mutate(|sv| {
                add_underperformer(&mut sv.current.producers, underperformer, &reserved)
            });
            for producer in producers.iter() {
                SessionValidatorBlockCount::<TestRuntime>::insert(
                    producer,
                    blocks_to_produce_per_session,
                );
            }
            // Make sure underperformer underperforms.
            SessionValidatorBlockCount::<TestRuntime>::insert(underperformer, 0);
            underperf_count += 1;
            session_index += 1;
        }

        let banned = vec![underperformer];
        assert_eq!(CommitteeManagement::banned(), banned);
        let ban_info = BanInfo {
            reason: primitives::BanReason::InsufficientProduction(
                underperformed_session_count_threshold,
            ),
            start: active_era() + 1,
        };

        // Make sure there are no more bans.
        ban_config.clean_session_counter_delay = 1;
        let ban_period = 2;
        ban_config.ban_period = ban_period;
        ProductionBanConfig::<TestRuntime>::put(ban_config);
        advance_era();

        let banned_info = vec![(underperformer, ban_info)];
        assert_eq!(
            *committee_management_events().last().unwrap(),
            Event::BanValidators(banned_info)
        );
        assert_eq!(CommitteeManagement::banned(), banned);
        advance_era();
        assert_eq!(CommitteeManagement::banned(), Vec::<AccountId>::new());
    })
}

#[test]
fn ban_underperforming_finalizers() {
    TestExtBuilder::new(gen_config()).build().execute_with(|| {
        let underperformer = 10;
        let mut ban_config = CommitteeManagement::finality_ban_config();
        let minimal_expected_performance = ban_config.minimal_expected_performance;
        let underperformed_session_count_threshold = 2;
        ban_config.underperformed_session_count_threshold = underperformed_session_count_threshold;
        FinalityBanConfig::<TestRuntime>::put(ban_config.clone());
        let reserved: BTreeSet<AccountId> = Elections::current_era_validators()
            .reserved
            .into_iter()
            .collect();
        let mut underperf_count = 0;
        let mut session_index = 2;
        loop {
            start_session(session_index);
            if underperf_count == underperformed_session_count_threshold {
                break;
            }
            assert_eq!(CommitteeManagement::banned(), Vec::<AccountId>::new());

            // Make sure underperformer is a finalizer in every session.
            let finalizers = CurrentAndNextSessionValidatorsStorage::<TestRuntime>::mutate(|sv| {
                add_underperformer(&mut sv.current.finalizers, underperformer, &reserved)
            });

            let mut points: Vec<u16> = finalizers
                .iter()
                .map(|_| minimal_expected_performance)
                .collect();
            // Make sure underperformer underperforms.
            *points.last_mut().unwrap() = u16::MAX;
            let score = Score {
                session_id: session_index,
                nonce: 1,
                points,
            };
            AbftScores::<TestRuntime>::insert(session_index, score);
            underperf_count += 1;
            session_index += 1;
        }

        let banned = vec![underperformer];
        assert_eq!(CommitteeManagement::banned(), banned);
        let ban_info = BanInfo {
            reason: primitives::BanReason::InsufficientFinalization(
                underperformed_session_count_threshold,
            ),
            start: active_era() + 1,
        };

        // Make sure there are no more bans.
        ban_config.clean_session_counter_delay = 1;
        let ban_period = 2;
        ban_config.ban_period = ban_period;
        FinalityBanConfig::<TestRuntime>::put(ban_config);
        let mut ban_config = CommitteeManagement::production_ban_config();
        ban_config.clean_session_counter_delay = 1;
        ban_config.ban_period = ban_period;
        ProductionBanConfig::<TestRuntime>::put(ban_config);
        advance_era();

        let banned_info = vec![(underperformer, ban_info)];
        assert_eq!(
            *committee_management_events().last().unwrap(),
            Event::BanValidators(banned_info)
        );
        assert_eq!(CommitteeManagement::banned(), banned);
        advance_era();
        assert_eq!(CommitteeManagement::banned(), Vec::<AccountId>::new());
    })
}
