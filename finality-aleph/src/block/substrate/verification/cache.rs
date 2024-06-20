use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::{Debug, Display, Error as FmtError, Formatter},
};

use parity_scale_codec::Encode;
use sc_consensus_aura::standalone::{check_header_slot_and_seal, slot_author};
use sp_consensus_aura::sr25519::AuthorityPair;
use sp_consensus_slots::Slot;
use sp_runtime::{traits::Header as SubstrateHeader, SaturatedConversion};

use crate::{
    aleph_primitives::{AccountId, AuraId, Block, BlockNumber, Header, MILLISECS_PER_BLOCK},
    block::{
        substrate::{
            verification::{
                verifier::SessionVerifier, EquivocationProof, FinalizationInfo,
                HeaderVerificationError, VerificationError,
            },
            InnerJustification, Justification,
        },
        BlockId, Header as HeaderT, HeaderVerifier, JustificationVerifier, VerifiedHeader,
    },
    session::{SessionBoundaryInfo, SessionId},
    session_map::{AuthorityProvider, FinalizedBlocksProvider},
};

// How many slots in the future (according to the system time) can the verified header be.
// Must be non-negative. Chosen arbitrarily by timorl.
const HEADER_VERIFICATION_SLOT_OFFSET: u64 = 10;

/// Ways in which a justification can fail verification.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CacheError {
    UnknownAuthorities(SessionId),
    UnknownAuraAuthorities(SessionId),
    SessionTooOld(SessionId, SessionId),
    SessionInFuture(SessionId, SessionId),
    BadGenesisHeader,
}

impl Display for CacheError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use CacheError::*;
        match self {
            SessionTooOld(session, lower_bound) => write!(
                f,
                "session {session:?} is too old. Should be at least {lower_bound:?}"
            ),
            SessionInFuture(session, upper_bound) => write!(
                f,
                "session {session:?} without known authorities. Should be at most {upper_bound:?}"
            ),
            UnknownAuthorities(session) => {
                write!(
                    f,
                    "authorities for session {session:?} not known even though they should be"
                )
            }
            UnknownAuraAuthorities(session) => {
                write!(
                    f,
                    "Aura authorities for session {session:?} not known even though they should be"
                )
            }
            BadGenesisHeader => {
                write!(
                    f,
                    "the provided genesis header does not match the cached genesis header"
                )
            }
        }
    }
}

#[derive(Clone)]
struct CachedData {
    session_verifier: SessionVerifier,
    aura_authorities: Vec<AuraId>,
    authority_accounts: Option<Vec<AccountId>>,
}

fn download_data<AP, BP>(
    authority_provider: &AP,
    session_block_provider: &BP,
    session_id: SessionId,
) -> Result<CachedData, CacheError>
where
    AP: AuthorityProvider,
    BP: FinalizedBlocksProvider,
{
    let authority_data = match session_id {
        SessionId(0) => CachedData {
            session_verifier: authority_provider
                .authority_data(0)
                .ok_or(CacheError::UnknownAuthorities(session_id))?
                .into(),
            aura_authorities: authority_provider
                .aura_authorities(0)
                .ok_or(CacheError::UnknownAuraAuthorities(session_id))?,
            authority_accounts: None,
        },
        SessionId(id) => {
            let previous_session = SessionId(id - 1);
            let finalized_block_from_previous_session = session_block_provider
                .available_finalized_block(previous_session)
                .ok_or(CacheError::UnknownAuthorities(session_id))?;

            let authority_data = authority_provider
                .next_authority_data(finalized_block_from_previous_session)
                .ok_or(CacheError::UnknownAuthorities(session_id))?;

            let (authority_accounts, aura_authorities) = authority_provider
                .next_aura_authorities(finalized_block_from_previous_session)
                .ok_or(CacheError::UnknownAuraAuthorities(session_id))?
                .into_iter()
                .unzip();

            CachedData {
                session_verifier: authority_data.into(),
                aura_authorities,
                authority_accounts: Some(authority_accounts),
            }
        }
    };
    Ok(authority_data)
}

// Equivocations only happen per time slot _and_ session..
type SessionSlot = (SessionId, Slot);

/// Cache storing SessionVerifier structs and Aura authorities for multiple sessions.
/// Keeps up to `cache_size` verifiers of top sessions.
/// If the session is too new or ancient it will fail to return requested data.
/// Highest session verifier this cache returns is for the session after the current finalization session.
/// Lowest session verifier this cache returns is for `top_returned_session` - `cache_size`.
#[derive(Clone)]
pub struct VerifierCache<AP, BP, FI, H>
where
    AP: AuthorityProvider,
    BP: FinalizedBlocksProvider,
    FI: FinalizationInfo,
    H: HeaderT,
{
    cached_data: HashMap<SessionId, CachedData>,
    equivocation_cache: HashMap<SessionSlot, (H, bool)>,
    own_blocks_cache: HashSet<BlockId>,
    session_info: SessionBoundaryInfo,
    finalization_info: FI,
    authority_provider: AP,
    session_block_provider: BP,
    cache_size: usize,
    /// Lowest currently available session.
    lower_bound: SessionId,
    genesis_header: H,
}

impl<AP, BP, FI, H> VerifierCache<AP, BP, FI, H>
where
    AP: AuthorityProvider,
    BP: FinalizedBlocksProvider + Clone,
    FI: FinalizationInfo,
    H: HeaderT,
{
    pub fn new(
        session_info: SessionBoundaryInfo,
        finalization_info: FI,
        authority_provider: AP,
        session_block_provider: BP,
        cache_size: usize,
        genesis_header: H,
    ) -> Self {
        Self {
            cached_data: HashMap::new(),
            equivocation_cache: HashMap::new(),
            own_blocks_cache: HashSet::new(),
            session_info,
            finalization_info,
            authority_provider,
            session_block_provider,
            cache_size,
            lower_bound: SessionId(0),
            genesis_header,
        }
    }

    // Prune old session data if necessary
    fn try_prune(&mut self, session_id: SessionId) {
        if session_id.0
            >= self
                .lower_bound
                .0
                .saturating_add(self.cache_size.saturated_into())
        {
            let new_lower_bound = SessionId(
                session_id
                    .0
                    .saturating_sub(self.cache_size.saturated_into())
                    + 1,
            );
            self.cached_data.retain(|&id, _| id >= new_lower_bound);
            let lower_bound_block = self.session_info.first_block_of_session(new_lower_bound);
            self.equivocation_cache
                .retain(|_, (header, _)| header.id().number() >= lower_bound_block);
            self.own_blocks_cache = self
                .equivocation_cache
                .iter()
                .filter_map(|(_, (header, own_block))| match own_block {
                    true => Some(header.id()),
                    false => None,
                })
                .collect();

            self.lower_bound = new_lower_bound;
        }
    }

    fn session_id_from_block_num(&self, number: BlockNumber) -> SessionId {
        self.session_info.session_id_from_block_num(number)
    }

    fn get_data(&mut self, session_id: SessionId) -> Result<&CachedData, CacheError> {
        if session_id < self.lower_bound {
            return Err(CacheError::SessionTooOld(session_id, self.lower_bound));
        }

        let best_finalized = self.finalization_info.finalized_number();
        // We are sure about authorities in all session that have first block
        // from previous session finalized.
        let upper_bound = SessionId(self.session_id_from_block_num(best_finalized).0 + 1);
        if session_id > upper_bound {
            return Err(CacheError::SessionInFuture(session_id, upper_bound));
        }

        self.try_prune(session_id);

        Ok(match self.cached_data.entry(session_id) {
            Entry::Occupied(occupied) => occupied.into_mut(),
            Entry::Vacant(vacant) => vacant.insert(download_data(
                &self.authority_provider,
                &self.session_block_provider,
                session_id,
            )?),
        })
    }

    /// Returns session verifier for block number if available. Updates cache if necessary.
    /// Must be called using the number of the verified block.
    pub fn get_verifier(&mut self, number: BlockNumber) -> Result<&SessionVerifier, CacheError> {
        Ok(&self
            .get_data(self.session_id_from_block_num(number))?
            .session_verifier)
    }
}

impl<AP, BP, FS> VerifierCache<AP, BP, FS, Header>
where
    AP: AuthorityProvider,
    BP: FinalizedBlocksProvider,
    FS: FinalizationInfo,
{
    /// Returns the list of Aura authorities for a given session. Updates cache if necessary.
    /// This method assumes that the queued Aura authorities will indeed become Aura authorities
    /// in the next session.
    fn get_aura_authorities(&mut self, session_id: SessionId) -> Result<&CachedData, CacheError> {
        self.get_data(session_id)
    }

    fn slot_author(
        slot: Slot,
        aura_authorities: &[AuraId],
        authority_accounts: Option<&Vec<AccountId>>,
    ) -> Result<(AuraId, Option<AccountId>), ()> {
        let expected_author = slot_author::<AuthorityPair>(slot, aura_authorities).ok_or(())?;

        let Some(authority_accounts) = authority_accounts else {
            return Ok((expected_author.clone(), None));
        };
        // find this author on our list
        // Aura: round robin
        let idx = (*slot % (aura_authorities.len() as u64)) as usize;
        // safety check in case something's changed for Aura
        if expected_author != aura_authorities.get(idx).ok_or(())? {
            return Err(());
        }
        Ok((
            expected_author.clone(),
            Some(authority_accounts.get(idx).ok_or(())?).cloned(),
        ))
    }

    // This function assumes that:
    // 1. This is not a genesis header
    // 2. Headers are created by Aura.
    // 3. Slot number is calculated using the current system time.
    fn check_header_slot_and_seal(
        header: Header,
        authorities: &[AuraId],
    ) -> Result<(Header, Slot), HeaderVerificationError> {
        // Aura: slot number is calculated using the system time.
        // This code duplicates one of the parameters that we pass to Aura when starting the node!
        let slot_now = Slot::from_timestamp(
            sp_timestamp::Timestamp::current(),
            sp_consensus_slots::SlotDuration::from_millis(MILLISECS_PER_BLOCK),
        );

        let (mut header, slot, seal) = check_header_slot_and_seal::<Block, AuthorityPair>(
            slot_now + HEADER_VERIFICATION_SLOT_OFFSET,
            header,
            authorities,
        )?;
        // we need to push the seal back after it was removed by the above call
        header.digest_mut().push(seal);
        Ok((header, slot))
    }

    // This function assumes that:
    // 1. This is not a genesis header
    // 2. Headers are created by Aura.
    fn check_for_equivocation(
        &mut self,
        header: &Header,
        session_slot: SessionSlot,
        author: AuraId,
        maybe_account_id: Option<AccountId>,
        just_created: bool,
    ) -> Result<Option<EquivocationProof>, VerificationError> {
        match self.equivocation_cache.entry(session_slot) {
            Entry::Occupied(occupied) => {
                let (cached_header, certainly_own) = occupied.into_mut();
                if cached_header == header {
                    *certainly_own |= just_created;
                    if *certainly_own {
                        self.own_blocks_cache.insert(header.id());
                    }
                    return Ok(None);
                }
                Ok(Some(EquivocationProof {
                    header_a: cached_header.clone(),
                    header_b: header.clone(),
                    are_we_equivocating: *certainly_own || just_created,
                    account_id: maybe_account_id,
                    author,
                }))
            }
            Entry::Vacant(vacant) => {
                vacant.insert((header.clone(), just_created));
                if just_created {
                    self.own_blocks_cache.insert(header.id());
                }
                Ok(None)
            }
        }
    }
}

impl<AP, BP, FS> JustificationVerifier<Justification> for VerifierCache<AP, BP, FS, Header>
where
    AP: AuthorityProvider,
    BP: FinalizedBlocksProvider,
    FS: FinalizationInfo,
{
    type Error = VerificationError;

    fn verify_justification(
        &mut self,
        justification: Justification,
    ) -> Result<Justification, Self::Error> {
        let header = &justification.header;
        match &justification.inner_justification {
            InnerJustification::AlephJustification(aleph_justification) => {
                let verifier = self.get_verifier(*header.number())?;
                verifier.verify_bytes(aleph_justification, header.hash().encode())?;
                Ok(justification)
            }
            InnerJustification::Genesis => match header == &self.genesis_header {
                true => Ok(justification),
                false => Err(Self::Error::Cache(CacheError::BadGenesisHeader)),
            },
        }
    }
}

impl<AP, BP, FS> HeaderVerifier<Header> for VerifierCache<AP, BP, FS, Header>
where
    AP: AuthorityProvider,
    BP: FinalizedBlocksProvider,
    FS: FinalizationInfo,
{
    type Error = VerificationError;
    type EquivocationProof = EquivocationProof;

    fn verify_header(
        &mut self,
        header: Header,
        just_created: bool,
    ) -> Result<VerifiedHeader<Header, Self::EquivocationProof>, Self::Error> {
        let parent_number = header.number().saturating_sub(1);
        if *header.number() == parent_number {
            // compare genesis header directly to the one we know
            return match header == self.genesis_header {
                true => Ok(VerifiedHeader {
                    header,
                    maybe_equivocation_proof: None,
                }),
                false => Err(VerificationError::HeaderVerification(
                    HeaderVerificationError::IncorrectGenesis,
                )),
            };
        }

        // Aura: authorities are stored in the parent block
        let session_id = self.session_id_from_block_num(parent_number);
        let authorities = self
            .get_aura_authorities(session_id)
            .map_err(|_| HeaderVerificationError::MissingAuthorityData)?;

        let (header, slot) =
            Self::check_header_slot_and_seal(header, &authorities.aura_authorities)
                .map_err(HeaderVerificationError::from)?;

        let (expected_author, maybe_account_id) = Self::slot_author(
            slot,
            &authorities.aura_authorities,
            authorities.authority_accounts.as_ref(),
        )
        .map_err(|_| HeaderVerificationError::MissingAuthorityData)?;

        let maybe_equivocation_proof = self.check_for_equivocation(
            &header,
            (session_id, slot),
            expected_author,
            maybe_account_id,
            just_created,
        )?;

        Ok(VerifiedHeader {
            header,
            maybe_equivocation_proof,
        })
    }

    fn own_block(&self, header: &Header) -> bool {
        self.own_blocks_cache.contains(&header.id())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use sp_runtime::testing::UintAuthorityId;

    use super::{
        AccountId, AuraId, AuthorityProvider, BlockNumber, CacheError, FinalizationInfo,
        SessionVerifier, VerifierCache,
    };
    use crate::{
        aleph_primitives::SessionAuthorityData,
        block::mock::MockHeader,
        session::{testing::authority_data, SessionBoundaryInfo, SessionId},
        session_map::FinalizedBlocksProvider,
        SessionPeriod,
    };

    const SESSION_PERIOD: u32 = 30;
    const CACHE_SIZE: usize = 3;

    type TestVerifierCache = VerifierCache<
        MockAuthorityProvider,
        MockFinalizationInfo,
        MockFinalizationInfo,
        MockHeader,
    >;

    #[derive(Clone)]
    struct MockFinalizationInfo {
        finalized_number: Arc<Mutex<BlockNumber>>,
        session_info: SessionBoundaryInfo,
    }

    impl FinalizationInfo for MockFinalizationInfo {
        fn finalized_number(&self) -> BlockNumber {
            *self.finalized_number.lock().expect("mutex works")
        }
    }

    impl FinalizedBlocksProvider for MockFinalizationInfo {
        fn available_finalized_block(&self, session_id: SessionId) -> Option<BlockNumber> {
            let finalized_block = self.finalized_number();
            let first_block_in_session = self.session_info.first_block_of_session(session_id);
            (first_block_in_session >= finalized_block).then_some(first_block_in_session)
        }
    }

    #[derive(Clone)]
    struct MockAuthorityProvider {
        session_map: HashMap<SessionId, SessionAuthorityData>,
        aura_authority_map: HashMap<SessionId, Vec<AuraId>>,
        session_info: SessionBoundaryInfo,
    }

    fn authority_data_for_session(session_id: u32) -> SessionAuthorityData {
        authority_data(session_id * 4, (session_id + 1) * 4)
    }

    fn aura_authority_data_for_session(session_id: u32) -> Vec<AuraId> {
        (session_id * 4..(session_id + 1) * 4)
            .map(|id| UintAuthorityId(id.into()).to_public_key())
            .collect()
    }

    impl MockAuthorityProvider {
        fn new(session_n: u32) -> Self {
            let session_map = (0..session_n + 1)
                .map(|s| (SessionId(s), authority_data_for_session(s)))
                .collect();
            let aura_authority_map = (0..session_n + 1)
                .map(|s| (SessionId(s), aura_authority_data_for_session(s)))
                .collect();
            Self {
                session_map,
                aura_authority_map,
                session_info: SessionBoundaryInfo::new(SessionPeriod(SESSION_PERIOD)),
            }
        }
    }

    impl AuthorityProvider for MockAuthorityProvider {
        fn authority_data(&self, block_number: BlockNumber) -> Option<SessionAuthorityData> {
            self.session_map
                .get(&self.session_info.session_id_from_block_num(block_number))
                .cloned()
        }

        fn next_authority_data(&self, block_number: BlockNumber) -> Option<SessionAuthorityData> {
            self.session_map
                .get(&SessionId(
                    self.session_info.session_id_from_block_num(block_number).0 + 1,
                ))
                .cloned()
        }

        fn aura_authorities(&self, block_number: BlockNumber) -> Option<Vec<AuraId>> {
            self.aura_authority_map
                .get(&self.session_info.session_id_from_block_num(block_number))
                .cloned()
        }

        fn next_aura_authorities(
            &self,
            block_number: BlockNumber,
        ) -> Option<Vec<(AccountId, AuraId)>> {
            let placeholder_id = AccountId::new([0; 32]);
            self.aura_authority_map
                .get(&SessionId(
                    self.session_info.session_id_from_block_num(block_number).0 + 1,
                ))
                .cloned()
                .map(|v| {
                    v.into_iter()
                        .map(|aura_id| (placeholder_id.clone(), aura_id))
                        .collect()
                })
        }
    }

    fn setup_test(max_session_n: u32, finalized_number: Arc<Mutex<u32>>) -> TestVerifierCache {
        let session_info = SessionBoundaryInfo::new(SessionPeriod(SESSION_PERIOD));
        let finalization_info = MockFinalizationInfo {
            finalized_number,
            session_info: session_info.clone(),
        };
        let authority_provider = MockAuthorityProvider::new(max_session_n);
        let genesis_header = MockHeader::random_parentless(0);

        VerifierCache::new(
            session_info,
            finalization_info.clone(),
            authority_provider,
            finalization_info,
            CACHE_SIZE,
            genesis_header,
        )
    }

    fn finalize_first_in_session(finalized_number: Arc<Mutex<u32>>, session_id: u32) {
        *finalized_number.lock().expect("mutex works") = session_id * SESSION_PERIOD;
    }

    fn session_verifier(
        verifier: &mut TestVerifierCache,
        session_id: u32,
    ) -> Result<SessionVerifier, CacheError> {
        verifier
            .get_verifier((session_id + 1) * SESSION_PERIOD - 1)
            .cloned()
    }

    fn check_session_verifier(verifier: &mut TestVerifierCache, session_id: u32) {
        let session_verifier =
            session_verifier(verifier, session_id).expect("Should return verifier. Got error");
        let expected_verifier: SessionVerifier = authority_data_for_session(session_id).into();
        assert_eq!(session_verifier, expected_verifier);
    }

    #[test]
    fn genesis_session() {
        let finalized_number = Arc::new(Mutex::new(0));

        let mut verifier = setup_test(0, finalized_number);

        check_session_verifier(&mut verifier, 0);
    }

    #[test]
    fn normal_session() {
        let finalized_number = Arc::new(Mutex::new(0));

        let mut verifier = setup_test(3, finalized_number.clone());

        check_session_verifier(&mut verifier, 0);
        check_session_verifier(&mut verifier, 1);

        finalize_first_in_session(finalized_number.clone(), 1);
        check_session_verifier(&mut verifier, 0);
        check_session_verifier(&mut verifier, 1);
        check_session_verifier(&mut verifier, 2);

        finalize_first_in_session(finalized_number, 2);
        check_session_verifier(&mut verifier, 1);
        check_session_verifier(&mut verifier, 2);
        check_session_verifier(&mut verifier, 3);
    }

    #[test]
    fn prunes_old_sessions() {
        assert_eq!(CACHE_SIZE, 3);

        let finalized_number = Arc::new(Mutex::new(0));

        let mut verifier = setup_test(4, finalized_number.clone());

        check_session_verifier(&mut verifier, 0);
        check_session_verifier(&mut verifier, 1);

        finalize_first_in_session(finalized_number.clone(), 1);
        check_session_verifier(&mut verifier, 2);

        finalize_first_in_session(finalized_number.clone(), 2);
        check_session_verifier(&mut verifier, 3);

        // Should no longer have verifier for session 0
        assert_eq!(
            session_verifier(&mut verifier, 0),
            Err(CacheError::SessionTooOld(SessionId(0), SessionId(1)))
        );

        finalize_first_in_session(finalized_number, 3);
        check_session_verifier(&mut verifier, 4);

        // Should no longer have verifier for session 1
        assert_eq!(
            session_verifier(&mut verifier, 1),
            Err(CacheError::SessionTooOld(SessionId(1), SessionId(2)))
        );
    }

    #[test]
    fn session_from_future() {
        let finalized_number = Arc::new(Mutex::new(0));

        let mut verifier = setup_test(3, finalized_number.clone());

        finalize_first_in_session(finalized_number, 1);

        // Did not finalize first block in session 2 yet
        assert_eq!(
            session_verifier(&mut verifier, 3),
            Err(CacheError::SessionInFuture(SessionId(3), SessionId(2)))
        );
    }

    #[test]
    fn authority_provider_error() {
        let finalized_number = Arc::new(Mutex::new(0));
        let mut verifier = setup_test(0, finalized_number.clone());

        assert_eq!(
            session_verifier(&mut verifier, 1),
            Err(CacheError::UnknownAuthorities(SessionId(1)))
        );

        finalize_first_in_session(finalized_number, 1);

        assert_eq!(
            session_verifier(&mut verifier, 2),
            Err(CacheError::UnknownAuthorities(SessionId(2)))
        );
    }
}
