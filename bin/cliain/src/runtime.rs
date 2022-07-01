use std::fs;

use aleph_client::{set_code, RootConnection};
use substrate_api_client::XtStatus;

pub fn update_runtime(connection: RootConnection, runtime: String) {
    let runtime = fs::read(runtime).expect("Runtime file not found");
    set_code(&connection, runtime, XtStatus::InBlock);
}
