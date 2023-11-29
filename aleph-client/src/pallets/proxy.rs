use subxt::utils::MultiAddress;

use crate::{
    aleph_runtime::{ProxyType, RuntimeCall},
    api, AccountId, SignedConnectionApi, TxInfo, TxStatus,
};

/// any object that implements pallet proxy api
#[async_trait::async_trait]
pub trait ProxyUserApi {
    /// API for [`proxy`](https://paritytech.github.io/polkadot-sdk/master/pallet_proxy/pallet/struct.Pallet.html#method.proxy) call.
    async fn proxy(
        &self,
        real: AccountId,
        call: RuntimeCall,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`add_proxy`](https://paritytech.github.io/polkadot-sdk/master/pallet_proxy/pallet/struct.Pallet.html#method.add_proxy) call.
    async fn add_proxy(
        &self,
        delegate: AccountId,
        proxy_type: ProxyType,
        delay: u32,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`remove_proxy`](https://paritytech.github.io/polkadot-sdk/master/pallet_proxy/pallet/struct.Pallet.html#method.remove_proxy) call.
    async fn remove_proxy(
        &self,
        delegate: AccountId,
        proxy_type: ProxyType,
        delay: u32,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> ProxyUserApi for S {
    async fn proxy(
        &self,
        real: AccountId,
        call: RuntimeCall,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx()
            .proxy()
            .proxy(MultiAddress::Id(real.into()), None, call);

        self.send_tx(tx, status).await
    }
    async fn add_proxy(
        &self,
        delegate: AccountId,
        proxy_type: ProxyType,
        delay: u32,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx()
            .proxy()
            .add_proxy(MultiAddress::Id(delegate.into()), proxy_type, delay);

        self.send_tx(tx, status).await
    }
    async fn remove_proxy(
        &self,
        delegate: AccountId,
        proxy_type: ProxyType,
        delay: u32,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx =
            api::tx()
                .proxy()
                .remove_proxy(MultiAddress::Id(delegate.into()), proxy_type, delay);

        self.send_tx(tx, status).await
    }
}
