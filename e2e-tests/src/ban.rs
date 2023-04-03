use aleph_client::{
    api::committee_management::events::BanValidators,
    pallets::{
        committee_management::CommitteeManagementApi,
        elections::{ElectionsApi, ElectionsSudoApi},
    },
    primitives::{BanConfig, BanInfo, CommitteeSeats, EraValidators},
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus, WaitingExt},
    AccountId, RootConnection, TxStatus,
};
use codec::Encode;
use log::info;
use primitives::{SessionCount, SessionIndex};
use sp_runtime::Perbill;

use crate::{
    accounts::account_ids_from_keys, config::Config, elections::get_members_subset_for_session,
    validators::get_test_validators,
};

const RESERVED_SEATS: u32 = 2;
const NON_RESERVED_SEATS: u32 = 2;

pub async fn setup_test(
    config: &Config,
) -> anyhow::Result<(
    RootConnection,
    Vec<AccountId>,
    Vec<AccountId>,
    CommitteeSeats,
)> {
    let root_connection = config.create_root_connection().await;

    let validator_keys = get_test_validators(config);
    let reserved_validators = account_ids_from_keys(&validator_keys.reserved);
    let non_reserved_validators = account_ids_from_keys(&validator_keys.non_reserved);

    let seats = CommitteeSeats {
        reserved_seats: RESERVED_SEATS,
        non_reserved_seats: NON_RESERVED_SEATS,
        non_reserved_finality_seats: NON_RESERVED_SEATS,
    };

    root_connection
        .change_validators(
            Some(reserved_validators.clone()),
            Some(non_reserved_validators.clone()),
            Some(seats.clone()),
            TxStatus::InBlock,
        )
        .await?;

    root_connection.wait_for_n_eras(2, BlockStatus::Best).await;

    Ok((
        root_connection,
        reserved_validators,
        non_reserved_validators,
        seats,
    ))
}

pub fn check_validators(
    expected_reserved: &[AccountId],
    expected_non_reserved: &[AccountId],
    era_validators: EraValidators<AccountId>,
) -> EraValidators<AccountId> {
    assert_eq!(era_validators.reserved, expected_reserved);
    assert_eq!(era_validators.non_reserved, expected_non_reserved);

    era_validators
}

pub async fn check_ban_config<C: CommitteeManagementApi>(
    connection: &C,
    expected_minimal_expected_performance: Perbill,
    expected_session_count_threshold: SessionCount,
    expected_clean_session_counter_delay: SessionCount,
) -> BanConfig {
    let ban_config = connection.get_ban_config(None).await;

    assert_eq!(
        ban_config.minimal_expected_performance.0,
        expected_minimal_expected_performance.deconstruct()
    );
    assert_eq!(
        ban_config.underperformed_session_count_threshold,
        expected_session_count_threshold
    );
    assert_eq!(
        ban_config.clean_session_counter_delay,
        expected_clean_session_counter_delay
    );

    ban_config
}

pub async fn check_underperformed_validator_session_count<C: CommitteeManagementApi>(
    connection: &C,
    validator: &AccountId,
    expected_session_count: SessionCount,
) -> SessionCount {
    let underperformed_validator_session_count = connection
        .get_underperformed_validator_session_count(validator.clone(), None)
        .await
        .unwrap_or_default();

    assert_eq!(
        underperformed_validator_session_count,
        expected_session_count
    );

    underperformed_validator_session_count
}

pub async fn check_underperformed_validator_reason<C: CommitteeManagementApi>(
    connection: &C,
    validator: &AccountId,
    expected_info: Option<&BanInfo>,
) -> Option<BanInfo> {
    let validator_ban_info = connection
        .get_ban_info_for_validator(validator.clone(), None)
        .await;

    assert_eq!(validator_ban_info.as_ref(), expected_info);
    validator_ban_info
}

pub async fn check_ban_info_for_validator<C: CommitteeManagementApi>(
    connection: &C,
    validator: &AccountId,
    expected_info: Option<&BanInfo>,
) -> Option<BanInfo> {
    let validator_ban_info = connection
        .get_ban_info_for_validator(validator.clone(), None)
        .await;

    assert_eq!(validator_ban_info.as_ref(), expected_info);

    validator_ban_info
}

pub async fn check_ban_event<C: AlephWaiting>(
    connection: &C,
    expected_banned_validators: &[(AccountId, BanInfo)],
) -> anyhow::Result<BanValidators> {
    let event = connection
        .wait_for_event(
            |event: &BanValidators| {
                info!("Received BanValidators event: {:?}", event.0);
                true
            },
            BlockStatus::Best,
        )
        .await;
    assert_eq!(event.0.encode(), expected_banned_validators.encode());

    Ok(event)
}

pub fn get_members_for_session(
    reserved_validators: &[AccountId],
    non_reserved_validators: &[AccountId],
    seats: &CommitteeSeats,
    session: SessionIndex,
) -> Vec<AccountId> {
    let reserved_members =
        get_members_subset_for_session(seats.reserved_seats, reserved_validators, session);
    let non_reserved_members =
        get_members_subset_for_session(seats.non_reserved_seats, non_reserved_validators, session);
    reserved_members
        .into_iter()
        .chain(non_reserved_members.into_iter())
        .collect()
}

/// Checks whether underperformed counts for validators change predictably. Assumes: (a) that the
/// sessions checked are in the past, (b) that all the checked validators are underperforming in
/// those sessions (e.g. due to a prohibitively high performance threshold).
pub async fn check_underperformed_count_for_sessions<
    C: ElectionsApi + CommitteeManagementApi + BlocksApi,
>(
    connection: &C,
    reserved_validators: &[AccountId],
    non_reserved_validators: &[AccountId],
    seats: &CommitteeSeats,
    start_session: SessionIndex,
    end_session: SessionIndex,
    ban_session_threshold: SessionCount,
) -> anyhow::Result<()> {
    let session_period = connection.get_session_period().await?;

    let validators: Vec<_> = reserved_validators
        .iter()
        .chain(non_reserved_validators.iter())
        .collect();

    for session in start_session..end_session {
        let session_end_block = (session + 1) * session_period;
        let session_end_block_hash = connection.get_block_hash(session_end_block).await?;

        let previous_session_end_block = session_end_block - session_period;
        let previous_session_end_block_hash = connection
            .get_block_hash(previous_session_end_block)
            .await?;

        let members =
            get_members_for_session(reserved_validators, non_reserved_validators, seats, session);

        for &val in validators.iter() {
            info!(
                "Checking session count | session {} | validator {}",
                session, val
            );
            let session_underperformed_count = connection
                .get_underperformed_validator_session_count(val.clone(), session_end_block_hash)
                .await
                .unwrap_or_default();
            let previous_session_underperformed_count = connection
                .get_underperformed_validator_session_count(
                    val.clone(),
                    previous_session_end_block_hash,
                )
                .await
                .unwrap_or_default();

            let underperformed_diff =
                session_underperformed_count.abs_diff(previous_session_underperformed_count);

            if members.contains(val) {
                // Counter for committee members legally incremented by 1 or reset to 0 (decremented
                // by ban_session_threshold - 1).
                if underperformed_diff != 1 && underperformed_diff != (ban_session_threshold - 1) {
                    panic!(
                        "Underperformed session count for committee validator {} for session {} changed from {} to {}.",
                        val, session, previous_session_underperformed_count, session_underperformed_count
                    );
                }
            } else if underperformed_diff != 0 {
                // Counter for validators on the bench should stay the same.
                panic!(
                    "Underperformed session count for non-committee validator {} for session {} changed from {} to {}.",
                    val, session, previous_session_underperformed_count, session_underperformed_count
                );
            }
        }
    }

    Ok(())
}
