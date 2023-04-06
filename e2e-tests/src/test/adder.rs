use std::{fmt::Debug, str::FromStr, sync::Arc};

use aleph_client::{
    contract::{
        event::{get_contract_events, listen_contract_events},
        ContractInstance,
    },
    contract_transcode::Value,
    pallets::system::SystemApi,
    AccountId, ConnectionApi, SignedConnectionApi, TxInfo,
};
use anyhow::{anyhow, Context, Result};
use assert2::assert;
use futures::{channel::mpsc::unbounded, StreamExt};

use crate::{config::setup_test, test::helpers::basic_test_context};

/// This test exercises the aleph-client code for interacting with contracts by testing a simple
/// contract that maintains some state and publishes some events. The events are obtained by
/// listening mechanism.
#[tokio::test]
pub async fn adder_events_listening() -> Result<()> {
    let config = setup_test();

    let (conn, _authority, account) = basic_test_context(config).await?;

    let contract = Arc::new(AdderInstance::new(
        &config.test_case_params.adder,
        &config.test_case_params.adder_metadata,
    )?);

    let listen_conn = conn.clone();
    let listen_contract = contract.clone();
    let (tx, mut rx) = unbounded();
    let listen = || async move {
        listen_contract_events(&listen_conn, &[listen_contract.as_ref().into()], tx).await?;
        <Result<(), anyhow::Error>>::Ok(())
    };
    let join = tokio::spawn(listen());

    let increment = 10;
    let before = contract.get(&conn).await?;

    contract.add(&account.sign(&conn), increment).await?;

    let event = rx.next().await.context("No event received")??;
    assert!(event.name == Some("ValueChanged".to_string()));
    assert!(event.contract == *contract.contract.address());
    assert!(event.data["new_value"] == Value::UInt(before as u128 + 10));

    let after = contract.get(&conn).await?;
    assert!(after == before + increment);

    let new_name = "test";
    contract.set_name(&account.sign(&conn), None).await?;
    assert!(contract.get_name(&conn).await?.is_none());
    contract
        .set_name(&account.sign(&conn), Some(new_name))
        .await?;
    assert!(contract.get_name(&conn).await? == Some(new_name.to_string()));

    rx.close();
    join.await??;

    Ok(())
}

/// This test exercises the aleph-client code for interacting with contracts by testing a simple
/// contract that maintains some state and publishes some events. The events are obtained by
/// fetching mechanism.
#[tokio::test]
pub async fn adder_fetching_events() -> Result<()> {
    let config = setup_test();

    let (conn, _authority, account) = basic_test_context(config).await?;

    let contract = AdderInstance::new(
        &config.test_case_params.adder,
        &config.test_case_params.adder_metadata,
    )?;

    let increment = 10;
    let before = contract.get(&conn).await?;

    let tx_info = contract.add(&account.sign(&conn), increment).await?;
    let events = get_contract_events(&conn, &contract.contract, tx_info).await?;
    let event = match &*events {
        [event] => event,
        _ => return Err(anyhow!("Expected single event, but got {events:?}")),
    };

    assert!(event.name == Some("ValueChanged".to_string()));
    assert!(event.contract == *contract.contract.address());
    assert!(event.data["new_value"] == Value::UInt(before as u128 + 10));

    let after = contract.get(&conn).await?;
    assert!(after == before + increment);

    let new_name = "test";
    contract.set_name(&account.sign(&conn), None).await?;
    assert!(contract.get_name(&conn).await?.is_none());
    contract
        .set_name(&account.sign(&conn), Some(new_name))
        .await?;
    assert!(contract.get_name(&conn).await? == Some(new_name.to_string()));

    Ok(())
}

/// This test ensures that `aleph-client` won't submit call if dry-run fails.
#[tokio::test]
pub async fn adder_dry_run_failure() -> Result<()> {
    let config = setup_test();

    let (conn, _authority, account) = basic_test_context(config).await?;

    let contract = AdderInstance::new(
        &config.test_case_params.adder,
        &config.test_case_params.adder_metadata,
    )?;

    // Make the counter value non-zero to enable overflow during next call.
    contract.add(&account.sign(&conn), 1).await?;

    let caller_balance_before = conn
        .get_free_balance(account.account_id().clone(), None)
        .await;

    // Should fail due to the overflow check in contract.
    let result = contract.add(&account.sign(&conn), u32::MAX).await;
    assert!(result.is_err());

    let caller_balance_after = conn
        .get_free_balance(account.account_id().clone(), None)
        .await;

    assert_eq!(caller_balance_before, caller_balance_after);

    Ok(())
}

#[derive(Debug)]
struct AdderInstance {
    contract: ContractInstance,
}

impl<'a> From<&'a AdderInstance> for &'a ContractInstance {
    fn from(instance: &'a AdderInstance) -> Self {
        &instance.contract
    }
}

impl<'a> From<&'a AdderInstance> for AccountId {
    fn from(instance: &'a AdderInstance) -> Self {
        instance.contract.address().clone()
    }
}

impl AdderInstance {
    pub fn new(address: &Option<String>, metadata_path: &Option<String>) -> Result<Self> {
        let address = address.as_ref().context("Adder contract address not set")?;
        let metadata_path = metadata_path
            .as_ref()
            .context("Adder contract metadata not set")?;

        let address = AccountId::from_str(address)
            .ok()
            .with_context(|| format!("Failed to parse address: {}", address))?;
        let contract = ContractInstance::new(address, metadata_path)?;
        Ok(Self { contract })
    }

    pub async fn get<C: ConnectionApi>(&self, conn: &C) -> Result<u32> {
        self.contract.contract_read0(conn, "get").await
    }

    pub async fn add<S: SignedConnectionApi>(&self, conn: &S, value: u32) -> Result<TxInfo> {
        self.contract
            .contract_exec(conn, "add", &[value.to_string()])
            .await
    }

    pub async fn set_name<S: SignedConnectionApi>(
        &self,
        conn: &S,
        name: Option<&str>,
    ) -> Result<TxInfo> {
        let name = name.map_or_else(
            || "None".to_string(),
            |name| {
                let mut bytes = name.bytes().take(20).collect::<Vec<_>>();
                bytes.extend(std::iter::repeat(0).take(20 - bytes.len()));
                format!("Some({:?})", bytes)
            },
        );

        self.contract.contract_exec(conn, "set_name", &[name]).await
    }

    pub async fn get_name<C: ConnectionApi>(&self, conn: &C) -> Result<Option<String>> {
        let res: Option<String> = self.contract.contract_read0(conn, "get_name").await?;
        Ok(res.map(|name| name.replace('\0', "")))
    }
}
