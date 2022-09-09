use primitives::SessionIndex;
pub use primitives::{CommitteeSeats, EraValidators};
use sp_core::H256;
use substrate_api_client::AccountId;

use crate::{get_session_first_block, ReadStorage};

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

pub fn get_era_validators<C: ReadStorage>(
    connection: &C,
    session: SessionIndex,
) -> EraValidators<AccountId> {
    let block_hash = get_session_first_block(connection, session);
    connection.read_storage_value_at_block(PALLET, "CurrentEraValidators", Some(block_hash))
}
