use primitives::{EraIndex, SessionCount};

use crate::{
    api,
    api::runtime_types::{
        pallet_elections::pallet::Call::set_ban_config,
        primitives::{BanReason, CommitteeSeats, EraValidators},
    },
    connections::AsConnection,
    pallet_elections::pallet::Call::{
        ban_from_committee, change_validators, set_elections_openness,
    },
    primitives::{BanConfig, BanInfo, ElectionOpenness},
    AccountId, BlockHash,
    Call::Elections,
    ConnectionApi, RootConnection, SudoCall, TxStatus,
};

// TODO once pallet elections docs are published, replace api docs with links to public docs
/// Pallet elections read-only api.
#[async_trait::async_trait]
pub trait ElectionsApi {
    /// Returns `elections.ban_config` storage of the elections pallet.
    /// * `at` - optional hash of a block to query state from
    async fn get_ban_config(&self, at: Option<BlockHash>) -> BanConfig;

    /// Returns `elections.committee_size` storage of the elections pallet.
    /// * `at` - optional hash of a block to query state from
    async fn get_committee_seats(&self, at: Option<BlockHash>) -> CommitteeSeats;

    /// Returns `elections.next_era_committee_seats` storage of the elections pallet.
    /// * `at` - optional hash of a block to query state from
    async fn get_next_era_committee_seats(&self, at: Option<BlockHash>) -> CommitteeSeats;

    /// Returns `elections.session_validator_block_count` of a given validator.
    /// * `validator` - a validator stash account id
    /// * `at` - optional hash of a block to query state from
    async fn get_validator_block_count(
        &self,
        validator: AccountId,
        at: Option<BlockHash>,
    ) -> Option<u32>;

    /// Returns `elections.current_era_validators` storage of the elections pallet.
    /// * `at` - optional hash of a block to query state from
    async fn get_current_era_validators(&self, at: Option<BlockHash>) -> EraValidators<AccountId>;

    /// Returns `elections.next_era_reserved_validators` storage of the elections pallet.
    /// * `at` - optional hash of a block to query state from
    async fn get_next_era_reserved_validators(&self, at: Option<BlockHash>) -> Vec<AccountId>;

    /// Returns `elections.next_era_non_reserved_validators` storage of the elections pallet.
    /// * `at` - optional hash of a block to query state from
    async fn get_next_era_non_reserved_validators(&self, at: Option<BlockHash>) -> Vec<AccountId>;

    /// Returns `elections.underperformed_validator_session_count` storage of a given validator.
    /// * `validator` - a validator stash account id
    /// * `at` - optional hash of a block to query state from
    async fn get_underperformed_validator_session_count(
        &self,
        validator: AccountId,
        at: Option<BlockHash>,
    ) -> Option<SessionCount>;

    /// Returns `elections.banned.reason` storage of a given validator.
    /// * `validator` - a validator stash account id
    /// * `at` - optional hash of a block to query state from
    async fn get_ban_reason_for_validator(
        &self,
        validator: AccountId,
        at: Option<BlockHash>,
    ) -> Option<BanReason>;

    /// Returns `elections.banned` storage of a given validator.
    /// * `validator` - a validator stash account id
    /// * `at` - optional hash of a block to query state from
    async fn get_ban_info_for_validator(
        &self,
        validator: AccountId,
        at: Option<BlockHash>,
    ) -> Option<BanInfo>;
    /// Returns `elections.session_period` const of the elections pallet.
    async fn get_session_period(&self) -> anyhow::Result<u32>;
}

/// any object that implements pallet elections api that requires sudo
#[async_trait::async_trait]
pub trait ElectionsSudoApi {
    /// Issues `elections.set_ban_config`. It has an immediate effect.
    /// * `minimal_expected_performance` - performance ratio threshold in a session
    /// * `underperformed_session_count_threshold` - how many bad uptime sessions force validator to be removed from the committee
    /// * `clean_session_counter_delay` - underperformed session counter is cleared every subsequent `clean_session_counter_delay` sessions
    /// * `ban_period` - how many eras a validator is banned for
    /// * `status` - a [`TxStatus`] for a tx to wait for
    async fn set_ban_config(
        &self,
        minimal_expected_performance: Option<u8>,
        underperformed_session_count_threshold: Option<u32>,
        clean_session_counter_delay: Option<u32>,
        ban_period: Option<EraIndex>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    /// Issues `elections.change_validators` that sets the committee for the next era.
    /// * `new_reserved_validators` - reserved validators to be in place in the next era; optional
    /// * `new_non_reserved_validators` - non reserved validators to be in place in the next era; optional
    /// * `committee_size` - committee size to be in place in the next era; optional
    /// * `status` - a [`TxStatus`] for a tx to wait for
    async fn change_validators(
        &self,
        new_reserved_validators: Option<Vec<AccountId>>,
        new_non_reserved_validators: Option<Vec<AccountId>>,
        committee_size: Option<CommitteeSeats>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    /// Schedule a non-reserved node to be banned out from the committee at the end of the era.
    /// * `account` - account to be banned,
    /// * `ben_reason` - reaons for ban, expressed as raw bytes
    /// * `status` - a [`TxStatus`] for a tx to wait for
    async fn ban_from_committee(
        &self,
        account: AccountId,
        ban_reason: Vec<u8>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    /// Set openness of the elections.
    /// * `mode` - new elections openness mode
    /// * `status` - a [`TxStatus`] for a tx to wait for
    async fn set_election_openness(
        &self,
        mode: ElectionOpenness,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi + AsConnection> ElectionsApi for C {
    async fn get_ban_config(&self, at: Option<BlockHash>) -> BanConfig {
        let addrs = api::storage().elections().ban_config();

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_committee_seats(&self, at: Option<BlockHash>) -> CommitteeSeats {
        let addrs = api::storage().elections().committee_size();

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_next_era_committee_seats(&self, at: Option<BlockHash>) -> CommitteeSeats {
        let addrs = api::storage().elections().next_era_committee_size();

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_validator_block_count(
        &self,
        validator: AccountId,
        at: Option<BlockHash>,
    ) -> Option<u32> {
        let addrs = api::storage()
            .elections()
            .session_validator_block_count(&validator);

        self.get_storage_entry_maybe(&addrs, at).await
    }

    async fn get_current_era_validators(&self, at: Option<BlockHash>) -> EraValidators<AccountId> {
        let addrs = api::storage().elections().current_era_validators();

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_next_era_reserved_validators(&self, at: Option<BlockHash>) -> Vec<AccountId> {
        let addrs = api::storage().elections().next_era_reserved_validators();

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_next_era_non_reserved_validators(&self, at: Option<BlockHash>) -> Vec<AccountId> {
        let addrs = api::storage()
            .elections()
            .next_era_non_reserved_validators();

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_underperformed_validator_session_count(
        &self,
        validator: AccountId,
        at: Option<BlockHash>,
    ) -> Option<SessionCount> {
        let addrs = api::storage()
            .elections()
            .underperformed_validator_session_count(&validator);

        self.get_storage_entry_maybe(&addrs, at).await
    }

    async fn get_ban_reason_for_validator(
        &self,
        validator: AccountId,
        at: Option<BlockHash>,
    ) -> Option<BanReason> {
        let addrs = api::storage().elections().banned(validator);

        match self.get_storage_entry_maybe(&addrs, at).await {
            None => None,
            Some(x) => Some(x.reason),
        }
    }

    async fn get_ban_info_for_validator(
        &self,
        validator: AccountId,
        at: Option<BlockHash>,
    ) -> Option<BanInfo> {
        let addrs = api::storage().elections().banned(validator);

        self.get_storage_entry_maybe(&addrs, at).await
    }

    async fn get_session_period(&self) -> anyhow::Result<u32> {
        let addrs = api::constants().elections().session_period();
        self.as_connection()
            .as_client()
            .constants()
            .at(&addrs)
            .map_err(|e| e.into())
    }
}

#[async_trait::async_trait]
impl ElectionsSudoApi for RootConnection {
    async fn set_ban_config(
        &self,
        minimal_expected_performance: Option<u8>,
        underperformed_session_count_threshold: Option<u32>,
        clean_session_counter_delay: Option<u32>,
        ban_period: Option<EraIndex>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let call = Elections(set_ban_config {
            minimal_expected_performance,
            underperformed_session_count_threshold,
            clean_session_counter_delay,
            ban_period,
        });

        self.sudo_unchecked(call, status).await
    }

    async fn change_validators(
        &self,
        new_reserved_validators: Option<Vec<AccountId>>,
        new_non_reserved_validators: Option<Vec<AccountId>>,
        committee_size: Option<CommitteeSeats>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let call = Elections(change_validators {
            reserved_validators: new_reserved_validators,
            non_reserved_validators: new_non_reserved_validators,
            committee_size,
        });

        self.sudo_unchecked(call, status).await
    }

    async fn ban_from_committee(
        &self,
        account: AccountId,
        ban_reason: Vec<u8>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let call = Elections(ban_from_committee {
            banned: account,
            ban_reason,
        });
        self.sudo_unchecked(call, status).await
    }

    async fn set_election_openness(
        &self,
        mode: ElectionOpenness,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let call = Elections(set_elections_openness { openness: mode });

        self.sudo_unchecked(call, status).await
    }
}
