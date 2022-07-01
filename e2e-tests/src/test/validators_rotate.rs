use std::collections::HashMap;

use aleph_client::{
    change_validators, get_current_session, wait_for_finalized_block, wait_for_full_era_completion,
    wait_for_session, AnyConnection, Header, KeyPair, RootConnection, SignedConnection,
};
use sp_core::Pair;
use substrate_api_client::{AccountId, XtStatus};

use crate::{
    accounts::{accounts_seeds_to_keys, get_sudo_key, get_validators_keys, get_validators_seeds},
    Config,
};

const TEST_LENGTH: u32 = 5;

fn get_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_keys(config)[0..2].to_vec()
}

fn get_non_reserved_validators(config: &Config) -> Vec<KeyPair> {
    get_validators_keys(config)[2..].to_vec()
}

fn get_non_reserved_validators_for_session(config: &Config, session: u32) -> Vec<AccountId> {
    // Test assumption
    const FREE_SEATS: u32 = 2;

    let mut non_reserved = vec![];

    let validators_seeds = get_validators_seeds(config);
    let non_reserved_nodes_order_from_runtime = validators_seeds[2..].to_vec();
    let non_reserved_nodes_order_from_runtime_len = non_reserved_nodes_order_from_runtime.len();

    for i in (FREE_SEATS * session)..(FREE_SEATS * (session + 1)) {
        non_reserved.push(
            non_reserved_nodes_order_from_runtime
                [i as usize % non_reserved_nodes_order_from_runtime_len]
                .clone(),
        );
    }

    accounts_seeds_to_keys(&non_reserved)
        .iter()
        .map(|pair| AccountId::from(pair.public()))
        .collect()
}

fn get_authorities_for_session<C: AnyConnection>(connection: &C, session: u32) -> Vec<AccountId> {
    const SESSION_PERIOD: u32 = 30;
    let first_block = SESSION_PERIOD * session;

    let block = connection
        .as_connection()
        .get_block_hash(Some(first_block))
        .expect("Api call should succeed")
        .expect("Session already started so the first block should be present");

    connection
        .as_connection()
        .get_storage_value("Session", "Validators", Some(block))
        .expect("Api call should succeed")
        .expect("Authorities should always be present")
}

pub fn validators_rotate(config: &Config) -> anyhow::Result<()> {
    let node = &config.node;
    let accounts = get_validators_keys(config);
    let sender = accounts.first().expect("Using default accounts").to_owned();
    let connection = SignedConnection::new(node, sender);

    let sudo = get_sudo_key(config);

    let root_connection = RootConnection::new(node, sudo);
    let reserved_validators: Vec<_> = get_reserved_validators(config)
        .iter()
        .map(|pair| AccountId::from(pair.public()))
        .collect();

    let non_reserved_validators = get_non_reserved_validators(config)
        .iter()
        .map(|pair| AccountId::from(pair.public()))
        .collect();

    change_validators(
        &root_connection,
        Some(reserved_validators.clone()),
        Some(non_reserved_validators),
        Some(4),
        XtStatus::InBlock,
    );
    wait_for_full_era_completion(&connection)?;

    let current_session = get_current_session(&connection);
    wait_for_session(&connection, current_session + TEST_LENGTH)?;

    let mut non_reserved_count = HashMap::new();

    for session in current_session..current_session + TEST_LENGTH {
        let elected = get_authorities_for_session(&connection, session);
        let non_reserved = get_non_reserved_validators_for_session(config, session);

        for nr in non_reserved.clone() {
            *non_reserved_count.entry(nr).or_insert(0) += 1;
        }

        let reserved_included = reserved_validators
            .clone()
            .iter()
            .all(|reserved| elected.contains(reserved));

        let non_reserved_include = non_reserved
            .iter()
            .all(|non_reserved| elected.contains(non_reserved));

        let only_expected_validators = elected
            .iter()
            .all(|elected| reserved_validators.contains(elected) || non_reserved.contains(elected));

        assert!(
            reserved_included,
            "Reserved nodes should always be present, session #{}",
            session
        );
        assert!(
            non_reserved_include,
            "Missing non reserved node, session #{}",
            session
        );
        assert!(
            only_expected_validators,
            "Only expected validators should be present, session #{}",
            session
        );
    }

    let max_elected = non_reserved_count.values().max().unwrap();
    let min_elected = non_reserved_count.values().min().unwrap();
    assert!(max_elected - min_elected <= 1);

    let block_number = connection
        .as_connection()
        .get_header::<Header>(None)
        .expect("Could not fetch header")
        .expect("Block exists; qed")
        .number;
    wait_for_finalized_block(&connection, block_number)?;

    Ok(())
}
