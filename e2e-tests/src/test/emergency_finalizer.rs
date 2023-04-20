use aleph_client::{
    pallets::aleph::{AlephApi, AlephSudoApi},
    waiting::{BlockStatus, WaitingExt},
    AlephKeyPair, AsConnection, Pair, TxStatus,
};

use crate::config::setup_test;

#[tokio::test]
async fn set_emergency_finalizer_test() -> anyhow::Result<()> {
    let config = setup_test();
    let (finalizer, _seed) = AlephKeyPair::generate();
    let public = finalizer.public().0;
    let root = config.create_root_connection().await;
    let current_finalizer = root.as_connection().emergency_finalizer(None).await;

    assert!(current_finalizer != Some(public));

    root.set_emergency_finalizer(finalizer.public().into(), TxStatus::Finalized)
        .await?;
    root.wait_for_n_sessions(2, BlockStatus::Finalized).await;

    let current_finalizer = root.as_connection().emergency_finalizer(None).await;
    assert_eq!(current_finalizer, Some(public));
    Ok(())
}
