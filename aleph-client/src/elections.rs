use primitives::{BanConfig, BanInfo, CommitteeSeats, EraValidators, SessionCount, SessionIndex};
use sp_core::H256;

use crate::{get_session_first_block, AccountId, ReadStorage};

const PALLET: &str = "Elections";

pub fn get_committee_seats<C: ReadStorage>(
    connection: &C,
    block_hash: Option<H256>,
) -> CommitteeSeats {
    connection.read_storage_value_at_block(PALLET, "CommitteeSize", block_hash)
}

pub fn get_next_era_committee_seats<C: ReadStorage>(connection: &C) -> CommitteeSeats {
    connection.read_storage_value(PALLET, "NextEraCommitteeSize")
}

pub fn get_validator_block_count<C: ReadStorage>(
    connection: &C,
    account_id: &AccountId,
    block_hash: Option<H256>,
) -> Option<u32> {
    connection.read_storage_map(PALLET, "SessionValidatorBlockCount", account_id, block_hash)
}

pub fn get_current_era_validators<C: ReadStorage>(connection: &C) -> EraValidators<AccountId> {
    connection.read_storage_value(PALLET, "CurrentEraValidators")
}

pub fn get_current_era_reserved_validators<C: ReadStorage>(connection: &C) -> Vec<AccountId> {
    get_current_era_validators(connection).reserved
}

pub fn get_current_era_non_reserved_validators<C: ReadStorage>(connection: &C) -> Vec<AccountId> {
    get_current_era_validators(connection).non_reserved
}

pub fn get_next_era_reserved_validators<C: ReadStorage>(connection: &C) -> Vec<AccountId> {
    connection.read_storage_value(PALLET, "NextEraReservedValidators")
}

pub fn get_next_era_non_reserved_validators<C: ReadStorage>(connection: &C) -> Vec<AccountId> {
    connection.read_storage_value(PALLET, "NextEraNonReservedValidators")
}

pub fn get_next_era_validators<C: ReadStorage>(connection: &C) -> EraValidators<AccountId> {
    let reserved: Vec<AccountId> =
        connection.read_storage_value(PALLET, "NextEraReservedValidators");
    let non_reserved: Vec<AccountId> =
        connection.read_storage_value(PALLET, "NextEraNonReservedValidators");
    EraValidators {
        reserved,
        non_reserved,
    }
}

pub fn get_era_validators<C: ReadStorage>(
    connection: &C,
    session: SessionIndex,
) -> EraValidators<AccountId> {
    let block_hash = get_session_first_block(connection, session);
    connection.read_storage_value_at_block(PALLET, "CurrentEraValidators", Some(block_hash))
}

pub fn get_ban_config<C: ReadStorage>(connection: &C) -> BanConfig {
    connection.read_storage_value(PALLET, "BanConfig")
}

pub fn get_underperformed_validator_session_count<C: ReadStorage>(
    connection: &C,
    account_id: &AccountId,
) -> SessionCount {
    connection
        .read_storage_map(
            PALLET,
            "UnderperformedValidatorSessionCount",
            account_id,
            None,
        )
        .unwrap_or(0)
}

pub fn get_ban_reason_for_validator<C: ReadStorage>(
    connection: &C,
    account_id: &AccountId,
) -> Option<BanInfo> {
    connection.read_storage_map(PALLET, "Banned", account_id, None)
}
