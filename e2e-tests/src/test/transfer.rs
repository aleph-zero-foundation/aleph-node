use crate::AccountId;
use aleph_client::balances_transfer;
use log::info;
use substrate_api_client::XtStatus;

use crate::accounts::get_free_balance;
use crate::config::Config;
use crate::transfer::setup_for_transfer;

pub fn token_transfer(config: &Config) -> anyhow::Result<()> {
    let (connection, _, to) = setup_for_transfer(config);

    let to = AccountId::from(to);
    let balance_before = get_free_balance(&to, &connection);
    info!("[+] Account {} balance before tx: {}", to, balance_before);

    let transfer_value = 1000u128;
    balances_transfer(&connection, &to, transfer_value, XtStatus::Finalized);

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
