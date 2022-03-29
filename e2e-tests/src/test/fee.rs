use codec::Encode;
use sp_core::Pair;
use sp_runtime::{traits::One, FixedPointNumber, FixedU128};
use substrate_api_client::{
    compose_extrinsic, AccountId, GenericAddress, UncheckedExtrinsicV4, XtStatus,
};

use aleph_client::{
    get_next_fee_multiplier, get_tx_fee_info, send_xt, Connection, FeeInfo, TransferTransaction,
};

use crate::{config::Config, transfer::setup_for_transfer};

pub fn fee_calculation(config: &Config) -> anyhow::Result<()> {
    let (connection, _from, _to) = setup_for_transfer(config);

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
    fill_blocks(15, 5, &connection);
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
    fill_blocks(60, 4, &connection);
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
    fill_blocks(60, 4, &connection);
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
    fill_blocks(0, 8, &connection);
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

fn check_current_fees<Call: Encode>(
    connection: &Connection,
    tx: &UncheckedExtrinsicV4<Call>,
) -> (FixedU128, FeeInfo) {
    // The storage query will return an u128 value which is the 'inner' representation
    // i.e. scaled up by 10^18 (see `implement_fixed!` for `FixedU128).
    let actual_multiplier = FixedU128::from_inner(get_next_fee_multiplier(connection));
    let fee_info = get_tx_fee_info(connection, tx);
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

fn prepare_transaction(connection: &Connection) -> TransferTransaction {
    compose_extrinsic!(
        connection,
        "Balances",
        "transfer",
        GenericAddress::Id(AccountId::default()),
        Compact(0u128)
    )
}

fn fill_blocks(target_ratio: u32, blocks: u32, connection: &Connection) {
    for _ in 0..blocks {
        let xt = compose_extrinsic!(
            connection,
            "System",
            "fill_block",
            target_ratio * 10_000_000
        );
        send_xt(connection, xt.hex_encode(), "fill block", XtStatus::InBlock);
    }
}
