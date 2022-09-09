use aleph_client::{
    balances_transfer, get_next_fee_multiplier, AccountId, BalanceTransfer, CallSystem, FeeInfo,
    GetTxInfo, ReadStorage, RootConnection, XtStatus,
};
use sp_runtime::{traits::One, FixedPointNumber, FixedU128};

use crate::{config::Config, transfer::setup_for_transfer};

pub fn fee_calculation(config: &Config) -> anyhow::Result<()> {
    // An initial transfer is needed to establish the fee multiplier.
    let (connection, to) = setup_for_transfer(config);
    let root_connection = config.create_root_connection();
    let transfer_value = 1000u128;
    balances_transfer(&connection, &to, transfer_value, XtStatus::Finalized);

    // An example transaction for which we will query fee details at different traffic level.
    let tx = prepare_transaction(&connection);

    let (actual_multiplier, fee_info) = check_current_fees(&connection, &tx);
    assert_no_scaling(
        actual_multiplier,
        fee_info,
        "In the beginning the fee multiplier should be equal to the minimum value",
        "In the beginning fees should not be scaled",
    );

    // The target saturation level is set to 25%, so unless we cross this limit,
    // the fees should not increase. Note that effectively it is 18.75% of the whole block.
    fill_blocks(15, 5, &root_connection);
    let (actual_multiplier, fee_info) = check_current_fees(&connection, &tx);
    assert_no_scaling(
        actual_multiplier,
        fee_info,
        "When the traffic is low the fee multiplier should not increase",
        "When the traffic is low fees should not be scaled",
    );

    // At 60% of occupancy the fees should increase by ~2.4% per block. However, the
    // intermediate blocks will be empty, so in order to have reliable reads we have to
    // simulate high traffic for a longer time.
    fill_blocks(60, 4, &root_connection);
    let (actual_multiplier, fee_info) = check_current_fees(&connection, &tx);
    assert!(
        actual_multiplier.gt(&FixedU128::one()),
        "When the traffic is high the fee multiplier should increase",
    );
    assert!(
        fee_info.unadjusted_weight < fee_info.adjusted_weight,
        "When the traffic is high fees should be scaled up",
    );

    let (prev_multiplier, prev_fee_info) = (actual_multiplier, fee_info);
    fill_blocks(60, 4, &root_connection);
    let (actual_multiplier, fee_info) = check_current_fees(&connection, &tx);
    assert!(
        actual_multiplier.gt(&prev_multiplier),
        "When the traffic is still high the fee multiplier should still increase",
    );
    assert!(
        prev_fee_info.adjusted_weight < fee_info.adjusted_weight,
        "When the traffic is still high fees should be scaled up even more",
    );

    let (prev_multiplier, prev_fee_info) = (actual_multiplier, fee_info);
    fill_blocks(0, 8, &root_connection);
    let (actual_multiplier, fee_info) = check_current_fees(&connection, &tx);
    // This is rather an ethical version of sleep.
    assert!(
        prev_multiplier.gt(&actual_multiplier),
        "When the traffic is low again the fee multiplier should decrease",
    );
    assert!(
        fee_info.adjusted_weight < prev_fee_info.adjusted_weight,
        "When the traffic is low again fees should be scaled down",
    );

    Ok(())
}

fn check_current_fees<C: ReadStorage + BalanceTransfer + GetTxInfo<C::TransferTx>>(
    connection: &C,
    tx: &C::TransferTx,
) -> (FixedU128, FeeInfo) {
    // The storage query will return an u128 value which is the 'inner' representation
    // i.e. scaled up by 10^18 (see `implement_fixed!` for `FixedU128).
    let actual_multiplier = FixedU128::from_inner(get_next_fee_multiplier(connection));
    let fee_info = connection.get_tx_info(tx);
    (actual_multiplier, fee_info)
}

fn assert_no_scaling(
    actual_multiplier: FixedU128,
    fee_info: FeeInfo,
    error_multiplier_msg: &str,
    error_fee_msg: &str,
) {
    // We should never drop below 1, in particular when there is no traffic.
    let minimum_multiplier = FixedU128::saturating_from_integer(1);

    assert_eq!(
        minimum_multiplier, actual_multiplier,
        "{} (actual multiplier: {})",
        error_multiplier_msg, actual_multiplier
    );
    assert_eq!(
        fee_info.unadjusted_weight, fee_info.adjusted_weight,
        "{} ({} was scaled to {})",
        error_fee_msg, fee_info.unadjusted_weight, fee_info.adjusted_weight,
    );
}

fn prepare_transaction<C: BalanceTransfer>(connection: &C) -> C::TransferTx {
    let bytes = [0u8; 32];
    let account = AccountId::from(bytes);

    connection.create_transfer_tx(account, 0u128)
}

fn fill_blocks(target_ratio: u32, blocks: u32, connection: &RootConnection) {
    for _ in 0..blocks {
        connection
            .fill_block(target_ratio, XtStatus::InBlock)
            .unwrap_or_else(|err| panic!("Error while sending a fill_block transation: {:?}", err));
    }
}
