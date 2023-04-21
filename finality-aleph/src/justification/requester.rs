use std::{collections::VecDeque, marker::PhantomData, time::Duration};

use log::{info, warn};
use tokio::time::sleep;

use crate::{
    justification::LOG_TARGET,
    network::RequestBlocks,
    session::SessionBoundaryInfo,
    sync::{ChainStatus, Header, Justification},
    BlockIdentifier,
};

pub struct Requester<J, RB, CS>
where
    J: Justification,
    RB: RequestBlocks<<J::Header as Header>::Identifier> + 'static,
    CS: ChainStatus<J>,
{
    block_requester: RB,
    chain_status: CS,
    boundary_info: SessionBoundaryInfo,
    _phantom: PhantomData<J>,
}

// This should only be necessary during the rolling update, so lets make the request period
// ridiculously long, so that it doesn't introduce too much weird behaviour otherwise.
const REQUEST_PERIOD: Duration = Duration::from_secs(60);

impl<J, RB, CS> Requester<J, RB, CS>
where
    J: Justification,
    RB: RequestBlocks<<J::Header as Header>::Identifier> + 'static,
    CS: ChainStatus<J>,
{
    pub fn new(block_requester: RB, chain_status: CS, boundary_info: SessionBoundaryInfo) -> Self {
        Requester {
            block_requester,
            chain_status,
            boundary_info,
            _phantom: PhantomData,
        }
    }

    fn children_ids(
        &self,
        id: <J::Header as Header>::Identifier,
    ) -> Result<VecDeque<<J::Header as Header>::Identifier>, CS::Error> {
        Ok(self
            .chain_status
            .children(id)?
            .into_iter()
            .map(|header| header.id())
            .collect())
    }

    fn perform_request(&self) -> Result<(), CS::Error> {
        let best_block_number = self.chain_status.best_block()?.id().number();
        let top_finalized = self.chain_status.top_finalized()?.header().id();
        let session_id = self
            .boundary_info
            .session_id_from_block_num(top_finalized.number());
        let last_block_number = self.boundary_info.last_block_of_session(session_id);
        let last_block_number = match last_block_number == top_finalized.number() {
            true => self.boundary_info.last_block_of_session(session_id.next()),
            false => last_block_number,
        };
        if best_block_number >= last_block_number {
            let mut blocks = self.children_ids(top_finalized)?;
            while let Some(block) = blocks.pop_front() {
                if block.number() == last_block_number {
                    info!(
                        target: LOG_TARGET,
                        "Performing auxiliary request for justification of block {:?}.", block
                    );
                    self.block_requester.request_justification(block);
                    continue;
                }
                blocks.append(&mut self.children_ids(block)?);
            }
        }
        Ok(())
    }

    pub async fn run(self) {
        loop {
            sleep(REQUEST_PERIOD).await;
            if let Err(e) = self.perform_request() {
                warn!(
                    target: LOG_TARGET,
                    "Failed to perform auxiliary justification request: {}.", e
                );
            }
        }
    }
}
