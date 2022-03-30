use crate::{send_xt, Connection};
use sp_core::Pair;
use substrate_api_client::{compose_call, compose_extrinsic, XtStatus};

pub fn set_code(connection: &Connection, runtime: Vec<u8>, status: XtStatus) {
    let call = compose_call!(connection.metadata, "System", "set_code", runtime);
    let xt = compose_extrinsic!(connection, "Sudo", "sudo_unchecked_weight", call, 0_u64);
    send_xt(&connection, xt.hex_encode(), "set_code", status);
}
