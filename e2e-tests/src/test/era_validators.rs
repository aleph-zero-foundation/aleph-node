use aleph_client::{
    pallets::elections::{ElectionsApi, ElectionsSudoApi},
    primitives::CommitteeSeats,
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus, WaitingExt},
    AccountId, KeyPair, TxStatus,
};
use anyhow::anyhow;

use crate::{
    accounts::{account_ids_from_keys, get_validators_raw_keys},
    config::{setup_test, Config},
};

fn get_initial_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_raw_keys(config)[..2]
        .iter()
        .map(|k| KeyPair::new(k.clone()))
        .collect()
}

fn get_initial_non_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_raw_keys(config)[2..]
        .iter()
        .map(|k| KeyPair::new(k.clone()))
        .collect()
}

fn get_new_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_raw_keys(config)[3..]
        .iter()
        .map(|k| KeyPair::new(k.clone()))
        .collect()
}

fn get_new_non_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_raw_keys(config)[..3]
        .iter()
        .map(|k| KeyPair::new(k.clone()))
        .collect()
}

async fn get_current_and_next_era_reserved_validators<C: ElectionsApi>(
    connection: &C,
) -> (Vec<AccountId>, Vec<AccountId>) {
    let stored_reserved = connection.get_next_era_reserved_validators(None).await;
    let current_reserved = connection.get_current_era_validators(None).await.reserved;
    (current_reserved, stored_reserved)
}

async fn get_current_and_next_era_non_reserved_validators<C: ElectionsApi>(
    connection: &C,
) -> (Vec<AccountId>, Vec<AccountId>) {
    let stored_non_reserved = connection.get_next_era_non_reserved_validators(None).await;
    let current_non_reserved = connection
        .get_current_era_validators(None)
        .await
        .non_reserved;
    (current_non_reserved, stored_non_reserved)
}

#[tokio::test]
pub async fn era_validators() -> anyhow::Result<()> {
    let config = setup_test();
    let connection = config.get_first_signed_connection().await;
    let root_connection = config.create_root_connection().await;

    let initial_reserved_validators_keys = get_initial_reserved_validators(config);
    let initial_reserved_validators = account_ids_from_keys(&initial_reserved_validators_keys);

    let initial_non_reserved_validators_keys = get_initial_non_reserved_validators(config);
    let initial_non_reserved_validators =
        account_ids_from_keys(&initial_non_reserved_validators_keys);

    let new_reserved_validators_keys = get_new_reserved_validators(config);
    let new_reserved_validators = account_ids_from_keys(&new_reserved_validators_keys);

    let new_non_reserved_validators_keys = get_new_non_reserved_validators(config);
    let new_non_reserved_validators = account_ids_from_keys(&new_non_reserved_validators_keys);

    root_connection
        .change_validators(
            Some(initial_reserved_validators.clone()),
            Some(initial_non_reserved_validators.clone()),
            Some(CommitteeSeats {
                reserved_seats: 2,
                non_reserved_seats: 2,
            }),
            TxStatus::InBlock,
        )
        .await?;
    root_connection
        .wait_for_n_eras(1, BlockStatus::Finalized)
        .await;

    root_connection
        .change_validators(
            Some(new_reserved_validators.clone()),
            Some(new_non_reserved_validators.clone()),
            Some(CommitteeSeats {
                reserved_seats: 2,
                non_reserved_seats: 2,
            }),
            TxStatus::InBlock,
        )
        .await?;

    root_connection
        .wait_for_session(1, BlockStatus::Finalized)
        .await;
    let (eras_reserved, stored_reserved) =
        get_current_and_next_era_reserved_validators(&connection).await;
    let (eras_non_reserved, stored_non_reserved) =
        get_current_and_next_era_non_reserved_validators(&connection).await;

    assert_eq!(
        stored_reserved, new_reserved_validators,
        "Reserved validators' storage not properly updated after change_validators."
    );
    assert_eq!(
        eras_reserved, initial_reserved_validators,
        "Reserved validators set has been updated too early."
    );

    assert_eq!(
        stored_non_reserved, new_non_reserved_validators,
        "Non-reserved validators' storage not properly updated after change_validators."
    );
    assert_eq!(
        eras_non_reserved, initial_non_reserved_validators,
        "Non-reserved validators set has been updated too early."
    );

    connection.wait_for_n_eras(1, BlockStatus::Finalized).await;

    let (eras_reserved, stored_reserved) =
        get_current_and_next_era_reserved_validators(&connection).await;
    let (eras_non_reserved, stored_non_reserved) =
        get_current_and_next_era_non_reserved_validators(&connection).await;

    assert_eq!(
        stored_reserved, new_reserved_validators,
        "Reserved validators' storage not properly updated after change_validators."
    );
    assert_eq!(
        eras_reserved, new_reserved_validators,
        "Reserved validators set is not properly updated in the next era."
    );

    assert_eq!(
        stored_non_reserved, new_non_reserved_validators,
        "Non-reserved validators' storage not properly updated after change_validators."
    );
    assert_eq!(
        eras_non_reserved, new_non_reserved_validators,
        "Non-reserved validators set is not properly updated in the next era."
    );

    let block_number = connection
        .get_best_block()
        .await?
        .ok_or(anyhow!("Failed to retrieve best block number!"))?;
    connection
        .wait_for_block(|n| n >= block_number, BlockStatus::Finalized)
        .await;

    Ok(())
}
