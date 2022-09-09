use sp_core::Pair;
use sp_runtime::Perbill;
use substrate_api_client::{compose_call, compose_extrinsic, ExtrinsicParams, XtStatus};

use crate::{send_xt, try_send_xt, AnyConnection, CallSystem, RootConnection};

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

impl CallSystem for RootConnection {
    type Error = substrate_api_client::error::Error;

    fn fill_block(&self, target_ratio_percent: u32, status: XtStatus) -> Result<(), Self::Error> {
        let connection = self.as_connection();
        let target_ratio_perbill = Perbill::from_percent(target_ratio_percent);
        let call = compose_call!(
            connection.metadata,
            "System",
            "fill_block",
            target_ratio_perbill.deconstruct()
        );
        let xt = compose_extrinsic!(connection, "Sudo", "sudo", call);
        try_send_xt(&connection, xt, Some("fill block"), status).map(|_| ())
    }
}
