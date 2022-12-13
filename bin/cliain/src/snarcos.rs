use std::{fs, path::PathBuf};

use aleph_client::{
    pallet_snarcos::systems::ProvingSystem,
    pallets::snarcos::{SnarcosSudoApi, SnarcosUserApi, VerificationKeyIdentifier},
    RootConnection, SignedConnection, TxStatus,
};
use anyhow::Result;

fn read_bytes(file: &PathBuf) -> Result<Vec<u8>> {
    fs::read(file).map_err(|e| e.into())
}

/// Calls `pallet_snarcos::store_key`.
pub async fn store_key(
    connection: SignedConnection,
    identifier: VerificationKeyIdentifier,
    vk_file: PathBuf,
) -> Result<()> {
    let vk = read_bytes(&vk_file)?;
    connection
        .store_key(identifier, vk, TxStatus::Finalized)
        .await
        .map(|_| ())
}

/// Calls `pallet_snarcos::delete_key`.
pub async fn delete_key(
    connection: RootConnection,
    identifier: VerificationKeyIdentifier,
) -> Result<()> {
    connection
        .delete_key(identifier, TxStatus::Finalized)
        .await
        .map(|_| ())
}

/// Calls `pallet_snarcos::overwrite_key`.
pub async fn overwrite_key(
    connection: RootConnection,
    identifier: VerificationKeyIdentifier,
    vk_file: PathBuf,
) -> Result<()> {
    let vk = read_bytes(&vk_file)?;
    connection
        .overwrite_key(identifier, vk, TxStatus::Finalized)
        .await
        .map(|_| ())
}

/// Calls `pallet_snarcos::verify`.
pub async fn verify(
    connection: SignedConnection,
    identifier: VerificationKeyIdentifier,
    proof_file: PathBuf,
    public_input_file: PathBuf,
    system: ProvingSystem,
) -> Result<()> {
    let proof = read_bytes(&proof_file)?;
    let input = read_bytes(&public_input_file)?;
    connection
        .verify(identifier, proof, input, system, TxStatus::Finalized)
        .await
        .map(|_| ())
}
