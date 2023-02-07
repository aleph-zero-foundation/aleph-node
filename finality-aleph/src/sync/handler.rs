use crate::{
    session::{last_block_of_session, session_id_from_block_num, SessionId, SessionPeriod},
    sync::{
        data::{NetworkData, State},
        forest::{Error as ForestError, Forest, JustificationAddResult},
        BlockIdFor, BlockIdentifier, ChainStatus, Finalizer, Header, Justification, PeerId,
        Verifier,
    },
};

/// How many justifications we will send at most in response to a broadcast.
const MAX_SMALL_JUSTIFICATION_BATCH: u32 = 10;
// Silly workaround to make range matching actually work...
const MAX_SMALL_JUSTIFICATION_BATCH_PLUS_ONE: u32 = MAX_SMALL_JUSTIFICATION_BATCH + 1;
/// How many justifications we will send at most in response to an explicit query.
const MAX_JUSTIFICATION_BATCH: u32 = 100;

/// Handler for data incoming from the network.
pub struct Handler<I: PeerId, J: Justification, CS: ChainStatus<J>, V: Verifier<J>, F: Finalizer<J>>
{
    chain_status: CS,
    verifier: V,
    finalizer: F,
    forest: Forest<I, J>,
    period: SessionPeriod,
}

/// What actions can the handler recommend as a reaction to some data.
#[derive(Clone, Debug)]
pub enum SyncActions<J: Justification> {
    /// A response for the peer that sent us the data.
    Response(NetworkData<J>),
    /// A task that should be performed periodically. At the moment these are only requests for blocks,
    /// so it always contains the id of the block.
    Task(BlockIdFor<J>),
    /// Do nothing.
    Noop,
}

impl<J: Justification> SyncActions<J> {
    fn noop() -> Self {
        SyncActions::Noop
    }

    fn response(response: NetworkData<J>) -> Self {
        SyncActions::Response(response)
    }

    fn task(id: BlockIdFor<J>) -> Self {
        SyncActions::Task(id)
    }
}

/// What can go wrong when handling a piece of data.
#[derive(Clone, Debug)]
pub enum Error<J: Justification, CS: ChainStatus<J>, V: Verifier<J>, F: Finalizer<J>> {
    Verifier(V::Error),
    ChainStatus(CS::Error),
    Finalizer(F::Error),
    Forest(ForestError),
    NoParent,
    MissingJustification,
}

impl<J: Justification, CS: ChainStatus<J>, V: Verifier<J>, F: Finalizer<J>> From<ForestError>
    for Error<J, CS, V, F>
{
    fn from(e: ForestError) -> Self {
        Error::Forest(e)
    }
}

impl<I: PeerId, J: Justification, CS: ChainStatus<J>, V: Verifier<J>, F: Finalizer<J>>
    Handler<I, J, CS, V, F>
{
    /// New handler with the provided chain interfaces.
    pub fn new(
        chain_status: CS,
        verifier: V,
        finalizer: F,
        period: SessionPeriod,
    ) -> Result<Self, Error<J, CS, V, F>> {
        let top_finalized = chain_status
            .top_finalized()
            .map_err(Error::ChainStatus)?
            .header()
            .id();
        let forest = Forest::new(top_finalized);
        Ok(Handler {
            chain_status,
            verifier,
            finalizer,
            forest,
            period,
        })
    }

    fn large_justification_batch_from(
        &self,
        id: BlockIdFor<J>,
    ) -> Result<NetworkData<J>, Error<J, CS, V, F>> {
        use Error::*;
        let mut result = Vec::with_capacity(MAX_SMALL_JUSTIFICATION_BATCH as usize);
        let mut number = id.number() + 1;
        let top_finalized_number = self
            .chain_status
            .top_finalized()
            .map_err(ChainStatus)?
            .header()
            .id()
            .number();
        while result.len() < MAX_JUSTIFICATION_BATCH as usize && number <= top_finalized_number {
            number = match self
                .chain_status
                .finalized_at(number)
                .map_err(ChainStatus)?
            {
                Some(justification) => {
                    result.push(justification.into_unverified());
                    number + 1
                }
                // We might skip all blocks of a session if we are missing a justification, but
                // this will happen only for sessions where we don't have all the justifications.
                // The last block of a session was always guaranteed to contain a justification, so
                // we only share that one. It can be missing only if the last block is above the
                // top finalized, in which case this will break the loop.
                None => last_block_of_session(
                    session_id_from_block_num(number, self.period),
                    self.period,
                ),
            }
        }
        Ok(NetworkData::Justifications(result, self.state()?))
    }

    fn small_justification_batch_from(
        &self,
        id: BlockIdFor<J>,
    ) -> Result<NetworkData<J>, Error<J, CS, V, F>> {
        let mut result = Vec::with_capacity(MAX_SMALL_JUSTIFICATION_BATCH as usize);
        let mut number = id.number() + 1;
        while result.len() < MAX_SMALL_JUSTIFICATION_BATCH as usize {
            match self
                .chain_status
                .finalized_at(number)
                .map_err(Error::ChainStatus)?
            {
                Some(justification) => result.push(justification.into_unverified()),
                None => break,
            }
            number += 1;
        }
        Ok(NetworkData::Justifications(result, self.state()?))
    }

    fn top_understandable_for(
        &self,
        id: BlockIdFor<J>,
    ) -> Result<NetworkData<J>, Error<J, CS, V, F>> {
        use Error::*;
        let block_session = session_id_from_block_num(id.number(), self.period);
        let understandable_session = SessionId(block_session.0 + 1);
        let last_understandable_block_number =
            last_block_of_session(understandable_session, self.period);
        match self
            .chain_status
            .finalized_at(last_understandable_block_number)
            .map_err(ChainStatus)?
        {
            Some(justification) => Ok(NetworkData::Justifications(
                vec![justification.into_unverified()],
                self.state()?,
            )),
            None => {
                let justification = self.chain_status.top_finalized().map_err(ChainStatus)?;
                match justification.header().id().number() <= last_understandable_block_number {
                    true => Ok(NetworkData::Justifications(
                        vec![justification.into_unverified()],
                        self.state()?,
                    )),
                    false => Err(Error::MissingJustification),
                }
            }
        }
    }

    fn try_finalize(&mut self) -> Result<(), Error<J, CS, V, F>> {
        while let Some(justification) = self.forest.try_finalize() {
            self.finalizer
                .finalize(justification)
                .map_err(Error::Finalizer)?;
        }
        Ok(())
    }

    fn handle_verified_justification(
        &mut self,
        justification: J,
        peer: I,
    ) -> Result<SyncActions<J>, Error<J, CS, V, F>> {
        use JustificationAddResult::*;
        let id = justification.header().id();
        match self
            .forest
            .update_justification(justification, Some(peer))?
        {
            Noop => Ok(SyncActions::noop()),
            Required => Ok(SyncActions::task(id)),
            Finalizable => {
                self.try_finalize()?;
                Ok(SyncActions::noop())
            }
        }
    }

    /// Inform the handler that a block has been imported.
    pub fn block_imported(&mut self, header: J::Header) -> Result<(), Error<J, CS, V, F>> {
        match self.forest.update_body(&header)? {
            true => self.try_finalize(),
            false => Ok(()),
        }
    }

    /// Handle a request for potentially substantial amounts of data.
    ///
    /// Currently ignores the requested id, it will only become important once we can request
    /// blocks.
    pub fn handle_request(
        &mut self,
        _requested_id: BlockIdFor<J>,
        state: State<J>,
    ) -> Result<SyncActions<J>, Error<J, CS, V, F>> {
        let remote_top_id = self
            .verifier
            .verify(state.top_justification())
            .map_err(Error::Verifier)?
            .header()
            .id();
        Ok(SyncActions::response(
            self.large_justification_batch_from(remote_top_id)?,
        ))
    }

    /// Handle a single justification.
    pub fn handle_justification(
        &mut self,
        justification: J::Unverified,
        peer: I,
    ) -> Result<SyncActions<J>, Error<J, CS, V, F>> {
        let justification = self
            .verifier
            .verify(justification)
            .map_err(Error::Verifier)?;
        self.handle_verified_justification(justification, peer)
    }

    /// Handle a state broadcast returning the actions we should take in response.
    pub fn handle_state(
        &mut self,
        state: State<J>,
        peer: I,
    ) -> Result<SyncActions<J>, Error<J, CS, V, F>> {
        use Error::*;
        let remote_top = self
            .verifier
            .verify(state.top_justification())
            .map_err(Verifier)?;
        let local_top = self.chain_status.top_finalized().map_err(ChainStatus)?;
        match local_top
            .header()
            .id()
            .number()
            .checked_sub(remote_top.header().id().number())
        {
            // If we are just one behind then normal broadcasts should remedy the situation.
            Some(0..=1) => Ok(SyncActions::noop()),
            Some(2..=MAX_SMALL_JUSTIFICATION_BATCH) => Ok(SyncActions::response(
                self.small_justification_batch_from(remote_top.header().id())?,
            )),
            Some(MAX_SMALL_JUSTIFICATION_BATCH_PLUS_ONE..) => Ok(SyncActions::response(
                self.top_understandable_for(remote_top.header().id())?,
            )),
            None => self.handle_verified_justification(remote_top, peer),
        }
    }

    /// The current state of our database.
    pub fn state(&self) -> Result<State<J>, Error<J, CS, V, F>> {
        let top_justification = self
            .chain_status
            .top_finalized()
            .map_err(Error::ChainStatus)?
            .into_unverified();
        Ok(State::new(top_justification))
    }
}

#[cfg(test)]
mod tests {
    use super::{Handler, SyncActions};
    use crate::{
        sync::{
            data::NetworkData,
            mock::{Backend, MockHeader, MockJustification, MockPeerId, MockVerifier},
            ChainStatus, Header, Justification,
        },
        SessionPeriod,
    };

    type MockHandler = Handler<MockPeerId, MockJustification, Backend, MockVerifier, Backend>;

    const SESSION_PERIOD: usize = 20;

    fn setup() -> (MockHandler, Backend, impl Send) {
        let (backend, _keep) = Backend::setup();
        let verifier = MockVerifier {};
        let handler = Handler::new(
            backend.clone(),
            verifier,
            backend.clone(),
            SessionPeriod(20),
        )
        .expect("mock backend works");
        (handler, backend, _keep)
    }

    fn import_branch(backend: &Backend, branch_length: usize) -> Vec<MockHeader> {
        let result: Vec<_> = backend
            .best_block()
            .expect("mock backend works")
            .random_branch()
            .take(branch_length)
            .collect();
        for header in &result {
            backend.import(header.clone());
        }
        result
    }

    #[test]
    fn finalizes_imported_and_justified() {
        let (mut handler, backend, _keep) = setup();
        let header = import_branch(&backend, 1)[0].clone();
        handler
            .block_imported(header.clone())
            .expect("importing in order");
        let justification = MockJustification::for_header(header);
        let peer = rand::random();
        assert!(matches!(
            handler
                .handle_justification(justification.clone().into_unverified(), peer)
                .expect("correct justification"),
            SyncActions::Noop
        ));
        assert_eq!(
            backend.top_finalized().expect("mock backend works"),
            justification
        );
    }

    #[test]
    fn finalizes_justified_and_imported() {
        let (mut handler, backend, _keep) = setup();
        let header = import_branch(&backend, 1)[0].clone();
        let justification = MockJustification::for_header(header.clone());
        let peer = rand::random();
        match handler
            .handle_justification(justification.clone().into_unverified(), peer)
            .expect("correct justification")
        {
            SyncActions::Task(id) => assert_eq!(id, header.id()),
            other_action => panic!("expected a task, got {:?}", other_action),
        }
        handler.block_imported(header).expect("importing in order");
        assert_eq!(
            backend.top_finalized().expect("mock backend works"),
            justification
        );
    }

    #[test]
    fn handles_state_with_small_difference() {
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        for justification in import_branch(&backend, 5)
            .into_iter()
            .map(MockJustification::for_header)
        {
            handler
                .block_imported(justification.header().clone())
                .expect("importing in order");
            handler
                .handle_justification(justification.clone().into_unverified(), peer)
                .expect("correct justification");
        }
        match handler
            .handle_state(initial_state, peer)
            .expect("correct justification")
        {
            SyncActions::Response(NetworkData::Justifications(justifications, _)) => {
                assert_eq!(justifications.len(), 5)
            }
            other_action => panic!(
                "expected a response with justifications, got {:?}",
                other_action
            ),
        }
    }

    #[test]
    fn handles_state_with_large_difference() {
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        let justifications: Vec<_> = import_branch(&backend, 500)
            .into_iter()
            .map(MockJustification::for_header)
            .collect();
        for justification in &justifications {
            handler
                .block_imported(justification.header().clone())
                .expect("importing in order");
            handler
                .handle_justification(justification.clone().into_unverified(), peer)
                .expect("correct justification");
        }
        match handler
            .handle_state(initial_state, peer)
            .expect("correct justification")
        {
            SyncActions::Response(NetworkData::Justifications(sent_justifications, _)) => {
                assert_eq!(sent_justifications.len(), 1);
                assert_eq!(
                    sent_justifications[0].header().id(),
                    justifications[SESSION_PERIOD * 2 - 2].header().id()
                );
            }
            other_action => panic!(
                "expected a response with justifications, got {:?}",
                other_action
            ),
        }
    }

    #[test]
    fn handles_request() {
        let (mut handler, backend, _keep) = setup();
        let initial_state = handler.state().expect("state works");
        let peer = rand::random();
        let justifications: Vec<_> = import_branch(&backend, 500)
            .into_iter()
            .map(MockJustification::for_header)
            .collect();
        for justification in &justifications {
            handler
                .block_imported(justification.header().clone())
                .expect("importing in order");
            handler
                .handle_justification(justification.clone().into_unverified(), peer)
                .expect("correct justification");
        }
        // currently ignored, so picking a random one
        let requested_id = justifications[43].header().id();
        match handler
            .handle_request(requested_id, initial_state)
            .expect("correct request")
        {
            SyncActions::Response(NetworkData::Justifications(sent_justifications, _)) => {
                assert_eq!(sent_justifications.len(), 100);
                for (sent_justification, justification) in
                    sent_justifications.iter().zip(justifications)
                {
                    assert_eq!(
                        sent_justification.header().id(),
                        justification.header().id()
                    );
                }
            }
            other_action => panic!(
                "expected a response with justifications, got {:?}",
                other_action
            ),
        }
    }
}
