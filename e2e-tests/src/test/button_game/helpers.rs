use std::{
    fmt::Debug,
    sync::Arc,
    time::{Duration, Instant},
};

use aleph_client::{
    contract::event::{listen_contract_events, ContractEvent},
    pallets::balances::BalanceUserApi,
    Connection, ConnectionApi, KeyPair, SignedConnection, TxStatus,
};
use anyhow::{bail, Result};
use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver},
    StreamExt,
};
use itertools::Itertools;
use log::{info, warn};
use primitives::Balance;
use rand::Rng;
use tokio::time::timeout;

use super::contracts::{
    ButtonInstance, MarketplaceInstance, PSP22TokenInstance, SimpleDexInstance, WAzeroInstance,
};
use crate::config::Config;

/// Creates a copy of the `connection` signed by `signer`
pub fn sign(conn: &Connection, signer: &KeyPair) -> SignedConnection {
    let signer = KeyPair::new(signer.signer().clone());
    SignedConnection::from_connection(conn.clone(), signer)
}

/// Returns a ticket token instance for the given button instance
pub(super) async fn ticket_token<C: ConnectionApi>(
    conn: &C,
    button: &ButtonInstance,
    config: &Config,
) -> Result<PSP22TokenInstance> {
    PSP22TokenInstance::new(
        button.ticket_token(conn).await?,
        &config.test_case_params.ticket_token_metadata,
    )
}

/// Returns a reward token instance for the given button instance
pub(super) async fn reward_token<C: ConnectionApi>(
    conn: &C,
    button: &ButtonInstance,
    config: &Config,
) -> Result<PSP22TokenInstance> {
    PSP22TokenInstance::new(
        button.reward_token(conn).await?,
        &config.test_case_params.reward_token_metadata,
    )
}

/// Returns a marketplace instance for the given button instance
pub(super) async fn marketplace<C: ConnectionApi>(
    conn: &C,
    button: &ButtonInstance,
    config: &Config,
) -> Result<MarketplaceInstance> {
    MarketplaceInstance::new(
        button.marketplace(conn).await?,
        &config.test_case_params.marketplace_metadata,
    )
}

/// Derives a test account based on a randomized string
pub fn random_account() -> KeyPair {
    aleph_client::keypair_from_string(&format!(
        "//TestAccount/{}",
        rand::thread_rng().gen::<u128>()
    ))
}

/// Transfer `amount` from `from` to `to`
pub async fn transfer(
    conn: &Connection,
    from: &KeyPair,
    to: &KeyPair,
    amount: Balance,
) -> Result<()> {
    let from = KeyPair::new(from.signer().clone());
    SignedConnection::from_connection(conn.clone(), from)
        .transfer_keep_alive(to.account_id().clone(), amount, TxStatus::Finalized)
        .await
        .map(|_| ())
}

/// Returns a number representing the given amount of alephs (adding decimals)
pub fn alephs(basic_unit_amount: Balance) -> Balance {
    basic_unit_amount * 1_000_000_000_000
}

/// Returns the given number multiplied by 10^6.
pub fn mega(x: Balance) -> Balance {
    x * 1_000_000
}

pub(super) struct ButtonTestContext {
    pub button: Arc<ButtonInstance>,
    pub ticket_token: Arc<PSP22TokenInstance>,
    pub reward_token: Arc<PSP22TokenInstance>,
    pub marketplace: Arc<MarketplaceInstance>,
    pub conn: Connection,
    /// A [BufferedReceiver] preconfigured to listen for events of `button`, `ticket_token`, `reward_token`, and
    /// `marketplace`.
    pub events: BufferedReceiver<Result<ContractEvent>>,
    /// The authority owning the initial supply of tickets and with the power to mint game tokens.
    pub authority: KeyPair,
    /// A random account with some money for transaction fees.
    pub player: KeyPair,
}

pub(super) struct DexTestContext {
    pub conn: Connection,
    /// An authority with the power to mint tokens and manage the dex.
    pub authority: KeyPair,
    /// A random account with some money for fees.
    pub account: KeyPair,
    pub dex: Arc<SimpleDexInstance>,
    pub token1: Arc<PSP22TokenInstance>,
    pub token2: Arc<PSP22TokenInstance>,
    pub token3: Arc<PSP22TokenInstance>,
    /// A [BufferedReceiver] preconfigured to listen for events of `dex`, `token1`, `token2`, and `token3`.
    pub events: BufferedReceiver<Result<ContractEvent>>,
}

pub(super) struct WAzeroTestContext {
    pub conn: Connection,
    /// A random account with some money for fees.
    pub account: KeyPair,
    pub wazero: Arc<WAzeroInstance>,
    /// A [BufferedReceiver] preconfigured to listen for events of `wazero`.
    pub events: BufferedReceiver<Result<ContractEvent>>,
}

pub(super) async fn setup_wrapped_azero_test(config: &Config) -> Result<WAzeroTestContext> {
    let (conn, _authority, account) = basic_test_context(config).await?;
    let wazero = Arc::new(WAzeroInstance::new(config)?);

    let contract = wazero.clone();
    let (events_tx, events_rx) = unbounded();
    let listen_conn = conn.clone();

    tokio::spawn(async move {
        let contract_metadata = vec![contract.as_ref().into()];

        listen_contract_events(&listen_conn, &contract_metadata, events_tx)
            .await
            .unwrap();
    });

    let events = BufferedReceiver::new(events_rx, Duration::from_secs(10));

    Ok(WAzeroTestContext {
        conn,
        account,
        wazero,
        events,
    })
}

pub(super) async fn setup_dex_test(config: &Config) -> Result<DexTestContext> {
    let (conn, authority, account) = basic_test_context(config).await?;

    let dex = Arc::new(SimpleDexInstance::new(config)?);
    let token1 =
        reward_token_for_button(config, &conn, &config.test_case_params.early_bird_special).await?;
    let token2 =
        reward_token_for_button(config, &conn, &config.test_case_params.the_pressiah_cometh)
            .await?;
    let token3 =
        reward_token_for_button(config, &conn, &config.test_case_params.back_to_the_future).await?;

    let c1 = dex.clone();
    let c2 = token1.clone();
    let c3 = token2.clone();
    let c4 = token3.clone();

    let (events_tx, events_rx) = unbounded();
    let listen_conn = conn.clone();

    tokio::spawn(async move {
        let contract_metadata = vec![
            c1.as_ref().into(),
            c2.as_ref().into(),
            c3.as_ref().into(),
            c4.as_ref().into(),
        ];

        listen_contract_events(&listen_conn, &contract_metadata, events_tx)
            .await
            .unwrap();
    });

    let events = BufferedReceiver::new(events_rx, Duration::from_secs(10));

    Ok(DexTestContext {
        conn,
        authority,
        account,
        dex,
        token1,
        token2,
        token3,
        events,
    })
}

async fn reward_token_for_button(
    config: &Config,
    conn: &Connection,
    button_contract_address: &Option<String>,
) -> Result<Arc<PSP22TokenInstance>> {
    let button = ButtonInstance::new(config, button_contract_address)?;
    Ok(Arc::new(reward_token(conn, &button, config).await?))
}

/// Sets up a number of objects commonly used in button game tests.
pub(super) async fn setup_button_test(
    config: &Config,
    button_contract_address: &Option<String>,
) -> Result<ButtonTestContext> {
    let (conn, authority, player) = basic_test_context(config).await?;

    info!("Setting up button contract instance");
    let button = Arc::new(ButtonInstance::new(config, button_contract_address)?);
    info!("Setting up ticket token contract instance");
    let ticket_token = Arc::new(ticket_token(&conn, &button, config).await?);
    info!("Setting up reward token contract instance");
    let reward_token = Arc::new(reward_token(&conn, &button, config).await?);
    info!("Setting up marketplace contract instance");
    let marketplace = Arc::new(marketplace(&conn, &button, config).await?);

    let c1 = button.clone();
    let c2 = ticket_token.clone();
    let c3 = reward_token.clone();
    let c4 = marketplace.clone();

    let (events_tx, events_rx) = unbounded();
    let listen_conn = conn.clone();

    tokio::spawn(async move {
        let contract_metadata = vec![
            c1.as_ref().into(),
            c2.as_ref().into(),
            c3.as_ref().into(),
            c4.as_ref().into(),
        ];

        info!("Listening for events from {:?}", contract_metadata);
        listen_contract_events(&listen_conn, &contract_metadata, events_tx)
            .await
            .unwrap();
    });

    let events = BufferedReceiver::new(events_rx, Duration::from_secs(10));

    Ok(ButtonTestContext {
        button,
        ticket_token,
        reward_token,
        marketplace,
        conn,
        events,
        authority,
        player,
    })
}

/// Prepares a `(conn, authority, account)` triple with some money in `account` for fees.
async fn basic_test_context(config: &Config) -> Result<(Connection, KeyPair, KeyPair)> {
    info!("Connecting to node at {}", config.node);
    let conn = Connection::new(&config.node).await;
    let authority = aleph_client::keypair_from_string(&config.sudo_seed);
    let account = random_account();

    info!("Transferring 100 ALEPH to a random test account");
    transfer(&conn, &authority, &account, alephs(100)).await?;

    Ok((conn, authority, account))
}

/// A receiver where it's possible to wait for messages out of order.
pub struct BufferedReceiver<T> {
    buffer: Vec<T>,
    receiver: UnboundedReceiver<T>,
    default_timeout: Duration,
}

impl<T> BufferedReceiver<T> {
    pub fn new(receiver: UnboundedReceiver<T>, default_timeout: Duration) -> Self {
        Self {
            buffer: Vec::new(),
            receiver,
            default_timeout,
        }
    }

    /// Receive a message satisfying `filter`.
    ///
    /// If such a message was received earlier and is waiting in the buffer, returns the message immediately and removes
    /// it from the buffer. Otherwise, listens for messages for `default_timeout`, storing them in the buffer. If a
    /// matching message is found during that time, it is returned. If not, `Err(RecvTimeoutError)` is returned.
    pub async fn recv_timeout<F: Fn(&T) -> bool>(&mut self, filter: F) -> Result<T>
    where
        T: Debug,
    {
        match self.buffer.iter().find_position(|m| filter(m)) {
            Some((i, _)) => Ok(self.buffer.remove(i)),
            None => {
                let mut remaining_timeout = self.default_timeout;

                while remaining_timeout > Duration::from_millis(0) {
                    let start = Instant::now();
                    match timeout(remaining_timeout, self.receiver.next()).await? {
                        Some(msg) => {
                            if filter(&msg) {
                                return Ok(msg);
                            } else {
                                info!("Buffering {:?}", msg);
                                self.buffer.push(msg);
                                remaining_timeout -= Instant::now().duration_since(start);
                            }
                        }
                        None => bail!("Receiver closed while waiting for message"),
                    }
                }

                bail!("Timeout while waiting for a message")
            }
        }
    }
}

/// Wait until `button` is dead.
///
/// Returns `Err(_)` if the button doesn't die within 30 seconds.
pub(super) async fn wait_for_death<C: ConnectionApi>(
    conn: &C,
    button: &ButtonInstance,
) -> Result<()> {
    info!("Waiting for the button to die");
    let mut iters = 0u8;
    let mut is_dead = false;

    while iters <= 10 {
        match timeout(Duration::from_secs(2), button.is_dead(conn)).await? {
            Err(e) => println!("Error while querying button.is_dead: {e:?}"),
            Ok(status) => is_dead = status,
        }

        if !is_dead {
            tokio::time::sleep(Duration::from_secs(3)).await;
            iters += 1;
        }

        if is_dead {
            break;
        }
    }

    if !is_dead {
        bail!("Button didn't die in time")
    }

    info!("Button died");
    Ok(())
}

/// Asserts that a message with `id` is received (within `events.default_timeout`) and returns it.
pub async fn assert_recv_id(
    events: &mut BufferedReceiver<Result<ContractEvent>>,
    id: &str,
) -> ContractEvent {
    assert_recv(
        events,
        |event| event.name == Some(id.to_string()),
        &format!("Expected {id:?} contract event"),
    )
    .await
}

/// Asserts that a message matching `filter` is received (within `events.default_timeout`) and returns it.
pub async fn assert_recv<T: Debug, F: Fn(&T) -> bool>(
    events: &mut BufferedReceiver<Result<T>>,
    filter: F,
    context: &str,
) -> T {
    let event = recv_timeout_with_log(events, filter).await;

    assert!(event.is_ok(), "{}", context);

    event.unwrap()
}

/// Asserts that a message with `id` is not received (within `events.default_timeout`).
pub async fn refute_recv_id(events: &mut BufferedReceiver<Result<ContractEvent>>, id: &str) {
    if let Ok(event) =
        recv_timeout_with_log(events, |event| event.name == Some(id.to_string())).await
    {
        panic!("Received unexpected event {event:?}");
    }
}

async fn recv_timeout_with_log<T: Debug, F: Fn(&T) -> bool>(
    events: &mut BufferedReceiver<Result<T>>,
    filter: F,
) -> Result<T> {
    match events
        .recv_timeout(|event_or_error| {
            if event_or_error.is_ok() {
                info!("Received contract event {:?}", event_or_error);
            } else {
                warn!("Contract event error {:?}", event_or_error);
            }

            event_or_error.as_ref().map(&filter).unwrap_or(false)
        })
        .await
    {
        Ok(event) => Ok(event.unwrap()),
        Err(err) => bail!(err),
    }
}
