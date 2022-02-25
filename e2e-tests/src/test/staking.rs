use crate::{
    accounts::{accounts_from_seeds, default_account_seeds},
    config::Config,
    waiting::{wait_for_event, wait_for_finalized_block},
    BlockNumber, Connection, Header, KeyPair,
};
use codec::{Compact, Decode};
use common::create_connection;
use log::info;
use pallet_staking::ValidatorPrefs;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use sp_core::Pair;
use sp_runtime::Perbill;
use substrate_api_client::{
    compose_call, compose_extrinsic, extrinsic::staking::RewardDestination, AccountId,
    GenericAddress, XtStatus,
};

fn send_xt(connection: &Connection, xt: String, xt_name: &'static str) {
    let block_hash = connection
        .send_extrinsic(xt, XtStatus::InBlock)
        .expect("Could not send extrinsc")
        .expect("Could not get tx hash");
    let block_number = connection
        .get_header::<Header>(Some(block_hash))
        .expect("Could not fetch header")
        .expect("Block exists; qed")
        .number;
    info!(
        "Transaction {} was included in block {}.",
        xt_name, block_number
    );
}

fn endow_stash_balances(connection: &Connection, keys: &[KeyPair], endowment: u128) {
    let batch_endow: Vec<_> = keys
        .iter()
        .map(|key| {
            compose_call!(
                connection.metadata,
                "Balances",
                "transfer",
                GenericAddress::Id(AccountId::from(key.public())),
                Compact(endowment)
            )
        })
        .collect();

    let xt = compose_extrinsic!(connection, "Utility", "batch", batch_endow);
    send_xt(connection, xt.hex_encode(), "batch of endow balances");
}

fn bond(address: &str, initial_stake: u128, controller: &KeyPair) {
    let connection = create_connection(address).set_signer(controller.clone());
    let account_id = GenericAddress::Id(AccountId::from(controller.public()));

    let xt = connection.staking_bond(account_id, initial_stake, RewardDestination::Staked);
    send_xt(&connection, xt.hex_encode(), "bond");
}

fn validate(address: &str, controller: &KeyPair) {
    let connection = create_connection(address).set_signer(controller.clone());
    let prefs = ValidatorPrefs {
        blocked: false,
        commission: Perbill::from_percent(10),
    };

    let xt = compose_extrinsic!(connection, "Staking", "validate", prefs);
    send_xt(&connection, xt.hex_encode(), "validate");
}

fn nominate(address: &str, nominator_key_pair: &KeyPair, nominee_key_pair: &KeyPair) {
    let nominee_account_id = AccountId::from(nominee_key_pair.public());
    let connection = create_connection(address).set_signer(nominator_key_pair.clone());

    let xt = connection.staking_nominate(vec![GenericAddress::Id(nominee_account_id)]);
    send_xt(&connection, xt.hex_encode(), "nominate");
}

fn payout_stakers(address: &str, validator: KeyPair, era_number: BlockNumber) {
    let account = AccountId::from(validator.public());
    let connection = create_connection(address).set_signer(validator);
    let xt = compose_extrinsic!(connection, "Staking", "payout_stakers", account, era_number);

    send_xt(&connection, xt.hex_encode(), "payout_stakers");
}

fn wait_for_full_era_completion(connection: &Connection) -> anyhow::Result<BlockNumber> {
    let sessions_per_era: u32 = connection
        .get_constant("Staking", "SessionsPerEra")
        .unwrap();
    let current_era: u32 = connection
        .get_storage_value("Staking", "ActiveEra", None)
        .unwrap()
        .unwrap();
    let payout_era = current_era + 2;

    let first_session_in_payout_era = payout_era * sessions_per_era;

    info!(
        "Current era: {}, waiting for the first session in the payout era {}",
        current_era, first_session_in_payout_era
    );

    #[derive(Debug, Decode, Clone)]
    struct NewSessionEvent {
        session_index: u32,
    }
    wait_for_event(
        connection,
        ("Session", "NewSession"),
        |e: NewSessionEvent| {
            info!("[+] new session {}", e.session_index);

            e.session_index == first_session_in_payout_era
        },
    )?;

    Ok(payout_era)
}

fn get_key_pairs() -> (Vec<KeyPair>, Vec<KeyPair>) {
    let validators = default_account_seeds();
    let validator_stashes: Vec<_> = validators
        .iter()
        .map(|v| String::from(v) + "//stash")
        .collect();
    let validator_accounts_key_pairs = accounts_from_seeds(&Some(validators));
    let stashes_accounts_key_pairs = accounts_from_seeds(&Some(validator_stashes));

    (stashes_accounts_key_pairs, validator_accounts_key_pairs)
}

// 1. endow stash accounts balances, controller accounts are already endowed in chainspec
// 2. bond controller account to stash account, stash = controller and set controller to StakerStatus::Validate
// 3. bond controller account to stash account, stash = controller and set controller to StakerStatus::Nominate
// 4. wait for new era
// 5. send payout stakers tx
pub fn staking_test(config: &Config) -> anyhow::Result<()> {
    const TOKEN: u128 = 1_000_000_000_000;
    const VALIDATOR_STAKE: u128 = 25_000 * TOKEN;
    const NOMINATOR_STAKE: u128 = 1_000 * TOKEN;

    let (stashes_accounts, validator_accounts) = get_key_pairs();

    let node = &config.node;
    let sender = validator_accounts[0].clone();
    let connection = create_connection(node).set_signer(sender);

    endow_stash_balances(&connection, &stashes_accounts, VALIDATOR_STAKE);

    validator_accounts.par_iter().for_each(|account| {
        bond(node, VALIDATOR_STAKE, account);
    });

    validator_accounts
        .par_iter()
        .for_each(|account| validate(node, account));

    stashes_accounts
        .par_iter()
        .for_each(|nominator| bond(node, NOMINATOR_STAKE, nominator));

    stashes_accounts
        .par_iter()
        .zip(validator_accounts.par_iter())
        .for_each(|(nominator, nominee)| nominate(node, nominator, nominee));

    // All the above calls influace the next era, so we need to wait that it passes.
    let current_era = wait_for_full_era_completion(&connection)?;
    info!(
        "Era {} started, claiming rewards for era {}",
        current_era,
        current_era - 1
    );

    validator_accounts
        .into_par_iter()
        .for_each(|account| payout_stakers(node, account, current_era - 1));

    // Sanity check
    let block_number = connection
        .get_header::<Header>(None)
        .unwrap()
        .unwrap()
        .number;
    info!(
        "Current block number is {}, waiting till it finalizes",
        block_number,
    );

    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
