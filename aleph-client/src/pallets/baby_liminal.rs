use anyhow::Result;

/// Verification key identifier alias, copied from `pallet_baby_liminal`.
pub type VerificationKeyIdentifier = [u8; 8];

use crate::{
    aleph_runtime::RuntimeCall::BabyLiminal,
    api,
    pallet_baby_liminal::pallet::Call::{delete_key, overwrite_key},
    RootConnection, SignedConnection, SignedConnectionApi, SudoCall, TxInfo, TxStatus,
};

/// Pallet baby liminal API.
#[async_trait::async_trait]
pub trait BabyLiminalUserApi {
    /// Store verifying key in pallet's storage.
    async fn store_key(
        &self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
        status: TxStatus,
    ) -> Result<TxInfo>;

    /// Verify a proof.
    async fn verify(
        &self,
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        public_input: Vec<u8>,
        status: TxStatus,
    ) -> Result<TxInfo>;
}

/// Pallet baby liminal API that requires sudo.
#[async_trait::async_trait]
pub trait BabyLiminalSudoApi {
    /// Delete verifying key from pallet's storage.
    async fn delete_key(
        &self,
        identifier: VerificationKeyIdentifier,
        status: TxStatus,
    ) -> Result<TxInfo>;

    /// Overwrite verifying key in pallet's storage.
    async fn overwrite_key(
        &self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
        status: TxStatus,
    ) -> Result<TxInfo>;
}

#[async_trait::async_trait]
impl BabyLiminalUserApi for SignedConnection {
    async fn store_key(
        &self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
        status: TxStatus,
    ) -> Result<TxInfo> {
        let tx = api::tx().baby_liminal().store_key(identifier, key);
        self.send_tx(tx, status).await
    }

    async fn verify(
        &self,
        identifier: VerificationKeyIdentifier,
        proof: Vec<u8>,
        public_input: Vec<u8>,
        status: TxStatus,
    ) -> Result<TxInfo> {
        let tx = api::tx()
            .baby_liminal()
            .verify(identifier, proof, public_input);
        self.send_tx(tx, status).await
    }
}

#[async_trait::async_trait]
impl BabyLiminalSudoApi for RootConnection {
    async fn delete_key(
        &self,
        identifier: VerificationKeyIdentifier,
        status: TxStatus,
    ) -> Result<TxInfo> {
        let call = BabyLiminal(delete_key { identifier });
        self.sudo_unchecked(call, status).await
    }

    async fn overwrite_key(
        &self,
        identifier: VerificationKeyIdentifier,
        key: Vec<u8>,
        status: TxStatus,
    ) -> Result<TxInfo> {
        let call = BabyLiminal(overwrite_key { identifier, key });
        self.sudo_unchecked(call, status).await
    }
}
