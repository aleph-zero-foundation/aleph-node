use aleph_client::{
    account_from_keypair, balances_batch_transfer, keypair_from_string, rotate_keys, set_keys,
    staking_bond, staking_validate, AccountId, KeyPair, SignedConnection, XtStatus,
};
use primitives::{staking::MIN_VALIDATOR_BOND, EraValidators, TOKEN};

use crate::{accounts::get_validators_keys, Config};

/// Get all validators assumed for test
pub fn get_test_validators(config: &Config) -> EraValidators<KeyPair> {
    let all_validators = get_validators_keys(config);
    let reserved = all_validators[0..2].to_vec();
    let non_reserved = all_validators[2..].to_vec();

    EraValidators {
        reserved,
        non_reserved,
    }
}

/// Gathers keys and accounts for all validators used in an experiment.
pub struct Accounts {
    stash_keys: Vec<KeyPair>,
    stash_accounts: Vec<AccountId>,

    controller_keys: Vec<KeyPair>,
    controller_accounts: Vec<AccountId>,
}

impl Accounts {
    pub fn get_stash_keys(&self) -> &Vec<KeyPair> {
        &self.stash_keys
    }
    pub fn get_stash_accounts(&self) -> &Vec<AccountId> {
        &self.stash_accounts
    }
    pub fn get_controller_keys(&self) -> &Vec<KeyPair> {
        &self.controller_keys
    }
    pub fn get_controller_accounts(&self) -> &Vec<AccountId> {
        &self.controller_accounts
    }
}

/// Generate `Accounts` struct.
pub fn setup_accounts(desired_validator_count: u32) -> Accounts {
    let seeds = (0..desired_validator_count).map(|idx| format!("//Validator//{}", idx));

    let stash_seeds = seeds.clone().map(|seed| format!("{}//Stash", seed));
    let stash_keys = stash_seeds.map(|s| keypair_from_string(&s));
    let stash_accounts = stash_keys.clone().map(|k| account_from_keypair(&k));

    let controller_seeds = seeds.map(|seed| format!("{}//Controller", seed));
    let controller_keys = controller_seeds.map(|s| keypair_from_string(&s));
    let controller_accounts = controller_keys.clone().map(|k| account_from_keypair(&k));

    Accounts {
        stash_keys: stash_keys.collect(),
        stash_accounts: stash_accounts.collect(),
        controller_keys: controller_keys.collect(),
        controller_accounts: controller_accounts.collect(),
    }
}

/// Endow validators (stashes and controllers), bond and rotate keys.
///
/// Signer of `connection` should have enough balance to endow new accounts.
pub fn prepare_validators(connection: &SignedConnection, node: &str, accounts: &Accounts) {
    balances_batch_transfer(
        connection,
        accounts.stash_accounts.clone(),
        MIN_VALIDATOR_BOND + TOKEN,
    );
    balances_batch_transfer(
        connection,
        accounts.get_controller_accounts().to_vec(),
        TOKEN,
    );

    for (stash, controller) in accounts
        .stash_keys
        .iter()
        .zip(accounts.get_controller_accounts().iter())
    {
        let connection = SignedConnection::new(node, stash.clone());
        staking_bond(
            &connection,
            MIN_VALIDATOR_BOND,
            controller,
            XtStatus::Finalized,
        );
    }

    for controller in accounts.controller_keys.iter() {
        let keys = rotate_keys(connection).expect("Failed to generate new keys");
        let connection = SignedConnection::new(node, controller.clone());
        set_keys(&connection, keys, XtStatus::Finalized);
        staking_validate(&connection, 10, XtStatus::Finalized);
    }
}
