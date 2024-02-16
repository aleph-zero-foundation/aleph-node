use codec::Decode;
use subxt::{ext::sp_core::Bytes, rpc_params};

use crate::{aleph_runtime::SessionKeys, connections::AsConnection};

/// Implements RPC calls for  [`author`](https://paritytech.github.io/substrate/master/sc_rpc/author/struct.Author.html) pallet
#[async_trait::async_trait]
pub trait AuthorRpc {
    /// API for [`rotate_keys`](https://paritytech.github.io/substrate/master/sc_rpc/author/struct.Author.html#method.rotate_keys) call
    async fn author_rotate_keys(&self) -> anyhow::Result<SessionKeys>;
    /// Returns the number of extrinsics pending in RPC node's transaction pool.
    /// See [`pending_extrinsics`](https://paritytech.github.io/substrate/master/sc_rpc/author/struct.Author.html#method.pending_extrinsics).
    async fn pending_extrinsics_len(&self) -> anyhow::Result<u64>;
}

#[async_trait::async_trait]
impl<C: AsConnection + Sync> AuthorRpc for C {
    async fn author_rotate_keys(&self) -> anyhow::Result<SessionKeys> {
        let bytes = self.as_connection().as_client().rpc().rotate_keys().await?;
        SessionKeys::decode(&mut bytes.0.as_slice()).map_err(|e| e.into())
    }

    async fn pending_extrinsics_len(&self) -> anyhow::Result<u64> {
        Ok(self
            .as_connection()
            .as_client()
            .rpc()
            .request::<Vec<Bytes>>("author_pendingExtrinsics", rpc_params![])
            .await?
            .len()
            .try_into()?)
    }
}
