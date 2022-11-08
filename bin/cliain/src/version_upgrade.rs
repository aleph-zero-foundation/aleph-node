use aleph_client::{schedule_upgrade_with_state, RootConnection};
use anyhow::Error;
use primitives::SessionIndex;

use crate::commands::{ExtrinsicState, Version};

pub fn schedule_upgrade(
    connection: RootConnection,
    version: Version,
    session_for_upgrade: SessionIndex,
    expected_state: ExtrinsicState,
) -> anyhow::Result<()> {
    schedule_upgrade_with_state(
        &connection,
        version,
        session_for_upgrade,
        expected_state.into(),
    )
    .map_err(Error::new)
}
