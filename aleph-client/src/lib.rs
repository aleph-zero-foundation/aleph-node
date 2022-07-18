use std::{default::Default, thread::sleep, time::Duration};

use ac_primitives::SubstrateDefaultSignedExtra;
pub use account::{get_free_balance, locks};
pub use balances::total_issuance;
use codec::{Decode, Encode};
pub use debug::print_storages;
pub use fee::{get_next_fee_multiplier, get_tx_fee_info, FeeInfo};
use log::{info, warn};
pub use multisig::{
    compute_call_hash, perform_multisig_with_threshold_1, MultisigError, MultisigParty,
    SignatureAggregation,
};
pub use rpc::{rotate_keys, rotate_keys_raw_result, state_query_storage_at};
pub use session::{
    change_next_era_reserved_validators, change_validators, get_current_session, get_session,
    get_session_period, set_keys, wait_for as wait_for_session,
    wait_for_at_least as wait_for_at_least_session, Keys as SessionKeys,
};
use sp_core::{sr25519, storage::StorageKey, Pair, H256};
use sp_runtime::{generic::Header as GenericHeader, traits::BlakeTwo256};
pub use staking::{
    batch_bond as staking_batch_bond, batch_nominate as staking_batch_nominate,
    bond as staking_bond, bonded as staking_bonded, force_new_era as staking_force_new_era,
    get_current_era, get_era, get_era_reward_points, get_exposure, get_payout_for_era,
    get_sessions_per_era, ledger as staking_ledger, multi_bond as staking_multi_bond,
    nominate as staking_nominate, payout_stakers, payout_stakers_and_assert_locked_balance,
    set_staking_limits as staking_set_staking_limits, validate as staking_validate,
    wait_for_full_era_completion, wait_for_next_era, RewardPoint, StakingLedger,
};
pub use substrate_api_client;
use substrate_api_client::{
    rpc::ws_client::WsRpcClient, std::error::Error, AccountId, Api, ApiResult,
    PlainTipExtrinsicParams, RpcClient, UncheckedExtrinsicV4, XtStatus,
};
pub use system::set_code;
pub use transfer::{
    batch_transfer as balances_batch_transfer, transfer as balances_transfer, TransferTransaction,
};
pub use treasury::{
    approve as approve_treasury_proposal, proposals_counter as treasury_proposals_counter,
    propose as make_treasury_proposal, reject as reject_treasury_proposal, staking_treasury_payout,
    treasury_account,
};
pub use vesting::{
    get_schedules, merge_schedules, vest, vest_other, vested_transfer, VestingError,
    VestingSchedule,
};
pub use waiting::{wait_for_event, wait_for_finalized_block};

mod account;
mod balances;
mod debug;
mod fee;
mod multisig;
mod rpc;
mod session;
mod staking;
mod system;
mod transfer;
mod treasury;
mod vesting;
mod waiting;

pub trait FromStr: Sized {
    type Err;

    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

impl FromStr for WsRpcClient {
    type Err = ();

    fn from_str(url: &str) -> Result<Self, Self::Err> {
        Ok(WsRpcClient::new(url))
    }
}

pub type BlockNumber = u32;
pub type Header = GenericHeader<BlockNumber, BlakeTwo256>;
pub type KeyPair = sr25519::Pair;
pub type Connection = Api<KeyPair, WsRpcClient, PlainTipExtrinsicParams>;
pub type Extrinsic<Call> = UncheckedExtrinsicV4<Call, SubstrateDefaultSignedExtra>;

/// Common abstraction for different types of connections.
pub trait AnyConnection: Clone + Send {
    /// 'Castability' to `Connection`.
    ///
    /// Direct casting is often more handy than generic `.into()`. Justification: `Connection`
    /// objects are often passed to some macro like `compose_extrinsic!` and thus there is not
    /// enough information for type inferring required for `Into<Connection>`.
    fn as_connection(&self) -> Connection;

    /// Reads value from storage. Panics if it couldn't be read.
    fn read_storage_value<T: Decode>(&self, pallet: &'static str, key: &'static str) -> T {
        self.read_storage_value_or_else(pallet, key, || {
            panic!("Value is `None` or couldn't have been decoded")
        })
    }

    /// Reads value from storage. In case value is `None` or couldn't have been decoded, result of
    /// `fallback` is returned.
    fn read_storage_value_or_else<F: FnOnce() -> T, T: Decode>(
        &self,
        pallet: &'static str,
        key: &'static str,
        fallback: F,
    ) -> T {
        self.as_connection()
            .get_storage_value(pallet, key, None)
            .unwrap_or_else(|e| {
                panic!(
                    "Key `{}::{}` should be present in storage, error {:?}",
                    pallet, key, e
                )
            })
            .unwrap_or_else(fallback)
    }

    /// Reads value from storage. In case value is `None` or couldn't have been decoded, the default
    /// value is returned.
    fn read_storage_value_or_default<T: Decode + Default>(
        &self,
        pallet: &'static str,
        key: &'static str,
    ) -> T {
        self.read_storage_value_or_else(pallet, key, Default::default)
    }

    /// Reads pallet's constant from metadata. Panics if it couldn't be read.
    fn read_constant<T: Decode>(&self, pallet: &'static str, constant: &'static str) -> T {
        self.read_constant_or_else(pallet, constant, || {
            panic!(
                "Constant `{}::{}` should be present and decodable",
                pallet, constant
            )
        })
    }

    /// Reads pallet's constant from metadata. In case value is `None` or couldn't have been
    /// decoded, result of `fallback` is returned.
    fn read_constant_or_else<F: FnOnce() -> T, T: Decode>(
        &self,
        pallet: &'static str,
        constant: &'static str,
        fallback: F,
    ) -> T {
        self.as_connection()
            .get_constant(pallet, constant)
            .unwrap_or_else(|_| fallback())
    }

    /// Reads pallet's constant from metadata. In case value is `None` or couldn't have been
    /// decoded, the default value is returned.
    fn read_constant_or_default<T: Decode + Default>(
        &self,
        pallet: &'static str,
        constant: &'static str,
    ) -> T {
        self.read_constant_or_else(pallet, constant, Default::default)
    }
}

impl AnyConnection for Connection {
    fn as_connection(&self) -> Connection {
        self.clone()
    }
}

/// A connection that is signed.
#[derive(Clone)]
pub struct SignedConnection {
    inner: Connection,
    signer: KeyPair,
}

impl SignedConnection {
    pub fn new(address: &str, signer: KeyPair) -> Self {
        let unsigned = create_connection(address);
        Self {
            inner: unsigned.set_signer(signer.clone()),
            signer,
        }
    }

    /// Semantically equivalent to `connection.set_signer(signer)`.
    pub fn from_any_connection<C: AnyConnection>(connection: &C, signer: KeyPair) -> Self {
        Self {
            inner: connection
                .clone()
                .as_connection()
                .set_signer(signer.clone()),
            signer,
        }
    }

    /// A signer corresponding to `self.inner`.
    pub fn signer(&self) -> KeyPair {
        self.signer.clone()
    }
}

impl AnyConnection for SignedConnection {
    fn as_connection(&self) -> Connection {
        self.inner.clone()
    }
}

/// We can always try casting `AnyConnection` to `SignedConnection`, which fails if it is not
/// signed.
impl TryFrom<Connection> for SignedConnection {
    type Error = &'static str;

    fn try_from(connection: Connection) -> Result<Self, Self::Error> {
        if let Some(signer) = connection.signer.clone() {
            Ok(Self::from_any_connection(&connection, signer))
        } else {
            Err("Connection should be signed.")
        }
    }
}

/// A connection that is signed by the root account.
///
/// Since verifying signature is expensive (requires interaction with the node for checking
/// storage), there is no guarantee that in fact the signer has sudo access. Hence, effectively it
/// is just a type wrapper requiring explicit casting.
#[derive(Clone)]
pub struct RootConnection {
    inner: SignedConnection,
}

impl RootConnection {
    pub fn new(address: &str, root: KeyPair) -> Self {
        Self {
            inner: SignedConnection::new(address, root),
        }
    }

    /// A direct casting is often more handy than a generic `.into()`.
    pub fn as_signed(&self) -> SignedConnection {
        self.inner.clone()
    }
}

impl From<SignedConnection> for RootConnection {
    fn from(signed: SignedConnection) -> Self {
        Self { inner: signed }
    }
}

impl AnyConnection for RootConnection {
    fn as_connection(&self) -> Connection {
        self.as_signed().as_connection()
    }
}

pub fn create_connection(address: &str) -> Connection {
    create_custom_connection(address).expect("Connection should be created")
}

enum Protocol {
    Ws,
    Wss,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Ws
    }
}

impl ToString for Protocol {
    fn to_string(&self) -> String {
        match self {
            Protocol::Ws => String::from("ws://"),
            Protocol::Wss => String::from("wss://"),
        }
    }
}

/// Unless `address` already contains protocol, we prepend to it `ws://`.
fn ensure_protocol(address: &str) -> String {
    if address.starts_with(&Protocol::Ws.to_string())
        || address.starts_with(&Protocol::Wss.to_string())
    {
        return address.to_string();
    }
    format!("{}{}", Protocol::default().to_string(), address)
}

pub fn create_custom_connection<Client: FromStr + RpcClient>(
    address: &str,
) -> Result<Api<sr25519::Pair, Client, PlainTipExtrinsicParams>, <Client as FromStr>::Err> {
    loop {
        let client = Client::from_str(&ensure_protocol(address))?;
        match Api::<sr25519::Pair, _, _>::new(client) {
            Ok(api) => return Ok(api),
            Err(why) => {
                warn!(
                    "[+] Can't create_connection because {:?}, will try again in 1s",
                    why
                );
                sleep(Duration::from_millis(1000));
            }
        }
    }
}

/// `panic`able utility wrapper for `try_send_xt`.
pub fn send_xt<T: Encode, C: AnyConnection>(
    connection: &C,
    xt: Extrinsic<T>,
    xt_name: Option<&'static str>,
    xt_status: XtStatus,
) -> Option<H256> {
    try_send_xt(connection, xt, xt_name, xt_status).expect("Should manage to send extrinsic")
}

/// Sends transaction `xt` using `connection`.
///
/// If `tx_status` is either `Finalized` or `InBlock`, additionally returns hash of the containing
/// block. `xt_name` is used only for logging purposes.
///
/// Recoverable.
pub fn try_send_xt<T: Encode, C: AnyConnection>(
    connection: &C,
    xt: Extrinsic<T>,
    xt_name: Option<&'static str>,
    xt_status: XtStatus,
) -> ApiResult<Option<H256>> {
    let hash = connection
        .as_connection()
        .send_extrinsic(xt.hex_encode(), xt_status)?
        .ok_or_else(|| Error::Other(String::from("Could not get tx/block hash").into()))?;

    match xt_status {
        XtStatus::Finalized | XtStatus::InBlock => {
            info!(target: "aleph-client",
                "Transaction `{}` was included in block with hash {}.",
                xt_name.unwrap_or_default(), hash);
            Ok(Some(hash))
        }
        // Other variants either do not return (see https://github.com/scs/substrate-api-client/issues/175)
        // or return xt hash, which is kinda useless here.
        _ => Ok(None),
    }
}

pub fn keypair_from_string(seed: &str) -> KeyPair {
    KeyPair::from_string(seed, None).expect("Can't create pair from seed value")
}

pub fn account_from_keypair(keypair: &KeyPair) -> AccountId {
    AccountId::from(keypair.public())
}

fn storage_key(module: &str, version: &str) -> [u8; 32] {
    let pallet_name = sp_core::hashing::twox_128(module.as_bytes());
    let postfix = sp_core::hashing::twox_128(version.as_bytes());
    let mut final_key = [0u8; 32];
    final_key[..16].copy_from_slice(&pallet_name);
    final_key[16..].copy_from_slice(&postfix);
    final_key
}

/// Computes hash of given pallet's call. You can use that to pass result to `state.getKeys` RPC call.
/// * `pallet` name of the pallet
/// * `call` name of the pallet's call
///
/// # Example
/// ```
/// use aleph_client::get_storage_key;
///
/// let staking_nominate_storage_key = get_storage_key("Staking", "Nominators");
/// assert_eq!(staking_nominate_storage_key, String::from("5f3e4907f716ac89b6347d15ececedca9c6a637f62ae2af1c7e31eed7e96be04"));
/// ```
pub fn get_storage_key(pallet: &str, call: &str) -> String {
    let bytes = storage_key(pallet, call);
    let storage_key = StorageKey(bytes.into());
    hex::encode(storage_key.0)
}

pub fn get_block_hash<C: AnyConnection>(connection: &C, block_number: u32) -> H256 {
    connection
        .as_connection()
        .get_block_hash(Some(block_number))
        .expect("API call should have succeeded.")
        .unwrap_or_else(|| {
            panic!("Failed to obtain block hash for block {}.", block_number);
        })
}
