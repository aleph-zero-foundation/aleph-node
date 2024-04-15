use std::string::ToString;

use aleph_runtime::{Feature, Perbill, WASM_BINARY};
use pallet_staking::{Forcing, StakerStatus};
use primitives::{
    staking::{MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND},
    AccountId, AlephNodeSessionKeys as SessionKeys, Version as FinalityVersion, ADDRESSES_ENCODING,
    TOKEN_DECIMALS,
};
use serde_json::{Number, Value};

use crate::{
    chain_spec::{cli::ChainParams, AlephNodeChainSpec},
    commands::AuthorityKeys,
};

fn to_account_ids(authorities: &[AuthorityKeys]) -> impl Iterator<Item = AccountId> + '_ {
    authorities.iter().map(|auth| auth.account_id.clone())
}

fn system_properties(token_symbol: String) -> serde_json::map::Map<String, Value> {
    [
        ("tokenSymbol".to_string(), Value::String(token_symbol)),
        (
            "tokenDecimals".to_string(),
            Value::Number(Number::from(TOKEN_DECIMALS)),
        ),
        (
            "ss58Format".to_string(),
            Value::Number(Number::from(ADDRESSES_ENCODING)),
        ),
    ]
    .iter()
    .cloned()
    .collect()
}

/// Generate chain spec for new AlephNode chains
pub fn build_chain_spec(
    chain_params: ChainParams,
    authorities: Vec<AuthorityKeys>,
) -> Result<AlephNodeChainSpec, String> {
    let token_symbol = String::from(chain_params.token_symbol());
    let sudo_account = chain_params.sudo_account_id();
    let rich_accounts = chain_params.rich_account_ids();
    let faucet_account = chain_params.faucet_account_id();
    let finality_version = chain_params.finality_version();

    Ok(AlephNodeChainSpec::builder(
        WASM_BINARY.ok_or("AlephNode development wasm not available")?,
        Default::default(),
    )
    .with_name(chain_params.chain_name())
    .with_id(chain_params.chain_id())
    .with_chain_type(chain_params.chain_type())
    .with_genesis_config_patch(generate_genesis_config(
        authorities.clone(),    // Initial PoA authorities, will receive funds
        sudo_account.clone(),   // Sudo account, will also be pre funded
        rich_accounts.clone(),  // Pre-funded accounts
        faucet_account.clone(), // Pre-funded faucet account
        finality_version,
    ))
    .with_properties(system_properties(token_symbol))
    .build())
}

/// Calculate initial endowments such that total issuance is kept approximately constant.
fn calculate_initial_endowment(accounts: &[AccountId]) -> u128 {
    let total_issuance = 300_000_000u128 * 10u128.pow(TOKEN_DECIMALS);
    // due to known issue https://github.com/paritytech/polkadot-sdk/pull/2987/files,
    // we need to make sure returned number is un u64 range, otherwise serde_json::json macro fails
    // this is fixed in polkadot-sdk 1.6.0
    total_issuance / (accounts.len() as u128) / 10
}

/// Configure initial storage state for FRAME modules.
fn generate_genesis_config(
    authorities: Vec<AuthorityKeys>,
    sudo_account: AccountId,
    rich_accounts: Option<Vec<AccountId>>,
    faucet_account: Option<AccountId>,
    finality_version: FinalityVersion,
) -> serde_json::Value {
    let mut endowed_accounts = to_account_ids(&authorities)
        .chain(
            rich_accounts
                .unwrap_or_default()
                .into_iter()
                .chain([sudo_account.clone()]),
        )
        .collect::<Vec<_>>();
    if let Some(faucet_account) = faucet_account {
        endowed_accounts.push(faucet_account);
    }
    endowed_accounts.sort();
    endowed_accounts.dedup();
    let initial_endowement = calculate_initial_endowment(&endowed_accounts);

    let initial_balances = endowed_accounts
        .into_iter()
        .map(|account| (account, initial_endowement))
        .collect::<Vec<_>>();

    serde_json::json!({
        "balances": {
            "balances": initial_balances,
        },
        "sudo": {
            "key": Some(sudo_account),
        },
        "elections": {
            "reservedValidators": to_account_ids(&authorities).collect::<Vec<_>>(),
        },
        "session": {
           "keys": authorities
                    .iter()
                    .map(|auth| {
                        (
                            auth.account_id.clone(),
                            auth.account_id.clone(),
                            SessionKeys {
                                aura: auth.aura_key.clone(),
                                aleph: auth.aleph_key.clone(),
                            },
                        )
                    })
                    .collect::<Vec<_>>(),
        },
        "staking": {
            "forceEra": Forcing::NotForcing,
            "validatorCount":  authorities.len() as u32,
            "minimumValidatorCount": 4,
            "slashRewardFraction": Perbill::from_percent(10),
            "stakers": authorities
                        .iter()
                        .enumerate()
                        .map(|(validator_idx, validator)| {
                            (
                                validator.account_id.clone(),
                                // this is controller account but in Substrate 1.0.0, it is omitted anyway,
                                // so it does not matter what we pass in the below line as always stash == controller
                                validator.account_id.clone(),
                                (validator_idx + 1) as u128 * MIN_VALIDATOR_BOND,
                                StakerStatus::<AccountId>::Validator,
                            )
                        })
                        .collect::<Vec<_>>(),
            "minValidatorBond": MIN_VALIDATOR_BOND,
            "minNominatorBond": MIN_NOMINATOR_BOND,
        },
        "aleph": {
            "finalityVersion": finality_version,
        },
        "committeeManagement": {
            "sessionValidators": {
                "committee": to_account_ids(&authorities).collect::<Vec<_>>(),
            },
        },
        "featureControl": {
            "activeFeatures": vec![Feature::OnChainVerifier],
        },
    })
}
