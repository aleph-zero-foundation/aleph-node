use codec::{Compact, Encode};
use pallet_contracts_primitives::ContractExecResult;
use subxt::{ext::sp_core::Bytes, rpc_params, utils::Static};

use crate::{
    api,
    api::runtime_types,
    pallet_contracts::wasm::{CodeInfo, Determinism},
    sp_weights::weight_v2::Weight,
    AccountId, Balance, BlockHash, CodeHash, ConnectionApi, SignedConnectionApi, TxInfo, TxStatus,
};

/// The Event that was emitted during execution of calls.
pub type EventRecord =
    runtime_types::frame_system::EventRecord<runtime_types::aleph_runtime::RuntimeEvent, BlockHash>;

/// Arguments to [`ContractRpc::call_and_get`].
#[derive(Encode)]
pub struct ContractCallArgs {
    /// Who is singing a tx.
    pub origin: AccountId,
    /// Address of the contract to call.
    pub dest: AccountId,
    /// The balance to transfer from the `origin` to `dest`.
    pub value: Balance,
    /// The gas limit enforced when executing the constructor.
    pub gas_limit: Option<Weight>,
    /// The maximum amount of balance that can be charged from the caller to pay for the storage consumed.
    pub storage_deposit_limit: Option<Balance>,
    /// The input data to pass to the contract.
    pub input_data: Vec<u8>,
}

/// Pallet contracts read-only api.
#[async_trait::async_trait]
pub trait ContractsApi {
    /// Returns `contracts.code_info` storage for a given code hash.
    /// * `code_hash` - a code hash
    /// * `at` - optional hash of a block to query state from
    async fn get_code_info(&self, code_hash: CodeHash, at: Option<BlockHash>) -> Option<CodeInfo>;
}

/// Pallet contracts api.
#[async_trait::async_trait]
pub trait ContractsUserApi {
    /// API for [`upload_code`](https://paritytech.github.io/substrate/master/pallet_contracts/pallet/struct.Pallet.html#method.upload_code) call.
    async fn upload_code(
        &self,
        code: Vec<u8>,
        storage_limit: Option<Compact<Balance>>,
        determinism: Determinism,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`instantiate`](https://paritytech.github.io/substrate/master/pallet_contracts/pallet/struct.Pallet.html#method.instantiate) call.
    #[allow(clippy::too_many_arguments)]
    async fn instantiate(
        &self,
        code_hash: CodeHash,
        balance: Balance,
        gas_limit: Weight,
        storage_limit: Option<Compact<Balance>>,
        data: Vec<u8>,
        salt: Vec<u8>,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`instantiate_with_code`](https://paritytech.github.io/substrate/master/pallet_contracts/pallet/struct.Pallet.html#method.instantiate_with_code) call.
    #[allow(clippy::too_many_arguments)]
    async fn instantiate_with_code(
        &self,
        code: Vec<u8>,
        balance: Balance,
        gas_limit: Weight,
        storage_limit: Option<Compact<Balance>>,
        data: Vec<u8>,
        salt: Vec<u8>,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`call`](https://paritytech.github.io/substrate/master/pallet_contracts/pallet/struct.Pallet.html#method.call) call.
    async fn call(
        &self,
        destination: AccountId,
        balance: Balance,
        gas_limit: Weight,
        storage_limit: Option<Compact<Balance>>,
        data: Vec<u8>,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`remove_code`](https://paritytech.github.io/substrate/master/pallet_contracts/pallet/struct.Pallet.html#method.remove_code) call.
    async fn remove_code(&self, code_hash: BlockHash, status: TxStatus) -> anyhow::Result<TxInfo>;
}

/// RPC for runtime ContractsApi
#[async_trait::async_trait]
pub trait ContractRpc {
    /// API for [`call`](https://paritytech.github.io/substrate/master/pallet_contracts/trait.ContractsApi.html#method.call) call.
    async fn call_and_get(
        &self,
        args: ContractCallArgs,
    ) -> anyhow::Result<ContractExecResult<Balance, EventRecord>>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi> ContractsApi for C {
    async fn get_code_info(&self, code_hash: CodeHash, at: Option<BlockHash>) -> Option<CodeInfo> {
        let addrs = api::storage().contracts().code_info_of(code_hash);

        self.get_storage_entry_maybe(&addrs, at).await
    }
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> ContractsUserApi for S {
    async fn upload_code(
        &self,
        code: Vec<u8>,
        storage_limit: Option<Compact<Balance>>,
        determinism: Determinism,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx()
            .contracts()
            .upload_code(code, storage_limit, determinism);

        self.send_tx(tx, status).await
    }

    async fn instantiate(
        &self,
        code_hash: CodeHash,
        balance: Balance,
        gas_limit: Weight,
        storage_limit: Option<Compact<Balance>>,
        data: Vec<u8>,
        salt: Vec<u8>,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx().contracts().instantiate(
            balance,
            gas_limit,
            storage_limit,
            code_hash,
            data,
            salt,
        );

        self.send_tx(tx, status).await
    }

    async fn instantiate_with_code(
        &self,
        code: Vec<u8>,
        balance: Balance,
        gas_limit: Weight,
        storage_limit: Option<Compact<Balance>>,
        data: Vec<u8>,
        salt: Vec<u8>,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx().contracts().instantiate_with_code(
            balance,
            gas_limit,
            storage_limit,
            code,
            data,
            salt,
        );

        self.send_tx(tx, status).await
    }

    async fn call(
        &self,
        destination: AccountId,
        balance: Balance,
        gas_limit: Weight,
        storage_limit: Option<Compact<Balance>>,
        data: Vec<u8>,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx().contracts().call(
            Static::from(destination).into(),
            balance,
            gas_limit,
            storage_limit,
            data,
        );
        self.send_tx(tx, status).await
    }

    async fn remove_code(&self, code_hash: BlockHash, status: TxStatus) -> anyhow::Result<TxInfo> {
        let tx = api::tx().contracts().remove_code(code_hash);

        self.send_tx(tx, status).await
    }
}

#[async_trait::async_trait]
impl<C: ConnectionApi> ContractRpc for C {
    async fn call_and_get(
        &self,
        args: ContractCallArgs,
    ) -> anyhow::Result<ContractExecResult<Balance, EventRecord>> {
        let params = rpc_params!["ContractsApi_call", Bytes(args.encode())];
        self.rpc_call("state_call".to_string(), params).await
    }
}
