mod config;

use clap::Parser;
use codec::Decode;
use common::create_connection;
use config::Config;
use log::{debug, error, info};
use sp_core::crypto::Ss58Codec;
use sp_core::{sr25519, Pair};
use sp_runtime::{generic, traits::BlakeTwo256};
use std::convert::TryFrom;
use std::env;
use std::iter;
use std::sync::mpsc::channel;
use substrate_api_client::rpc::ws_client::{EventsDecoder, RuntimeEvent};
use substrate_api_client::rpc::WsRpcClient;
use substrate_api_client::utils::FromHexString;
use substrate_api_client::{
    compose_call, compose_extrinsic, AccountId, Api, UncheckedExtrinsicV4, XtStatus,
};

type BlockNumber = u32;
type Header = generic::Header<BlockNumber, BlakeTwo256>;

fn main() -> anyhow::Result<()> {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        env::set_var(env_logger::DEFAULT_FILTER_ENV, "warn");
    }
    env_logger::init();

    let config: Config = Config::parse();

    test_finalization(config.clone())?;
    test_token_transfer(config.clone())?;
    test_change_validators(config)?;

    Ok(())
}

/// wait until blocks are getting finalized
fn test_finalization(config: Config) -> anyhow::Result<u32> {
    let connection = create_connection(config.node);
    // wait till at least one block is finalized
    wait_for_finalized_block(connection, 1)
}

fn test_token_transfer(config: Config) -> anyhow::Result<()> {
    let Config { node, seeds, .. } = config;

    let accounts: Vec<sr25519::Pair> = accounts(seeds);
    let from: sr25519::Pair = accounts.get(0).expect("No accounts passed").to_owned();
    let to = AccountId::from(
        accounts
            .get(1)
            .expect("Pass at least two accounts")
            .public(),
    );

    let connection = create_connection(node).set_signer(from);

    let balance_before = connection
        .get_account_data(&to)?
        .expect("Could not get account data")
        .free;

    info!("[+] Account {} balance before tx: {}", to, balance_before);

    let transfer_value = 1000u128;

    let tx: UncheckedExtrinsicV4<_> = compose_extrinsic!(
        connection,
        "Balances",
        "transfer",
        GenericAddress::Id(to.clone()),
        Compact(transfer_value)
    );

    // send and watch extrinsic until InBlock
    let tx_hash = connection
        .send_extrinsic(tx.hex_encode(), XtStatus::InBlock)?
        .expect("Could not get tx hash");

    info!("[+] Transaction hash: {}", tx_hash);

    let balance_after = connection
        .get_account_data(&to)?
        .expect("Could not get account data")
        .free;

    info!("[+] Account {} balance after tx: {}", to, balance_after);

    assert!(
        balance_before + transfer_value == balance_after,
        "before = {}, after = {}, tx = {}",
        balance_before,
        balance_after,
        transfer_value
    );

    Ok(())
}

#[derive(Debug, Decode)]
struct NewSessionEvent {
    session_index: u32,
}

fn test_change_validators(config: Config) -> anyhow::Result<()> {
    let Config { node, seeds, sudo } = config;

    let accounts = accounts(seeds);

    let sudo = match sudo {
        Some(seed) => keypair_from_string(seed),
        None => accounts.get(0).expect("whoops").to_owned(),
    };

    let connection = create_connection(node).set_signer(sudo);

    let validators_before: Vec<AccountId> = connection
        .get_storage_value("Session", "Validators", None)?
        .unwrap();

    info!("[+] Validators before tx: {:#?}", validators_before);

    let new_validators: Vec<AccountId> = accounts
        .into_iter()
        .map(|pair| pair.public().into())
        .chain(iter::once(
            AccountId::from_ss58check("5EHkv1FCd4jeQmVrbYhrETL1EAr8NJxNbukDRT4FaYWbjW8f").unwrap(),
        ))
        .collect();

    info!("[+] New validators {:#?}", new_validators);

    // wait beyond session 1
    let current_session_index = wait_for_session(connection.clone(), 1)?;
    let session_for_change = current_session_index + 2;
    info!("[+] Current session index {:?}", current_session_index);

    let call = compose_call!(
        connection.metadata,
        "Aleph",
        "change_validators",
        new_validators.clone(),
        session_for_change
    );

    let tx = compose_extrinsic!(connection, "Sudo", "sudo_unchecked_weight", call, 0_u64);

    // send and watch extrinsic until finalized
    let tx_hash = connection
        .send_extrinsic(tx.hex_encode(), XtStatus::Finalized)
        .expect("Could not send extrinsc")
        .expect("Could not get tx hash");

    info!("[+] change_validators transaction hash: {}", tx_hash);

    // wait for the change to be aplied
    wait_for_session(connection.clone(), session_for_change)?;

    let validators_after: Vec<AccountId> = connection
        .get_storage_value("Session", "Validators", None)?
        .unwrap();

    info!("[+] Validators after tx: {:#?}", validators_after);

    assert!(new_validators.eq(&validators_after));

    Ok(())
}

/// blocking wait, if ongoing session index is >= new_session_index returns the current
fn wait_for_session(
    connection: Api<sr25519::Pair, WsRpcClient>,
    new_session_index: u32,
) -> anyhow::Result<u32> {
    let module = "Session";
    let variant = "NewSession";
    info!("[+] Creating event subscription {}/{}", module, variant);
    let (events_in, events_out) = channel();
    connection.subscribe_events(events_in)?;

    let event_decoder = EventsDecoder::try_from(connection.metadata)?;

    loop {
        let event_str = events_out.recv().unwrap();
        let events = event_decoder.decode_events(&mut Vec::from_hex(event_str)?.as_slice());

        match events {
            Ok(raw_events) => {
                for (phase, event) in raw_events.into_iter() {
                    info!("[+] Received event: {:?}, {:?}", phase, event);
                    match event {
                        RuntimeEvent::Raw(raw)
                            if raw.module == module && raw.variant == variant =>
                        {
                            let NewSessionEvent { session_index } =
                                NewSessionEvent::decode(&mut &raw.data[..])?;
                            info!("[+] Decoded NewSession event {:?}", &session_index);
                            if session_index.ge(&new_session_index) {
                                return Ok(session_index);
                            }
                        }
                        _ => debug!("Ignoring some other event: {:?}", event),
                    }
                }
            }
            Err(why) => error!("Error {:?}", why),
        }
    }
}

/// blocks the main thread waiting for a block with a number at least `block_number`
fn wait_for_finalized_block(
    connection: Api<sr25519::Pair, WsRpcClient>,
    block_number: u32,
) -> anyhow::Result<u32> {
    let (sender, receiver) = channel();
    connection.subscribe_finalized_heads(sender)?;

    while let Ok(header) = receiver
        .recv()
        .map(|h| serde_json::from_str::<Header>(&h).unwrap())
    {
        info!("[+] Received header for a block number {:?}", header.number);

        if header.number.ge(&block_number) {
            return Ok(block_number);
        }
    }

    Err(anyhow::anyhow!("Giving up"))
}

fn keypair_from_string(seed: String) -> sr25519::Pair {
    sr25519::Pair::from_string(&seed, None).expect("Can't create pair from seed value")
}

fn accounts(seeds: Option<Vec<String>>) -> Vec<sr25519::Pair> {
    let seeds = seeds.unwrap_or_else(|| {
        vec![
            "//Damian".into(),
            "//Tomasz".into(),
            "//Zbyszko".into(),
            "//Hansu".into(),
        ]
    });
    seeds.into_iter().map(keypair_from_string).collect()
}
