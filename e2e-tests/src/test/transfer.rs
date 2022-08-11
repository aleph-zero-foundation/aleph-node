use aleph_client::{balances_transfer, get_free_balance, XtStatus};
use log::info;

use crate::{config::Config, transfer::setup_for_transfer};

pub fn token_transfer(config: &Config) -> anyhow::Result<()> {
    let (connection, to) = setup_for_transfer(config);

    let balance_before = get_free_balance(&connection, &to);
    info!("[+] Account {} balance before tx: {}", to, balance_before);

    let transfer_value = 1000u128;
    balances_transfer(&connection, &to, transfer_value, XtStatus::Finalized);

    let balance_after = get_free_balance(&connection, &to);
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
