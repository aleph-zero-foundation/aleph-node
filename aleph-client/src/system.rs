use crate::{send_xt, AnyConnection, RootConnection};
use sp_core::Pair;
use substrate_api_client::{compose_call, compose_extrinsic, XtStatus};

pub fn set_code(connection: &RootConnection, runtime: Vec<u8>, status: XtStatus) {
    let call = compose_call!(
        connection.as_connection().metadata,
        "System",
        "set_code",
        runtime
    );
    let xt = compose_extrinsic!(
        connection.as_connection(),
        "Sudo",
        "sudo_unchecked_weight",
        call,
        0_u64
    );
    send_xt(connection, xt, Some("set_code"), status);
}
