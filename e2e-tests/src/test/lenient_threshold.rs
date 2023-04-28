use aleph_client::{
    pallets::committee_management::{CommitteeManagementApi, CommitteeManagementSudoApi},
    sp_runtime::Perquintill,
    waiting::{BlockStatus, WaitingExt},
    TxStatus,
};

use crate::config::setup_test;

#[tokio::test]
pub async fn set_lenient_threshold_test() -> anyhow::Result<()> {
    let config = setup_test();
    let root_connection = config.create_root_connection().await;

    root_connection
        .set_lenient_threshold(69, TxStatus::Finalized)
        .await?;

    assert_eq!(
        Some(Perquintill::from_percent(69)),
        root_connection.get_lenient_threshold_percentage(None).await
    );

    Ok(())
}
