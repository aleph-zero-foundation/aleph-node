use std::{collections::HashMap, fmt::Formatter, str::FromStr};

use codec::{Decode, Encode};
use frame_system::AccountInfo as SubstrateAccountInfo;
use log::info;
use pallet_balances::{AccountData as SubstrateAccountData, ExtraFlags};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    types::{AccountId, Balance, StorageKey, StoragePath, StorageValue},
    Storage,
};

/// Deserializable `AccountData`.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, Default)]
pub struct AccountData(SubstrateAccountData<Balance>);

/// Deserializable `AccountInfo`.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq, Default)]
pub struct AccountInfo(SubstrateAccountInfo<u32, AccountData>);

/// hack to deserialize ExtraFlags
fn deserialize_flags(repr: u128) -> ExtraFlags {
    let mut def = ExtraFlags::old_logic();
    if def.encode() != repr.encode() {
        def.set_new_logic();
    }

    def
}

impl<'de> Deserialize<'de> for AccountInfo {
    fn deserialize<D>(deserializer: D) -> Result<AccountInfo, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Nonce,
            Consumers,
            Providers,
            Sufficients,
            Free,
            Reserved,
            Frozen,
            Flags,
        }

        struct AccountInfoVisitor;

        impl<'de> Visitor<'de> for AccountInfoVisitor {
            type Value = AccountInfo;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("struct AccountInfo")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let (
                    mut nonce,
                    mut consumers,
                    mut providers,
                    mut sufficients,
                    mut free,
                    mut reserved,
                    mut frozen,
                    mut flags,
                ) = (None, None, None, None, None, None, None, None);
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Nonce => nonce = Some(map.next_value()?),
                        Field::Consumers => consumers = Some(map.next_value()?),
                        Field::Providers => providers = Some(map.next_value()?),
                        Field::Sufficients => sufficients = Some(map.next_value()?),
                        Field::Free => free = Some(map.next_value()?),
                        Field::Reserved => reserved = Some(map.next_value()?),
                        Field::Frozen => frozen = Some(map.next_value()?),
                        Field::Flags => flags = Some(map.next_value()?),
                    }
                }
                Ok(AccountInfo(SubstrateAccountInfo {
                    nonce: nonce.expect("Missing `nonce`"),
                    consumers: consumers.expect("Missing `consumers`"),
                    providers: providers.expect("Missing `providers`"),
                    sufficients: sufficients.expect("Missing `sufficients`"),
                    data: AccountData(SubstrateAccountData {
                        free: free.expect("Missing `free`"),
                        reserved: reserved.expect("Missing `reserved`"),
                        frozen: frozen.expect("Missing `frozen`"),
                        flags: deserialize_flags(flags.expect("Missing `flags`")),
                    }),
                }))
            }
        }

        const FIELDS: &[&str] = &[
            "nonce",
            "consumers",
            "providers",
            "sufficients",
            "free",
            "reserved",
            "frozen",
            "flags",
        ];
        deserializer.deserialize_struct("AccountInfo", FIELDS, AccountInfoVisitor)
    }
}

impl From<AccountInfo> for StorageValue {
    fn from(account_info: AccountInfo) -> StorageValue {
        StorageValue::new(&hex::encode(Encode::encode(&account_info)))
    }
}

/// Create `AccountInfo` with all parameters set to `0` apart from free balances, which is
/// set to `free` and number of providers, which is set to `1`.
pub fn account_info_from_free(free: Balance) -> AccountInfo {
    AccountInfo(SubstrateAccountInfo {
        providers: 1,
        data: AccountData(SubstrateAccountData {
            free,
            ..SubstrateAccountData::default()
        }),
        ..SubstrateAccountInfo::default()
    })
}

pub type AccountSetting = HashMap<AccountId, AccountInfo>;

fn get_account_map() -> StoragePath {
    StoragePath::from_str("System.Account").unwrap()
}

pub fn apply_account_setting(mut state: Storage, setting: AccountSetting) -> Storage {
    let account_map: StorageKey = get_account_map().into();
    for (account, info) in setting {
        let account_hash = account.clone().into();
        let key = &account_map.join(&account_hash);

        state.top.insert(key.clone(), info.clone().into());
        info!(target: "fork-off", "Account info of `{:?}` set to `{:?}`", account, info);
    }
    state
}
