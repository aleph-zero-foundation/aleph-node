use aleph_client::{
    pallets::staking::{StakingSudoApi, StakingUserApi},
    AccountId, RootConnection, SignedConnection, TxStatus,
};
use primitives::TOKEN;
use subxt::ext::sp_core::crypto::Ss58Codec;

pub async fn bond(
    stash_connection: SignedConnection,
    initial_stake_in_tokens: u32,
    controller_account: String,
) {
    let controller_account =
        AccountId::from_ss58check(&controller_account).expect("Address is valid");

    let initial_stake = initial_stake_in_tokens as u128 * TOKEN;
    stash_connection
        .bond(initial_stake, controller_account, TxStatus::Finalized)
        .await
        .unwrap();
}

pub async fn validate(connection: SignedConnection, commission_percentage: u8) {
    connection
        .validate(commission_percentage, TxStatus::Finalized)
        .await
        .unwrap();
}

pub async fn nominate(connection: SignedConnection, nominee: String) {
    let nominee_account = AccountId::from_ss58check(&nominee).expect("Address is valid");
    connection
        .nominate(nominee_account, TxStatus::InBlock)
        .await
        .unwrap();
}

pub async fn set_staking_limits(
    root_connection: RootConnection,
    minimal_nominator_stake_tokens: u64,
    minimal_validator_stake_tokens: u64,
    max_nominators_count: Option<u32>,
    max_validators_count: Option<u32>,
) {
    root_connection
        .set_staking_config(
            Some(minimal_nominator_stake_tokens as u128 * TOKEN),
            Some(minimal_validator_stake_tokens as u128 * TOKEN),
            max_nominators_count,
            max_validators_count,
            TxStatus::Finalized,
        )
        .await
        .unwrap();
}

pub async fn force_new_era(root_connection: RootConnection) {
    root_connection
        .force_new_era(TxStatus::Finalized)
        .await
        .unwrap();
}
