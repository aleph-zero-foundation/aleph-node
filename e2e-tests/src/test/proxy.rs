use aleph_client::{
    aleph_runtime::{ProxyType, RuntimeCall},
    api::{proxy::events::ProxyExecuted, utility::events::BatchInterrupted},
    keypair_from_string,
    pallet_balances::pallet::Call::transfer_allow_death,
    pallet_session::pallet::Call::set_keys,
    pallet_staking::{
        pallet::pallet::Call::{bond, chill, nominate},
        RewardDestination,
    },
    pallet_utility::pallet::Call::batch,
    pallets::{balances::BalanceUserApi, proxy::ProxyUserApi, staking::StakingUserApi},
    utility::BlocksApi,
    AsConnection, AsSigned, Connection, KeyPair, RootConnection, SignedConnection, TxStatus,
};
use primitives::TOKEN;
use subxt::utils::{MultiAddress, Static};

use crate::{
    accounts::{accounts_seeds_to_keys, get_validators_seeds},
    config::setup_test,
    test::proxy::ProxyCallExpectedStatus::{Failure, Success},
};

struct ProxyHandle {
    proxy: KeyPair,
    account: KeyPair,
}

enum ProxyCallExpectedStatus {
    Success,
    Failure,
}

async fn setup_proxy(connection: RootConnection, proxy_type: ProxyType) -> ProxyHandle {
    let seed = "//test_account";
    let proxy_seed = format!("{}//proxy", seed);

    let account = keypair_from_string(seed);
    let proxy = keypair_from_string(&proxy_seed);

    connection
        .as_signed()
        .transfer_keep_alive(
            account.account_id().clone(),
            10_000 * TOKEN,
            TxStatus::Finalized,
        )
        .await
        .expect("transfer should succeed");
    connection
        .as_signed()
        .transfer_keep_alive(
            proxy.account_id().clone(),
            1000 * TOKEN,
            TxStatus::Finalized,
        )
        .await
        .expect("transfer should succeed");

    let account_connection =
        SignedConnection::from_connection(connection.as_connection().clone(), account.clone());

    account_connection
        .add_proxy(
            proxy.account_id().clone(),
            proxy_type,
            0,
            TxStatus::Finalized,
        )
        .await
        .expect("we have funds");

    ProxyHandle { account, proxy }
}

async fn perform_and_check_calls(
    connection: Connection,
    handle: &ProxyHandle,
    calls: Vec<(RuntimeCall, ProxyCallExpectedStatus)>,
) {
    let proxy_connection =
        SignedConnection::from_connection(connection.clone(), handle.proxy.clone());

    for (call, status) in calls {
        let tx_info = proxy_connection
            .proxy(
                handle.account.account_id().clone(),
                call.clone(),
                TxStatus::InBlock,
            )
            .await
            .unwrap();

        let events = connection.get_tx_events(tx_info).await.unwrap();

        let res = match events.find_first::<ProxyExecuted>() {
            Ok(Some(res)) => res,
            _ => panic!("expected one `ProxyExecuted` event"),
        };
        match status {
            Success => {
                if res.result.is_err() {
                    panic!("Result of the call should be ok, {:?} {:?}", res, call);
                }

                if let Ok(Some(bi)) = events.find_first::<BatchInterrupted>() {
                    panic!("No batch interrupted event should be present {:?}", bi);
                };
            }
            Failure => {
                if res.result.is_err() {
                    continue;
                }

                match events.find_first::<BatchInterrupted>() {
                    Ok(Some(_)) => {}
                    _ => panic!("Batch interrupted event should be present"),
                };
            }
        }
    }
}

#[tokio::test]
pub async fn any_proxy_works() -> anyhow::Result<()> {
    let config = setup_test();

    let root_connection = config.create_root_connection().await;

    let handle = setup_proxy(root_connection.clone(), ProxyType::Any).await;
    let test_id = keypair_from_string("//test_account").account_id().clone();

    let calls = vec![
        (
            RuntimeCall::Balances(transfer_allow_death {
                dest: MultiAddress::Id(Static(test_id.clone())),
                value: 100,
            }),
            Success,
        ),
        (
            RuntimeCall::Utility(batch {
                calls: vec![RuntimeCall::Balances(transfer_allow_death {
                    dest: MultiAddress::Id(Static(test_id)),
                    value: 100,
                })],
            }),
            Success,
        ),
        (
            RuntimeCall::Staking(bond {
                value: 1000,
                payee: RewardDestination::Staked,
            }),
            Success,
        ),
        (
            RuntimeCall::Session(set_keys {
                keys: vec![0u8; 64].into(),
                proof: vec![],
            }),
            Success,
        ),
    ];

    perform_and_check_calls(root_connection.as_connection().clone(), &handle, calls).await;

    Ok(())
}

#[tokio::test]
pub async fn staking_proxy_works() -> anyhow::Result<()> {
    let config = setup_test();

    let root_connection = config.create_root_connection().await;

    let handle = setup_proxy(root_connection.clone(), ProxyType::Staking).await;
    let test_id = keypair_from_string("//test_account").account_id().clone();

    let calls = vec![
        (
            RuntimeCall::Balances(transfer_allow_death {
                dest: MultiAddress::Id(Static(test_id.clone())),
                value: 100,
            }),
            Failure,
        ),
        (
            RuntimeCall::Utility(batch {
                calls: vec![RuntimeCall::Balances(transfer_allow_death {
                    dest: MultiAddress::Id(Static(test_id)),
                    value: 100,
                })],
            }),
            Failure,
        ),
        (
            RuntimeCall::Staking(bond {
                value: 1000,
                payee: RewardDestination::Staked,
            }),
            Success,
        ),
        (
            RuntimeCall::Session(set_keys {
                keys: vec![0u8; 64].into(),
                proof: vec![],
            }),
            Success,
        ),
    ];

    perform_and_check_calls(root_connection.as_connection().clone(), &handle, calls).await;

    Ok(())
}
#[tokio::test]
pub async fn non_transfer_proxy_works() -> anyhow::Result<()> {
    let config = setup_test();

    let root_connection = config.create_root_connection().await;

    let handle = setup_proxy(root_connection.clone(), ProxyType::Staking).await;
    let test_id = keypair_from_string("//test_account").account_id().clone();

    let calls = vec![
        (
            RuntimeCall::Balances(transfer_allow_death {
                dest: MultiAddress::Id(Static(test_id.clone())),
                value: 100,
            }),
            Failure,
        ),
        (
            RuntimeCall::Utility(batch {
                calls: vec![RuntimeCall::Balances(transfer_allow_death {
                    dest: MultiAddress::Id(Static(test_id)),
                    value: 100,
                })],
            }),
            Failure,
        ),
        (
            RuntimeCall::Staking(bond {
                value: 1000,
                payee: RewardDestination::Staked,
            }),
            Success,
        ),
        (
            RuntimeCall::Session(set_keys {
                keys: vec![0u8; 64].into(),
                proof: vec![],
            }),
            Success,
        ),
    ];

    perform_and_check_calls(root_connection.as_connection().clone(), &handle, calls).await;

    Ok(())
}

#[tokio::test]
pub async fn nomination_proxy_works() -> anyhow::Result<()> {
    let config = setup_test();
    let root_connection = config.create_root_connection().await;
    let handle = setup_proxy(root_connection.clone(), ProxyType::Nomination).await;
    let test_account_connection = SignedConnection::from_connection(
        root_connection.as_connection().clone(),
        handle.account.clone(),
    );
    let validator_0_id = accounts_seeds_to_keys(&get_validators_seeds(config))[0]
        .account_id()
        .clone();

    let calls = vec![
        (
            RuntimeCall::Staking(bond {
                value: 1000,
                payee: RewardDestination::Staked,
            }),
            Failure,
        ),
        (
            RuntimeCall::Utility(batch {
                calls: vec![RuntimeCall::Staking(bond {
                    value: 1000,
                    payee: RewardDestination::Staked,
                })],
            }),
            Failure,
        ),
    ];
    perform_and_check_calls(root_connection.as_connection().clone(), &handle, calls).await;

    test_account_connection
        .bond(2500 * TOKEN, TxStatus::Finalized)
        .await?;
    let calls = vec![
        (
            RuntimeCall::Staking(nominate {
                targets: vec![MultiAddress::Id(Static(validator_0_id.clone()))],
            }),
            Success,
        ),
        (RuntimeCall::Staking(chill {}), Failure),
    ];
    perform_and_check_calls(root_connection.as_connection().clone(), &handle, calls).await;

    Ok(())
}
