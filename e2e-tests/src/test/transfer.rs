use log::info;

use crate::accounts::get_free_balance;
use crate::config::Config;
use crate::fee::{get_tx_fee_info, FeeInfo};
use crate::transfer::{setup_for_transfer, transfer};

pub fn fee_calculation(config: Config) -> anyhow::Result<()> {
    let (connection, from, to) = setup_for_transfer(config);

    let balance_before = get_free_balance(&from, &connection);
    info!("[+] Account {} balance before tx: {}", to, balance_before);

    let transfer_value = 1000u128;
    let tx = transfer(&to, transfer_value, &connection);

    let balance_after = get_free_balance(&from, &connection);
    info!("[+] Account {} balance after tx: {}", to, balance_after);

    let FeeInfo {
        fee_without_weight,
        unadjusted_weight,
        adjusted_weight,
    } = get_tx_fee_info(&connection, &tx);
    let multiplier = 1; // corresponds to `ConstantFeeMultiplierUpdate`
    assert_eq!(
        multiplier * unadjusted_weight,
        adjusted_weight,
        "Weight fee was adjusted incorrectly: raw fee = {}, adjusted fee = {}",
        unadjusted_weight,
        adjusted_weight
    );

    let expected_fee = fee_without_weight + adjusted_weight;
    assert_eq!(
        balance_before - transfer_value - expected_fee,
        balance_after,
        "Incorrect balance: before = {}, after = {}, tx = {}, expected fee = {}",
        balance_before,
        balance_after,
        transfer_value,
        expected_fee
    );

    Ok(())
}

pub fn token_transfer(config: Config) -> anyhow::Result<()> {
    let (connection, _, to) = setup_for_transfer(config);

    let balance_before = get_free_balance(&to, &connection);
    info!("[+] Account {} balance before tx: {}", to, balance_before);

    let transfer_value = 1000u128;
    transfer(&to, transfer_value, &connection);

    let balance_after = get_free_balance(&to, &connection);
    info!("[+] Account {} balance after tx: {}", to, balance_after);

    assert_eq!(
        balance_before + transfer_value,
        balance_after,
        "before = {}, after = {}, tx = {}",
        balance_before,
        balance_after,
        transfer_value
    );

    Ok(())
}
