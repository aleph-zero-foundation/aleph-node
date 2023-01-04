use aleph_client::{
    account_from_keypair, keypair_from_string,
    pallets::{
        author::AuthorRpc, balances::BalanceUserBatchExtApi, session::SessionUserApi,
        staking::StakingUserApi,
    },
    primitives::EraValidators,
    raw_keypair_from_string, AccountId, KeyPair, RawKeyPair, SignedConnection, SignedConnectionApi,
    TxStatus,
};
use futures::future::join_all;
use primitives::{staking::MIN_VALIDATOR_BOND, TOKEN};

use crate::{accounts::get_validators_raw_keys, config::Config};

/// Get all validators assumed for test
pub fn get_test_validators(config: &Config) -> EraValidators<KeyPair> {
    let all_validators = get_validators_raw_keys(config);
    let reserved = all_validators[0..2]
        .iter()
        .map(|k| KeyPair::new(k.clone()))
        .collect();
    let non_reserved = all_validators[2..]
        .iter()
        .map(|k| KeyPair::new(k.clone()))
        .collect();

    EraValidators {
        reserved,
        non_reserved,
    }
}

/// Gathers keys and accounts for all validators used in an experiment.
pub struct Accounts {
    stash_keys: Vec<KeyPair>,
    stash_accounts: Vec<AccountId>,
    stash_raw_keys: Vec<RawKeyPair>,
    controller_keys: Vec<KeyPair>,
    controller_accounts: Vec<AccountId>,
    controller_raw_keys: Vec<RawKeyPair>,
}

#[allow(dead_code)]
impl Accounts {
    pub fn get_stash_keys(&self) -> &Vec<KeyPair> {
        &self.stash_keys
    }
    pub fn get_stash_raw_keys(&self) -> &Vec<RawKeyPair> {
        &self.stash_raw_keys
    }
    pub fn get_stash_accounts(&self) -> &Vec<AccountId> {
        &self.stash_accounts
    }
    pub fn get_controller_keys(&self) -> &Vec<KeyPair> {
        &self.controller_keys
    }
    pub fn get_controller_raw_keys(&self) -> &Vec<RawKeyPair> {
        &self.controller_raw_keys
    }
    pub fn get_controller_accounts(&self) -> &Vec<AccountId> {
        &self.controller_accounts
    }
}

/// Generate `Accounts` struct.
pub fn setup_accounts(desired_validator_count: u32) -> Accounts {
    let seeds = (0..desired_validator_count).map(|idx| format!("//Validator//{}", idx));

    let stash_seeds = seeds.clone().map(|seed| format!("{}//Stash", seed));
    let stash_keys: Vec<_> = stash_seeds
        .clone()
        .map(|s| keypair_from_string(&s))
        .collect();
    let stash_raw_keys = stash_seeds.map(|s| raw_keypair_from_string(&s)).collect();
    let stash_accounts = stash_keys
        .iter()
        .map(|k| account_from_keypair(k.signer()))
        .collect();

    let controller_seeds = seeds.map(|seed| format!("{}//Controller", seed));
    let controller_keys: Vec<_> = controller_seeds
        .clone()
        .map(|s| keypair_from_string(&s))
        .collect();
    let controller_raw_keys = controller_seeds
        .map(|s| raw_keypair_from_string(&s))
        .collect();
    let controller_accounts = controller_keys
        .iter()
        .map(|k| account_from_keypair(k.signer()))
        .collect();

    Accounts {
        stash_keys,
        stash_accounts,
        controller_keys,
        controller_accounts,
        stash_raw_keys,
        controller_raw_keys,
    }
}

/// Endow validators (stashes and controllers), bond and rotate keys.
///
/// Signer of `connection` should have enough balance to endow new accounts.
pub async fn prepare_validators<S: SignedConnectionApi + AuthorRpc>(
    connection: &S,
    node: &str,
    accounts: &Accounts,
) -> anyhow::Result<()> {
    connection
        .batch_transfer(
            &accounts.stash_accounts,
            MIN_VALIDATOR_BOND + TOKEN,
            TxStatus::Finalized,
        )
        .await
        .unwrap();
    connection
        .batch_transfer(&accounts.controller_accounts, TOKEN, TxStatus::Finalized)
        .await
        .unwrap();

    let mut handles = vec![];
    for (stash, controller) in accounts
        .stash_raw_keys
        .iter()
        .zip(accounts.get_controller_accounts().iter())
    {
        let connection = SignedConnection::new(node, KeyPair::new(stash.clone())).await;
        let contr = controller.clone();
        handles.push(tokio::spawn(async move {
            connection
                .bond(MIN_VALIDATOR_BOND, contr, TxStatus::Finalized)
                .await
                .unwrap();
        }));
    }

    for controller in accounts.controller_raw_keys.iter() {
        let keys = connection.author_rotate_keys().await?;
        let connection = SignedConnection::new(node, KeyPair::new(controller.clone())).await;
        handles.push(tokio::spawn(async move {
            connection
                .set_keys(keys, TxStatus::Finalized)
                .await
                .unwrap();
            connection.validate(10, TxStatus::Finalized).await.unwrap();
        }));
    }

    join_all(handles).await;
    Ok(())
}
