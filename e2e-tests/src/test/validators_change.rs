use aleph_client::{
    api::elections::events::ChangeValidators,
    pallets::elections::{ElectionsApi, ElectionsSudoApi},
    primitives::CommitteeSeats,
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus},
    AccountId, Pair, TxStatus,
};
use anyhow::anyhow;
use log::info;

use crate::{accounts::get_validators_keys, config::setup_test};

#[tokio::test]
pub async fn change_validators() -> anyhow::Result<()> {
    let config = setup_test();

    let accounts = get_validators_keys(config);
    let connection = config.create_root_connection().await;

    let reserved_before = connection.get_next_era_reserved_validators(None).await;
    let non_reserved_before = connection.get_next_era_non_reserved_validators(None).await;
    let committee_size_before = connection.get_next_era_committee_seats(None).await;

    info!(
        "[+] state before tx: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}",
        reserved_before, non_reserved_before, committee_size_before
    );

    let new_validators: Vec<AccountId> = accounts
        .iter()
        .map(|pair| pair.signer().public().into())
        .collect();
    connection
        .change_validators(
            Some(new_validators[0..2].to_vec()),
            Some(new_validators[2..].to_vec()),
            Some(CommitteeSeats {
                reserved_seats: 2,
                non_reserved_seats: 2,
            }),
            TxStatus::InBlock,
        )
        .await?;

    connection.wait_for_event(|e: &ChangeValidators| {
        info!("[+] NewValidatorsEvent: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}", e.0, e.1, e.2);

        e.0 == new_validators[0..2]
            && e.1 == new_validators[2..]
            && e.2
            == CommitteeSeats {
            reserved_seats: 2,
            non_reserved_seats: 2,
        }
    }, BlockStatus::Best).await;

    let reserved_after = connection.get_next_era_reserved_validators(None).await;
    let non_reserved_after = connection.get_next_era_non_reserved_validators(None).await;
    let committee_size_after = connection.get_next_era_committee_seats(None).await;

    info!(
        "[+] state before tx: reserved: {:#?}, non_reserved: {:#?}, committee_size: {:#?}",
        reserved_after, non_reserved_after, committee_size_after
    );

    assert_eq!(new_validators[..2], reserved_after);
    assert_eq!(new_validators[2..], non_reserved_after);
    assert_eq!(
        CommitteeSeats {
            reserved_seats: 2,
            non_reserved_seats: 2
        },
        committee_size_after
    );

    let block_number = connection
        .get_best_block()
        .await?
        .ok_or(anyhow!("Failed to retrieve best block!"))?;
    connection
        .wait_for_block(|n| n >= block_number, BlockStatus::Finalized)
        .await;

    Ok(())
}
