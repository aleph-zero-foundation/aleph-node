use aleph_client::{
    api::transaction_payment::events::TransactionFeePaid,
    pallets::{balances::BalanceUserApi, fee::TransactionPaymentApi, system::SystemSudoApi},
    waiting::{AlephWaiting, BlockStatus},
    AccountId, RootConnection, SignedConnection, SignedConnectionApi, TxStatus,
};
use log::info;
use primitives::Balance;
use sp_runtime::{FixedPointNumber, FixedU128};

use crate::{config::setup_test, transfer::setup_for_transfer};

#[tokio::test]
pub async fn fee_calculation() -> anyhow::Result<()> {
    let config = setup_test();
    // An initial transfer is needed to establish the fee multiplier.
    let (connection, to) = setup_for_transfer(config).await;
    let root_connection = config.create_root_connection().await;
    let transfer_value = 1000u128;

    let minimum_multiplier = FixedU128::saturating_from_integer(1);
    let (old_fee, actual_multiplier) =
        current_fees(&connection, to.clone(), None, transfer_value).await;
    assert_eq!(
        actual_multiplier,
        minimum_multiplier.into_inner(),
        "In the beginning the fee multiplier should be equal to the minimum value",
    );

    // The target saturation level is set to 25%, so unless we cross this limit,
    // the fees should not increase. Note that effectively it is 18.75% of the whole block.
    fill_blocks(15, 5, &root_connection).await;
    let (new_fee, actual_multiplier) =
        current_fees(&connection, to.clone(), None, transfer_value).await;
    assert_eq!(
        actual_multiplier,
        minimum_multiplier.into_inner(),
        "In the beginning the fee multiplier should be equal to the minimum value",
    );

    assert_eq!(
        new_fee, old_fee,
        "In the beginning the fee should not be adjusted",
    );

    // At 60% of occupancy the fees should increase by ~2.4% per block. However, the
    // intermediate blocks will be empty, so in order to have reliable reads we have to
    // simulate high traffic for a longer time.
    fill_blocks(60, 4, &root_connection).await;
    let (new_fee, actual_multiplier) =
        current_fees(&connection, to.clone(), None, transfer_value).await;
    assert!(
        actual_multiplier > 1,
        "When the traffic is high the fee multiplier should increase, {:?}",
        actual_multiplier,
    );
    assert!(
        new_fee > old_fee,
        "When the traffic is high fees should be scaled up: {:?} !> {:?}",
        new_fee,
        old_fee
    );

    let (prev_multiplier, prev_fee) = (actual_multiplier, new_fee);
    fill_blocks(60, 4, &root_connection).await;
    let (new_fee, actual_multiplier) =
        current_fees(&connection, to.clone(), None, transfer_value).await;
    assert!(
        actual_multiplier.gt(&prev_multiplier),
        "When the traffic is still high the fee multiplier should still increase",
    );
    assert!(
        prev_fee < new_fee,
        "When the traffic is still high fees should be scaled up even more",
    );

    let (prev_multiplier, prev_fee) = (actual_multiplier, new_fee);
    fill_blocks(0, 8, &root_connection).await;
    let (new_fee, actual_multiplier) = current_fees(&connection, to, None, transfer_value).await;
    // This is rather an ethical version of sleep.
    assert!(
        prev_multiplier.gt(&actual_multiplier),
        "When the traffic is low again the fee multiplier should decrease",
    );
    assert!(
        new_fee < prev_fee,
        "When the traffic is low again fees should be scaled down",
    );

    Ok(())
}

async fn fill_blocks(target_ratio: u8, blocks: u32, connection: &RootConnection) {
    for _ in 0..blocks {
        connection
            .fill_block(target_ratio, TxStatus::InBlock)
            .await
            .unwrap_or_else(|err| panic!("Error while sending a fill_block transation: {:?}", err));
    }
}

pub async fn current_fees(
    connection: &SignedConnection,
    to: AccountId,
    tip: Option<Balance>,
    transfer_value: Balance,
) -> (Balance, u128) {
    let actual_multiplier = connection.get_next_fee_multiplier(None).await;

    let waiting_connection = connection.clone();
    let signer = connection.account_id().clone();
    let event_handle = tokio::spawn(async move {
        waiting_connection
            .wait_for_event(
                |e: &TransactionFeePaid| e.who == signer,
                BlockStatus::Finalized,
            )
            .await
    });
    match tip {
        None => {
            connection
                .transfer(to, transfer_value, TxStatus::Finalized)
                .await
                .unwrap();
        }
        Some(tip) => {
            connection
                .transfer_with_tip(to, transfer_value, tip, TxStatus::Finalized)
                .await
                .unwrap();
        }
    }
    let event = event_handle.await.unwrap();
    let fee = event.actual_fee;

    info!("fee payed: {}", fee);

    (fee, actual_multiplier)
}
