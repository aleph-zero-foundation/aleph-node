use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter, Result as FmtResult},
    fs,
    path::PathBuf,
};

use aleph_client::{
    aleph_keypair_from_string, api, pallets::aleph::AlephRpc, primitives::app::Public,
    sp_core::H256, AlephKeyPair, BlockNumber, Connection, ConnectionApi, Pair,
};
use anyhow::Result;
use dialoguer::Confirm;
use futures::{stream::FuturesUnordered, StreamExt};
use subxt::config::Header;

fn pretty_print_h256(h: &H256) -> String {
    let prefix =
        h.0.iter()
            .take(4)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
    format!("0x{prefix}..")
}

#[derive(Debug, PartialEq, Eq)]
struct HashNum {
    num: BlockNumber,
    hash: H256,
}

impl Display for HashNum {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "[{}, {}]", self.num, pretty_print_h256(&self.hash))
    }
}

pub struct Connections {
    primary: Connection,
    secondaries: BTreeMap<String, Connection>,
}

impl Connections {
    pub async fn new(primary_endpoint: String, secondary_endpoints: Vec<String>) -> Self {
        let primary = Connection::new(&primary_endpoint).await;
        let mut secondaries: BTreeMap<String, Connection> = BTreeMap::new();
        for endpoint in secondary_endpoints {
            secondaries.insert(endpoint.clone(), Connection::new(&endpoint).await);
        }

        Connections {
            primary,
            secondaries,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
struct ChainStatus {
    best: HashNum,
    finalized: HashNum,
}

struct AllChainStatuses {
    primary: ChainStatus,
    secondaries: BTreeMap<String, ChainStatus>,
}

struct AllBlocksAtNum {
    primary: HashNum,
    secondaries: BTreeMap<String, HashNum>,
}

struct FinalizationPlan {
    finalized_base: HashNum,
    target: HashNum,
}

impl Display for ChainStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "best: {}, finalized: {}", self.best, self.finalized)
    }
}

async fn get_block_at_num(connection: &Connection, num: BlockNumber) -> Result<HashNum> {
    let hash = connection
        .as_client()
        .rpc()
        .block_hash(Some(num.into()))
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to fetch block hash at height {}.", num))?;
    Ok(HashNum { hash, num })
}

async fn get_all_blocks_at_num(
    connections: &Connections,
    num: BlockNumber,
) -> Result<AllBlocksAtNum> {
    let primary = get_block_at_num(&connections.primary, num).await?;
    let mut futures = FuturesUnordered::new();
    for (name, conn) in connections.secondaries.iter() {
        let name = name.clone();
        let conn = conn.clone();
        futures.push(async move { (name, get_block_at_num(&conn, num).await) });
    }
    let mut secondaries = BTreeMap::new();
    while let Some(result) = futures.next().await {
        let (name, status) = result;
        secondaries.insert(name, status?);
    }
    Ok(AllBlocksAtNum {
        primary,
        secondaries,
    })
}

async fn get_chain_status(connection: &Connection) -> Result<ChainStatus> {
    let finalized_hash = connection.as_client().rpc().finalized_head().await?;

    let finalized_block = connection
        .as_client()
        .rpc()
        .block(Some(finalized_hash))
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to fetch finalized block."))?;

    let finalized = HashNum {
        num: finalized_block.block.header.number,
        hash: finalized_block.block.header.hash(),
    };

    let best_block = connection
        .as_client()
        .rpc()
        .block(None)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to fetch finalized block."))?;

    let best = HashNum {
        num: best_block.block.header.number,
        hash: best_block.block.header.hash(),
    };

    Ok(ChainStatus { best, finalized })
}

async fn get_all_chain_statuses(connections: &Connections) -> Result<AllChainStatuses> {
    let primary = get_chain_status(&connections.primary).await?;
    let mut futures = FuturesUnordered::new();
    for (name, conn) in connections.secondaries.iter() {
        let name = name.clone();
        let conn = conn.clone();
        futures.push(async move { (name, get_chain_status(&conn).await) });
    }

    let mut secondaries = BTreeMap::new();
    while let Some(result) = futures.next().await {
        let (name, status) = result;
        secondaries.insert(name, status?);
    }
    Ok(AllChainStatuses {
        primary,
        secondaries,
    })
}

pub async fn status(connections: Connections) -> Result<()> {
    let statuses = get_all_chain_statuses(&connections).await?;
    println!("{:<30}  {}", "primary", statuses.primary);
    for (name, status) in statuses.secondaries.iter() {
        println!("{name:<30}  {status}");
    }
    Ok(())
}

fn assert_best_finalized_match(statuses: &AllChainStatuses) -> Result<()> {
    for (name, status) in statuses.secondaries.iter() {
        if status.finalized != statuses.primary.finalized {
            return Err(anyhow::anyhow!(
                "Not compatible finalized statuses of primary and {}: {} vs {}",
                name,
                status.finalized,
                statuses.primary.finalized
            ));
        }
    }
    Ok(())
}

fn assert_blocks_match(blocks: &AllBlocksAtNum, target_num: BlockNumber) -> Result<()> {
    for (name, block) in blocks.secondaries.iter() {
        if &blocks.primary != block {
            return Err(anyhow::anyhow!(
                "Not compatible blocks at target num {} blocks of primary and {}: {} vs {}",
                target_num,
                name,
                blocks.primary,
                block
            ));
        }
    }
    Ok(())
}

async fn pre_sequence_finalization_check(
    connections: &Connections,
    how_many: BlockNumber,
) -> Result<FinalizationPlan> {
    let statuses = get_all_chain_statuses(connections).await?;
    assert_best_finalized_match(&statuses)?;

    let latest_finalized_num = statuses.primary.finalized.num;
    let target_finalized_num = latest_finalized_num + how_many;

    let blocks_at_target = get_all_blocks_at_num(connections, target_finalized_num).await?;
    assert_blocks_match(&blocks_at_target, target_finalized_num)?;

    Ok(FinalizationPlan {
        finalized_base: statuses.primary.finalized,
        target: blocks_at_target.primary,
    })
}

async fn pre_single_finalization_check(
    connections: &Connections,
    target_num: BlockNumber,
) -> Result<HashNum> {
    let statuses = get_all_chain_statuses(connections).await?;
    assert_best_finalized_match(&statuses)?;

    if statuses.primary.finalized.num + 1 != target_num {
        return Err(anyhow::anyhow!(
            "Best finalized currently is {}, expected {}",
            statuses.primary.finalized.num,
            target_num - 1
        ));
    }

    let blocks_at_target = get_all_blocks_at_num(connections, target_num).await?;
    assert_blocks_match(&blocks_at_target, target_num)?;

    Ok(blocks_at_target.primary)
}

async fn try_finalize_single_block(
    connections: &Connections,
    key: &AlephKeyPair,
    num: BlockNumber,
) -> Result<()> {
    println!("Trying to finalize block number {num}");
    loop {
        match pre_single_finalization_check(connections, num).await {
            Ok(HashNum { num, hash }) => {
                println!(
                    "Sanity check passed. Sending finalization call for {} and {}",
                    num,
                    hex::encode(hash)
                );
                connections
                    .primary
                    .emergency_finalize(num, hash, *key)
                    .await?;
                println!("Finalization call for {num} sent.",);
                break;
            }
            Err(e) => {
                println!("Not all preconditions for finalizing {num} satisfied: {e:?}.");
                println!("We wait 1000ms and will try again. You can cancel by ctrl-c.\n");
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                continue;
            }
        }
    }
    Ok(())
}

pub async fn try_finalize(
    connections: Connections,
    seed_path: PathBuf,
    how_many: BlockNumber,
) -> Result<()> {
    let key = read_key_from_file(seed_path)?;
    let on_chain_pubkey = get_finalizer_pubkey(&connections.primary)
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to get the finalizer PK from chain."))?;
    if key.public().0 != on_chain_pubkey.0 .0 {
        return Err(anyhow::anyhow!(
            "On chain key does not match the key from file {} != {}",
            hex::encode(on_chain_pubkey.0 .0),
            hex::encode(key.public().0),
        ));
    }
    let plan = pre_sequence_finalization_check(&connections, how_many).await?;
    println!(
        "Sanity check passed. Will proceed to finalizing blocks from {} to {} (last hash {})",
        plan.finalized_base.num + 1,
        plan.target.num,
        hex::encode(plan.target.hash),
    );
    let proceed = Confirm::new()
        .with_prompt("Do you want to continue?")
        .default(true)
        .interact()?;
    if !proceed {
        return Err(anyhow::anyhow!("Cancelled by user."));
    }

    for num in (plan.finalized_base.num + 1)..=plan.target.num {
        try_finalize_single_block(&connections, &key, num).await?;
    }
    Ok(())
}

async fn get_finalizer_pubkey(connection: &Connection) -> Option<Public> {
    let addrs = api::storage().aleph().emergency_finalizer();
    connection.get_storage_entry_maybe(&addrs, None).await
}

fn read_key_from_file(seed_path: PathBuf) -> Result<AlephKeyPair> {
    println!("Reading the finalizer key from file {:?}", &seed_path);
    let suri = fs::read_to_string(seed_path)?;
    let key = aleph_keypair_from_string(suri.trim());
    println!("Read a pubkey {}\n", hex::encode(key.public().0));
    Ok(key)
}
