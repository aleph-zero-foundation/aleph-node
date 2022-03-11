use crate::{send_xt, Connection, KeyPair};
use pallet_staking::RewardDestination;
use sp_core::Pair;
use substrate_api_client::{AccountId, GenericAddress, XtStatus};

pub fn bond(connection: &Connection, initial_stake: u128, controller: &KeyPair, status: XtStatus) {
    let controller_account_id = GenericAddress::Id(AccountId::from(controller.public()));

    let xt = connection.staking_bond(
        controller_account_id,
        initial_stake,
        RewardDestination::Staked,
    );
    send_xt(&connection, xt.hex_encode(), "bond", status);
}
