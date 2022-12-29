use std::str::FromStr;

use aleph_client::{contract::ContractInstance, AccountId, Connection, SignedConnection};
use anyhow::{Context, Result};
use assert2::assert;

use crate::{config::setup_test, test::helpers::basic_test_context};

/// This test exercises the aleph-client code for interacting with contracts by testing a simple contract that maintains
/// some state and publishes some events.
#[tokio::test]
pub async fn adder() -> Result<()> {
    let config = setup_test();

    let (conn, _authority, account) = basic_test_context(config).await?;
    let contract = AdderInstance::new(
        &config.test_case_params.adder,
        &config.test_case_params.adder_metadata,
    )?;

    let increment = 10;
    let before = contract.get(&conn).await?;
    contract.add(&account.sign(&conn), increment).await?;
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

pub(super) struct AdderInstance {
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

    pub async fn get(&self, conn: &Connection) -> Result<u32> {
        self.contract.contract_read0(conn, "get").await
    }

    pub async fn add(&self, conn: &SignedConnection, value: u32) -> Result<()> {
        self.contract
            .contract_exec(conn, "add", &[value.to_string()])
            .await
    }

    pub async fn set_name(&self, conn: &SignedConnection, name: Option<&str>) -> Result<()> {
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

    pub async fn get_name(&self, conn: &Connection) -> Result<Option<String>> {
        let res: Option<String> = self.contract.contract_read0(conn, "get_name").await?;
        Ok(res.map(|name| name.replace("\0", "")))
    }
}
