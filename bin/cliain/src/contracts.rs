use std::{
    fs::{self, File},
    path::Path,
};

use aleph_client::{
    api::contracts::events::{CodeRemoved, CodeStored, Instantiated},
    contract_transcode,
    pallet_contracts::wasm::OwnerInfo,
    pallets::contract::{ContractsApi, ContractsUserApi},
    sp_weights::weight_v2::Weight,
    waiting::{AlephWaiting, BlockStatus},
    AccountId, Balance, CodeHash, Connection, SignedConnection, SignedConnectionApi, TxStatus,
};
use codec::{Compact, Decode};
use contract_metadata::ContractMetadata;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::{
    commands::{
        ContractCall, ContractInstantiate, ContractInstantiateWithCode, ContractOptions,
        ContractOwnerInfo, ContractRemoveCode, ContractUploadCode,
    },
    contracts::contract_transcode::ContractMessageTranscoder,
};

#[derive(Debug, Decode, Clone, Serialize, Deserialize)]
pub struct InstantiateWithCodeReturnValue {
    pub contract: AccountId,
    pub code_hash: CodeHash,
}

fn storage_deposit(storage_deposit_limit: Option<Balance>) -> Option<Compact<u128>> {
    storage_deposit_limit.map(Compact)
}

pub async fn upload_code(
    signed_connection: SignedConnection,
    command: ContractUploadCode,
) -> anyhow::Result<CodeStored> {
    let ContractUploadCode {
        wasm_path,
        storage_deposit_limit,
    } = command;

    let wasm = fs::read(wasm_path).expect("WASM artifact not found");
    debug!(target: "contracts", "Found WASM contract code {:?}", wasm);

    let connection = signed_connection.clone();
    let event_handler = tokio::spawn(async move {
        connection
            .wait_for_event(
                |e: &CodeStored| {
                    info!(target : "contracts", "Received CodeStored event {:?}", e);
                    true
                },
                BlockStatus::Finalized,
            )
            .await
    });

    let _tx_info = signed_connection
        .upload_code(
            wasm,
            storage_deposit(storage_deposit_limit),
            aleph_client::pallet_contracts::wasm::Determinism::Deterministic,
            TxStatus::InBlock,
        )
        .await?;
    let code_stored_event = event_handler.await?;

    Ok(code_stored_event)
}

pub async fn instantiate(
    signed_connection: SignedConnection,
    command: ContractInstantiate,
) -> anyhow::Result<Instantiated> {
    let ContractInstantiate {
        code_hash,
        metadata_path,
        constructor,
        args,
        options,
    } = command;

    let ContractOptions {
        balance,
        gas_limit,
        storage_deposit_limit,
    } = options;

    let metadata = load_metadata(&metadata_path)?;
    let transcoder = ContractMessageTranscoder::new(metadata);
    let data = transcoder.encode(&constructor, args.unwrap_or_default())?;

    debug!("Encoded constructor data {:?}", data);

    let connection = signed_connection.clone();
    let signer_id = signed_connection.account_id().clone();

    let event_handler = tokio::spawn(async move {
        connection
            .wait_for_event(
                |e: &Instantiated| {
                    info!(target : "contracts", "Received ContractInstantiated event {:?}", e);
                    signer_id.eq(&e.deployer)
                },
                BlockStatus::Finalized,
            )
            .await
    });

    let _tx_info = signed_connection
        .instantiate(
            code_hash,
            balance,
            Weight::new(gas_limit, u64::MAX),
            storage_deposit(storage_deposit_limit),
            data,
            vec![],
            TxStatus::InBlock,
        )
        .await?;

    let contract_instantiated_event = event_handler.await?;

    Ok(contract_instantiated_event)
}

pub async fn instantiate_with_code(
    signed_connection: SignedConnection,
    command: ContractInstantiateWithCode,
) -> anyhow::Result<InstantiateWithCodeReturnValue> {
    let ContractInstantiateWithCode {
        wasm_path,
        metadata_path,
        constructor,
        args,
        options,
    } = command;

    let ContractOptions {
        balance,
        gas_limit,
        storage_deposit_limit,
    } = options;

    let wasm = fs::read(wasm_path).expect("WASM artifact not found");
    debug!(target: "contracts", "Found WASM contract code {:?}", wasm);

    let metadata = load_metadata(&metadata_path)?;
    let transcoder = ContractMessageTranscoder::new(metadata);
    let data = transcoder.encode(&constructor, args.unwrap_or_default())?;

    debug!("Encoded constructor data {:?}", data);

    let signer_id = signed_connection.account_id().clone();
    let connection_0 = signed_connection.clone();
    let connection_1 = signed_connection.clone();

    let event_handler_0 = tokio::spawn(async move {
        connection_0
            .wait_for_event(
                |e: &CodeStored| {
                    info!(target : "contracts", "Received CodeStored event {:?}", e);
                    // TODO : can we pre-calculate what the code hash will be?
                    true
                },
                BlockStatus::Finalized,
            )
            .await
    });

    let event_handler_1 = tokio::spawn(async move {
        connection_1
            .wait_for_event(
                |e: &Instantiated| {
                    info!(target : "contracts", "Received ContractInstantiated event {:?}", e);
                    signer_id.eq(&e.deployer)
                },
                BlockStatus::Finalized,
            )
            .await
    });

    let _tx_info = signed_connection
        .instantiate_with_code(
            wasm,
            balance,
            Weight::new(gas_limit, u64::MAX),
            storage_deposit(storage_deposit_limit),
            data,
            vec![],
            TxStatus::InBlock,
        )
        .await?;

    let code_stored_event = event_handler_0.await?;
    let contract_instantiated_event = event_handler_1.await?;

    Ok(InstantiateWithCodeReturnValue {
        contract: contract_instantiated_event.contract,
        code_hash: code_stored_event.code_hash,
    })
}

pub async fn call(
    signed_connection: SignedConnection,
    command: ContractCall,
) -> anyhow::Result<()> {
    let ContractCall {
        destination,
        message,
        args,
        metadata_path,
        options,
    } = command;

    let ContractOptions {
        balance,
        gas_limit,
        storage_deposit_limit,
    } = options;

    let metadata = load_metadata(&metadata_path)?;
    let transcoder = ContractMessageTranscoder::new(metadata);
    let data = transcoder.encode(&message, args.unwrap_or_default())?;

    debug!("Encoded call data {:?}", data);

    let _tx_info = signed_connection
        .call(
            destination,
            balance,
            Weight::new(gas_limit, u64::MAX),
            storage_deposit(storage_deposit_limit),
            data,
            TxStatus::InBlock,
        )
        .await?;

    Ok(())
}

pub async fn owner_info(connection: Connection, command: ContractOwnerInfo) -> Option<OwnerInfo> {
    let ContractOwnerInfo { code_hash } = command;

    connection.get_owner_info(code_hash, None).await
}

pub async fn remove_code(
    signed_connection: SignedConnection,
    command: ContractRemoveCode,
) -> anyhow::Result<CodeRemoved> {
    let ContractRemoveCode { code_hash } = command;

    let connection = signed_connection.clone();

    let event_handler = tokio::spawn(async move {
        connection
            .wait_for_event(
                |e: &CodeRemoved| {
                    info!(target : "contracts", "Received ContractCodeRemoved event {:?}", e);
                    e.code_hash.eq(&code_hash)
                },
                BlockStatus::Finalized,
            )
            .await
    });

    let _tx_info = signed_connection
        .remove_code(code_hash, TxStatus::InBlock)
        .await?;

    let contract_removed_event = event_handler.await?;

    Ok(contract_removed_event)
}

fn load_metadata(path: &Path) -> anyhow::Result<ink_metadata::InkProject> {
    let file = File::open(path).expect("Failed to open metadata file");
    let metadata: ContractMetadata =
        serde_json::from_reader(file).expect("Failed to deserialize metadata file");
    let ink_metadata = serde_json::from_value(serde_json::Value::Object(metadata.abi))
        .expect("Failed to deserialize ink project metadata");

    Ok(ink_metadata)
}
