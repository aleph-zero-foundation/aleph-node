use primitives::{Balance, EraIndex};
use subxt::{
    ext::{
        sp_core::storage::StorageKey,
        sp_runtime::{MultiAddress, Perbill as SPerbill},
    },
    storage::address::{StorageHasher, StorageMapKey},
};

use crate::{
    api,
    connections::{AsConnection, TxInfo},
    pallet_staking::{
        pallet::pallet::{
            Call::{bond, force_new_era, nominate, set_staking_configs},
            ConfigOp,
            ConfigOp::{Noop, Set},
        },
        EraRewardPoints, Exposure, RewardDestination, StakingLedger, ValidatorPrefs,
    },
    pallet_sudo::pallet::Call::sudo_as,
    pallets::utility::UtilityApi,
    sp_arithmetic::per_things::Perbill,
    AccountId, BlockHash,
    Call::{Staking, Sudo},
    ConnectionApi, RootConnection, SignedConnectionApi, SudoCall, TxStatus,
};

/// Any object that implemnts pallet staking read-only api.
#[async_trait::async_trait]
pub trait StakingApi {
    /// Returns [`active_era`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.active_era).
    /// * `at` - optional hash of a block to query state from
    async fn get_active_era(&self, at: Option<BlockHash>) -> EraIndex;

    /// Returns [`current_era`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.current_era).
    /// * `at` - optional hash of a block to query state from
    async fn get_current_era(&self, at: Option<BlockHash>) -> EraIndex;

    /// Returns [`bonded`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.bonded) for a given stash account.
    /// * `stash` - a stash account id
    /// * `at` - optional hash of a block to query state from
    async fn get_bonded(&self, stash: AccountId, at: Option<BlockHash>) -> Option<AccountId>;

    /// Returns [`ledger`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.ledger) for a given controller account.
    /// * `controller` - a controller account id
    /// * `at` - optional hash of a block to query state from
    async fn get_ledger(&self, controller: AccountId, at: Option<BlockHash>) -> StakingLedger;

    /// Returns [`eras_validator_reward`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.eras_validator_reward) for a given era.
    /// * `era` - an era index
    /// * `at` - optional hash of a block to query state from
    async fn get_payout_for_era(&self, era: EraIndex, at: Option<BlockHash>) -> Balance;

    /// Returns [`eras_stakers`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.eras_stakers) for a given era and account id.
    /// * `era` - an era index
    /// * `account_id` - an account id
    /// * `at` - optional hash of a block to query state from
    async fn get_exposure(
        &self,
        era: EraIndex,
        account_id: &AccountId,
        at: Option<BlockHash>,
    ) -> Exposure<AccountId, Balance>;

    /// Returns [`eras_reward_points`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.eras_reward_points) for a given era.
    /// * `era` - an era index
    /// * `at` - optional hash of a block to query state from
    async fn get_era_reward_points(
        &self,
        era: EraIndex,
        at: Option<BlockHash>,
    ) -> Option<EraRewardPoints<AccountId>>;

    /// Returns [`minimum_validator_count`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.minimum_validator_count).
    /// * `at` - optional hash of a block to query state from
    async fn get_minimum_validator_count(&self, at: Option<BlockHash>) -> u32;
    /// Returns [`SessionsPerEra`](https://paritytech.github.io/substrate/master/pallet_staking/trait.Config.html#associatedtype.SessionsPerEra) const.
    async fn get_session_per_era(&self) -> anyhow::Result<u32>;
}

/// Pallet staking api
#[async_trait::async_trait]
pub trait StakingUserApi {
    /// API for [`bond`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.bond) call.
    async fn bond(
        &self,
        initial_stake: Balance,
        controller_id: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`validate`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.validate) call.
    async fn validate(
        &self,
        validator_commission_percentage: u8,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`payout_stakers`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.payout_stakers) call.
    async fn payout_stakers(
        &self,
        stash_account: AccountId,
        era: EraIndex,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`nominate`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.nominate) call.
    async fn nominate(
        &self,
        nominee_account_id: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// API for [`chill`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.chill) call.
    async fn chill(&self, status: TxStatus) -> anyhow::Result<TxInfo>;

    /// API for [`bond_extra`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.bond_extra) call.
    async fn bond_extra_stake(
        &self,
        extra_stake: Balance,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;
}

/// Pallet staking logic, not directly related to any particular pallet call.
#[async_trait::async_trait]
pub trait StakingApiExt {
    /// Send batch of [`bond`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.bond) calls.
    /// * `accounts` - a slice of account ids pairs (stash, controller)
    /// * `stake` - what amount should be bonded,
    /// * `status` - a [`TxStatus`] of a tx to wait for
    ///
    /// # Examples
    /// ```ignore
    /// async fn nominate_validator(
    ///     connection: &RootConnection,
    ///     nominator_controller_accounts: Vec<AccountId>,
    ///     nominator_stash_accounts: Vec<AccountId>,
    ///     nominee_account: AccountId,
    /// ) {
    ///     let stash_controller_accounts = nominator_stash_accounts
    ///         .iter()
    ///         .cloned()
    ///         .zip(nominator_controller_accounts.iter().cloned())
    ///         .collect::<Vec<_>>();
    ///
    ///     let mut rng = thread_rng();
    ///     for chunk in stash_controller_accounts
    ///         .chunks(256)
    ///         .map(|c| c.to_vec())
    ///     {
    ///         let stake = 100 * 1_000_000_000_000u128;
    ///         connection
    ///             .batch_bond(&chunk, stake, TxStatus::Submitted)
    ///             .await
    ///             .unwrap();
    ///     }
    ///     let nominator_nominee_accounts = nominator_controller_accounts
    ///        .iter()
    ///        .cloned()
    ///        .zip(iter::repeat(&nominee_account).cloned())
    ///        .collect::<Vec<_>>();
    ///     for chunks in nominator_nominee_accounts.chunks(128) {
    ///        connection
    ///            .batch_nominate(chunks, TxStatus::InBlock)
    ///            .await
    ///            .unwrap();
    ///    }
    /// }
    /// ```
    async fn batch_bond(
        &self,
        accounts: &[(AccountId, AccountId)],
        stake: Balance,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;

    /// Send batch of [`nominate`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.nominate) calls.
    /// * `nominator_nominee_pairs` - a slice of account ids pairs (nominator, nominee)
    /// * `status` - a [`TxStatus`] of a tx to wait for
    ///
    /// # Examples
    /// see [`Self::batch_bond`] example above
    async fn batch_nominate(
        &self,
        nominator_nominee_pairs: &[(AccountId, AccountId)],
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;
}

/// Pallet staking api that requires sudo.
#[async_trait::async_trait]
pub trait StakingSudoApi {
    /// API for [`force_new_era`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.force_new_era) call.
    async fn force_new_era(&self, status: TxStatus) -> anyhow::Result<TxInfo>;

    /// API for [`set_staking_config`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.set_staking_configs) call.
    async fn set_staking_config(
        &self,
        minimal_nominator_bond: Option<Balance>,
        minimal_validator_bond: Option<Balance>,
        max_nominators_count: Option<u32>,
        max_validators_count: Option<u32>,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo>;
}

/// Logic for retrieving raw storage keys or values from a pallet staking.
#[async_trait::async_trait]
pub trait StakingRawApi {
    /// Returns all encoded [`eras_stakers`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.eras_stakers).
    /// storage keys for a given era
    /// * `era` - an era index
    /// * `at` - optional hash of a block to query state from
    ///
    /// # Examples
    /// ```ignore
    /// let stakers = connection
    ///         .get_stakers_storage_keys(current_era, None)
    ///         .await
    ///         .into_iter()
    ///         .map(|key| key.0);
    /// ```
    async fn get_stakers_storage_keys(
        &self,
        era: EraIndex,
        at: Option<BlockHash>,
    ) -> anyhow::Result<Vec<StorageKey>>;

    /// Returns encoded [`eras_stakers`](https://paritytech.github.io/substrate/master/pallet_staking/struct.Pallet.html#method.eras_stakers).
    /// storage keys for a given era and given account ids
    /// * `era` - an era index
    /// * `accounts` - list of account ids
    /// * `at` - optional hash of a block to query state from
    async fn get_stakers_storage_keys_from_accounts(
        &self,
        era: EraIndex,
        accounts: &[AccountId],
        at: Option<BlockHash>,
    ) -> Vec<StorageKey>;
}

#[async_trait::async_trait]
impl<C: ConnectionApi + AsConnection> StakingApi for C {
    async fn get_active_era(&self, at: Option<BlockHash>) -> EraIndex {
        let addrs = api::storage().staking().active_era();

        self.get_storage_entry(&addrs, at).await.index
    }

    async fn get_current_era(&self, at: Option<BlockHash>) -> EraIndex {
        let addrs = api::storage().staking().current_era();

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_bonded(&self, stash: AccountId, at: Option<BlockHash>) -> Option<AccountId> {
        let addrs = api::storage().staking().bonded(stash);

        self.get_storage_entry_maybe(&addrs, at).await
    }

    async fn get_ledger(&self, controller: AccountId, at: Option<BlockHash>) -> StakingLedger {
        let addrs = api::storage().staking().ledger(controller);

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_payout_for_era(&self, era: EraIndex, at: Option<BlockHash>) -> Balance {
        let addrs = api::storage().staking().eras_validator_reward(era);

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_exposure(
        &self,
        era: EraIndex,
        account_id: &AccountId,
        at: Option<BlockHash>,
    ) -> Exposure<AccountId, Balance> {
        let addrs = api::storage().staking().eras_stakers(era, account_id);

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_era_reward_points(
        &self,
        era: EraIndex,
        at: Option<BlockHash>,
    ) -> Option<EraRewardPoints<AccountId>> {
        let addrs = api::storage().staking().eras_reward_points(era);

        self.get_storage_entry_maybe(&addrs, at).await
    }

    async fn get_minimum_validator_count(&self, at: Option<BlockHash>) -> u32 {
        let addrs = api::storage().staking().minimum_validator_count();

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_session_per_era(&self) -> anyhow::Result<u32> {
        let addrs = api::constants().staking().sessions_per_era();
        self.as_connection()
            .as_client()
            .constants()
            .at(&addrs)
            .map_err(|e| e.into())
    }
}

#[async_trait::async_trait]
impl<S: SignedConnectionApi> StakingUserApi for S {
    async fn bond(
        &self,
        initial_stake: Balance,
        controller_id: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx().staking().bond(
            MultiAddress::<AccountId, ()>::Id(controller_id),
            initial_stake,
            RewardDestination::Staked,
        );

        self.send_tx(tx, status).await
    }

    async fn validate(
        &self,
        validator_commission_percentage: u8,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx().staking().validate(ValidatorPrefs {
            commission: Perbill(
                SPerbill::from_percent(validator_commission_percentage as u32).deconstruct(),
            ),
            blocked: false,
        });

        self.send_tx(tx, status).await
    }

    async fn payout_stakers(
        &self,
        stash_account: AccountId,
        era: EraIndex,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx().staking().payout_stakers(stash_account, era);

        self.send_tx(tx, status).await
    }

    async fn nominate(
        &self,
        nominee_account_id: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx()
            .staking()
            .nominate(vec![MultiAddress::Id(nominee_account_id)]);

        self.send_tx(tx, status).await
    }

    async fn chill(&self, status: TxStatus) -> anyhow::Result<TxInfo> {
        let tx = api::tx().staking().chill();

        self.send_tx(tx, status).await
    }

    async fn bond_extra_stake(
        &self,
        extra_stake: Balance,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let tx = api::tx().staking().bond_extra(extra_stake);

        self.send_tx(tx, status).await
    }
}

#[async_trait::async_trait]
impl StakingSudoApi for RootConnection {
    async fn force_new_era(&self, status: TxStatus) -> anyhow::Result<TxInfo> {
        let call = Staking(force_new_era);

        self.sudo_unchecked(call, status).await
    }

    async fn set_staking_config(
        &self,
        min_nominator_bond: Option<Balance>,
        min_validator_bond: Option<Balance>,
        max_nominator_count: Option<u32>,
        max_validator_count: Option<u32>,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        fn convert<T>(arg: Option<T>) -> ConfigOp<T> {
            match arg {
                Some(v) => Set(v),
                None => Noop,
            }
        }
        let call = Staking(set_staking_configs {
            min_nominator_bond: convert(min_nominator_bond),
            min_validator_bond: convert(min_validator_bond),
            max_nominator_count: convert(max_nominator_count),
            max_validator_count: convert(max_validator_count),
            chill_threshold: ConfigOp::Noop,
            min_commission: ConfigOp::Noop,
        });
        self.sudo_unchecked(call, status).await
    }
}

#[async_trait::async_trait]
impl<C: AsConnection + Sync> StakingRawApi for C {
    async fn get_stakers_storage_keys(
        &self,
        era: EraIndex,
        at: Option<BlockHash>,
    ) -> anyhow::Result<Vec<StorageKey>> {
        let key_addrs = api::storage().staking().eras_stakers_root();
        let mut key = key_addrs.to_root_bytes();
        StorageMapKey::new(era, StorageHasher::Twox64Concat).to_bytes(&mut key);
        self.as_connection()
            .as_client()
            .storage()
            .fetch_keys(&key, 10, None, at)
            .await
            .map_err(|e| e.into())
    }

    async fn get_stakers_storage_keys_from_accounts(
        &self,
        era: EraIndex,
        accounts: &[AccountId],
        _: Option<BlockHash>,
    ) -> Vec<StorageKey> {
        let key_addrs = api::storage().staking().eras_stakers_root();
        let mut key = key_addrs.to_root_bytes();
        StorageMapKey::new(era, StorageHasher::Twox64Concat).to_bytes(&mut key);
        accounts
            .iter()
            .map(|account| {
                let mut key = key.clone();
                StorageMapKey::new(account, StorageHasher::Twox64Concat).to_bytes(&mut key);

                StorageKey(key)
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl StakingApiExt for RootConnection {
    async fn batch_bond(
        &self,
        accounts: &[(AccountId, AccountId)],
        stake: Balance,
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let calls = accounts
            .iter()
            .map(|(s, c)| {
                let b = Staking(bond {
                    controller: MultiAddress::Id(c.clone()),
                    value: stake,
                    payee: RewardDestination::Staked,
                });

                Sudo(sudo_as {
                    who: MultiAddress::Id(s.clone()),
                    call: Box::new(b),
                })
            })
            .collect();

        self.batch_call(calls, status).await
    }

    async fn batch_nominate(
        &self,
        nominator_nominee_pairs: &[(AccountId, AccountId)],
        status: TxStatus,
    ) -> anyhow::Result<TxInfo> {
        let calls = nominator_nominee_pairs
            .iter()
            .map(|(nominator, nominee)| {
                let call = Staking(nominate {
                    targets: vec![MultiAddress::Id(nominee.clone())],
                });
                Sudo(sudo_as {
                    who: MultiAddress::Id(nominator.clone()),
                    call: Box::new(call),
                })
            })
            .collect();

        self.batch_call(calls, status).await
    }
}
