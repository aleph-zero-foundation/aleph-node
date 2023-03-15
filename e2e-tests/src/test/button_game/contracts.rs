use std::{fmt::Debug, str::FromStr};

use aleph_client::{
    contract::ContractInstance, AccountId, Connection, ConnectionApi, SignedConnection, TxInfo,
};
use anyhow::{Context, Result};
use primitives::Balance;

use crate::config::Config;

/// A wrapper around the simple dex contract.
///
/// The methods on this type match contract methods.
#[derive(Debug)]
pub(super) struct SimpleDexInstance {
    contract: ContractInstance,
}

impl<'a> From<&'a SimpleDexInstance> for &'a ContractInstance {
    fn from(dex: &'a SimpleDexInstance) -> Self {
        &dex.contract
    }
}

impl<'a> From<&'a SimpleDexInstance> for AccountId {
    fn from(dex: &'a SimpleDexInstance) -> Self {
        dex.contract.address().clone()
    }
}

impl SimpleDexInstance {
    pub fn new(config: &Config) -> Result<Self> {
        let dex_address = config
            .test_case_params
            .simple_dex
            .clone()
            .context("Simple dex address not set.")?;
        let dex_address = AccountId::from_str(&dex_address)
            .ok()
            .context("Invalid dex address.")?;
        let metadata_path = config
            .test_case_params
            .simple_dex_metadata
            .clone()
            .context("Simple dex metadata not set")?;

        Ok(Self {
            contract: ContractInstance::new(dex_address, &metadata_path)?,
        })
    }

    pub async fn add_swap_pair(
        &self,
        conn: &SignedConnection,
        from: AccountId,
        to: AccountId,
    ) -> Result<TxInfo> {
        self.contract
            .contract_exec(conn, "add_swap_pair", &[from.to_string(), to.to_string()])
            .await
    }

    pub async fn remove_swap_pair(
        &self,
        conn: &SignedConnection,
        from: AccountId,
        to: AccountId,
    ) -> Result<TxInfo> {
        self.contract
            .contract_exec(
                conn,
                "remove_swap_pair",
                &[&from.to_string(), &to.to_string()],
            )
            .await
    }

    pub async fn deposit(
        &self,
        conn: &SignedConnection,
        amounts: &[(&PSP22TokenInstance, Balance)],
    ) -> Result<TxInfo> {
        let deposits = amounts
            .iter()
            .map(|(token, amount)| {
                let address: AccountId = (*token).try_into()?;
                Ok(format!("({:}, {:})", address, amount))
            })
            .collect::<Result<Vec<String>>>()?;

        self.contract
            .contract_exec(conn, "deposit", &[format!("[{:}]", deposits.join(","))])
            .await
    }

    pub async fn out_given_in<C: ConnectionApi>(
        &self,
        conn: &C,
        token_in: &PSP22TokenInstance,
        token_out: &PSP22TokenInstance,
        amount_token_in: Balance,
    ) -> Result<Balance> {
        let token_in: AccountId = token_in.into();
        let token_out: AccountId = token_out.into();

        self.contract
            .contract_read(
                conn,
                "out_given_in",
                &[
                    token_in.to_string(),
                    token_out.to_string(),
                    amount_token_in.to_string(),
                ],
            )
            .await?
    }

    pub async fn swap(
        &self,
        conn: &SignedConnection,
        token_in: &PSP22TokenInstance,
        amount_token_in: Balance,
        token_out: &PSP22TokenInstance,
        min_amount_token_out: Balance,
    ) -> Result<TxInfo> {
        let token_in: AccountId = token_in.into();
        let token_out: AccountId = token_out.into();

        self.contract
            .contract_exec(
                conn,
                "swap",
                &[
                    token_in.to_string(),
                    token_out.to_string(),
                    amount_token_in.to_string(),
                    min_amount_token_out.to_string(),
                ],
            )
            .await
    }
}

/// A wrapper around a button game contract.
///
/// The methods on this type match contract methods.
#[derive(Debug)]
pub(super) struct ButtonInstance {
    contract: ContractInstance,
}

impl ButtonInstance {
    pub fn new(config: &Config, button_address: &Option<String>) -> Result<Self> {
        let button_address = button_address
            .clone()
            .context("Button game address not set.")?;
        let button_address = AccountId::from_str(&button_address)
            .ok()
            .context("Invalid button game address")?;
        let metadata_path = config
            .test_case_params
            .button_game_metadata
            .clone()
            .context("Button game metadata path not set.")?;
        Ok(Self {
            contract: ContractInstance::new(button_address, &metadata_path)?,
        })
    }

    pub async fn deadline<C: ConnectionApi>(&self, conn: &C) -> Result<u128> {
        self.contract.contract_read0(conn, "deadline").await
    }

    pub async fn is_dead<C: ConnectionApi>(&self, conn: &C) -> Result<bool> {
        self.contract.contract_read0(conn, "is_dead").await
    }

    pub async fn ticket_token<C: ConnectionApi>(&self, conn: &C) -> Result<AccountId> {
        self.contract.contract_read0(conn, "ticket_token").await
    }

    pub async fn reward_token<C: ConnectionApi>(&self, conn: &C) -> Result<AccountId> {
        self.contract.contract_read0(conn, "reward_token").await
    }

    pub async fn marketplace<C: ConnectionApi>(&self, conn: &C) -> Result<AccountId> {
        self.contract.contract_read0(conn, "marketplace").await
    }

    pub async fn press(&self, conn: &SignedConnection) -> Result<TxInfo> {
        self.contract.contract_exec0(conn, "press").await
    }

    pub async fn reset(&self, conn: &SignedConnection) -> Result<TxInfo> {
        self.contract.contract_exec0(conn, "reset").await
    }
}

impl<'a> From<&'a ButtonInstance> for &'a ContractInstance {
    fn from(button: &'a ButtonInstance) -> Self {
        &button.contract
    }
}

impl From<&ButtonInstance> for AccountId {
    fn from(button: &ButtonInstance) -> Self {
        button.contract.address().clone()
    }
}

/// A wrapper around a PSP22 contract.
///
/// The methods on this type match contract methods.
#[derive(Debug)]
pub(super) struct PSP22TokenInstance {
    contract: ContractInstance,
}

impl PSP22TokenInstance {
    pub fn new(address: AccountId, metadata_path: &Option<String>) -> Result<Self> {
        let metadata_path = metadata_path
            .as_ref()
            .context("PSP22Token metadata not set.")?;
        Ok(Self {
            contract: ContractInstance::new(address, metadata_path)?,
        })
    }

    pub async fn transfer(
        &self,
        conn: &SignedConnection,
        to: &AccountId,
        amount: Balance,
    ) -> Result<TxInfo> {
        self.contract
            .contract_exec(
                conn,
                "PSP22::transfer",
                &[to.to_string(), amount.to_string(), "0x00".to_string()],
            )
            .await
    }

    pub async fn mint(
        &self,
        conn: &SignedConnection,
        to: &AccountId,
        amount: Balance,
    ) -> Result<TxInfo> {
        self.contract
            .contract_exec(
                conn,
                "PSP22Mintable::mint",
                &[to.to_string(), amount.to_string()],
            )
            .await
    }

    pub async fn approve(
        &self,
        conn: &SignedConnection,
        spender: &AccountId,
        value: Balance,
    ) -> Result<TxInfo> {
        self.contract
            .contract_exec(
                conn,
                "PSP22::approve",
                &[spender.to_string(), value.to_string()],
            )
            .await
    }

    pub async fn balance_of(&self, conn: &Connection, account: &AccountId) -> Result<Balance> {
        self.contract
            .contract_read(conn, "PSP22::balance_of", &[account.to_string()])
            .await
    }
}

impl<'a> From<&'a PSP22TokenInstance> for &'a ContractInstance {
    fn from(token: &'a PSP22TokenInstance) -> Self {
        &token.contract
    }
}

impl From<&PSP22TokenInstance> for AccountId {
    fn from(token: &PSP22TokenInstance) -> AccountId {
        token.contract.address().clone()
    }
}

/// A wrapper around a marketplace contract instance.
///
/// The methods on this type match contract methods.
#[derive(Debug)]
pub(super) struct MarketplaceInstance {
    contract: ContractInstance,
}

impl MarketplaceInstance {
    pub fn new(address: AccountId, metadata_path: &Option<String>) -> Result<Self> {
        Ok(Self {
            contract: ContractInstance::new(
                address,
                metadata_path
                    .as_ref()
                    .context("Marketplace metadata not set.")?,
            )?,
        })
    }

    pub async fn reset(&self, conn: &SignedConnection) -> Result<TxInfo> {
        self.contract.contract_exec0(conn, "reset").await
    }

    pub async fn buy(&self, conn: &SignedConnection, max_price: Option<Balance>) -> Result<TxInfo> {
        let max_price = max_price.map_or_else(|| "None".to_string(), |x| format!("Some({})", x));

        self.contract
            .contract_exec(conn, "buy", &[max_price.as_str()])
            .await
    }

    pub async fn price<C: ConnectionApi>(&self, conn: &C) -> Result<Balance> {
        self.contract.contract_read0(conn, "price").await
    }
}

impl<'a> From<&'a MarketplaceInstance> for &'a ContractInstance {
    fn from(marketplace: &'a MarketplaceInstance) -> Self {
        &marketplace.contract
    }
}

impl From<&MarketplaceInstance> for AccountId {
    fn from(marketplace: &MarketplaceInstance) -> AccountId {
        marketplace.contract.address().clone()
    }
}

#[derive(Debug)]
pub struct WAzeroInstance {
    contract: ContractInstance,
}

impl WAzeroInstance {
    pub fn new(config: &Config) -> Result<Self> {
        let wazero_address = config
            .test_case_params
            .wrapped_azero
            .clone()
            .context("Wrapped AZERO address not set.")?;
        let wazero_address = AccountId::from_str(&wazero_address)
            .ok()
            .context("Invalid address.")?;
        let metadata_path = config
            .test_case_params
            .wrapped_azero_metadata
            .clone()
            .context("Wrapped AZERO metadata path not set.")?;

        Ok(Self {
            contract: ContractInstance::new(wazero_address, &metadata_path)?,
        })
    }

    pub async fn wrap(&self, conn: &SignedConnection, value: Balance) -> Result<TxInfo> {
        self.contract
            .contract_exec_value0(conn, "wrap", value)
            .await
    }

    pub async fn unwrap(&self, conn: &SignedConnection, amount: Balance) -> Result<TxInfo> {
        self.contract
            .contract_exec(conn, "unwrap", &[amount.to_string()])
            .await
    }

    pub async fn balance_of<C: ConnectionApi>(
        &self,
        conn: &C,
        account: &AccountId,
    ) -> Result<Balance> {
        self.contract
            .contract_read(conn, "PSP22::balance_of", &[account.to_string()])
            .await
    }
}

impl<'a> From<&'a WAzeroInstance> for &'a ContractInstance {
    fn from(wazero: &'a WAzeroInstance) -> Self {
        &wazero.contract
    }
}

impl<'a> From<&'a WAzeroInstance> for AccountId {
    fn from(wazero: &'a WAzeroInstance) -> Self {
        wazero.contract.address().clone()
    }
}
