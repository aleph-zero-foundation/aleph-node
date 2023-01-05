use codec::Decode;

use crate::{aleph_runtime::SessionKeys, connections::AsConnection};

/// Implements RPC calls for  [`author`](https://paritytech.github.io/substrate/master/sc_rpc/author/struct.Author.html) pallet
#[async_trait::async_trait]
pub trait AuthorRpc {
    /// API for [`rotate_keys`](https://paritytech.github.io/substrate/master/sc_rpc/author/struct.Author.html#method.rotate_keys) call
    async fn author_rotate_keys(&self) -> anyhow::Result<SessionKeys>;
}

#[async_trait::async_trait]
impl<C: AsConnection + Sync> AuthorRpc for C {
    async fn author_rotate_keys(&self) -> anyhow::Result<SessionKeys> {
        let bytes = self.as_connection().as_client().rpc().rotate_keys().await?;
        SessionKeys::decode(&mut bytes.0.as_slice()).map_err(|e| e.into())
    }
}
