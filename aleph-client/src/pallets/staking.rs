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
    Connection, RootConnection, SignedConnection, SudoCall, TxStatus,
};

#[async_trait::async_trait]
pub trait StakingApi {
    async fn get_active_era(&self, at: Option<BlockHash>) -> EraIndex;
    async fn get_current_era(&self, at: Option<BlockHash>) -> EraIndex;
    async fn get_bonded(&self, stash: AccountId, at: Option<BlockHash>) -> Option<AccountId>;
    async fn get_ledger(&self, controller: AccountId, at: Option<BlockHash>) -> StakingLedger;
    async fn get_payout_for_era(&self, era: EraIndex, at: Option<BlockHash>) -> u128;
    async fn get_exposure(
        &self,
        era: EraIndex,
        account_id: &AccountId,
        at: Option<BlockHash>,
    ) -> Exposure<AccountId, u128>;
    async fn get_era_reward_points(
        &self,
        era: EraIndex,
        at: Option<BlockHash>,
    ) -> Option<EraRewardPoints<AccountId>>;
    async fn get_minimum_validator_count(&self, at: Option<BlockHash>) -> u32;
    async fn get_session_per_era(&self) -> anyhow::Result<u32>;
}

#[async_trait::async_trait]
pub trait StakingUserApi {
    async fn bond(
        &self,
        initial_stake: Balance,
        controller_id: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    async fn validate(
        &self,
        validator_commission_percentage: u8,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    async fn payout_stakers(
        &self,
        stash_account: AccountId,
        era: EraIndex,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    async fn nominate(
        &self,
        nominee_account_id: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    async fn chill(&self, status: TxStatus) -> anyhow::Result<BlockHash>;
    async fn bond_extra_stake(
        &self,
        extra_stake: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
pub trait StakingApiExt {
    async fn batch_bond(
        &self,
        accounts: &[(AccountId, AccountId)],
        stake: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
    async fn batch_nominate(
        &self,
        nominator_nominee_pairs: &[(AccountId, AccountId)],
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
pub trait StakingSudoApi {
    async fn force_new_era(&self, status: TxStatus) -> anyhow::Result<BlockHash>;
    async fn set_staking_config(
        &self,
        minimal_nominator_bond: Option<u128>,
        minimal_validator_bond: Option<u128>,
        max_nominators_count: Option<u32>,
        max_validators_count: Option<u32>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
pub trait StakingRawApi {
    async fn get_stakers_storage_keys(
        &self,
        era: EraIndex,
        at: Option<BlockHash>,
    ) -> anyhow::Result<Vec<StorageKey>>;
    async fn get_stakers_storage_keys_from_accounts(
        &self,
        era: EraIndex,
        accounts: &[AccountId],
        at: Option<BlockHash>,
    ) -> Vec<StorageKey>;
}

#[async_trait::async_trait]
impl StakingApi for Connection {
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

    async fn get_payout_for_era(&self, era: EraIndex, at: Option<BlockHash>) -> u128 {
        let addrs = api::storage().staking().eras_validator_reward(era);

        self.get_storage_entry(&addrs, at).await
    }

    async fn get_exposure(
        &self,
        era: EraIndex,
        account_id: &AccountId,
        at: Option<BlockHash>,
    ) -> Exposure<AccountId, u128> {
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

        self.client.constants().at(&addrs).map_err(|e| e.into())
    }
}

#[async_trait::async_trait]
impl StakingUserApi for SignedConnection {
    async fn bond(
        &self,
        initial_stake: Balance,
        controller_id: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
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
    ) -> anyhow::Result<BlockHash> {
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
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx().staking().payout_stakers(stash_account, era);

        self.send_tx(tx, status).await
    }

    async fn nominate(
        &self,
        nominee_account_id: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx()
            .staking()
            .nominate(vec![MultiAddress::Id(nominee_account_id)]);

        self.send_tx(tx, status).await
    }

    async fn chill(&self, status: TxStatus) -> anyhow::Result<BlockHash> {
        let tx = api::tx().staking().chill();

        self.send_tx(tx, status).await
    }

    async fn bond_extra_stake(
        &self,
        extra_stake: Balance,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let tx = api::tx().staking().bond_extra(extra_stake);

        self.send_tx(tx, status).await
    }
}

#[async_trait::async_trait]
impl StakingSudoApi for RootConnection {
    async fn force_new_era(&self, status: TxStatus) -> anyhow::Result<BlockHash> {
        let call = Staking(force_new_era);

        self.sudo_unchecked(call, status).await
    }

    async fn set_staking_config(
        &self,
        min_nominator_bond: Option<u128>,
        min_validator_bond: Option<u128>,
        max_nominator_count: Option<u32>,
        max_validator_count: Option<u32>,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
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
impl StakingRawApi for Connection {
    async fn get_stakers_storage_keys(
        &self,
        era: EraIndex,
        at: Option<BlockHash>,
    ) -> anyhow::Result<Vec<StorageKey>> {
        let key_addrs = api::storage().staking().eras_stakers_root();
        let mut key = key_addrs.to_root_bytes();
        StorageMapKey::new(era, StorageHasher::Twox64Concat).to_bytes(&mut key);
        self.client
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
    ) -> anyhow::Result<BlockHash> {
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

        self.as_signed().batch_call(calls, status).await
    }

    async fn batch_nominate(
        &self,
        nominator_nominee_pairs: &[(AccountId, AccountId)],
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
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

        self.as_signed().batch_call(calls, status).await
    }
}
