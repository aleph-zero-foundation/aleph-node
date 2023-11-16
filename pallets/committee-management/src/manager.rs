use frame_system::pallet_prelude::BlockNumberFor;
use log::debug;
use pallet_session::SessionManager;
use primitives::{EraManager, FinalityCommitteeManager, SessionCommittee};
use sp_staking::{EraIndex, SessionIndex};
use sp_std::{marker::PhantomData, vec::Vec};

use crate::{
    pallet::{Config, Pallet, SessionValidatorBlockCount},
    traits::EraInfoProvider,
    LOG_TARGET,
};

/// We assume that block `B` ends session nr `S`, and current era index is `E`.
///
/// 1. Block `B` initialized
/// 2. `end_session(S)` is called
/// *  Based on block count we might mark the session for a given validator as underperformed
/// *  We update rewards and clear block count for the session `S`.
/// 3. `start_session(S + 1)` is called.
/// *  if session `S+1` starts new era we populate totals and unban all validators whose ban expired.
/// *  if session `S+1` % `clean_session_counter_delay` == 0, we clean up underperformed session counter.
/// * `clean_session_counter_delay` is read from pallet's storage
/// 4. `new_session(S + 2)` is called.
/// *  If session `S+2` starts new era we emit fresh bans events
/// *  We rotate the validators for session `S + 2` using the information about reserved and non reserved validators.

impl<T> pallet_authorship::EventHandler<T::AccountId, BlockNumberFor<T>> for Pallet<T>
where
    T: Config,
{
    fn note_author(validator: T::AccountId) {
        SessionValidatorBlockCount::<T>::mutate(&validator, |count| {
            *count += 1;
        });
    }
}

/// SessionManager that also fires EraManager functions. It is responsible for rotation of the committee,
/// bans and rewards logic.
///
/// The order of the calls are as follows:
/// First call is always from the inner SessionManager then the call to EraManager fn if applicable.
/// * New session is planned:
/// 1. Inner T::new_session invoked
/// 2. If session starts era EM::on_new_era invoked
/// 3. Logic related to new session from this pallet is invoked
/// * Session ends:
/// 1. Inner T::end_session invoked
/// 2. Logic related to new session from this pallet is invoked
/// * Session starts:
/// 1. Inner T::start_session invoked
/// 2. Logic related to new session from this pallet is invoked
/// 3. If session starts era EM::new_era_start invoked
/// 4. If session starts era logic related to new era from this pallet is invoked
///
/// In the runtime we set EM to pallet_elections and T to combination of staking and historical_session.
pub struct SessionAndEraManager<E, EM, T, C>(PhantomData<(E, EM, T, C)>)
where
    E: EraInfoProvider,
    EM: EraManager,
    T: SessionManager<C::AccountId>,
    C: Config;

impl<E, EM, T, C> SessionAndEraManager<E, EM, T, C>
where
    E: EraInfoProvider,
    EM: EraManager,
    T: SessionManager<C::AccountId>,
    C: Config,
{
    fn session_starts_era(session: SessionIndex) -> Option<EraIndex> {
        let active_era = match E::active_era() {
            Some(ae) => ae,
            // no active era, session can't start it
            _ => return None,
        };

        if Self::is_start_of_the_era(active_era, session) {
            return Some(active_era);
        }

        None
    }

    fn session_starts_next_era(session: SessionIndex) -> Option<EraIndex> {
        let active_era = match E::active_era() {
            Some(ae) => ae + 1,
            // no active era, session can't start it
            _ => return None,
        };

        if Self::is_start_of_the_era(active_era, session) {
            return Some(active_era);
        }

        None
    }

    fn is_start_of_the_era(era: EraIndex, session: SessionIndex) -> bool {
        if let Some(era_start_index) = E::era_start_session_index(era) {
            return era_start_index == session;
        }

        false
    }
}

impl<E, EM, T, C> SessionManager<C::AccountId> for SessionAndEraManager<E, EM, T, C>
where
    E: EraInfoProvider,
    EM: EraManager,
    T: SessionManager<C::AccountId>,
    C: Config,
{
    fn new_session(new_index: SessionIndex) -> Option<Vec<C::AccountId>> {
        T::new_session(new_index);
        if let Some(era) = Self::session_starts_next_era(new_index) {
            EM::on_new_era(era);
            Pallet::<C>::emit_fresh_bans_event();
        }

        let SessionCommittee {
            finality_committee,
            block_producers,
        } = Pallet::<C>::rotate_committee(new_index)?;
        // Notify about elected next session finality committee
        C::FinalityCommitteeManager::on_next_session_finality_committee(finality_committee);

        Some(block_producers)
    }

    fn end_session(end_index: SessionIndex) {
        T::end_session(end_index);
        Pallet::<C>::adjust_rewards_for_session();
        Pallet::<C>::calculate_underperforming_validators();
        // clear block count after calculating stats for underperforming validators, as they use
        // SessionValidatorBlockCount for that
        let result = SessionValidatorBlockCount::<C>::clear(u32::MAX, None);
        debug!(
            target: LOG_TARGET,
            "Result of clearing the `SessionValidatorBlockCount`, {:?}",
            result.deconstruct()
        );
    }

    fn start_session(start_index: SessionIndex) {
        T::start_session(start_index);
        Pallet::<C>::clear_underperformance_session_counter(start_index);

        if let Some(era) = Self::session_starts_era(start_index) {
            Pallet::<C>::update_validator_total_rewards(era);
            Pallet::<C>::clear_expired_bans(era);
        }
    }
}
