use primitives::{AbftScoresProvider, FinalityCommitteeManager, Score, SessionIndex};
use sp_std::vec::Vec;

use crate::{
    AbftScores, Config, Event, FinalityScheduledVersionChange, FinalityVersion, LastScoreNonce,
    NextFinalityCommittee, Pallet,
};

impl<T> pallet_session::SessionManager<T::AccountId> for Pallet<T>
where
    T: Config,
{
    fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        <T as Config>::SessionManager::new_session(new_index)
    }

    fn new_session_genesis(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
        <T as Config>::SessionManager::new_session_genesis(new_index)
    }

    fn end_session(end_index: SessionIndex) {
        <T as Config>::SessionManager::end_session(end_index);
    }

    fn start_session(start_index: SessionIndex) {
        <T as Config>::SessionManager::start_session(start_index);
        Self::update_version_change_history();
    }
}

impl<T> Pallet<T>
where
    T: Config,
{
    // Check if a schedule version change has moved into the past. Update history, even if there is
    // no change. Resets the scheduled version.
    fn update_version_change_history() {
        let current_session = Self::current_session();

        if let Some(scheduled_version_change) = <FinalityScheduledVersionChange<T>>::get() {
            let scheduled_session = scheduled_version_change.session;
            let scheduled_version = scheduled_version_change.version_incoming;

            // Record the scheduled version as the current version as it moves into the past.
            if scheduled_session == current_session {
                <FinalityVersion<T>>::put(scheduled_version);

                // Reset the scheduled version.
                <FinalityScheduledVersionChange<T>>::kill();

                Self::deposit_event(Event::FinalityVersionChange(scheduled_version_change));
            }
        }
    }
}

impl<T: Config> FinalityCommitteeManager<T::AccountId> for Pallet<T> {
    fn on_next_session_finality_committee(committee: Vec<T::AccountId>) {
        NextFinalityCommittee::<T>::put(committee);
    }
}

impl<T: Config> AbftScoresProvider for Pallet<T> {
    fn scores_for_session(session_id: SessionIndex) -> Option<Score> {
        AbftScores::<T>::get(session_id)
    }

    fn clear_scores() {
        let _result = AbftScores::<T>::clear(u32::MAX, None);
    }

    fn clear_nonce() {
        LastScoreNonce::<T>::kill();
    }
}
