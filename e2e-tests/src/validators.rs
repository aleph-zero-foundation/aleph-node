use aleph_client::{
    account_from_keypair, balances_batch_transfer, keypair_from_string, rotate_keys, set_keys,
    staking_bond, staking_validate, KeyPair, SignedConnection,
};
use primitives::{staking::MIN_VALIDATOR_BOND, SessionIndex, TOKEN};
use substrate_api_client::{AccountId, XtStatus};

use crate::{accounts::get_validators_keys, Config};

/// Get all the reserved validators for the chain.
pub fn get_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_keys(config)[0..2].to_vec()
}

/// Get all the non-reserved validators for the chain.
pub fn get_non_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_keys(config)[2..].to_vec()
}

/// Get the non-reserved validators selected for a particular session.
pub fn get_non_reserved_validators_for_session(
    config: &Config,
    session: SessionIndex,
) -> Vec<AccountId> {
    // Test assumption
    const FREE_SEATS: u32 = 2;

    let mut non_reserved = vec![];

    let non_reserved_nodes_order_from_runtime = get_non_reserved_validators(config);
    let non_reserved_nodes_order_from_runtime_len = non_reserved_nodes_order_from_runtime.len();

    for i in (FREE_SEATS * session)..(FREE_SEATS * (session + 1)) {
        non_reserved.push(
            non_reserved_nodes_order_from_runtime
                [i as usize % non_reserved_nodes_order_from_runtime_len]
                .clone(),
        );
    }

    non_reserved.iter().map(account_from_keypair).collect()
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
pub fn setup_accounts() -> Accounts {
    let seeds = (0..6).map(|idx| format!("//Validator//{}", idx));

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
