use anyhow::Result;
use primitives::SessionIndex;
use substrate_api_client::{compose_call, compose_extrinsic, ApiClientError, XtStatus};

use crate::{try_send_xt, AnyConnection, RootConnection};

pub type Version = u32;

pub fn schedule_upgrade_with_state(
    connection: &RootConnection,
    version: Version,
    session: SessionIndex,
    state: XtStatus,
) -> Result<(), ApiClientError> {
    let connection = connection.as_connection();
    let upgrade_call = compose_call!(
        connection.metadata,
        "Aleph",
        "schedule_finality_version_change",
        version,
        session
    );
    let xt = compose_extrinsic!(
        connection,
        "Sudo",
        "sudo_unchecked_weight",
        upgrade_call,
        0_u64
    );
    try_send_xt(
        &connection,
        xt,
        Some("schedule finality version change"),
        state,
    )
    .map(|_| ())
}

pub fn schedule_upgrade(
    connection: &RootConnection,
    version: Version,
    session: SessionIndex,
) -> Result<(), ApiClientError> {
    schedule_upgrade_with_state(connection, version, session, XtStatus::Finalized)
}
