use aleph_client::{set_code, Connection};
use std::fs;
use substrate_api_client::XtStatus;

pub fn update_runtime(connection: Connection, runtime: String) {
    let runtime = fs::read(runtime).expect("Runtime file not found");
    set_code(&connection, runtime, XtStatus::Finalized);
}
