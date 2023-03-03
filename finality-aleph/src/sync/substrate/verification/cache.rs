use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{Display, Error as FmtError, Formatter},
};

use aleph_primitives::BlockNumber;
use sp_runtime::SaturatedConversion;

use crate::{
    session::{first_block_of_session, session_id_from_block_num, SessionId},
    session_map::AuthorityProvider,
    sync::substrate::verification::{verifier::SessionVerifier, FinalizationInfo},
    SessionPeriod,
};

/// Ways in which a justification can fail verification.
#[derive(Debug, PartialEq, Eq)]
pub enum CacheError {
    UnknownAuthorities(SessionId),
    SessionTooOld(SessionId, SessionId),
    SessionInFuture(SessionId, SessionId),
}

impl Display for CacheError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use CacheError::*;
        match self {
            SessionTooOld(session, lower_bound) => write!(
                f,
                "session {:?} is too old. Should be at least {:?}",
                session, lower_bound
            ),
            SessionInFuture(session, upper_bound) => write!(
                f,
                "session {:?} without known authorities. Should be at most {:?}",
                session, upper_bound
            ),
            UnknownAuthorities(session) => {
                write!(
                    f,
                    "authorities for session {:?} not known even though they should be",
                    session
                )
            }
        }
    }
}

/// Cache storing SessionVerifier structs for multiple sessions. Keeps up to `cache_size` verifiers of top sessions.
/// If the session is too new or ancient it will fail to return a SessionVerifier.
/// Highest session verifier this cache returns is for the session after the current finalization session.
/// Lowest session verifier this cache returns is for `top_returned_session` - `cache_size`.
pub struct VerifierCache<AP, FI>
where
    AP: AuthorityProvider<BlockNumber>,
    FI: FinalizationInfo,
{
    sessions: HashMap<SessionId, SessionVerifier>,
    session_period: SessionPeriod,
    finalization_info: FI,
    authority_provider: AP,
    cache_size: usize,
    /// Lowest currently available session.
    lower_bound: SessionId,
}

impl<AP, FI> VerifierCache<AP, FI>
where
    AP: AuthorityProvider<BlockNumber>,
    FI: FinalizationInfo,
{
    pub fn new(
        session_period: SessionPeriod,
        finalization_info: FI,
        authority_provider: AP,
        cache_size: usize,
    ) -> Self {
        Self {
            sessions: HashMap::new(),
            session_period,
            finalization_info,
            authority_provider,
            cache_size,
            lower_bound: SessionId(0),
        }
    }
}

/// Download authorities for the session and return `SessionVerifier` for them. `session_id` should be the first session,
/// or the first block from the session number `session_id - 1` should be finalized.
fn download_session_verifier<AP: AuthorityProvider<BlockNumber>>(
    authority_provider: &AP,
    session_id: SessionId,
    session_period: SessionPeriod,
) -> Option<SessionVerifier> {
    let maybe_authority_data = match session_id {
        SessionId(0) => authority_provider.authority_data(0),
        SessionId(id) => {
            let prev_first = first_block_of_session(SessionId(id - 1), session_period);
            authority_provider.next_authority_data(prev_first)
        }
    };

    maybe_authority_data.map(|a| a.into())
}

impl<AP, FI> VerifierCache<AP, FI>
where
    AP: AuthorityProvider<BlockNumber>,
    FI: FinalizationInfo,
{
    /// Prune all sessions with a number smaller than `session_id`
    fn prune(&mut self, session_id: SessionId) {
        self.sessions.retain(|&id, _| id >= session_id);
        self.lower_bound = session_id;
    }

    /// Returns session verifier for block number if available. Updates cache if necessary.
    pub fn get(&mut self, number: BlockNumber) -> Result<&SessionVerifier, CacheError> {
        let session_id = session_id_from_block_num(number, self.session_period);

        if session_id < self.lower_bound {
            return Err(CacheError::SessionTooOld(session_id, self.lower_bound));
        }

        // We are sure about authorities in all session that have first block from previous session finalized.
        let upper_bound = SessionId(
            session_id_from_block_num(
                self.finalization_info.finalized_number(),
                self.session_period,
            )
            .0 + 1,
        );
        if session_id > upper_bound {
            return Err(CacheError::SessionInFuture(session_id, upper_bound));
        }

        if session_id.0
            >= self
                .lower_bound
                .0
                .saturating_add(self.cache_size.saturated_into())
        {
            self.prune(SessionId(
                session_id
                    .0
                    .saturating_sub(self.cache_size.saturated_into())
                    + 1,
            ));
        }

        let verifier = match self.sessions.entry(session_id) {
            Entry::Occupied(occupied) => occupied.into_mut(),
            Entry::Vacant(vacant) => {
                let verifier = download_session_verifier(
                    &self.authority_provider,
                    session_id,
                    self.session_period,
                )
                .ok_or(CacheError::UnknownAuthorities(session_id))?;
                vacant.insert(verifier)
            }
        };

        Ok(verifier)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::HashMap};

    use aleph_primitives::SessionAuthorityData;
    use sp_runtime::SaturatedConversion;

    use super::{
        AuthorityProvider, BlockNumber, CacheError, FinalizationInfo, SessionVerifier,
        VerifierCache,
    };
    use crate::{
        session::{session_id_from_block_num, testing::authority_data, SessionId},
        SessionPeriod,
    };

    const SESSION_PERIOD: u32 = 30;
    const CACHE_SIZE: usize = 2;

    type TestVerifierCache<'a> = VerifierCache<MockAuthorityProvider, MockFinalizationInfo<'a>>;

    struct MockFinalizationInfo<'a> {
        finalized_number: &'a Cell<BlockNumber>,
    }

    impl<'a> FinalizationInfo for MockFinalizationInfo<'a> {
        fn finalized_number(&self) -> BlockNumber {
            self.finalized_number.get()
        }
    }

    struct MockAuthorityProvider {
        session_map: HashMap<SessionId, SessionAuthorityData>,
        session_period: SessionPeriod,
    }

    fn authority_data_for_session(session_id: u64) -> SessionAuthorityData {
        authority_data(session_id * 4, (session_id + 1) * 4)
    }

    impl MockAuthorityProvider {
        fn new(session_n: u64) -> Self {
            let session_map = (0..session_n + 1)
                .map(|s| (SessionId(s.saturated_into()), authority_data_for_session(s)))
                .collect();

            Self {
                session_map,
                session_period: SessionPeriod(SESSION_PERIOD),
            }
        }
    }

    impl AuthorityProvider<BlockNumber> for MockAuthorityProvider {
        fn authority_data(&self, block: BlockNumber) -> Option<SessionAuthorityData> {
            self.session_map
                .get(&session_id_from_block_num(block, self.session_period))
                .cloned()
        }

        fn next_authority_data(&self, block: BlockNumber) -> Option<SessionAuthorityData> {
            self.session_map
                .get(&SessionId(
                    session_id_from_block_num(block, self.session_period).0 + 1,
                ))
                .cloned()
        }
    }

    fn setup_test(max_session_n: u64, finalized_number: &'_ Cell<u32>) -> TestVerifierCache<'_> {
        let finalization_info = MockFinalizationInfo { finalized_number };
        let authority_provider = MockAuthorityProvider::new(max_session_n);

        VerifierCache::new(
            SessionPeriod(SESSION_PERIOD),
            finalization_info,
            authority_provider,
            CACHE_SIZE,
        )
    }

    fn finalize_first_in_session(finalized_number: &Cell<u32>, session_id: u32) {
        finalized_number.set(session_id * SESSION_PERIOD);
    }

    fn session_verifier(
        verifier: &mut TestVerifierCache,
        session_id: u32,
    ) -> Result<SessionVerifier, CacheError> {
        verifier.get((session_id + 1) * SESSION_PERIOD - 1).cloned()
    }

    fn check_session_verifier(verifier: &mut TestVerifierCache, session_id: u32) {
        let session_verifier =
            session_verifier(verifier, session_id).expect("Should return verifier. Got error");
        let expected_verifier: SessionVerifier =
            authority_data_for_session(session_id as u64).into();
        assert_eq!(session_verifier, expected_verifier);
    }

    #[test]
    fn genesis_session() {
        let finalized_number = Cell::new(0);

        let mut verifier = setup_test(0, &finalized_number);

        check_session_verifier(&mut verifier, 0);
    }

    #[test]
    fn normal_session() {
        let finalized_number = Cell::new(0);

        let mut verifier = setup_test(3, &finalized_number);

        check_session_verifier(&mut verifier, 0);
        check_session_verifier(&mut verifier, 1);

        finalize_first_in_session(&finalized_number, 1);
        check_session_verifier(&mut verifier, 0);
        check_session_verifier(&mut verifier, 1);
        check_session_verifier(&mut verifier, 2);

        finalize_first_in_session(&finalized_number, 2);
        check_session_verifier(&mut verifier, 1);
        check_session_verifier(&mut verifier, 2);
        check_session_verifier(&mut verifier, 3);
    }

    #[test]
    fn prunes_old_sessions() {
        let finalized_number = Cell::new(0);

        let mut verifier = setup_test(3, &finalized_number);

        check_session_verifier(&mut verifier, 0);
        check_session_verifier(&mut verifier, 1);

        finalize_first_in_session(&finalized_number, 1);
        check_session_verifier(&mut verifier, 2);

        // Should no longer have verifier for session 0
        assert_eq!(
            session_verifier(&mut verifier, 0),
            Err(CacheError::SessionTooOld(SessionId(0), SessionId(1)))
        );

        finalize_first_in_session(&finalized_number, 2);
        check_session_verifier(&mut verifier, 3);

        // Should no longer have verifier for session 1
        assert_eq!(
            session_verifier(&mut verifier, 1),
            Err(CacheError::SessionTooOld(SessionId(1), SessionId(2)))
        );
    }

    #[test]
    fn session_from_future() {
        let finalized_number = Cell::new(0);

        let mut verifier = setup_test(3, &finalized_number);

        finalize_first_in_session(&finalized_number, 1);

        // Did not finalize first block in session 2 yet
        assert_eq!(
            session_verifier(&mut verifier, 3),
            Err(CacheError::SessionInFuture(SessionId(3), SessionId(2)))
        );
    }

    #[test]
    fn authority_provider_error() {
        let finalized_number = Cell::new(0);
        let mut verifier = setup_test(0, &finalized_number);

        assert_eq!(
            session_verifier(&mut verifier, 1),
            Err(CacheError::UnknownAuthorities(SessionId(1)))
        );

        finalize_first_in_session(&finalized_number, 1);

        assert_eq!(
            session_verifier(&mut verifier, 2),
            Err(CacheError::UnknownAuthorities(SessionId(2)))
        );
    }
}
