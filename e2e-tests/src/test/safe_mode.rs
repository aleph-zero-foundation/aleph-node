use std::fmt::Debug;

use aleph_client::{
    aleph_runtime::ProxyType,
    keypair_from_string,
    pallets::{
        balances::BalanceUserApi,
        proxy::ProxyUserApi,
        safe_mode::{SafeModeSudoApi, SafeModeUserApi},
        session::SessionUserApi,
        staking::{StakingSudoApi, StakingUserApi},
        vesting::VestingUserApi,
    },
    AccountId, AsSigned, SignedConnectionApi, TxStatus,
};

use crate::config::setup_test;

fn should_be_err<T, R>(result: Result<T, R>, msg: &str) {
    if result.is_ok() {
        panic!("{}", msg);
    }
}

fn should_be_ok<T, R: Debug>(result: Result<T, R>, msg: &str) {
    if let Err(e) = result {
        panic!("{}, {:?}", msg, e);
    }
}

fn test_account() -> AccountId {
    let seed = "//test_account";
    let account = keypair_from_string(seed);
    account.account_id().clone()
}

/// Tests checks if some user can call the enter or extend function. It fails if the calls succeed.
#[tokio::test]
async fn safe_mode_operations_are_disabled_for_users() -> anyhow::Result<()> {
    let config = setup_test();
    let connection = config.get_first_signed_connection().await;

    should_be_err(
        connection.enter(TxStatus::InBlock).await,
        "Enter function should fail for any user",
    );
    should_be_err(
        connection.extend(TxStatus::InBlock).await,
        "Extend function should fail for any user",
    );

    Ok(())
}

#[tokio::test]
async fn safe_mode_is_configured_correctly() -> anyhow::Result<()> {
    let config = setup_test();
    let root = config.create_root_connection().await;

    root.force_enter(TxStatus::Finalized).await?;

    let signed_connection = root.as_signed();

    let should_be_fails = vec![
        (
            signed_connection.add_proxy(test_account(), ProxyType::Any, 0, TxStatus::InBlock),
            "Proxy call should fail",
        ),
        (
            signed_connection.transfer_keep_alive(test_account(), 100, TxStatus::InBlock),
            "Balance call should fail",
        ),
        (
            signed_connection.set_keys(vec![0; 64].into(), TxStatus::InBlock),
            "Session call should fail",
        ),
        (
            signed_connection.validate(10, TxStatus::InBlock),
            "Staking call should fail",
        ),
        (
            signed_connection.vest(TxStatus::InBlock),
            "Vesting call should fail",
        ),
    ];

    let should_be_successes = vec![
        (
            root.force_new_era(TxStatus::InBlock),
            "Root call should succeed",
        ), // sudo with a call that normally would be filtered should still pass through
    ];

    for (should_be_fail, msg) in should_be_fails {
        should_be_err(should_be_fail.await, msg)
    }

    for (should_be_success, msg) in should_be_successes {
        should_be_ok(should_be_success.await, msg);
    }

    Ok(())
}
