use std::sync::Arc;

use primitives::{AccountId, AlephSessionApi, AuraId, BlockHash, BlockNumber};
use sc_client_api::Backend;
use sp_consensus_aura::AuraApi;
use sp_runtime::traits::{Block, Header};

use crate::{
    abft::NodeIndex,
    runtime_api::RuntimeApi,
    session::{SessionBoundaryInfo, SessionId},
    session_map::{AuthorityProvider, AuthorityProviderImpl},
    ClientForAleph,
};

pub trait ValidatorIndexToAccountIdConverter {
    fn account(&self, session: SessionId, validator_index: NodeIndex) -> Option<AccountId>;
}

pub struct ValidatorIndexToAccountIdConverterImpl<C, B, BE, RA>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: crate::aleph_primitives::AlephSessionApi<B> + AuraApi<B, AuraId>,
    B: Block<Hash = BlockHash>,
    BE: Backend<B> + 'static,
    RA: RuntimeApi,
{
    client: Arc<C>,
    session_boundary_info: SessionBoundaryInfo,
    authority_provider: AuthorityProviderImpl<C, B, BE, RA>,
}

impl<C, B, BE, RA> ValidatorIndexToAccountIdConverterImpl<C, B, BE, RA>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: crate::aleph_primitives::AlephSessionApi<B> + AuraApi<B, AuraId>,
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
    BE: Backend<B> + 'static,
    RA: RuntimeApi,
{
    pub fn new(client: Arc<C>, session_boundary_info: SessionBoundaryInfo, api: RA) -> Self {
        Self {
            client: client.clone(),
            session_boundary_info,
            authority_provider: AuthorityProviderImpl::new(client, api),
        }
    }
}

impl<C, B, BE, RA> ValidatorIndexToAccountIdConverter
    for ValidatorIndexToAccountIdConverterImpl<C, B, BE, RA>
where
    C: ClientForAleph<B, BE> + Send + Sync + 'static,
    C::Api: crate::aleph_primitives::AlephSessionApi<B> + AuraApi<B, AuraId>,
    B: Block<Hash = BlockHash>,
    B::Header: Header<Number = BlockNumber>,
    BE: Backend<B> + 'static,
    RA: RuntimeApi,
{
    fn account(&self, session: SessionId, validator_index: NodeIndex) -> Option<AccountId> {
        let block_number = self
            .session_boundary_info
            .boundaries_for_session(session)
            .first_block();
        let block_hash = self.client.block_hash(block_number).ok()??;

        let authority_data = self.authority_provider.authority_data(block_number)?;
        let aleph_key = authority_data.authorities()[validator_index.0].clone();
        self.client
            .runtime_api()
            .key_owner(block_hash, aleph_key)
            .ok()?
    }
}

#[cfg(test)]
pub struct MockConverter;

#[cfg(test)]
impl ValidatorIndexToAccountIdConverter for MockConverter {
    fn account(&self, _: SessionId, _: NodeIndex) -> Option<AccountId> {
        None
    }
}
