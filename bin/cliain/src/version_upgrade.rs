use aleph_client::{pallets::aleph::AlephSudoApi, RootConnection};
use primitives::SessionIndex;

use crate::commands::{ExtrinsicState, Version};

pub async fn schedule_upgrade(
    connection: RootConnection,
    version: Version,
    session_for_upgrade: SessionIndex,
    expected_state: ExtrinsicState,
) -> anyhow::Result<()> {
    connection
        .schedule_finality_version_change(version, session_for_upgrade, expected_state.into())
        .await?;

    Ok(())
}
