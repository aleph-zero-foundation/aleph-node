use std::fs;

use aleph_client::{pallets::system::SystemSudoApi, RootConnection, TxStatus};

pub async fn update_runtime(connection: RootConnection, runtime: String) {
    let runtime = fs::read(runtime).expect("Runtime file not found");
    connection
        .set_code(runtime, TxStatus::InBlock)
        .await
        .unwrap();
}
