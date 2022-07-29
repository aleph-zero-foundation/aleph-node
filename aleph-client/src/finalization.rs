use sp_core::Pair;
use substrate_api_client::{compose_call, compose_extrinsic, AccountId, ExtrinsicParams, XtStatus};

use crate::{send_xt, AnyConnection, RootConnection};

/// Sets the emergency finalizer to the provided `AccountId`.
pub fn set_emergency_finalizer(
    connection: &RootConnection,
    finalizer: AccountId,
    status: XtStatus,
) {
    let set_emergency_finalizer_call = compose_call!(
        connection.as_connection().metadata,
        "Aleph",
        "set_emergency_finalizer",
        finalizer
    );
    let xt = compose_extrinsic!(
        connection.as_connection(),
        "Sudo",
        "sudo_unchecked_weight",
        set_emergency_finalizer_call,
        0_u64
    );
    send_xt(connection, xt, Some("set_emergency_finalizer"), status);
}
