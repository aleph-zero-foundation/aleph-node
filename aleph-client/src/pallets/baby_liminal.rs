use anyhow::Result;
pub use pallet_baby_liminal::VerificationKeyIdentifier;

use crate::{
    aleph_runtime::RuntimeCall::BabyLiminal,
    api,
    pallet_baby_liminal::{
        pallet::Call::{delete_key, overwrite_key},
        systems::ProvingSystem,
    },
    BlockHash, RootConnection, SignedConnection, SudoCall, TxStatus,
};

#[async_trait::async_trait]
pub trait BabyLiminalUserApi {
    async fn store_key(
        &self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
        status: TxStatus,
    ) -> Result<BlockHash>;

    async fn verify(
        &self,
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        public_input: Vec<u8>,
        system: ProvingSystem,
        status: TxStatus,
    ) -> Result<BlockHash>;
}

#[async_trait::async_trait]
pub trait BabyLiminalSudoApi {
    async fn delete_key(
        &self,
        identifier: VerificationKeyIdentifier,
        status: TxStatus,
    ) -> Result<BlockHash>;

    async fn overwrite_key(
        &self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
        status: TxStatus,
    ) -> Result<BlockHash>;
}

#[async_trait::async_trait]
impl BabyLiminalUserApi for SignedConnection {
    async fn store_key(
        &self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
        status: TxStatus,
    ) -> Result<BlockHash> {
        let tx = api::tx().baby_liminal().store_key(identifier, key);
        self.send_tx(tx, status).await
    }

    async fn verify(
        &self,
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        public_input: Vec<u8>,
        system: ProvingSystem,
        status: TxStatus,
    ) -> Result<BlockHash> {
        let tx = api::tx()
            .baby_liminal()
            .verify(identifier, proof, public_input, system);
        self.send_tx(tx, status).await
    }
}

#[async_trait::async_trait]
impl BabyLiminalSudoApi for RootConnection {
    async fn delete_key(
        &self,
        identifier: VerificationKeyIdentifier,
        status: TxStatus,
    ) -> Result<BlockHash> {
        let call = BabyLiminal(delete_key { identifier });
        self.sudo_unchecked(call, status).await
    }

    async fn overwrite_key(
        &self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
        status: TxStatus,
    ) -> Result<BlockHash> {
        let call = BabyLiminal(overwrite_key { identifier, key });
        self.sudo_unchecked(call, status).await
    }
}
